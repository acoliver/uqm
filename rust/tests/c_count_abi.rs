//! Rust declarations of C functions that use `COUNT` must use its real width.
//!
//! COUNT is the unsigned sixteen-bit engine count. Check its Rust schema and
//! every retained extern declaration so changes cannot widen the ABI.

use std::path::{Path, PathBuf};

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
    use uqm_rust::c_bindings::global_state::COUNT;
    let _: COUNT = u16::MAX;
    assert_eq!(std::mem::size_of::<COUNT>(), 2);
    assert_eq!(std::mem::align_of::<COUNT>(), 2);
}

#[test]
fn every_rust_declaration_of_get_frame_count_returns_a_sixteen_bit_count() {
    let declarations = rust_extern_declarations("GetFrameCount");
    assert!(declarations.len() >= 4, "{declarations:?}");
    for (file, declaration) in declarations {
        assert!(
            declaration.ends_with("-> u16") || declaration.ends_with("-> COUNT"),
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
            declaration.ends_with("-> u16") || declaration.ends_with("-> COUNT"),
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
            compact.contains(": u16) -> *mut") || compact.contains(": COUNT) -> STRING"),
            "{}: `{declaration}` must take a u16 index like `SetAbsStringTableIndex (STRING, COUNT)`",
            file.display()
        );
    }
}
