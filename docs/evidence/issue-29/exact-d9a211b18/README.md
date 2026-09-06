# Process control and lifecycle evidence for issue #29

This directory durably retains the acceptance evidence for issue #29 at the post-merge commit where all seven required-scope items are present. CI artifacts expire; these do not. Every file is content-addressed in `CHECKSUMS.tsv`.

## Source and artifact identity

- Artifact git HEAD: `d9a211b1885f18f5c58b2c4019383cd3b663eead`
- Worktree: `dirty: false`
- Toolchain: `rustc 1.97.1 (8bab26f4f 2026-07-14)`
- Machine authority: `rust/ci/gates.json` sha256 `ca91023bd80b26e7525fc42ab7c2d0b0865a9873a7541014b9e78f521b18e02e`
- Acceptance script: `scripts/linked-playable-v1.json` sha256 `95aee4114081c72222f50df6ccf7b8f01bafdb954f613381a4be6457562ed934`, 2169 bytes
- Content package: `uqm-0.8.0-content.uqm` sha256 `77d75ac25e6fb755a33c4ba3b38a7b7bc41fcbc02896891b0cc9ac9214b72eef`, 11547353 bytes
- Executable under test: sha256 `7627f7b0f22217ef444a2a8fd53646f5798245957aa8fbf33963927834123b9b`

## Where each file came from

`window/` and `captures/` and `trace/native-acceptance-trace.jsonl` come from CI run 33979575720, job `S4 gates (macos-aarch64, non-merge)`, schema `uqm-native-window-acceptance-v2`, `passed: true`.

`manifest/production-artifacts.json` comes from `cargo run --locked --manifest-path rust/xtask/Cargo.toml -- prove` on `aarch64-apple-darwin`, validated offline by the same binary's `verify` with exit 0.

`trace/local-replay-trace.jsonl` and `trace/local-teardown-complete.json` come from an independent local replay of the same script over the same content.

## Two kinds of capture, kept separate

`window/` holds OS-level screenshots of the real game window. `captures/` holds presentation-pipeline captures of the presented frame. They are not interchangeable, and only the first shows what a person at the machine would have seen. Both are retained because they answer different questions.

The window captures are bound to one process, so they cannot be recycled from another run: pid 65154, start time `1788629055273092`, executable sha256 as above, nonce `40c62d301e515c46c20966b7a74ef19d55d29065fd0aea9b216a8b6179563019`, window id 24, OS bounds `80,52 1280x988`, client bounds `80,80 1280x960`.

The run happened in a real desktop session, not a headless one: `execution_identity` records `launchd_manager_name: "Aqua"` with real, effective and manager uid all 501.

## What the run reached

| measure | observed | required floor |
| --- | --- | --- |
| stable presentations | 837 | 120 |
| final committed presentation | 838 | 300 |
| battle frames | 336 | 300 |
| input events | 207 | not floored |

## Independent replay agreement

The local replay used byte-identical script and content, verified by the hashes above, and independently reached `battle_frames_verified:count=336`. Two machines, two sessions, the same count. It also recorded 813 presentations, 311 observed player inputs and 627 semantic assertions with no failure anywhere in 1976 trace records.

Its teardown receipt is `trace/local-teardown-complete.json`:

```json
{"schema":"uqm-teardown-v1","terminal":"success","game_status":0,"process_status":0,
 "runtime_finalized":true,"runtime_deactivated":true,"callbacks_quiescent":true,"trace_durable":true}
```

The CI child record in `window/native-acceptance.json` reports the same outcome from the other direction: `exit_code: 0`, no `SIGTERM` and no `SIGKILL` sent, output drained, initial process group empty, config root removed, materialized content removed. That is the containment and teardown work from this issue verifying its own cleanup.

## Logs

`harness-logs/` holds the harness and gameplay logs from the same CI run: the child's stdout and stderr, and the Rust bridge log. Build and test logs for the whole gate run stay in the CI artifact, which is far too large to retain here; this directory keeps the gameplay evidence that the acceptance itself produced, plus `local-replay-stderr.log` from the independent local replay.

## Replaying this

```
git checkout d9a211b1885f18f5c58b2c4019383cd3b663eead
cargo run --locked --manifest-path rust/xtask/Cargo.toml -- prove
cargo run --locked --manifest-path rust/xtask/Cargo.toml -- verify
```

`prove` builds twice and compares, so a matching `manifest/production-artifacts.json` means this commit is byte-reproducible on the same toolchain and platform.

The gameplay run itself needs a real desktop session. `xtask native-test` requires `UQM_CI_NATIVE_ACCEPTANCE_EVIDENCE_ROOT`, `UQM_CI_NATIVE_CONTENT_ROOT`, and matching non-root real, effective and Aqua manager uids. Without an Aqua session it stops with `manager_name="Background"`. A functional replay without OS-level screenshots is available by running the built binary directly:

```
cd sc2 && ../rust/target/release/uqm \
  --automation-script=../rust/scripts/linked-playable-v1.json \
  --automation-output=<dir>
```

## Scope note

This directory holds acceptance evidence only. The single-PR delivery contract stated in issue #29 was not met: the work landed across seven pull requests. That is recorded on the issue and is not altered by this evidence.
