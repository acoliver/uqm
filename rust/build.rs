fn main() {
    if std::env::var_os("CARGO_FEATURE_BUNDLED_SDL").is_none()
        && std::env::var("CARGO_CFG_TARGET_FAMILY").as_deref() == Ok("unix")
    {
        pkg_config::probe_library("sdl2")
            .expect("SDL2 development library must be available to pkg-config");
    }
}
