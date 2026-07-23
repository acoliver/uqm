# Phase 04a: Verify Pure Sticky-Terminal Runtime Model

Phase ID: `PLAN-20260723-RUNTIME-AUTOMATION.P04.VERIFY`

Require P03 marker/P04 evidence. Verify only `REQ-STATE-001..004` and the classification model for `REQ-WATCH-004` against the domain model, execution-contract §§3-4, and pseudocode 003. Run focused/property tests and strict gates.

Mutation checks must fail for: inactive TLS/allocation/lock/log/external work; ABI/active counters conflated; catch limited to an inner observer; transition calling effects under lock; missing reservation cancellation; commit without matching version/generation; non-lock-free key/terminal fallback; resume after poison; lock during reentry; conservative result weakened; finalization before shell/reservation drain; duplicate run_end; late writer access; or any runtime/ordered-I/O/external lock overlap.

FAIL if actual C activity/keys/FFI/clear-site/lifecycle/graphics integration appears or is claimed complete; those belong to P05-P07. FAIL on nondeterministic sleeps, panic-driven production control, placeholders, lost user edits, or strict failure.

On PASS emit `Phase 04: PASS`, update tracker, create `.completed/P04.md` recording pure scope and explicitly OPEN integration requirements. Otherwise no marker.
