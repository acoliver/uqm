fn main() {
    // Build the minimal C wrapper for Phase 0
    cc::Build::new()
        .include("../sc2/src")
        .warnings(true)
        .file("../sc2/src/mem_wrapper.c")
        .cpp(false)
        .compile("uqm_core");

    println!("cargo:rerun-if-changed=../sc2/src/mem_wrapper.c");

    // Create minimal manual FFI bindings for Phase 0
    // In later phases, we'll use bindgen to auto-generate these
    let bindings_content = r#"
/* FFI Bindings for Phase 0 */

// Re-export libc types for convenience
pub use libc::{c_int, c_char, c_void};

extern "C" {
    /// Entry point that Rust calls to start the C code
    /// This is defined in mem_wrapper.c
    pub fn c_entry_point(argc: c_int, argv: *mut *mut c_char) -> c_int;
}
"#;

    let out_path = std::path::PathBuf::from("src");
    std::fs::write(out_path.join("c_bindings.rs"), bindings_content)
        .expect("Failed to write c_bindings.rs");
}
