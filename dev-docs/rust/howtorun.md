# Build, run and test

The production executable is `rust/src/main.rs`, backed by the `uqm_rust` library.
Follow [building from source](../../docs/building.md) for platform prerequisites,
bundled/static SDL2 builds and running with the repository's content.

From `rust/`, run the development checks:

```sh
cargo fmt --all --check
cargo run --locked --manifest-path xtask/Cargo.toml -- abi-check
cargo clippy --workspace --all-targets --locked --no-default-features --features audio_heart,debug-process -- -D warnings
cargo test --workspace --locked --no-default-features --features audio_heart,debug-process
```

These checks use installed SDL2. On macOS install `pkg-config` and `sdl2` through
Homebrew; on Linux install `pkg-config`, `libsdl2-dev` and ALSA development headers.
CI pins its toolchain in [ci.yaml](../../.github/workflows/ci.yaml).

The xtask crate provides `debug`, `release`, `test` and `native` commands.
For example, from the repository root:

```sh
cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native run \
  --scenario starbase-visit-v1 --artifacts tmp/starbase --content sc2/content/packages
```

The native runner takes the package directory as `--content`, unlike the game's
`--contentdir`, which takes the content root. Native scenarios require a real
macOS window. See [the scenario guide](../testing/scenarios.md) for scenario and
save handling. Remove run output when it is no longer needed.
