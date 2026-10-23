fn main() {
    pkg_config::probe_library("sdl2")
        .expect("SDL2 development library must be available to pkg-config");
}
