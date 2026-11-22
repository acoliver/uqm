# Build, run and test

The production executable is `rust/src/main.rs`, backed by the `uqm_rust` library.
Follow [building from source](../../docs/building.md) for platform prerequisites,
bundled/static SDL2 builds and running with the repository's content.

From `rust/`, run the development checks:

```sh
cargo fmt --all --check
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

### Optional content in native checks

A scenario can select pinned optional content in a sibling `.native.toml` file:

```toml
addons = ["3dovideo"]
```

The native runner copies only the selected addon's pinned files from
`sc2/content/addons/<name>` in the source checkout into its isolated content root.
Each file must match the length and SHA-256 in
`rust/tests/fixtures/native/3dovideo-pins.json`. Missing files, symlinks, unknown
addons and digest mismatches fail before the game launches. Extra host files are
not copied. The game receives `--addon=3dovideo`; it never receives a host addon
path. The retained bundle's offline validation checks the addon selection and
pinned bytes too. With no sidecar, requests and game arguments remain unchanged.

The ship-spin fixture is native-only, outside the replay corpus. From the
repository root:

```sh
cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native run \
  --scenarios rust/tests/fixtures/native/ship-spin-3do-v1.json \
  --artifacts tmp/ship-spin --content sc2/content/packages
```

It opens Arilou in the melee ship picker and captures OS checkpoints before,
during and after its 3DO video. The shared iTerm launcher takes the fixture path
relative to `rust/`: `tests/fixtures/native/ship-spin-3do-v1.json`.
