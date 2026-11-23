# CI

[The CI workflow](../../.github/workflows/ci.yaml) runs on pull requests and pushes
to `main`. It pins Rust 1.98.0.

| Job | Runner | Checks |
| --- | --- | --- |
| `rust` | Ubuntu | Formatting, the Stage C raw-reader report, `xtask quality` against the base branch (no debt growth), `xtask clippy` (every workspace lint is `deny`, so any hit fails) and workspace tests, using `--no-default-features --features audio_heart,debug-process,native` |
| `supply` | Ubuntu | `xtask deny`: licences, advisories, sources and the native-build allowlist |
| `coverage` | Ubuntu | `xtask coverage` against the base branch; skipped on drafts |
| `game-build` | macOS ARM64, Linux x64/ARM64, Windows x64/ARM64 | Release build with `bundled-sdl`, version/content checks and the platform smoke example; the macOS leg also runs `xtask clippy`. Skipped on drafts |
| `parity-policy`, `parity-check`, `parity`, `parity-platform-compare` | Ubuntu, macOS | Replay parity against the committed goldens |
| `failure-scenarios` | macOS | Real-window `lander-destroyed-v1`, `out-of-fuel-v1`, `lost-battle-v1` and `super-melee-v1` runs |
| `release-tools` | Ubuntu | Python packaging tests and Homebrew helper shell syntax; skipped on drafts |

The parity and failure-scenario jobs are suspended until the idiomatic-Rust
work (epic #588) is finished: they run only when the repository variable
`UQM_PARITY` is `on`, and otherwise show as skipped. Set the variable under the
repository's Actions variables to run them again. This is a GitHub variable,
not the `UQM_PARITY` environment variable the replay harness sets for the
game.

Failed scenario runs upload diagnostic artifacts with a three-day retention.
See [scenario tooling](../testing/scenarios.md) for local runs. CI does not pin
scenario source digests.

[The Release workflow](../../.github/workflows/release.yml) runs for version tags
or an explicit existing tag. It builds and smoke-tests all release packages before
publishing them. See [releasing](../releasing.md) for tag, artifact and tap details.
