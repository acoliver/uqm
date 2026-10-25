# Build inputs

Cargo builds the Rust game directly. Dependencies are locked in
`rust/Cargo.lock`; use `--locked` for repeatable dependency resolution.
There is no first-party C archive or source manifest to generate.

From `rust/`, the release build is:

```sh
cargo build --release --locked --bin uqm --features bundled-sdl
```

`bundled-sdl` builds SDL2 from the crate's external source and statically links it.
Without that feature, `rust/build.rs` discovers installed SDL2 through pkg-config
on Unix targets. See [building](../../docs/building.md) for platform prerequisites
and [releasing](../releasing.md) for package checks.

## Scaler pixel format

The framebuffer uses RGBA8888. `fast_image_resize` has `only_u8x4` enabled so
only its four-channel byte paths are compiled. Preserve this format contract
when changing the scaler.
