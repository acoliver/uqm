# CI

`.github/workflows/ci.yaml` is the only workflow. It runs on pull requests and on
pushes to `main`.

| Job | Runner | What it checks |
|---|---|---|
| `rust` | ubuntu-latest | `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --locked`, with `audio_heart,debug-process` enabled. |
| `c-ratchet` | ubuntu-latest | The number of tracked C, C++ and Objective-C files (`.c .h .cc .cpp .hpp .m .mm`) in the pull request must not exceed the base branch. The count is printed on every run. |
| `game-build` | macos-latest | `cargo build --release --locked --manifest-path rust/Cargo.toml --bin uqm`. |

The checks do not pin digests of repository files. Scenario scripts under
`rust/scripts` can be edited without updating any hash. The local scenario runner
(`cargo xtask native ...`, see `rust/scripts/SCENARIOS.md`) drives playthroughs on
a developer machine and is not part of CI.
