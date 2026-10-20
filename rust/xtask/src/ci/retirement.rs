//! Source proof for newly measured, existing C functions retired to Rust.
//!
//! This admits a measurement identity, not a new ABI. The ownership-link gate
//! still measures the actual archives and rejects missing or duplicate providers.

use std::collections::BTreeSet;
use std::ffi::{CStr, CString};
use std::path::{Path, PathBuf};
use std::ptr;

use clang_sys::*;
use quote::ToTokens;

#[derive(Debug)]
pub(super) struct CFunction {
    pub header: PathBuf,
    pub included: BTreeSet<PathBuf>,
    pub arguments: Vec<String>,
    pub result: String,
}

struct TranslationUnit {
    index: CXIndex,
    unit: CXTranslationUnit,
}

impl Drop for TranslationUnit {
    fn drop(&mut self) {
        // SAFETY: both handles belong to this instance, with the unit disposed first.
        unsafe {
            clang_disposeTranslationUnit(self.unit);
            clang_disposeIndex(self.index);
        }
    }
}

unsafe fn text(value: CXString) -> String {
    let pointer = clang_getCString(value);
    let result = if pointer.is_null() {
        String::new()
    } else {
        CStr::from_ptr(pointer).to_string_lossy().into_owned()
    };
    clang_disposeString(value);
    result
}

unsafe fn cursor_path(cursor: CXCursor) -> Option<PathBuf> {
    let mut file = ptr::null_mut();
    clang_getSpellingLocation(
        clang_getCursorLocation(cursor),
        &mut file,
        ptr::null_mut(),
        ptr::null_mut(),
        ptr::null_mut(),
    );
    (!file.is_null()).then(|| PathBuf::from(text(clang_getFileName(file))))
}

struct CScan<'a> {
    symbol: &'a str,
    definitions: Vec<CXCursor>,
    declarations: Vec<CXCursor>,
}

extern "C" fn visit(cursor: CXCursor, _: CXCursor, data: CXClientData) -> CXChildVisitResult {
    // SAFETY: clang invokes this synchronously with the live CScan passed below.
    unsafe {
        let scan = &mut *data.cast::<CScan<'_>>();
        if clang_getCursorKind(cursor) == CXCursor_FunctionDecl
            && text(clang_getCursorSpelling(cursor)) == scan.symbol
        {
            if clang_isCursorDefinition(cursor) != 0 {
                scan.definitions.push(cursor);
            } else {
                scan.declarations.push(cursor);
            }
        }
    }
    CXChildVisit_Continue
}

extern "C" fn inclusion(file: CXFile, _: *mut CXSourceLocation, _: u32, data: CXClientData) {
    // SAFETY: clang invokes this synchronously with the live path set below.
    unsafe {
        let included = &mut *data.cast::<BTreeSet<PathBuf>>();
        included.insert(PathBuf::from(text(clang_getFileName(file))));
    }
}

unsafe fn rust_type(ty: CXType) -> Result<String, String> {
    let primitives = [
        (CXType_Void, "()"),
        (CXType_Bool, "bool"),
        (CXType_UChar, "u8"),
        (CXType_SChar, "i8"),
        (CXType_UShort, "u16"),
        (CXType_Short, "i16"),
        (CXType_UInt, "u32"),
        (CXType_Int, "i32"),
        (CXType_ULongLong, "u64"),
        (CXType_LongLong, "i64"),
        (CXType_Float, "f32"),
        (CXType_Double, "f64"),
    ];
    if let Some((_, name)) = primitives.iter().find(|(kind, _)| *kind == ty.kind) {
        return Ok((*name).into());
    }
    Ok(match ty.kind {
        kind if kind == CXType_Elaborated => rust_type(clang_Type_getNamedType(ty))?,
        kind if kind == CXType_Typedef => {
            let name = text(clang_getTypeSpelling(ty));
            let _: syn::Ident =
                syn::parse_str(&name).map_err(|_| format!("unsupported C typedef {name:?}"))?;
            format!("c::{name}")
        }
        kind if kind == CXType_Pointer => {
            let pointee = clang_getPointeeType(ty);
            let qualifier = if clang_isConstQualifiedType(pointee) != 0 {
                "const"
            } else {
                "mut"
            };
            // bindgen names a const-qualified typedef without the C qualifier.
            let inner = if pointee.kind == CXType_Typedef {
                let name = text(clang_getCursorSpelling(clang_getTypeDeclaration(pointee)));
                let _: syn::Ident =
                    syn::parse_str(&name).map_err(|_| format!("unsupported C typedef {name:?}"))?;
                format!("c::{name}")
            } else {
                rust_type(pointee)?
            };
            format!("*{qualifier} {inner}")
        }
        _ => {
            return Err(format!(
                "unsupported C type {}",
                text(clang_getTypeSpelling(ty))
            ))
        }
    })
}

