fn main() {
    let manifest = std::path::PathBuf::from(
        std::env::var_os("CARGO_MANIFEST_DIR")
            .unwrap_or_else(|| panic!("Cargo must supply CARGO_MANIFEST_DIR")),
    );
    let workspace = manifest
        .ancestors()
        .find(|dir| dir.join("Cargo.lock").is_file())
        .unwrap_or_else(|| panic!("engine manifest must be inside the Cargo workspace"));
    println!("cargo:rustc-env=UQM_RUST_ROOT={}", workspace.display());
}
