# Building the transitional C archive

`cargo run --locked --manifest-path rust/xtask/Cargo.toml -- debug` (or
`release`) builds the game. With the `linked_c_archive` feature, `rust/build.rs`
compiles every C source named in `rust/build/c-sources.txt` with the defines,
include roots and flags declared at the top of `build.rs`, archives the objects
into `libuqm_c.a`, and links it into `uqm` (`-force_load` on macOS,
`+whole-archive` on Linux). SDL2, libpng, liblzma and bzip2 come from
`pkg-config`. Linking is strict: an unresolved symbol fails the build.

To delete a ported C file: delete the file, remove its line from
`rust/build/c-sources.txt`, and provide the symbols it exported from Rust.
`cargo run --locked --manifest-path rust/xtask/Cargo.toml -- test` runs the
linked fixtures that need the archive.

## Scaler pixel format contract: only_u8x4

UQM's scaler contract is RGBA U8x4 (RGBA8888). The `fast_image_resize` dependency is configured with the `only_u8x4` feature, which restricts the compiled scaler to exclusively support the U8x4 (four u8 channels) pixel format. This is required because:

1. UQM's framebuffer is always RGBA8888; no other pixel format is ever used.
2. The `only_u8x4` feature eliminates dead code paths for U8, U16x2, U16x3, and F32 pixel types, reducing binary size and improving reproducibility.
3. Any future change to this feature must document the new pixel format contract here.