/// Parse the original Git blob as an unsaved translation unit. No base source
/// checkout or source copy is created. Callers must verify every included
/// repository header against both Git trees before using the result.
pub(super) fn c_function(
    root: &Path,
    source: &str,
    bytes: &[u8],
    symbol: &str,
    defines: &[String],
) -> Result<CFunction, String> {
    clang_sys::load()
        .map_err(|error| format!("cannot load libclang for retirement proof: {error}"))?;
    let path = root.join(source);
    let filename = CString::new(path.to_str().ok_or("C source path is not UTF-8")?)
        .map_err(|_| "C source path contains NUL")?;
    let contents = CString::new(bytes).map_err(|_| "C source contains NUL")?;
    let mut arguments = vec![
        CString::new("-std=gnu99").unwrap(),
        CString::new(format!("-I{}", root.join("sc2").display())).unwrap(),
        CString::new(format!("-I{}", root.join("sc2/src").display())).unwrap(),
    ];
    for define in defines {
        arguments.push(CString::new(define.as_str()).map_err(|_| "C define contains NUL")?);
    }
    let pointers = arguments.iter().map(|arg| arg.as_ptr()).collect::<Vec<_>>();
    let mut unsaved = CXUnsavedFile {
        Filename: filename.as_ptr(),
        Contents: contents.as_ptr(),
        Length: bytes.len() as _,
    };
    // SAFETY: strings and unsaved data remain alive through parsing. The RAII
    // owner disposes the resulting unit before its index on every exit path.
    unsafe {
        let index = clang_createIndex(0, 0);
        if index.is_null() {
            return Err("libclang could not allocate an index".into());
        }
        let unit = clang_parseTranslationUnit(
            index,
            filename.as_ptr(),
            pointers.as_ptr(),
            pointers.len() as _,
            &mut unsaved,
            1,
            CXTranslationUnit_None,
        );
        if unit.is_null() {
            clang_disposeIndex(index);
            return Err("libclang could not parse the original C source".into());
        }
        let owner = TranslationUnit { index, unit };
        for offset in 0..clang_getNumDiagnostics(owner.unit) {
            let diagnostic = clang_getDiagnostic(owner.unit, offset);
            let severity = clang_getDiagnosticSeverity(diagnostic);
            let message = text(clang_getDiagnosticSpelling(diagnostic));
            clang_disposeDiagnostic(diagnostic);
            if severity >= CXDiagnostic_Error {
                return Err(format!(
                    "original C source is not independently parseable: {message}"
                ));
            }
        }
        let mut scan = CScan {
            symbol,
            definitions: Vec::new(),
            declarations: Vec::new(),
        };
        clang_visitChildren(
            clang_getTranslationUnitCursor(owner.unit),
            visit,
            ptr::from_mut(&mut scan).cast(),
        );
        let [definition] = scan.definitions.as_slice() else {
            return Err(format!("expected one original C definition of {symbol}"));
        };
        if cursor_path(*definition).as_deref() != Some(path.as_path())
            || clang_getCursorLinkage(*definition) != CXLinkage_External
        {
            return Err(
                "original C definition is not an external function in the retired source".into(),
            );
        }
        let signature = clang_getCursorType(*definition);
        if clang_isFunctionTypeVariadic(signature) != 0
            || clang_getFunctionTypeCallingConv(signature) != CXCallingConv_C
        {
            return Err("unsupported C calling convention or variadic signature".into());
        }
        let mut headers = BTreeSet::new();
        for declaration in scan.declarations {
            let Some(header) = cursor_path(declaration) else {
                continue;
            };
            if header != path && clang_equalTypes(signature, clang_getCursorType(declaration)) != 0
            {
                headers.insert(header);
            }
        }
        let header = headers
            .into_iter()
            .next()
            .ok_or("original C function has no matching header declaration")?;
        let mut included = BTreeSet::new();
        clang_getInclusions(owner.unit, inclusion, ptr::from_mut(&mut included).cast());
        included.remove(&path);
        let count = clang_getNumArgTypes(signature);
        if count < 0 {
            return Err("original C function has no prototype".into());
        }
        let arguments = (0..count)
            .map(|n| rust_type(clang_getArgType(signature, n as _)))
            .collect::<Result<_, _>>()?;
        let result = rust_type(clang_getResultType(signature))?;
        Ok(CFunction {
            header,
            included,
            arguments,
            result,
        })
    }
}

fn normalized(tokens: impl ToTokens) -> String {
    tokens.into_token_stream().to_string()
}

