//! Compare foreign declarations with their Rust-owned symbol definitions.
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use quote::ToTokens;
use syn::visit::{self, Visit};
use syn::{Attribute, FnArg, ForeignItem, ReturnType, Signature, Type, UseTree};

#[derive(Default)]
struct Scope {
    parent: Option<usize>,
    path: String,
    aliases: BTreeMap<String, Type>,
    constants: BTreeMap<String, syn::Expr>,
    imports: BTreeMap<String, String>,
    globs: Vec<String>,
    types: BTreeSet<String>,
    static_macros: BTreeMap<String, bool>,
}

struct Function {
    symbol: String,
    signature: Signature,
    scope: usize,
    location: String,
    cfg: Vec<syn::Meta>,
}

struct Static {
    symbol: String,
    ty: Type,
    mutable: bool,
    scope: usize,
    location: String,
    cfg: Vec<syn::Meta>,
}

#[derive(Default)]
struct Catalog {
    scopes: Vec<Scope>,
    modules: BTreeMap<String, usize>,
    definitions: Vec<Function>,
    static_definitions: Vec<Static>,
    static_declarations: Vec<Static>,
    dependencies: BTreeMap<String, BTreeSet<String>>,
    declarations: Vec<Function>,
    files: BTreeSet<PathBuf>,
    unsupported_export_macros: BTreeSet<String>,
}

fn meta_named_value(meta: &syn::Meta, name: &str) -> Option<String> {
    if meta.path().is_ident("unsafe") {
        if let syn::Meta::List(list) = meta {
            let nested = syn::parse2::<syn::Meta>(list.tokens.clone()).ok()?;
            return meta_named_value(&nested, name);
        }
    }
    if meta.path().is_ident(name) {
        if let syn::Meta::NameValue(value) = meta {
            if let syn::Expr::Lit(literal) = &value.value {
                if let syn::Lit::Str(value) = &literal.lit {
                    return Some(value.value());
                }
            }
        }
    }
    None
}

fn named_attribute(attributes: &[Attribute], name: &str) -> Option<String> {
    attributes
        .iter()
        .find_map(|attribute| meta_named_value(&attribute.meta, name))
}

fn meta_has_name(meta: &syn::Meta, name: &str) -> bool {
    if meta.path().is_ident(name) {
        return true;
    }
    if meta.path().is_ident("unsafe") {
        if let syn::Meta::List(list) = meta {
            return syn::parse2::<syn::Meta>(list.tokens.clone())
                .is_ok_and(|nested| meta_has_name(&nested, name));
        }
    }
    false
}

fn has_attribute(attributes: &[Attribute], name: &str) -> bool {
    attributes
        .iter()
        .any(|attribute| meta_has_name(&attribute.meta, name))
}

fn exported(attributes: &[Attribute], signature: &Signature) -> Option<String> {
    named_attribute(attributes, "export_name")
        .or_else(|| has_attribute(attributes, "no_mangle").then(|| signature.ident.to_string()))
}

fn is_c(abi: &syn::Abi) -> bool {
    abi.name.as_ref().is_none_or(|name| name.value() == "C")
}

impl Catalog {
    fn scope(&mut self, parent: Option<usize>, path: String) -> usize {
        let index = self.scopes.len();
        self.modules.insert(path.clone(), index);
        self.scopes.push(Scope {
            parent,
            path,
            ..Scope::default()
        });
        index
    }

    fn load(
        &mut self,
        path: &Path,
        scope: usize,
        inherited_cfg: Vec<syn::Meta>,
        config: Option<&BuildCfg>,
    ) -> Result<(), String> {
        let path = path
            .canonicalize()
            .map_err(|error| format!("{}: {error}", path.display()))?;
        if !self.files.insert(path.clone()) {
            return Ok(());
        }
        let text = fs::read_to_string(&path).map_err(|error| error.to_string())?;
        let file =
            syn::parse_file(&text).map_err(|error| format!("{}: {error}", path.display()))?;
        let directory = path.parent().unwrap();
        let stem = path.file_stem().unwrap().to_string_lossy();
        let module_directory = if matches!(stem.as_ref(), "lib" | "main" | "mod") {
            directory.to_path_buf()
        } else {
            directory.join(stem.as_ref())
        };
        let mut cfg = inherited_cfg;
        cfg.extend(cfg_attributes(&file.attrs));
        let mut collector = Collector {
            catalog: self,
            scope,
            file: path,
            module_directory,
            errors: Vec::new(),
            cfg,
            config,
        };
        collector.visit_file(&file);
        if collector.errors.is_empty() {
            Ok(())
        } else {
            Err(collector.errors.join("\n"))
        }
    }

    fn absolute(&self, path: &str, scope: usize) -> String {
        let module = &self.scopes[scope].path;
        if let Some(rest) = path.strip_prefix("crate::") {
            format!("{}::{rest}", module.split("::").next().unwrap())
        } else if let Some(rest) = path.strip_prefix("self::") {
            format!("{module}::{rest}")
        } else if let Some(rest) = path.strip_prefix("super::") {
            let parent = self.scopes[scope].parent.unwrap_or(scope);
            self.absolute(&format!("self::{rest}"), parent)
        } else {
            let head = path.split("::").next().unwrap();
            if let Some(import) = self.scopes[scope].imports.get(head) {
                format!("{import}{}", &path[head.len()..])
            } else if self.modules.contains_key(&format!("{module}::{head}")) {
                format!("{module}::{path}")
            } else {
                path.to_string()
            }
        }
    }

    fn lookup(
        &self,
        path: &str,
        scope: usize,
        seen: &mut BTreeSet<(usize, String)>,
    ) -> Option<(usize, String)> {
        if !seen.insert((scope, path.to_string())) {
            return None;
        }
        let current = &self.scopes[scope];
        if let Some((prefix, name)) = path.rsplit_once("::") {
            let prefix = self.absolute(prefix, scope);
            if let Some(module) = self.modules.get(&prefix) {
                return self.lookup(name, *module, seen);
            }
            // Resolve an imported module alias, such as gfx::POINT.
            if let Some((head, tail)) = path.split_once("::") {
                if let Some(import) = current.imports.get(head) {
                    return self.lookup(&format!("{import}::{tail}"), scope, seen);
                }
            }
            if let Some(parent) = current.parent {
                return self.lookup(path, parent, seen);
            }
        } else {
            if current.aliases.contains_key(path)
                || current.types.contains(path)
                || current.constants.contains_key(path)
            {
                return Some((scope, path.to_string()));
            }
            if let Some(import) = current.imports.get(path) {
                if let Some(found) = self.lookup(import, scope, seen) {
                    return Some(found);
                }
            }
            for glob in &current.globs {
                if let Some(found) = self.lookup(&format!("{glob}::{path}"), scope, seen) {
                    return Some(found);
                }
            }
            if let Some(parent) = current.parent {
                return self.lookup(path, parent, seen);
            }
        }
        None
    }

