# Contributor rules

[Project standards](project-standards.md) define the engineering requirements.
The game is Rust-only; do not reintroduce a C game implementation or fallback.
External SDL2 builds are managed by Cargo.

For behaviour changes, write a focused regression test, demonstrate its failure,
then implement and verify the fix. Keep tests deterministic and use real module
boundaries where integration is the subject of the test. Preserve unrelated work.

Use [the development commands](rust/howtorun.md) for formatting, strict Clippy
and workspace tests. Runtime changes also need relevant
[real-window scenarios](testing/scenarios.md). Do not weaken assertions or CI
checks to pass. Report what was verified and any remaining limits.

Put player instructions in `docs/`; keep implementation, automation and contributor
documentation in `dev-docs/`. Update callers and documentation links when moving
files. See [the planning guide](PLAN.md) for multi-phase implementation plans.