fn imported_binding(tree: &syn::UseTree, prefix: &str) -> bool {
    match tree {
        syn::UseTree::Path(path) => {
            imported_binding(&path.tree, &format!("{prefix}{}::", path.ident))
        }
        syn::UseTree::Group(group) => group
            .items
            .iter()
            .any(|item| imported_binding(item, prefix)),
        syn::UseTree::Rename(rename) => {
            prefix == "crate::c_bindings::"
                && rename.ident == "global_state"
                && rename.rename == "c"
        }
        _ => false,
    }
}

/// The ABI is checked against actual exported Rust syntax and an unconditional
/// pair of compiler type probes against the bindgen declaration. A JSON claim
/// about a matching signature cannot satisfy this check.
pub(super) fn rust_function(source: &str, symbol: &str, c: &CFunction) -> Result<(), String> {
    let file =
        syn::parse_file(source).map_err(|error| format!("cannot parse Rust provider: {error}"))?;
    if !file.items.iter().any(|item| matches!(item, syn::Item::Use(item) if item.attrs.is_empty() && imported_binding(&item.tree, ""))) {
        return Err("Rust provider must import the generated global-state binding as c".into());
    }
    let functions = file.items.iter().filter_map(|item| {
        let syn::Item::Fn(function) = item else { return None };
        function.attrs.iter().any(|attr| {
            attr.path().is_ident("export_name") && matches!(&attr.meta, syn::Meta::NameValue(value)
                if matches!(&value.value, syn::Expr::Lit(value) if matches!(&value.lit, syn::Lit::Str(value) if value.value() == symbol)))
        }).then_some(function)
    }).collect::<Vec<_>>();
    let [function] = functions.as_slice() else {
        return Err("expected one explicit Rust export for the original symbol".into());
    };
    if function
        .attrs
        .iter()
        .any(|attr| !attr.path().is_ident("doc") && !attr.path().is_ident("export_name"))
        || function.sig.unsafety.is_none()
        || function.sig.asyncness.is_some()
        || function.sig.variadic.is_some()
        || !function.sig.generics.params.is_empty()
        || function.sig.generics.where_clause.is_some()
        || function
            .sig
            .abi
            .as_ref()
            .and_then(|abi| abi.name.as_ref())
            .map(syn::LitStr::value)
            .as_deref()
            != Some("C")
    {
        return Err("unsupported or conditional Rust export".into());
    }
    let expected: syn::TypeBareFn = syn::parse_str(&format!(
        "unsafe extern \"C\" fn({}){}",
        c.arguments.join(", "),
        if c.result == "()" {
            String::new()
        } else {
            format!(" -> {}", c.result)
        }
    ))
    .map_err(|error| format!("unsupported original ABI: {error}"))?;
    let actual = function
        .sig
        .inputs
        .iter()
        .map(|arg| match arg {
            syn::FnArg::Typed(arg) => Ok(normalized(&arg.ty)),
            _ => Err("Rust provider has a receiver".to_string()),
        })
        .collect::<Result<Vec<_>, _>>()?;
    let expected_inputs = expected
        .inputs
        .iter()
        .map(|arg| normalized(&arg.ty))
        .collect::<Vec<_>>();
    if actual != expected_inputs || normalized(&function.sig.output) != normalized(&expected.output)
    {
        return Err("Rust export changes the original C function signature".into());
    }
    let mut probes = BTreeSet::new();
    for item in &file.items {
        let syn::Item::Const(item) = item else {
            continue;
        };
        if !item.attrs.is_empty() || normalized(&item.ty) != normalized(&expected) {
            continue;
        }
        let syn::Expr::Path(path) = item.expr.as_ref() else {
            continue;
        };
        probes.insert(normalized(&path.path));
    }
    let c_probe: syn::Path =
        syn::parse_str(&format!("c::{symbol}")).map_err(|_| "invalid original C symbol")?;
    if !probes.contains(&normalized(c_probe)) || !probes.contains(&function.sig.ident.to_string()) {
        return Err(
            "missing unconditional C-declaration and Rust-function compiler ABI probes".into(),
        );
    }
    Ok(())
}

