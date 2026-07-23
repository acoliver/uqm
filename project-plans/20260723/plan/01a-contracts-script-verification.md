# Phase 01a: Verify Typed Script and Activation Contracts

Phase ID: `PLAN-20260723-RUNTIME-AUTOMATION.P01.VERIFY`

Require `.completed/P00.md`, P01 handoff, no P01 marker. Separate verifier; do not fix production.

Independently inspect `automation/{mod,error,script}.rs`, manifest/lock, and tests against `REQ-BUILD-002`, `REQ-DEP-001..003`, `REQ-SCRIPT-001..006`, execution-contract inclusive limits, and pseudocode 001. Run P01 commands plus mutations: remove deny-unknown-fields, change one key mapping, allow early finish, accept N required updates with maximum N instead of at least N+1, overflow the required-budget sum, and disable semantic validation; tests must fail. Restore verifier-local mutations without disturbing worker changes.

FAIL if dependencies are transitive only, async/feature flags appear, semantic assertion is raw/stringly/fake, parsing causes runtime/file side effects, errors lack path/step, RED evidence is compile-only beyond the first API, user edits changed, or any strict command fails.

Verify this pure phase does not claim runtime scheduling, FFI, shutdown, capture, lifecycle, or proof completion.

On FAIL emit `Phase 01: FAIL`, no marker. On PASS emit `Phase 01: PASS`, update tracker, and create `.completed/P01.md` with requirements, files, independent commands/exits, mutation results, preservation review, and semantic PASS.