    fn normalize(&self, ty: &Type, scope: usize, stack: &mut BTreeSet<(usize, String)>) -> String {
        match ty {
            Type::Path(path) if path.qself.is_none() => {
                let name = path.path.to_token_stream().to_string().replace(' ', "");
                if path.path.segments.len() == 1 {
                    let imported = self.scopes[scope]
                        .imports
                        .get(&path.path.segments[0].ident.to_string());
                    if matches!(imported.map(String::as_str), Some("libc::size_t")) {
                        return "usize".into();
                    }
                    if matches!(imported.map(String::as_str), Some("libc::ssize_t")) {
                        return "isize".into();
                    }
                    if matches!(imported.map(String::as_str), Some("libc::socklen_t")) {
                        return "u32".into();
                    }
                }
                if let Some((owner, ident)) = self.lookup(&name, scope, &mut BTreeSet::new()) {
                    let key = (owner, ident.clone());
                    if let Some(alias) = self.scopes[owner].aliases.get(&ident) {
                        if stack.insert(key.clone()) {
                            let result = self.normalize(alias, owner, stack);
                            stack.remove(&key);
                            return result;
                        }
                    }
                    return format!("{}::{ident}", self.scopes[owner].path);
                }
                let last = path.path.segments.last().unwrap();
                let libc_alias = path
                    .path
                    .segments
                    .first()
                    .is_some_and(|segment| segment.ident == "libc");
                if libc_alias {
                    match last.ident.to_string().as_str() {
                        "size_t" => return "usize".into(),
                        "ssize_t" => return "isize".into(),
                        "socklen_t" => return "u32".into(),
                        _ => {}
                    }
                }
                let primitive = match last.ident.to_string().as_str() {
                    "c_int" => Some("i32"),
                    "c_uint" => Some("u32"),
                    "c_short" => Some("i16"),
                    "c_ushort" => Some("u16"),
                    "c_schar" => Some("i8"),
                    "c_uchar" => Some("u8"),
                    "c_longlong" => Some("i64"),
                    "c_ulonglong" => Some("u64"),
                    "c_float" => Some("f32"),
                    "c_double" => Some("f64"),
                    "c_char" => Some(if std::any::type_name::<std::ffi::c_char>() == "i8" {
                        "i8"
                    } else {
                        "u8"
                    }),
                    "c_long" => Some(if cfg!(windows) || cfg!(target_pointer_width = "32") {
                        "i32"
                    } else {
                        "i64"
                    }),
                    "c_ulong" => Some(if cfg!(windows) || cfg!(target_pointer_width = "32") {
                        "u32"
                    } else {
                        "u64"
                    }),
                    "c_void" => Some("c_void"),
                    _ => None,
                };
                if let Some(primitive) = primitive {
                    return primitive.into();
                }
                if let syn::PathArguments::AngleBracketed(arguments) = &last.arguments {
                    let args = arguments
                        .args
                        .iter()
                        .map(|arg| match arg {
                            syn::GenericArgument::Type(ty) => self.normalize(ty, scope, stack),
                            _ => arg.to_token_stream().to_string(),
                        })
                        .collect::<Vec<_>>()
                        .join(",");
                    let mut components: Vec<_> = path
                        .path
                        .segments
                        .iter()
                        .map(|segment| segment.ident.to_string())
                        .collect();
                    if components
                        .last()
                        .is_some_and(|component| component == "Option")
                    {
                        components = vec!["Option".into()];
                    }
                    return format!("{}<{args}>", components.join("::"));
                }
                name
            }
            Type::Ptr(pointer) => format!(
                "*{} {}",
                if pointer.mutability.is_some() {
                    "mut"
                } else {
                    "const"
                },
                self.normalize(&pointer.elem, scope, stack)
            ),
            Type::Reference(reference) => format!(
                "&{}{}",
                if reference.mutability.is_some() {
                    "mut "
                } else {
                    ""
                },
                self.normalize(&reference.elem, scope, stack)
            ),
            Type::Tuple(tuple) => format!(
                "({})",
                tuple
                    .elems
                    .iter()
                    .map(|ty| self.normalize(ty, scope, stack))
                    .collect::<Vec<_>>()
                    .join(",")
            ),
            Type::Paren(paren) => self.normalize(&paren.elem, scope, stack),
            Type::Group(group) => self.normalize(&group.elem, scope, stack),
            Type::Array(array) => format!(
                "[{};{}]",
                self.normalize(&array.elem, scope, stack),
                self.array_length(&array.len, scope)
            ),
            Type::BareFn(function) => {
                let inputs = function
                    .inputs
                    .iter()
                    .map(|arg| self.normalize(&arg.ty, scope, stack))
                    .collect::<Vec<_>>()
                    .join(",");
                format!(
                    "{}{}fn({inputs}{}) -> {}",
                    if function.unsafety.is_some() {
                        "unsafe "
                    } else {
                        ""
                    },
                    function
                        .abi
                        .as_ref()
                        .map(|abi| format!("extern {} ", abi.to_token_stream()))
                        .unwrap_or_default(),
                    if function.variadic.is_some() {
                        ",..."
                    } else {
                        ""
                    },
                    self.output(&function.output, scope)
                )
            }
            _ => ty.to_token_stream().to_string(),
        }
    }

    fn const_integer(
        &self,
        expression: &syn::Expr,
        scope: usize,
        seen: &mut BTreeSet<(usize, String)>,
    ) -> Option<i128> {
        match expression {
            syn::Expr::Lit(literal) => match &literal.lit {
                syn::Lit::Int(value) => value.base10_parse().ok(),
                _ => None,
            },
            syn::Expr::Path(path) if path.qself.is_none() => {
                let name = path.path.to_token_stream().to_string().replace(' ', "");
                let (owner, ident) = self.lookup(&name, scope, &mut BTreeSet::new())?;
                let key = (owner, ident.clone());
                if !seen.insert(key.clone()) {
                    return None;
                }
                let result = self.scopes[owner]
                    .constants
                    .get(&ident)
                    .and_then(|value| self.const_integer(value, owner, seen));
                seen.remove(&key);
                result
            }
            syn::Expr::Paren(value) => self.const_integer(&value.expr, scope, seen),
            syn::Expr::Group(value) => self.const_integer(&value.expr, scope, seen),
            syn::Expr::Cast(value) => {
                let integer = self.const_integer(&value.expr, scope, seen)?;
                let ty = self.normalize(&value.ty, scope, &mut BTreeSet::new());
                let fits = match ty.as_str() {
                    "usize" => usize::try_from(integer).is_ok(),
                    "u8" => u8::try_from(integer).is_ok(),
                    "u16" => u16::try_from(integer).is_ok(),
                    "u32" => u32::try_from(integer).is_ok(),
                    "u64" => u64::try_from(integer).is_ok(),
                    "isize" => isize::try_from(integer).is_ok(),
                    "i8" => i8::try_from(integer).is_ok(),
                    "i16" => i16::try_from(integer).is_ok(),
                    "i32" => i32::try_from(integer).is_ok(),
                    "i64" => i64::try_from(integer).is_ok(),
                    "i128" => true,
                    _ => false,
                };
                fits.then_some(integer)
            }
            syn::Expr::Unary(value) if matches!(value.op, syn::UnOp::Neg(_)) => {
                self.const_integer(&value.expr, scope, seen)?.checked_neg()
            }
            syn::Expr::Binary(value) => {
                let left = self.const_integer(&value.left, scope, seen)?;
                let right = self.const_integer(&value.right, scope, seen)?;
                match value.op {
                    syn::BinOp::Add(_) => left.checked_add(right),
                    syn::BinOp::Sub(_) => left.checked_sub(right),
                    syn::BinOp::Mul(_) => left.checked_mul(right),
                    syn::BinOp::Div(_) => left.checked_div(right),
                    syn::BinOp::Rem(_) => left.checked_rem(right),
                    syn::BinOp::Shl(_) => left.checked_shl(right.try_into().ok()?),
                    syn::BinOp::Shr(_) => left.checked_shr(right.try_into().ok()?),
                    syn::BinOp::BitOr(_) => Some(left | right),
                    syn::BinOp::BitAnd(_) => Some(left & right),
                    syn::BinOp::BitXor(_) => Some(left ^ right),
                    _ => None,
                }
            }
            _ => None,
        }
    }

    fn array_length(&self, expression: &syn::Expr, scope: usize) -> String {
        match self.const_integer(expression, scope, &mut BTreeSet::new()) {
            Some(value) if value >= 0 => value.to_string(),
            _ => format!(
                "{}::{{{}}}",
                self.scopes[scope].path,
                expression.to_token_stream()
            ),
        }
    }

    fn output(&self, output: &ReturnType, scope: usize) -> String {
        match output {
            ReturnType::Default => "()".into(),
            ReturnType::Type(_, ty) => self.normalize(ty, scope, &mut BTreeSet::new()),
        }
    }

    fn signature(&self, function: &Function) -> String {
        let args = function
            .signature
            .inputs
            .iter()
            .map(|arg| match arg {
                FnArg::Typed(arg) => self.normalize(&arg.ty, function.scope, &mut BTreeSet::new()),
                FnArg::Receiver(_) => "self".into(),
            })
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            "({args}{}) -> {}",
            if function.signature.variadic.is_some() {
                ", ..."
            } else {
                ""
            },
            self.output(&function.signature.output, function.scope)
        )
    }
}

struct Collector<'a> {
    catalog: &'a mut Catalog,
    scope: usize,
    file: PathBuf,
    module_directory: PathBuf,
    errors: Vec<String>,
    cfg: Vec<syn::Meta>,
    config: Option<&'a BuildCfg>,
}

