//! Rust declarations of C functions that use `COUNT` must use its real width.
//!
//! `COUNT` is `typedef UWORD COUNT` and `UWORD` is `uint16` (`compiler.h`).
//! A Rust declaration with `c_int` for such a return value or parameter
//! compiles, links and works while the values stay small, but it does not
//! describe the C function: the caller reads 32 bits of a 16-bit return and
//! passes 32 bits for a 16-bit parameter. These tests read the C headers and
//! every Rust declaration, so a new declaration cannot drift from the header.

use std::path::{Path, PathBuf};

fn repository_file(relative: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("repository root")
        .join(relative);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("read {}: {error}", path.display()))
}

/// The C prototype text of `name`, whitespace-normalised, up to its `;`.
fn c_prototype(header: &str, name: &str) -> String {
    let start = header
        .find(&format!(" {name} ("))
        .unwrap_or_else(|| panic!("{name} is not declared"));
    let line_start = header[..start].rfind('\n').map_or(0, |index| index + 1);
    let end = header[start..].find(';').expect("prototype ends") + start;
    header[line_start..end]
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn rust_sources(root: &Path, found: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(root)
        .expect("read source directory")
        .flatten()
    {
        let path = entry.path();
        if path.is_dir() {
            rust_sources(&path, found);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            found.push(path);
        }
    }
}

/// Every Rust `fn name(...)` declaration ending in `;` (an extern item),
/// whitespace-normalised, as `(file, declaration)`.
fn rust_extern_declarations(name: &str) -> Vec<(PathBuf, String)> {
    let mut files = Vec::new();
    rust_sources(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src"),
        &mut files,
    );
    let needle = format!("fn {name}(");
    let mut found = Vec::new();
    for file in files {
        let text = std::fs::read_to_string(&file).expect("read source");
        let mut offset = 0;
        while let Some(index) = text[offset..].find(&needle) {
            let start = offset + index;
            offset = start + needle.len();
            let rest = &text[start..];
            let semicolon = rest.find(';').unwrap_or(usize::MAX);
            let brace = rest.find('{').unwrap_or(usize::MAX);
            if semicolon < brace {
                let declaration = rest[..semicolon]
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" ");
                found.push((file.clone(), declaration));
            }
        }
    }
    found
}

#[test]
fn count_is_a_sixteen_bit_unsigned_integer() {
    let compiler = repository_file("sc2/src/libs/compiler.h");
    assert!(compiler.contains("typedef UWORD             COUNT;"));
    assert!(compiler.contains("typedef uint16            UWORD;"));
}

#[test]
fn the_headers_declare_the_count_signatures_this_suite_checks() {
    let gfx = repository_file("sc2/src/libs/gfxlib.h");
    assert_eq!(
        c_prototype(&gfx, "GetFrameCount"),
        "extern COUNT GetFrameCount (FRAME Frame)"
    );
    assert_eq!(
        c_prototype(&gfx, "GetFrameIndex"),
        "extern COUNT GetFrameIndex (FRAME Frame)"
    );
    let strings = repository_file("sc2/src/libs/strlib.h");
    assert!(
        c_prototype(&strings, "SetAbsStringTableIndex")
            .starts_with("extern STRING SetAbsStringTableIndex (STRING String, COUNT"),
        "{}",
        c_prototype(&strings, "SetAbsStringTableIndex")
    );
}

#[test]
fn every_rust_declaration_of_get_frame_count_returns_a_sixteen_bit_count() {
    let declarations = rust_extern_declarations("GetFrameCount");
    assert!(declarations.len() >= 4, "{declarations:?}");
    for (file, declaration) in declarations {
        assert!(
            declaration.ends_with("-> u16"),
            "{}: `{declaration}` must return u16 like `COUNT GetFrameCount (FRAME)`",
            file.display()
        );
    }
}

#[test]
fn every_rust_declaration_of_get_frame_index_returns_a_sixteen_bit_count() {
    let declarations = rust_extern_declarations("GetFrameIndex");
    assert!(!declarations.is_empty());
    for (file, declaration) in declarations {
        assert!(
            declaration.ends_with("-> u16"),
            "{}: `{declaration}` must return u16",
            file.display()
        );
    }
}

#[test]
fn every_rust_declaration_of_set_abs_string_table_index_takes_a_sixteen_bit_count() {
    let declarations = rust_extern_declarations("SetAbsStringTableIndex");
    assert!(declarations.len() >= 6, "{declarations:?}");
    for (file, declaration) in declarations {
        let compact = declaration.replace(", )", ")");
        assert!(
            compact.contains(": u16) -> *mut"),
            "{}: `{declaration}` must take a u16 index like `SetAbsStringTableIndex (STRING, COUNT)`",
            file.display()
        );
    }
}
