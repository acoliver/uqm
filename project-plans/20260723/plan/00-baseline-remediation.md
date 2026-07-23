# Phase 00: Preserve and Remediate the Strict Baseline

Phase ID: `PLAN-20260723-RUNTIME-AUTOMATION.P00`

## Prerequisites and scope

First worker phase. Create all tracker TODOs. Modify only files required to make the existing repository pass mandatory Rust gates; do not implement automation. Preserve current edits, especially graphics/input/options/communication work. No reset/restore/checkout, lint weakening, blanket allow, skipped target, or broad unreviewed semantic rewrite.

## Requirements

- `REQ-QUALITY-001`: all strict commands must pass.
- `REQ-QUALITY-002`: remediation precedes feature work and has no waiver.
- `REQ-QUALITY-003`: preserve user work and separate formatting from behavior.

## Preservation evidence

Create a unique evidence directory outside the repository (for example with `mktemp -d /tmp/uqm-runtime-baseline.XXXXXX`) and record its path in handoff. Save:

```bash
git rev-parse HEAD
git status --short
git diff --binary
git ls-files --others --exclude-standard
cargo --version
rustc --version
cargo clippy --version
cc --version
```

Also save SHA-256 hashes of every modified tracked source before editing. Never auto-apply the snapshot.

## Baseline RED

From `rust/`, capture complete stdout/stderr and exits independently:

```bash
cargo check --workspace --all-features
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

Planning-time evidence confirms failures, not waivers: strict Clippy exits 101 (2,198 captured `error` headers; Cargo's lib-test summary says 2,035 previous errors), and Cargo test exits 101 because `input_integration_tests` requests `-luqm_rust` but the archive is not discoverable. Re-run and preserve the complete fresh inventory; do not target only these counts.

## Mandatory executable preflight probes

Before feature work, create checked scripts/evidence that execute—not merely inspect—these assumptions: current `build.vars`/`config_unix.h` capabilities; required lock-free atomics; monotonic `Instant`; Unix datagram path length, mode/permissions, nonce roundtrip and peer credentials where supported; create-new/rename-no-replace and directory sync; PID/start/executable identity; `SDL_VIDEODRIVER=dummy` plus hidden 320x240 software renderer; linked SDL ABI/accessors and a deterministic real `SDL_MUSTLOCK` surface; actual initialized-child `menu.down.N` `VCONTROL_KEY` query feasibility; and C/Rust tool/archive availability.

Also create the minimal declared Cargo linked-harness probe required by execution-contract §8. It must prove deterministic `libuqm_c.a`, search/archive/external-library order, Rust export retention, rerun dependencies, and production member extraction for at least source-grounded `DoInput`/`AnyButtonPress`, `DoConfirmExit`, `TFB_ProcessEvents`/`TFB_SwapBuffers`, `ProcessInputEvent`, and `TFB_FlushGraphicsEx`. If direct extraction is infeasible, extract a small shared production guard/order helper and make every real site and harness call it; do not copy logic into the shim. P00 fixes the `-luqm_rust` test linker issue before this probe can pass.

## Remediation procedure

1. Classify every fmt diff and Clippy diagnostic by file/symbol and whether current user-modified.
2. Apply rustfmt only to explicitly reviewed files. Save format-only patches separately. Do not use formatting as an excuse to change behavior.
3. Fix diagnostics idiomatically: declare intentional feature cfgs, remove/fix genuinely unused items, replace ambiguous glob exports with explicit exports, repair docs/style, and refactor warning-producing code. Do not suppress with global/module allows.
4. For any behavior/API/visibility change, first add or identify a failing regression test, then implement and rerun affected tests. Formatting-only changes do not need a fabricated test.
5. Review user-edited hunks with `showGitChanges`/`git diff`; merge forward. Preserve the no-flush `DoInput` edit, current `rust_gfx_postprocess` behavior, options edit, communication changes, and all unrelated edits unless an exact quality correction is unavoidable and tested.
6. Iterate full check/fmt/Clippy/test to zero exits.
7. Run `git diff --check`; inspect every changed file; compare saved hashes/diffs and describe each intentional delta.

## Handoff/semantic checks

- No automation module/options/hooks or proof code exists.
- No lint configuration was weakened and no blanket allow added.
- Every non-format change has behavior evidence.
- All original user changes remain represented.
- Full strict commands return zero, not “no new diagnostics.”

Worker does not create marker. Hand off evidence root, full diagnostic inventory/count method, `-luqm_rust` remediation evidence, exact file list, format-only and semantic patches/tests, every probe script/output, minimal harness link map/symbol origins/rerun manifest/order, commands/exits, and preservation comparison. P00a alone creates `.completed/P00.md`.