impl Collector<'_> {
    fn item_active(&self, attributes: &[Attribute]) -> Result<bool, String> {
        let Some(config) = self.config else {
            return Ok(true);
        };
        let predicates = cfg_attributes(attributes);
        self.cfg
            .iter()
            .chain(predicates.iter())
            .map(|predicate| cfg_enabled(predicate, config))
            .collect::<Result<Vec<_>, _>>()
            .map(|values| values.into_iter().all(|value| value))
    }

    fn enabled_item(&mut self, attributes: &[Attribute]) -> bool {
        match self.item_active(attributes) {
            Ok(enabled) => enabled,
            Err(error) => {
                self.errors.push(error);
                false
            }
        }
    }

    fn function(&self, symbol: String, signature: &Signature) -> Function {
        Function {
            symbol,
            signature: signature.clone(),
            scope: self.scope,
            location: format!(
                "{}:{}",
                self.file.display(),
                signature.ident.span().start().line
            ),
            cfg: self.cfg.clone(),
        }
    }

    fn import(&mut self, tree: &UseTree, prefix: &str) {
        match tree {
            UseTree::Path(path) => self.import(&path.tree, &format!("{prefix}{}::", path.ident)),
            UseTree::Name(name) => {
                let path = self
                    .catalog
                    .absolute(&format!("{prefix}{}", name.ident), self.scope);
                self.catalog.scopes[self.scope]
                    .imports
                    .insert(name.ident.to_string(), path);
            }
            UseTree::Rename(rename) => {
                let path = self
                    .catalog
                    .absolute(&format!("{prefix}{}", rename.ident), self.scope);
                self.catalog.scopes[self.scope]
                    .imports
                    .insert(rename.rename.to_string(), path);
            }
            UseTree::Glob(_) => {
                let path = self
                    .catalog
                    .absolute(prefix.trim_end_matches("::"), self.scope);
                self.catalog.scopes[self.scope].globs.push(path);
            }
            UseTree::Group(group) => {
                for tree in &group.items {
                    self.import(tree, prefix);
                }
            }
        }
    }
}

impl<'ast> Visit<'ast> for Collector<'_> {
    fn visit_item_mod(&mut self, module: &'ast syn::ItemMod) {
        if !self.enabled_item(&module.attrs) {
            return;
        }
        let parent_cfg = self.cfg.clone();
        self.cfg.extend(cfg_attributes(&module.attrs));
        let parent = self.scope;
        self.scope = self.catalog.scope(
            Some(parent),
            format!("{}::{}", self.catalog.scopes[parent].path, module.ident),
        );
        if let Some((_, items)) = &module.content {
            let directory = self.module_directory.clone();
            self.module_directory.push(module.ident.to_string());
            for item in items {
                self.visit_item(item);
            }
            self.module_directory = directory;
        } else {
            let directory = self.file.parent().unwrap();
            let explicit = named_attribute(&module.attrs, "path");
            let path = if let Some(path) = explicit {
                directory.join(path)
            } else {
                let base = &self.module_directory;
                let flat = base.join(format!("{}.rs", module.ident));
                if flat.exists() {
                    flat
                } else {
                    base.join(module.ident.to_string()).join("mod.rs")
                }
            };
            if let Err(error) = self
                .catalog
                .load(&path, self.scope, self.cfg.clone(), self.config)
            {
                self.errors.push(error);
            }
        }
        self.scope = parent;
        self.cfg = parent_cfg;
    }

    fn visit_block(&mut self, block: &'ast syn::Block) {
        let parent = self.scope;
        self.scope = self.catalog.scope(
            Some(parent),
            format!(
                "{}::block{}",
                self.catalog.scopes[parent].path,
                self.catalog.scopes.len()
            ),
        );
        visit::visit_block(self, block);
        self.scope = parent;
    }

    fn visit_item_use(&mut self, item: &'ast syn::ItemUse) {
        if self.enabled_item(&item.attrs) {
            self.import(&item.tree, "");
        }
    }
    fn visit_item_const(&mut self, item: &'ast syn::ItemConst) {
        if self.enabled_item(&item.attrs) {
            self.catalog.scopes[self.scope]
                .constants
                .insert(item.ident.to_string(), *item.expr.clone());
        }
    }

    fn visit_item_type(&mut self, item: &'ast syn::ItemType) {
        if self.enabled_item(&item.attrs) {
            self.catalog.scopes[self.scope]
                .aliases
                .insert(item.ident.to_string(), *item.ty.clone());
        }
    }
    fn visit_item_struct(&mut self, item: &'ast syn::ItemStruct) {
        self.catalog.scopes[self.scope]
            .types
            .insert(item.ident.to_string());
    }
    fn visit_item_enum(&mut self, item: &'ast syn::ItemEnum) {
        self.catalog.scopes[self.scope]
            .types
            .insert(item.ident.to_string());
    }
    fn visit_item_union(&mut self, item: &'ast syn::ItemUnion) {
        self.catalog.scopes[self.scope]
            .types
            .insert(item.ident.to_string());
    }

    fn visit_item_static(&mut self, item: &'ast syn::ItemStatic) {
        if !self.enabled_item(&item.attrs) {
            return;
        }
        let symbol = named_attribute(&item.attrs, "export_name")
            .or_else(|| has_attribute(&item.attrs, "no_mangle").then(|| item.ident.to_string()));
        if let Some(symbol) = symbol {
            let mut cfg = self.cfg.clone();
            cfg.extend(cfg_attributes(&item.attrs));
            self.catalog.static_definitions.push(Static {
                symbol,
                ty: *item.ty.clone(),
                mutable: matches!(item.mutability, syn::StaticMutability::Mut(_)),
                scope: self.scope,
                location: format!("{}:{}", self.file.display(), item.ident.span().start().line),
                cfg,
            });
        }
        visit::visit_item_static(self, item);
    }

    fn visit_item_fn(&mut self, item: &'ast syn::ItemFn) {
        let parent_cfg = self.cfg.clone();
        self.cfg.extend(cfg_attributes(&item.attrs));
        if item.sig.abi.as_ref().is_some_and(is_c) {
            if let Some(symbol) = exported(&item.attrs, &item.sig) {
                self.catalog
                    .definitions
                    .push(self.function(symbol, &item.sig));
            }
        }
        visit::visit_item_fn(self, item);
        self.cfg = parent_cfg;
    }

    fn visit_item_foreign_mod(&mut self, item: &'ast syn::ItemForeignMod) {
        let parent_cfg = self.cfg.clone();
        self.cfg.extend(cfg_attributes(&item.attrs));
        if is_c(&item.abi) {
            for item in &item.items {
                if let ForeignItem::Static(item) = item {
                    let mut cfg = self.cfg.clone();
                    cfg.extend(cfg_attributes(&item.attrs));
                    self.catalog.static_declarations.push(Static {
                        symbol: named_attribute(&item.attrs, "link_name")
                            .unwrap_or_else(|| item.ident.to_string()),
                        ty: *item.ty.clone(),
                        mutable: matches!(item.mutability, syn::StaticMutability::Mut(_)),
                        scope: self.scope,
                        location: format!(
                            "{}:{}",
                            self.file.display(),
                            item.ident.span().start().line
                        ),
                        cfg,
                    });
                }
                if let ForeignItem::Fn(function) = item {
                    let inherited = self.cfg.clone();
                    self.cfg.extend(cfg_attributes(&function.attrs));
                    let symbol = named_attribute(&function.attrs, "link_name")
                        .unwrap_or_else(|| function.sig.ident.to_string());
                    self.catalog
                        .declarations
                        .push(self.function(symbol, &function.sig));
                    self.cfg = inherited;
                }
            }
        }
        self.cfg = parent_cfg;
    }

    fn visit_item_macro(&mut self, item: &'ast syn::ItemMacro) {
        if let Some(ident) = &item.ident {
            let tokens = item.mac.tokens.to_string();
            let compact: String = tokens.chars().filter(|ch| !ch.is_whitespace()).collect();
            let typed = compact == "($name:ident:$ty:ty)=>{#[no_mangle]pubstaticmut$name:$ty=unsafe{mem::zeroed()};};"
                || compact == "($($name:ident:$ty:ty),*$(,)?)=>{$(#[no_mangle]pubstaticmut$name:$ty=unsafe{std::mem::zeroed()};)*};";
            let screen =
                compact == "($($name:ident),*)=>{$(#[no_mangle]pubstaticmut$name:i32=0;)*};";
            if typed || screen {
                self.catalog.scopes[self.scope]
                    .static_macros
                    .insert(ident.to_string(), typed);
                return;
            }
            if (tokens.contains("no_mangle") || tokens.contains("export_name"))
                && (tokens.contains("fn") || tokens.contains("static"))
            {
                self.catalog.unsupported_export_macros.insert(format!(
                    "{}:{} ({ident})",
                    self.file.display(),
                    item.mac.path.to_token_stream()
                ));
            }
        }
        if let Some(name) = item.mac.path.get_ident() {
            let mut scope = Some(self.scope);
            while let Some(owner) = scope {
                if let Some(typed) = self.catalog.scopes[owner]
                    .static_macros
                    .get(&name.to_string())
                {
                    let typed = *typed;
                    let entries = syn::parse::Parser::parse2(
                        |input: syn::parse::ParseStream<'_>| {
                            let mut entries = Vec::new();
                            while !input.is_empty() {
                                let ident: syn::Ident = input.parse()?;
                                let ty: Type = if typed {
                                    input.parse::<syn::Token![:]>()?;
                                    input.parse()?
                                } else {
                                    syn::parse_quote!(i32)
                                };
                                entries.push((ident, ty));
                                if input.is_empty() {
                                    break;
                                }
                                input.parse::<syn::Token![,]>()?;
                            }
                            Ok(entries)
                        },
                        item.mac.tokens.clone(),
                    );
                    match entries {
                        Ok(entries) => {
                            for (ident, ty) in entries {
                                let mut cfg = self.cfg.clone();
                                cfg.extend(cfg_attributes(&item.attrs));
                                self.catalog.static_definitions.push(Static {
                                    symbol: ident.to_string(),
                                    ty,
                                    mutable: true,
                                    scope: self.scope,
                                    location: format!(
                                        "{}:{}",
                                        self.file.display(),
                                        ident.span().start().line
                                    ),
                                    cfg,
                                });
                            }
                        }
                        Err(error) => self
                            .errors
                            .push(format!("{}: static macro: {error}", self.file.display())),
                    }
                    return;
                }
                scope = self.catalog.scopes[owner].parent;
            }
        }
        if item.mac.path.is_ident("include") {
            if let Ok(literal) = syn::parse2::<syn::LitStr>(item.mac.tokens.clone()) {
                let mut cfg = self.cfg.clone();
                cfg.extend(cfg_attributes(&item.attrs));
                if !self.enabled_item(&item.attrs) {
                    return;
                }
                if let Err(error) = self.catalog.load(
                    &self.file.parent().unwrap().join(literal.value()),
                    self.scope,
                    cfg,
                    self.config,
                ) {
                    self.errors.push(error);
                }
            }
        }
    }
}

