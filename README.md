# The Ur-Quan Masters, Rust port

This repository runs Star Control II entirely in Rust. The original C game
archive and build system have been removed. SDL2 remains an external dependency.

## Build and run

Install Rust, pkg-config and SDL2 development libraries. On macOS:

```sh
brew install pkg-config sdl2
cargo build --release --locked --manifest-path rust/Cargo.toml --bin uqm
rust/target/release/uqm --contentdir sc2/content/packages
```

Game content is under `sc2/content/packages`. The upstream game's licensing
and attribution files remain under `sc2/`.

## Development

See [the development guide](dev-docs/README.md) for checks and
[the scenario guide](rust/scripts/SCENARIOS.md) for real-window playthroughs.
