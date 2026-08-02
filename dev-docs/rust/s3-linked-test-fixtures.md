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

## Evidence limits

The focused CI job retains its complete command log plus native build evidence,
provider report, and archive-input sidecar. These artifacts prove parsing,
building, profile selection, and provider authority. They are not autonomous
gameplay proof, screenshots, child-supervision receipts, or LCAR gameplay
artifacts. No gameplay completion is claimed until actual game execution,
capture correlation, and teardown evidence are implemented and replayed.