/// Resolve clang's parent-relative include spelling without authorizing a
/// symlink to change which file supplies the declaration.
pub(super) fn repository_header(root: &Path, path: &Path) -> Result<Option<PathBuf>, String> {
    use std::path::Component;

    let Ok(relative) = path.strip_prefix(root) else {
        return Ok(None);
    };
    let mut normalized = PathBuf::new();
    for component in relative.components() {
        match component {
            Component::Normal(name) => normalized.push(name),
            Component::CurDir => {}
            Component::ParentDir if normalized.pop() => {}
            _ => return Err("included header escapes the repository".into()),
        }
    }
    let resolved = path
        .canonicalize()
        .map_err(|error| format!("cannot resolve included header {}: {error}", path.display()))?;
    let expected = root
        .canonicalize()
        .map_err(|error| format!("cannot resolve repository root: {error}"))?
        .join(&normalized);
    if resolved != expected {
        return Err(format!(
            "included header resolves through a symlink: {}",
            path.display()
        ));
    }
    Ok(Some(normalized))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_spelling_normalizes_parent_components_but_rejects_symlinks_and_escape() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(root.path().join("include/nested")).unwrap();
        std::fs::write(root.path().join("include/api.h"), "void Existing(void);").unwrap();
        assert_eq!(
            repository_header(root.path(), &root.path().join("include/nested/../api.h")).unwrap(),
            Some(PathBuf::from("include/api.h"))
        );
        std::os::unix::fs::symlink("api.h", root.path().join("include/alias.h")).unwrap();
        assert!(repository_header(root.path(), &root.path().join("include/alias.h")).is_err());
        assert!(repository_header(root.path(), &root.path().join("../outside.h")).is_err());
    }

    const RUST: &str = r#"
use crate::c_bindings::{global_state as c};
const _: unsafe extern "C" fn(c::BYTE) = c::Existing;
const _: unsafe extern "C" fn(c::BYTE) = replacement;
#[export_name = "Existing"]
pub unsafe extern "C" fn replacement(value: c::BYTE) { let _ = value; }
"#;

    fn fixture() -> (tempfile::TempDir, CFunction) {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(
            root.path().join("api.h"),
            "typedef unsigned char BYTE;\nvoid Existing(BYTE);\n",
        )
        .unwrap();
        let c = c_function(
            root.path(),
            "retired.c",
            b"#include \"api.h\"\nvoid Existing(BYTE value) { (void)value; }\n",
            "Existing",
            &[],
        )
        .unwrap();
        (root, c)
    }

    #[test]
    fn actual_c_definition_header_and_paired_rust_probes_match() {
        let (root, c) = fixture();
        assert_eq!(c.header, root.path().join("api.h"));
        assert_eq!(c.arguments, ["c::BYTE"]);
        assert_eq!(c.result, "()");
        rust_function(RUST, "Existing", &c).unwrap();
    }

    #[test]
    fn invented_c_symbol_and_missing_header_are_rejected() {
        let (root, _) = fixture();
        assert!(c_function(
            root.path(),
            "retired.c",
            b"#include \"api.h\"\nvoid Existing(BYTE value) {}",
            "Invented",
            &[],
        )
        .is_err());
        assert!(c_function(
            root.path(),
            "retired.c",
            b"void Existing(unsigned char value) {}",
            "Existing",
            &[],
        )
        .is_err());
    }

    #[test]
    fn static_variadic_and_unparseable_c_are_rejected() {
        let root = tempfile::tempdir().unwrap();
        for (header, source) in [
            (
                "static void Existing(int);",
                "static void Existing(int x) {}",
            ),
            ("void Existing(int, ...);", "void Existing(int x, ...) {}"),
            (
                "void Existing(int);",
                "void Existing(int x) { syntax error }",
            ),
        ] {
            std::fs::write(root.path().join("api.h"), header).unwrap();
            let source = format!("#include \"api.h\"\n{source}");
            assert!(
                c_function(root.path(), "retired.c", source.as_bytes(), "Existing", &[]).is_err()
            );
        }
    }

    #[test]
    fn changed_rust_signature_missing_or_conditional_probes_and_fake_import_fail() {
        let (_root, c) = fixture();
        for source in [
            RUST.replace("value: c::BYTE", "value: u16"),
            RUST.replace(
                "const _: unsafe extern \"C\" fn(c::BYTE) = c::Existing;",
                "",
            ),
            RUST.replace(
                "const _: unsafe extern \"C\" fn(c::BYTE) = replacement;",
                "",
            ),
            RUST.replace(
                "const _: unsafe extern \"C\" fn(c::BYTE) = c::Existing;",
                "#[cfg(any())] const _: unsafe extern \"C\" fn(c::BYTE) = c::Existing;",
            ),
            RUST.replace("#[export_name", "#[cfg(any())] #[export_name"),
            RUST.replace("crate::c_bindings", "crate::fake_bindings"),
            RUST.replace("use crate", "#[cfg(any())] use crate"),
            format!("{RUST}\n{RUST}"),
        ] {
            assert!(
                rust_function(&source, "Existing", &c).is_err(),
                "accepted {source}"
            );
        }
    }
}
