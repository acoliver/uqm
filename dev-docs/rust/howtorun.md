# Build, run and test

The production game is `rust/src/main.rs`, backed by the `uqm_rust` library.
All game code is Rust. SDL2 development libraries and pkg-config must be installed.
Linux also needs ALSA development headers for audio.

From the repository root:

```sh
cargo build --release --locked --manifest-path rust/Cargo.toml --bin uqm
rust/target/release/uqm --contentdir sc2/content/packages
```

From `rust/`, run the same checks used for development:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked --no-default-features --features audio_heart,debug-process -- -D warnings
cargo test --workspace --locked --no-default-features --features audio_heart,debug-process
```

CI pins its Rust version in `.github/workflows/ci.yaml`.

The xtask crate provides `debug`, `release`, `test` and `native` commands:

```sh
cargo run --locked --manifest-path rust/xtask/Cargo.toml -- native run \
  --scenario starbase-visit-v1 --artifacts tmp/starbase --content sc2/content/packages
```

Native scenarios require a real macOS window. See
[SCENARIOS.md](../../rust/scripts/SCENARIOS.md) for scenario and save handling.
