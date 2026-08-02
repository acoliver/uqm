# S3 Linked-Test Profile

Issue 23 adds one separately selected native fixture profile without changing
the accepted production profile.

## Profiles

- `production`: Cargo features `audio_heart,linked_c_archive`; production
  defines and flags remain exact.
- `linked-test`: Cargo features
  `audio_heart,debug-process,linked_c_archive`; defines are exactly the
  production defines plus `DEBUG`; compile flags are unchanged.

The linked-test profile is declared identically in
`rust/build/native-inputs.json` and
`rust/ownership/native-provider-manifest.json`. `rust/build.rs` selects it only
when `UQM_NATIVE_PROFILE=linked-test`, rejects unknown selectors, and requires
the active Cargo features to exactly match the selected authority.

## Canonical verification

```sh
cargo run --locked --manifest-path rust/xtask/Cargo.toml -- test
```

The command first runs all workspace targets with the pure feature set
`audio_heart,debug-process`, which does not activate native linking. It then
builds the real `uqm` binary and executes `linked_provider_fixture` with the
exact linked-test profile through the canonical S2 toolchain and S1
native-provider validation. The fixture enters the DEBUG C consumer and compares
all creature values it observes with the canonical Rust catalog, so dormant
provider imports cannot pass by merely linking the game binary.

Checked-in automation JSON is parsed and validated by
`automation::script::tests::every_checked_in_script_parses_and_validates`.
Unsupported S3 JSON drafts are not retained as fixtures.

## Runtime observations

The graphics hook commits a presentation callback only after the backend's
`postprocess` operation. Redraw-skipped calls therefore do not advance the
presentation counter or complete a capture generation.

The battle loop increments a read-only Rust counter at its stable frame seam.
The typed `assert_battle_frames` script action validates a positive minimum,
remains callback-bound in the scheduler, and is checked by the coordinator
before the action advances.

The DEBUG-only biological-value consumer calls the narrow
`rust_creature_bio_value` accessor. Invalid creature IDs return `-1` and cause
the C consumer to fail fast; valid values preserve the low-nibble semantics of
`calculateBioValue`.

## Native gameplay acceptance and evidence

Issue 23 now runs native acceptance on macOS 14 arm64, Ubuntu 22.04 x86_64,
the supported GitHub-hosted Ubuntu ARM runner, and the maintained macOS Intel
runner. The requested macOS 13 hosted label has been retired from GitHub
Actions; CI emits a machine-readable excluded-execution record with that reason
instead of pretending the tuple ran. Each applicable job reuses one canonical
production artifact while executing the strict linked
test, probes, the composed P00/menu-binding harness, menu, communication,
PlanetSide, and battle scenarios. Every scenario is supervised and emits a
closed LCAR with typed process/teardown receipts, exact artifact inventory,
immutable source/config identity snapshots, correlated trace/capture evidence,
and verified config cleanup.

The checked-in battle scenario is executed twice from the same production
artifact. The acceptance command normalizes only process-specific elapsed time,
requires both super-melee menu and battle RNG-boundary records, and compares
semantic-trace and PNG digests byte-for-byte. Offline validation and deterministic
negative LCAR mutations run in every native tuple. CI uploads passing or failing
LCARs, logs, captures, harness maps, provider reports, and native build evidence
with `if: always()`. A child failure is therefore evidence-bearing rather than a
missing-artifact success claim.