fn cfg_attributes(attributes: &[Attribute]) -> Vec<syn::Meta> {
    let mut predicates = Vec::new();
    for attribute in attributes {
        if attribute.path().is_ident("cfg") {
            predicates.push(attribute.parse_args().expect("cfg predicate parses"));
        } else if attribute.path().is_ident("cfg_attr") {
            let (condition, effects) = attribute
                .parse_args_with(|input: syn::parse::ParseStream<'_>| {
                    let condition = input.parse::<syn::Meta>()?;
                    let mut effects = Vec::new();
                    while input.parse::<syn::Token![,]>().is_ok() {
                        effects.push(input.parse::<syn::Meta>()?);
                    }
                    Ok((condition, effects))
                })
                .expect("cfg_attr parses");
            for effect in effects
                .into_iter()
                .filter(|effect| effect.path().is_ident("cfg"))
            {
                if let syn::Meta::List(list) = effect {
                    let nested: syn::Meta = syn::parse2(list.tokens).expect("nested cfg parses");
                    predicates.push(
                        syn::parse2(quote::quote!(any(not(#condition), #nested)))
                            .expect("cfg_attr cfg parses"),
                    );
                }
            }
        }
    }
    predicates
}

#[derive(Clone)]
struct BuildCfg {
    debug_process: bool,
    values: BTreeSet<(String, String)>,
    flags: BTreeSet<String>,
}

impl BuildCfg {
    fn from_rustc(debug_process: bool) -> Result<Self, String> {
        let rustc = std::env::var("RUSTC").unwrap_or_else(|_| "rustc".into());
        let output = Command::new(&rustc)
            .arg("--print")
            .arg("cfg")
            .output()
            .map_err(|error| format!("run {rustc} --print cfg: {error}"))?;
        if !output.status.success() {
            return Err(format!(
                "{rustc} --print cfg failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }
        let mut cfg = Self {
            debug_process,
            values: BTreeSet::new(),
            flags: BTreeSet::new(),
        };
        for line in String::from_utf8(output.stdout)
            .map_err(|error| error.to_string())?
            .lines()
        {
            if let Some((key, value)) = line.split_once("=\"") {
                cfg.values
                    .insert((key.to_string(), value.trim_end_matches('\"').to_string()));
            } else {
                cfg.flags.insert(line.to_string());
            }
        }
        cfg.flags.remove("test");
        Ok(cfg)
    }

    #[cfg(test)]
    fn for_test(debug_process: bool, target: &[(&str, &str)]) -> Self {
        Self {
            debug_process,
            values: target
                .iter()
                .map(|(key, value)| (key.to_string(), value.to_string()))
                .collect(),
            flags: ["unix".to_string(), "debug_assertions".to_string()].into(),
        }
    }
}

fn cfg_enabled(meta: &syn::Meta, config: &BuildCfg) -> Result<bool, String> {
    match meta {
        syn::Meta::Path(path) => match path.get_ident().map(ToString::to_string).as_deref() {
            Some(key) if key == "feature" => {
                Err(format!("unsupported cfg input `{key}` in ABI source"))
            }
            Some(key)
                if [
                    "unix",
                    "windows",
                    "test",
                    "debug_assertions",
                    "proc_macro",
                    "target_thread_local",
                ]
                .contains(&key) =>
            {
                Ok(config.flags.contains(key))
            }
            Some(key) => Err(format!("unsupported cfg input `{key}` in ABI source")),
            None => Err(format!(
                "unsupported cfg predicate `{}`",
                meta.to_token_stream()
            )),
        },
        syn::Meta::NameValue(value) => {
            let key = value
                .path
                .get_ident()
                .map(ToString::to_string)
                .unwrap_or_default();
            let expected = match &value.value {
                syn::Expr::Lit(value) => match &value.lit {
                    syn::Lit::Str(v) => v.value(),
                    _ => {
                        return Err(format!(
                            "unsupported cfg value `{}`",
                            meta.to_token_stream()
                        ));
                    }
                },
                _ => {
                    return Err(format!(
                        "unsupported cfg value `{}`",
                        meta.to_token_stream()
                    ));
                }
            };
            let actual = match key.as_str() {
                "feature" => match expected.as_str() {
                    "audio_heart" => true,
                    "debug-process" => config.debug_process,
                    _ => {
                        return Err(format!(
                            "unsupported feature cfg `{expected}` in ABI source"
                        ));
                    }
                },
                "target_arch"
                | "target_os"
                | "target_env"
                | "target_vendor"
                | "target_family"
                | "target_endian"
                | "target_pointer_width"
                | "target_abi"
                | "target_has_atomic"
                | "target_has_atomic_load_store"
                | "target_has_atomic_equal_alignment"
                | "target_feature"
                | "target_thread_local" => config.values.contains(&(key.clone(), expected)),
                _ => return Err(format!("unsupported cfg input `{key}` in ABI source")),
            };
            Ok(actual)
        }
        syn::Meta::List(list) => {
            let items = list
                .parse_args_with(
                    syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated,
                )
                .map_err(|error| error.to_string())?;
            match list.path.get_ident().map(ToString::to_string).as_deref() {
                Some("all") => items
                    .iter()
                    .map(|item| cfg_enabled(item, config))
                    .collect::<Result<Vec<_>, _>>()
                    .map(|values| values.into_iter().all(|value| value)),
                Some("any") => items
                    .iter()
                    .map(|item| cfg_enabled(item, config))
                    .collect::<Result<Vec<_>, _>>()
                    .map(|values| values.into_iter().any(|value| value)),
                Some("not") if items.len() == 1 => Ok(!cfg_enabled(&items[0], config)?),
                _ => Err(format!(
                    "unsupported cfg expression `{}`",
                    meta.to_token_stream()
                )),
            }
        }
    }
}

fn active(function: &Function, config: &BuildCfg) -> Result<bool, String> {
    function
        .cfg
        .iter()
        .map(|cfg| cfg_enabled(cfg, config))
        .collect::<Result<Vec<_>, _>>()
        .map(|values| values.into_iter().all(|value| value))
}

fn collect_source_files(directory: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    for entry in
        fs::read_dir(directory).map_err(|error| format!("{}: {error}", directory.display()))?
    {
        let entry = entry.map_err(|error| error.to_string())?;
        let path = entry.path();
        if path.is_dir() {
            collect_source_files(&path, files)?;
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            files.push(path);
        }
    }
    Ok(())
}

fn ensure_scope(catalog: &mut Catalog, module_path: &[String]) -> usize {
    let mut parent = *catalog.modules.get("crate").unwrap();
    let mut path = String::from("crate");
    for component in module_path {
        path.push_str("::");
        path.push_str(component);
        if let Some(index) = catalog.modules.get(&path) {
            parent = *index;
        } else {
            let index = catalog.scopes.len();
            catalog.scopes.push(Scope {
                parent: Some(parent),
                path: path.clone(),
                ..Scope::default()
            });
            catalog.modules.insert(path.clone(), index);
            parent = index;
        }
    }
    parent
}

fn load_all_source_files(
    catalog: &mut Catalog,
    source_root: &Path,
    config: Option<&BuildCfg>,
) -> Result<(), String> {
    let mut files = Vec::new();
    collect_source_files(source_root, &mut files)?;
    files.sort();
    for file in files {
        let canonical = file.canonicalize().map_err(|error| error.to_string())?;
        if catalog.files.contains(&canonical) {
            continue;
        }
        let relative = file
            .strip_prefix(source_root)
            .map_err(|error| error.to_string())?;
        let mut modules: Vec<String> = relative
            .parent()
            .into_iter()
            .flat_map(Path::components)
            .map(|component| component.as_os_str().to_string_lossy().into_owned())
            .collect();
        let stem = file.file_stem().unwrap().to_string_lossy();
        if stem != "mod" && stem != "lib" && stem != "main" {
            modules.push(stem.into_owned());
        }
        let scope = ensure_scope(catalog, &modules);
        catalog.load(&file, scope, Vec::new(), config)?;
    }
    Ok(())
}

fn external_provider(symbol: &str) -> Option<(&'static str, &'static str)> {
    let entry = match symbol {
        "SDL_CreateRGBSurface" | "SDL_FreeSurface" | "SDL_GetVersion" | "SDL_Init" => (
            "SDL2",
            "SDL2 headers: SDL_surface.h, SDL_version.h, SDL.h; SDL2 public API",
        ),
        "atexit" | "printf" | "sprintf" | "qsort" | "strcmp" | "strcpy" | "vasprintf" => (
            "libSystem.B.dylib",
            "macOS SDK stdlib.h, stdio.h, string.h; Darwin libc API",
        ),
        "send" | "sendmsg" | "sendto" => (
            "libSystem.B.dylib",
            "macOS SDK sys/socket.h; Darwin socket API",
        ),
        "CFArrayGetCount"
        | "CFArrayGetValueAtIndex"
        | "CFBooleanGetValue"
        | "CFDictionaryGetValue"
        | "CFNumberGetValue"
        | "CFRelease" => (
            "CoreFoundation.framework",
            "macOS SDK CoreFoundation.framework/Headers; CoreFoundation public API",
        ),
        "CGRectMakeWithDictionaryRepresentation"
        | "CGWindowListCopyWindowInfo"
        | "CGMainDisplayID"
        | "CGDisplayBounds" => (
            "CoreGraphics.framework",
            "macOS SDK CoreGraphics.framework/Headers; CoreGraphics public API",
        ),
        "sel_registerName" | "objc_msgSend" | "objc_msgSend_stret" => (
            "libobjc.A.dylib",
            "macOS SDK objc/runtime.h and objc/message.h; Objective-C runtime API",
        ),
        "proc_pidpath" => (
            "libproc.dylib",
            "macOS SDK usr/include/libproc.h; libproc public API",
        ),
        "pthread_get_qos_class_np" | "pthread_set_qos_class_self_np" => (
            "libSystem.B.dylib",
            "macOS SDK usr/include/pthread/qos.h; Darwin pthread QoS API",
        ),
        "sandbox_init" | "sandbox_free_error" => (
            "libsandbox.1.dylib",
            "macOS SDK usr/include/sandbox.h; Apple sandbox API",
        ),
        _ => return None,
    };
    Some(entry)
}

fn external_static_provider(symbol: &str) -> Option<(&'static str, &'static str)> {
    match symbol {
        "__stderrp" | "__stdoutp" => Some((
            "libSystem.B.dylib",
            "macOS SDK stdio.h: FILE *__stderrp, *__stdoutp",
        )),
        "stderr" | "stdout" => Some(("libc", "POSIX stdio.h: FILE *stderr, *stdout")),
        "kCGWindowOwnerPID"
        | "kCGWindowNumber"
        | "kCGWindowBounds"
        | "kCGWindowIsOnscreen"
        | "kCGWindowLayer" => Some((
            "CoreGraphics.framework",
            "macOS SDK CGWindow.h: const CFStringRef window information keys",
        )),
        _ => None,
    }
}

fn dependency_allowed(
    dependencies: &BTreeMap<String, BTreeSet<String>>,
    caller: &str,
    owner: &str,
) -> bool {
    let mut pending = vec![caller];
    let mut visited = BTreeSet::new();
    while let Some(name) = pending.pop() {
        if name == owner {
            return true;
        }
        if visited.insert(name) {
            if let Some(edges) = dependencies.get(name) {
                pending.extend(edges.iter().map(String::as_str));
            }
        }
    }
    false
}

impl Catalog {
    fn direction_allowed(&self, declaration: usize, definition: usize) -> bool {
        let owner = |scope: usize| self.scopes[scope].path.split("::").next().unwrap();
        dependency_allowed(&self.dependencies, owner(declaration), owner(definition))
    }

    fn static_signature(&self, item: &Static) -> String {
        format!(
            "{}{}",
            if item.mutable { "mut " } else { "" },
            self.normalize(&item.ty, item.scope, &mut BTreeSet::new())
        )
    }
}

#[derive(serde::Deserialize)]
struct WorkspaceMetadata {
    packages: Vec<WorkspacePackage>,
    workspace_members: BTreeSet<String>,
}

#[derive(serde::Deserialize)]
struct WorkspacePackage {
    id: String,
    name: String,
    targets: Vec<WorkspaceTarget>,
    dependencies: Vec<WorkspaceDependency>,
}

#[derive(serde::Deserialize)]
struct WorkspaceTarget {
    name: String,
    kind: Vec<String>,
    src_path: PathBuf,
    #[serde(default, rename = "required-features")]
    required_features: Vec<String>,
}

#[derive(serde::Deserialize)]
struct WorkspaceDependency {
    name: String,
}

fn workspace_metadata(root: &Path) -> Result<WorkspaceMetadata, String> {
    let output = Command::new("cargo")
        .args([
            "metadata",
            "--locked",
            "--no-deps",
            "--format-version",
            "1",
            "--manifest-path",
        ])
        .arg(root.join("rust/Cargo.toml"))
        .output()
        .map_err(|error| format!("cargo metadata: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "cargo metadata: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    serde_json::from_slice(&output.stdout).map_err(|error| format!("cargo metadata: {error}"))
}

fn load_workspace(root: &Path, config: &BuildCfg) -> Result<Catalog, String> {
    let metadata = workspace_metadata(root)?;
    let packages: Vec<_> = metadata
        .packages
        .iter()
        .filter(|package| metadata.workspace_members.contains(&package.id))
        .collect();
    let crate_name = |package: &WorkspacePackage| {
        package
            .targets
            .iter()
            .find(|target| {
                target
                    .kind
                    .iter()
                    .any(|kind| matches!(kind.as_str(), "lib" | "rlib" | "cdylib" | "staticlib"))
            })
            .map_or_else(
                || package.name.replace('-', "_"),
                |target| target.name.replace('-', "_"),
            )
    };
    let mut catalog = Catalog::default();
    for package in &packages {
        let name = crate_name(package);
        let dependencies = package
            .dependencies
            .iter()
            .filter_map(|dependency| {
                packages
                    .iter()
                    .find(|candidate| candidate.name == dependency.name)
                    .map(|candidate| crate_name(candidate))
            })
            .collect();
        catalog.dependencies.insert(name.clone(), dependencies);
        // Load libraries before binaries so shared modules retain their library identity.
        let mut targets: Vec<_> = package
            .targets
            .iter()
            .filter(|target| {
                target.kind.iter().any(|kind| {
                    matches!(kind.as_str(), "lib" | "rlib" | "cdylib" | "staticlib")
                        || kind == "bin"
                })
            })
            .collect();
        targets.sort_by_key(|target| {
            !target
                .kind
                .iter()
                .any(|kind| matches!(kind.as_str(), "lib" | "rlib" | "cdylib" | "staticlib"))
        });
        for target in targets {
            if target.required_features.iter().any(|feature| {
                feature != "audio_heart" && !(feature == "debug-process" && config.debug_process)
            }) {
                continue;
            }
            let scope = catalog
                .modules
                .get(&name)
                .copied()
                .unwrap_or_else(|| catalog.scope(None, name.clone()));
            catalog.load(&target.src_path, scope, Vec::new(), Some(config))?;
        }
    }
    Ok(catalog)
}

pub(crate) fn run(root: &Path) -> Result<(), String> {
    let mut catalog = Catalog::default();
    let scope = catalog.scope(None, "crate".into());
    catalog.load(&root.join("rust/src/lib.rs"), scope, Vec::new(), None)?;
    load_all_source_files(&mut catalog, &root.join("rust/src"), None)?;
    if std::env::var_os("UQM_ABI_INVENTORY").is_some() {
        for definition in &catalog.definitions {
            println!(
                "OWNER {} {}::{}",
                definition.symbol,
                catalog.scopes[definition.scope].path,
                definition.signature.ident
            );
        }
    }
    let production_cfg = BuildCfg::from_rustc(false)?;
    let debug_cfg = BuildCfg {
        debug_process: true,
        ..production_cfg.clone()
    };
    let configurations = [production_cfg, debug_cfg];
    let mut mismatch_sites = BTreeSet::new();
    let mut matched = 0;
    let mut missing: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut external = BTreeMap::<String, (&str, &str)>::new();
    let mut batches: BTreeMap<String, usize> = BTreeMap::new();
    for config in configurations {
        let catalog = load_workspace(root, &config)?;
        let mut configuration_mismatches = 0;
        for declaration in &catalog.declarations {
            if !active(declaration, &config)? {
                continue;
            }
            let definitions: Vec<_> = catalog
                .definitions
                .iter()
                .filter(|definition| definition.symbol == declaration.symbol)
                .map(|definition| active(definition, &config).map(|enabled| (definition, enabled)))
                .collect::<Result<Vec<_>, _>>()?
                .into_iter()
                .filter_map(|(definition, enabled)| enabled.then_some(definition))
                .collect();
            if definitions.is_empty() {
                if let Some(provider) = external_provider(&declaration.symbol) {
                    external.insert(declaration.symbol.clone(), provider);
                } else {
                    missing
                        .entry(declaration.symbol.clone())
                        .or_default()
                        .insert(declaration.location.clone());
                }
                continue;
            }
            matched += 1;
            *batches
                .entry(declaration.location.rsplit_once(':').unwrap().0.to_string())
                .or_default() += 1;
            let signature = catalog.signature(declaration);
            let definition_signatures: BTreeSet<_> = definitions
                .iter()
                .map(|definition| catalog.signature(definition))
                .collect();
            if definition_signatures.len() > 1
                || !definition_signatures.contains(&signature)
                || definitions.iter().any(|definition| {
                    !catalog.direction_allowed(declaration.scope, definition.scope)
                })
            {
                configuration_mismatches += 1;
                mismatch_sites.insert((declaration.symbol.clone(), declaration.location.clone()));
                println!(
                    "MISMATCH {} at {} ({})\n  declared {signature}",
                    declaration.symbol,
                    declaration.location,
                    if config.debug_process {
                        "audio_heart+debug-process"
                    } else {
                        "audio_heart"
                    }
                );
                for definition in definitions {
                    println!(
                        "  defined  {} at {}",
                        catalog.signature(definition),
                        definition.location
                    );
                }
            }
        }
        for declaration in &catalog.static_declarations {
            if !declaration
                .cfg
                .iter()
                .map(|cfg| cfg_enabled(cfg, &config))
                .collect::<Result<Vec<_>, _>>()?
                .into_iter()
                .all(|enabled| enabled)
            {
                continue;
            }
            let definitions: Vec<_> = catalog
                .static_definitions
                .iter()
                .filter(|definition| definition.symbol == declaration.symbol)
                .filter_map(|definition| {
                    match definition
                        .cfg
                        .iter()
                        .map(|cfg| cfg_enabled(cfg, &config))
                        .collect::<Result<Vec<_>, _>>()
                    {
                        Ok(enabled) if enabled.iter().all(|enabled| *enabled) => {
                            Some(Ok(definition))
                        }
                        Ok(_) => None,
                        Err(error) => Some(Err(error)),
                    }
                })
                .collect::<Result<_, _>>()?;
            if definitions.is_empty() {
                if let Some(provider) = external_static_provider(&declaration.symbol) {
                    external.insert(declaration.symbol.clone(), provider);
                } else {
                    missing
                        .entry(declaration.symbol.clone())
                        .or_default()
                        .insert(declaration.location.clone());
                }
                continue;
            }
            let signature = catalog.static_signature(declaration);
            if definitions.iter().any(|definition| {
                catalog.static_signature(definition) != signature
                    || !catalog.direction_allowed(declaration.scope, definition.scope)
            }) {
                configuration_mismatches += 1;
                mismatch_sites.insert((declaration.symbol.clone(), declaration.location.clone()));
                println!(
                    "MISMATCH static {} at {}\n  declared {signature}",
                    declaration.symbol, declaration.location
                );
                for definition in definitions {
                    println!(
                        "  defined {} at {}",
                        catalog.static_signature(definition),
                        definition.location
                    );
                }
            }
        }
        println!(
            "CONFIG {}: {configuration_mismatches} signature mismatches",
            if config.debug_process {
                "audio_heart+debug-process"
            } else {
                "audio_heart"
            }
        );
    }
    for (symbol, (provider, reference)) in &external {
        println!("EXTERNAL {symbol}: provider={provider}; {reference}");
    }
    for (symbol, locations) in &missing {
        println!(
            "UNRESOLVED {symbol}: {}",
            locations.iter().cloned().collect::<Vec<_>>().join(", ")
        );
    }
    let mut batches: Vec<_> = batches.into_iter().collect();
    batches.sort_by_key(|(_, count)| std::cmp::Reverse(*count));
    for (path, count) in batches {
        println!("INTERNAL {count:4} {path}");
    }
    let unique_mismatches = mismatch_sites.len();
    println!(
        "ABI check: {} definitions, {} declarations, {matched} internal declarations, {unique_mismatches} unique signature mismatches, {} unresolved symbols",
        catalog.definitions.len(),
        catalog.declarations.len(),
        missing.len()
    );
    for macro_location in &catalog.unsupported_export_macros {
        println!("UNSUPPORTED EXPORTED MACRO: {macro_location}");
    }
    let blockers = [
        (unique_mismatches > 0)
            .then(|| format!("{unique_mismatches} Rust-to-Rust foreign signature mismatches")),
        (!missing.is_empty()).then(|| {
            format!(
                "{} declarations have no Rust definition or verified external provider",
                missing.len()
            )
        }),
        (!catalog.unsupported_export_macros.is_empty()).then(|| {
            format!(
                "{} macros generate exports but are not expanded by the checker",
                catalog.unsupported_export_macros.len()
            )
        }),
    ];
    let blockers: Vec<_> = blockers.into_iter().flatten().collect();
    if blockers.is_empty() {
        Ok(())
    } else {
        Err(blockers.join("; "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn catalog(source: &str) -> Catalog {
        catalog_with_config(source, None)
    }

    fn catalog_with_config(source: &str, config: Option<&BuildCfg>) -> Catalog {
        let mut catalog = Catalog::default();
        let scope = catalog.scope(None, "crate".into());
        Collector {
            catalog: &mut catalog,
            scope,
            file: PathBuf::from("fixture.rs"),
            module_directory: PathBuf::from("."),
            errors: Vec::new(),
            cfg: Vec::new(),
            config,
        }
        .visit_file(&syn::parse_file(source).unwrap());
        catalog
    }

    #[test]
    fn array_lengths_resolve_scoped_constants_imports_casts_and_arithmetic() {
        let collected = catalog(
            r#"
            mod sizes { pub const PLAYERS: u32 = 2; pub const EXTRA: usize = 3; }
            use sizes::PLAYERS as COUNT;
            extern "C" { static mut table: [u32; (COUNT as usize + sizes::EXTRA) * 2]; }
            #[no_mangle] static mut table: [u32; 10usize] = [0; 10];
        "#,
        );
        assert_eq!(
            collected.static_signature(&collected.static_declarations[0]),
            collected.static_signature(&collected.static_definitions[0])
        );
        let collected = catalog(
            r#"
            mod a { const N: usize = 2; extern "C" { static mut table: [u8; N]; } }
            mod b { const N: usize = 3; #[no_mangle] static mut table: [u8; N] = [0; N]; }
        "#,
        );
        assert_ne!(
            collected.static_signature(&collected.static_declarations[0]),
            collected.static_signature(&collected.static_definitions[0])
        );
    }

    #[test]
    fn foreign_statics_compare_nominal_types_mutability_and_array_lengths() {
        for (declaration, definition, matches) in [
            (
                "static mut table: [u32; 150]",
                "static mut table: [u32; 330] = [0; 330]",
                false,
            ),
            ("static table: u32", "static mut table: u32 = 0", false),
            (
                "static mut table: *mut u32",
                "static mut table: *const u32 = core::ptr::null()",
                false,
            ),
            ("static mut table: Alias", "static mut table: u32 = 0", true),
        ] {
            let catalog = catalog(&format!(
                "type Alias = u32; extern \"C\" {{ {declaration}; }} #[no_mangle] {definition};"
            ));
            assert_eq!(
                catalog.static_signature(&catalog.static_declarations[0])
                    == catalog.static_signature(&catalog.static_definitions[0]),
                matches
            );
        }
    }

    #[test]
    fn foreign_symbols_cannot_link_upward_or_across_unrelated_crates() {
        let dependencies = BTreeMap::from([
            ("app".into(), BTreeSet::from(["engine".into()])),
            ("engine".into(), BTreeSet::from(["platform".into()])),
        ]);
        assert!(dependency_allowed(&dependencies, "app", "app"));
        assert!(dependency_allowed(&dependencies, "app", "engine"));
        assert!(dependency_allowed(&dependencies, "app", "platform"));
        assert!(!dependency_allowed(&dependencies, "engine", "app"));
        assert!(!dependency_allowed(&dependencies, "platform", "engine"));
        assert!(!dependency_allowed(&dependencies, "other", "app"));
    }

    #[test]
    fn resolves_aliases_and_imports_in_their_own_scope() {
        let catalog = catalog(
            r#"
            mod api { pub type Handle = *mut u64; }
            use api::Handle as Object;

            extern "C" { fn operation(object: Object) -> libc::c_int; }
            mod implementation {
                use crate::api::*;
                #[no_mangle] pub extern "C" fn operation(object: Handle) -> i32 { 0 }
            }
        "#,
        );
        assert_eq!(
            catalog.signature(&catalog.declarations[0]),
            catalog.signature(&catalog.definitions[0])
        );
    }

    #[test]
    fn cfg_conditional_aliases_are_resolved_for_each_build_mode() {
        let source = r#"
            #[cfg(feature = "debug-process")] type Choice = u32;
            #[cfg(not(feature = "debug-process"))] type Choice = u64;
            extern "C" { fn configured(value: u32); }
            #[no_mangle] extern "C" fn configured(value: Choice) {}
        "#;
        for (debug_process, matches) in [(false, false), (true, true)] {
            let config = BuildCfg::for_test(
                debug_process,
                &[("target_os", "linux"), ("target_arch", "x86_64")],
            );
            let catalog = catalog_with_config(source, Some(&config));
            assert_eq!(
                catalog.signature(&catalog.declarations[0])
                    == catalog.signature(&catalog.definitions[0]),
                matches,
                "debug_process={debug_process}"
            );
        }
    }

    #[test]
    fn cfg_conditional_import_aliases_are_resolved_for_each_build_mode() {
        let source = r#"
            mod narrow { pub type Number = u32; }
            mod wide { pub type Number = u64; }
            #[cfg(feature = "debug-process")] use narrow::Number as Choice;
            #[cfg(not(feature = "debug-process"))] use wide::Number as Choice;
            extern "C" { fn configured(value: u32); }
            #[no_mangle] extern "C" fn configured(value: Choice) {}
        "#;
        for (debug_process, matches) in [(false, false), (true, true)] {
            let config = BuildCfg::for_test(
                debug_process,
                &[("target_os", "linux"), ("target_arch", "x86_64")],
            );
            let catalog = catalog_with_config(source, Some(&config));
            assert_eq!(
                catalog.signature(&catalog.declarations[0])
                    == catalog.signature(&catalog.definitions[0]),
                matches,
                "debug_process={debug_process}"
            );
        }
    }

    #[test]
    fn normalizes_libc_aliases_and_keeps_nominal_generic_paths_distinct() {
        let catalog = catalog(
            r#"
            use libc::{size_t, ssize_t, socklen_t};
            extern "C" { fn values(a: size_t, b: ssize_t, c: socklen_t); }
        "#,
        );
        let scope = catalog.declarations[0].scope;
        assert_eq!(
            catalog.normalize(&syn::parse_quote!(size_t), scope, &mut BTreeSet::new()),
            "usize"
        );
        assert_eq!(
            catalog.normalize(&syn::parse_quote!(ssize_t), scope, &mut BTreeSet::new()),
            "isize"
        );
        assert_eq!(
            catalog.normalize(&syn::parse_quote!(socklen_t), scope, &mut BTreeSet::new()),
            "u32"
        );
        let std_mutex = catalog.normalize(
            &syn::parse_quote!(std::sync::Mutex<u32>),
            scope,
            &mut BTreeSet::new(),
        );
        let parking_mutex = catalog.normalize(
            &syn::parse_quote!(parking_lot::Mutex<u32>),
            scope,
            &mut BTreeSet::new(),
        );
        assert_ne!(std_mutex, parking_mutex);
    }

    #[test]
    fn normalizes_set_input_callback_and_splash_screen_callback_signatures() {
        let catalog = catalog(
            r#"
            extern "C" {
                fn SetInputCallback(callback: Option<unsafe extern "C" fn()>)
                    -> Option<unsafe extern "C" fn()>;
                fn SplashScreen(callback: Option<unsafe extern "C" fn(TimeOut: u32)>);
            }
            #[no_mangle]
            extern "C" fn SetInputCallback(callback: ::std::option::Option<unsafe extern "C" fn()>)
                -> ::std::option::Option<unsafe extern "C" fn()> { callback }
            #[no_mangle]
            extern "C" fn SplashScreen(callback: ::std::option::Option<unsafe extern "C" fn(timeout: u32)>) {}
        "#,
        );
        for (declaration, definition) in catalog.declarations.iter().zip(&catalog.definitions) {
            assert_eq!(
                catalog.signature(declaration),
                catalog.signature(definition),
                "{}",
                declaration.symbol
            );
        }
    }

    #[test]
    fn detects_pointer_width_count_return_and_by_value_errors() {
        for (declaration, definition) in [
            ("fn f(x: u32)", "fn f(x: *mut u8)"),
            ("fn f(x: u32, y: u32)", "fn f(x: u32)"),
            ("fn f() -> bool", "fn f()"),
            ("fn f(x: *const Value)", "fn f(x: Value)"),
            ("fn f(x: i32)", "fn f(x: u32)"),
            (
                "fn f(x: Option<unsafe extern \"C\" fn(u32)>)",
                "fn f(x: Option<unsafe extern \"C\" fn(u64)>)",
            ),
        ] {
            let catalog = catalog(&format!(
                "struct Value {{ x: u32 }} extern \"C\" {{ {declaration}; }} #[no_mangle] extern \"C\" {definition} {{}}"
            ));
            assert_ne!(
                catalog.signature(&catalog.declarations[0]),
                catalog.signature(&catalog.definitions[0]),
                "{declaration} vs {definition}"
            );
        }
    }

    #[test]
    fn respects_link_and_export_names_and_nested_foreign_blocks() {
        let catalog = catalog(
            r#"
            #[unsafe(export_name = "symbol")] extern "C" fn provider() {}
            fn caller() { unsafe extern "C" { #[link_name = "symbol"] fn renamed(); } }
        "#,
        );
        assert_eq!(catalog.declarations[0].symbol, "symbol");
        assert_eq!(catalog.definitions[0].symbol, "symbol");
    }

    #[test]
    fn aliases_are_scope_local_and_cycles_do_not_loop() {
        let catalog = catalog(
            r#"
            mod first {
                type Handle = u32;
                type Cycle = Cycle;
                extern "C" { fn takes_handle(value: Handle); }
            }
            mod second {
                type Handle = *mut u32;
                #[no_mangle] extern "C" fn takes_handle(value: Handle) {}
            }
            "#,
        );
        assert_ne!(
            catalog.signature(&catalog.declarations[0]),
            catalog.signature(&catalog.definitions[0]),
            "the same alias name in separate modules must resolve locally"
        );
        let cycle = catalog.normalize(
            &syn::parse_quote!(Cycle),
            catalog.modules["crate::first"],
            &mut BTreeSet::new(),
        );
        assert_eq!(cycle, "crate::first::Cycle");
    }

    #[test]
    fn records_macro_generated_exports_as_a_hard_blocker() {
        let macro_catalog = catalog(
            r#"
            macro_rules! make_export {
                () => { #[no_mangle] extern "C" fn generated(value: u32) {} };
            }
            make_export!();
            "#,
        );
        assert_eq!(macro_catalog.definitions.len(), 0);
        assert_eq!(macro_catalog.unsupported_export_macros.len(), 1);
        let static_only = catalog(
            r#"
            macro_rules! export_table {
                () => { #[no_mangle] static PACKET_DATA: u32 = 0; };
            }
            export_table!();
        "#,
        );
        assert_eq!(static_only.unsupported_export_macros.len(), 1);
    }

    #[test]
    fn mutually_exclusive_feature_signatures_are_checked_per_production_configuration() {
        let catalog = catalog(
            r#"
            #[cfg(feature = "debug-process")] extern "C" { fn configured(value: u32); }
            #[cfg(not(feature = "debug-process"))] extern "C" { fn configured(); }
            #[cfg(feature = "debug-process")] #[no_mangle] extern "C" fn configured(value: u32) {}
            #[cfg(not(feature = "debug-process"))] #[no_mangle] extern "C" fn configured() {}
            #[cfg(test)] #[no_mangle] extern "C" fn configured(value: *mut u8) {}
        "#,
        );
        for config in [
            BuildCfg::for_test(
                false,
                &[
                    ("target_os", "linux"),
                    ("target_arch", "x86_64"),
                    ("target_vendor", "unknown"),
                    ("target_env", "gnu"),
                    ("target_family", "unix"),
                ],
            ),
            BuildCfg::for_test(
                true,
                &[
                    ("target_os", "linux"),
                    ("target_arch", "x86_64"),
                    ("target_vendor", "unknown"),
                    ("target_env", "gnu"),
                    ("target_family", "unix"),
                ],
            ),
        ] {
            let declaration = catalog
                .declarations
                .iter()
                .find(|item| active(item, &config).unwrap())
                .unwrap();
            let definitions: Vec<_> = catalog
                .definitions
                .iter()
                .filter(|item| active(item, &config).unwrap())
                .collect();
            assert_eq!(definitions.len(), 1);
            assert_eq!(
                catalog.signature(declaration),
                catalog.signature(definitions[0])
            );
        }
    }

    #[test]
    fn a_test_stub_cannot_mask_a_production_mismatch() {
        let catalog = catalog(
            r#"
            extern "C" { fn configured(value: u32); }
            #[no_mangle] extern "C" fn configured(value: *mut u8) {}
            #[cfg(test)] #[no_mangle] extern "C" fn configured(value: u32) {}
        "#,
        );
        let config =
            BuildCfg::for_test(false, &[("target_os", "linux"), ("target_arch", "x86_64")]);
        let declaration = &catalog.declarations[0];
        let definitions: Vec<_> = catalog
            .definitions
            .iter()
            .filter(|item| active(item, &config).unwrap())
            .collect();
        assert_eq!(definitions.len(), 1);
        assert_ne!(
            catalog.signature(declaration),
            catalog.signature(definitions[0])
        );
    }

    #[test]
    fn external_provider_is_an_exact_separate_classification() {
        assert_eq!(external_provider("SDL_Init").unwrap().0, "SDL2");
        assert!(external_provider("SDL_FutureInternalHelper").is_none());
        assert_eq!(
            external_static_provider("__stderrp").unwrap().0,
            "libSystem.B.dylib"
        );
        assert_eq!(
            external_static_provider("kCGWindowBounds").unwrap().0,
            "CoreGraphics.framework"
        );
        assert!(external_static_provider("kCGWindowUnknown").is_none());
        assert!(external_static_provider("GlobData").is_none());
        assert!(external_static_provider("SDL_Init").is_none());
        let collected = catalog(r#"extern "C" { fn missing_internal(); }"#);
        assert!(external_provider(&collected.declarations[0].symbol).is_none());
    }

    #[test]
    fn inherited_and_cfg_attr_predicates_apply_to_definitions_and_declarations() {
        let catalog = catalog(
            r#"
            #[cfg(feature = "debug-process")]
            mod nested {
                extern "C" { fn nested_symbol(value: u32); }
                #[no_mangle] extern "C" fn nested_symbol(value: u32) {}
            }
            #[cfg_attr(feature = "debug-process", cfg(target_os = "macos"))]
            #[no_mangle] extern "C" fn conditional_symbol() {}
        "#,
        );
        let linux = BuildCfg::for_test(false, &[("target_os", "linux"), ("target_arch", "x86_64")]);
        let mac = BuildCfg::for_test(true, &[("target_os", "macos"), ("target_arch", "aarch64")]);
        assert!(!active(&catalog.declarations[0], &linux).unwrap());
        assert!(active(&catalog.declarations[0], &mac).unwrap());
        assert!(active(&catalog.definitions[1], &mac).unwrap());
        assert!(cfg_enabled(&syn::parse_quote!(unknown_target_key = "value"), &linux).is_err());
    }

    #[test]
    fn out_of_line_modules_and_includes_keep_inherited_cfg() {
        let temporary = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tmp");
        fs::create_dir_all(&temporary).unwrap();
        let root = tempfile::tempdir_in(temporary).unwrap();
        let lib = root.path().join("lib.rs");
        let test_module = root.path().join("test_support.rs");
        let included = root.path().join("included.rs");
        fs::write(
            &lib,
            r##"#[cfg(test)] mod test_support; #[cfg(feature = "debug-process")] include!("included.rs");"##,
        )
        .unwrap();
        fs::write(
            &test_module,
            r##"#[no_mangle] extern "C" fn test_only() {}"##,
        )
        .unwrap();
        fs::write(
            &included,
            r##"#[no_mangle] extern "C" fn included_only() {}"##,
        )
        .unwrap();
        let mut catalog = Catalog::default();
        let scope = catalog.scope(None, "crate".into());
        catalog.load(&lib, scope, Vec::new(), None).unwrap();
        load_all_source_files(&mut catalog, root.path(), None).unwrap();

        let linux = BuildCfg::for_test(false, &[("target_os", "linux"), ("target_arch", "x86_64")]);
        let with_debug =
            BuildCfg::for_test(true, &[("target_os", "linux"), ("target_arch", "x86_64")]);
        let test_only = catalog
            .definitions
            .iter()
            .find(|item| item.symbol == "test_only")
            .unwrap();
        let included_only = catalog
            .definitions
            .iter()
            .find(|item| item.symbol == "included_only")
            .unwrap();
        assert!(!active(test_only, &linux).unwrap());
        assert!(!active(test_only, &with_debug).unwrap());
        assert!(!active(included_only, &linux).unwrap());
        assert!(active(included_only, &with_debug).unwrap());
    }
}
