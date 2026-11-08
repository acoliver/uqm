# Cargo outputs belong to one checkout

Each checkout uses its own real `rust/target` directory. Do not symlink targets,
share `CARGO_TARGET_DIR` across worktrees, or copy compiled artifacts between
them. Downloaded registry and Git sources remain shared through `CARGO_HOME`.
Incremental compilation stays disabled.

Cargo outputs contain absolute source and fixture paths. Sharing them can retain
references to a removed worktree. Unhashed executables such as `uqm-xtask` also
overwrite each other when different worktrees build to the same target directory.
Running through Cargo alone cannot prevent that race.

Run `cargo xtask` and `cargo xtask-native` from the checkout's `rust/` directory.
Both aliases select its local `target`; xtask checks its compiled-in source root
against the current checkout before dispatch, rejects a symlinked `rust/target`,
and rejects a foreign `CARGO_TARGET_DIR` before launching child builds. This
checks checkout identity, not whether a manually invoked binary has the latest
commit. Use the Cargo aliases to rebuild changed sources. Internal observer
helpers run from retained evidence directories, do not build from a checkout,
and retain their controller's executable-digest validation rather than a
current-directory source check.

## Measured disk cost

A fresh target on the M4 Max, November 8, 2026, measured with `du -sk`:

| Cumulative workload | KiB | GiB |
| --- | ---: | ---: |
| `cargo build --locked --bin uqm` | 1,063,400 | 1.01 |
| Engine `link_queue` tests and non-native xtask tests | 1,991,812 | 1.90 |
| `cargo build --locked --release --bin uqm` | 2,551,236 | 2.43 |
| `cargo build --locked --release -p uqm-xtask` | 2,783,332 | 2.65 |

The final debug and release directories were 1,991,804 and 791,520 KiB.
Five equivalent targets use about 13.3 GiB. Full suites, coverage and feature
variants need additional space; these figures describe the commands above,
not an upper bound. ROOT's accumulated target was 55 GB and the drive had
189 GiB free before measurement. Check disk use at lane boundaries and delete
each lane's target when removing its worktree after merge.

`sccache` was absent. The existing `/Users/acoliver/.cargo` cache held 2.1 GB
of registry sources and 961 MB of Git dependencies. Separate compiled targets
with shared downloaded sources fit the measured budget without introducing
another cache service or a split dependency/workspace output scheme.

## Migrate active lanes without interrupting them

After your own build/native jobs finish, from your worktree root:

```sh
WT="$PWD"
if [ -L "$WT/rust/target" ]; then unlink "$WT/rust/target"; fi
mkdir -p "$WT/rust/target"
export CARGO_TARGET_DIR="$WT/rust/target"
cd "$WT/rust"
cargo build --locked -p uqm-xtask --no-default-features
cargo test --locked -p uqm-engine link_queue
# Rebuild your usual changed-crate checks and native controller before next push.
```

Do not delete or clean the old shared target while other lanes still use it.
After all lanes detach, the orchestrator must rebuild ROOT's previously shared
artifacts at idle. In particular, `cargo clean -p uqm-engine` followed by its
tests regenerates the engine build-script output containing fixture paths.
Other lanes must not perform this ROOT cleanup. Remove a merged worktree and
its own target together; never follow an old target symlink during cleanup.
