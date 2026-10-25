# CI

[The CI workflow](../../.github/workflows/ci.yaml) runs on pull requests and pushes
to `main`. It pins Rust 1.98.0.

| Job | Runner | Checks |
| --- | --- | --- |
| `rust` | Ubuntu | Formatting, `xtask abi-check`, strict Clippy and workspace tests, using `--no-default-features --features audio_heart,debug-process` |
| `c-ratchet` | Ubuntu | Tracked C/C++/Objective-C file count must not exceed the base branch count |
| `game-build` | macOS ARM64, Linux x64/ARM64, Windows x64/ARM64 | Release build with `bundled-sdl`, version/content checks and native networking smoke |
| `failure-scenarios` | macOS | Real-window `lander-destroyed-v1`, `out-of-fuel-v1`, `lost-battle-v1` and `super-melee-v1` runs |
| `release-tools` | Ubuntu | Python packaging tests and Homebrew helper shell syntax |

Failed scenario runs upload diagnostic artifacts with a three-day retention.
See [scenario tooling](../testing/scenarios.md) for local runs. CI does not pin
scenario source digests.

[The Release workflow](../../.github/workflows/release.yml) runs for version tags
or an explicit existing tag. It builds and smoke-tests all release packages before
publishing them. See [releasing](../releasing.md) for tag, artifact and tap details.
