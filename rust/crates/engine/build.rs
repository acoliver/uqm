fn main() {
    let manifest = std::path::PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let workspace = manifest
        .ancestors()
        .find(|dir| dir.join("Cargo.lock").is_file())
        .expect("workspace Cargo.lock");
    println!("cargo:rustc-env=UQM_RUST_ROOT={}", workspace.display());
    if std::env::var_os("CARGO_FEATURE_BUNDLED_SDL").is_none()
        && std::env::var("CARGO_CFG_TARGET_FAMILY").as_deref() == Ok("unix")
    {
        pkg_config::probe_library("sdl2")
            .expect("SDL2 development library must be available to pkg-config");
    }
}
