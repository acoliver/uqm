# Phase 03a: Verify Trace, Artifact, and Identity Primitives

Phase ID: `PLAN-20260723-RUNTIME-AUTOMATION.P03.VERIFY`

Require P02 marker/P03 evidence. Independently verify only `REQ-IO-001..003` and the ordered `REQ-TRACE-001` primitive. Run focused/concurrent tests and strict gates. Mutations must fail for missing newline/sequence, publish before turn, holding runtime lock while waiting/writing, dropped reservation leaving a gap, duplicate cursor advance, success after sink failure, ignored flush/recover/sync/close error, temporary/final collision overwrite, nonexclusive final publication, unsupported misclassification, unsorted manifest, path-only identity, and content mutation.

FAIL if directory-sync unsupported is silently claimed successful, writer success precedes drop, existing files overwrite, tree escapes root, screenshot/checkpoint becomes semantic pass, or this phase claims graphics capture/FFI/shutdown/lifecycle/proof. FAIL on placeholders, unsafe without need, user-edit loss, or gate failure.

On PASS emit `Phase 03: PASS`, update tracker, create `.completed/P03.md` with exact durability semantics and mutation evidence. Otherwise no marker.
