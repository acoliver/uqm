# Build inputs

Cargo builds the Rust game directly. `rust/build.rs` discovers the installed
SDL2 library through pkg-config. Dependencies are locked in `rust/Cargo.lock`;
use `--locked` for repeatable dependency resolution.

```sh
cargo build --release --locked --manifest-path rust/Cargo.toml --bin uqm
```

There is no first-party C archive or source manifest to generate.

## Scaler pixel format

The framebuffer uses RGBA8888. `fast_image_resize` has `only_u8x4` enabled so
only its four-channel byte paths are compiled. Preserve this format contract
when changing the scaler.
