# P06a Verification: Mount Ordering and Access Semantics

## Verdict
REJECT

## Reviewed Inputs
1. `.completed/P06.md`
2. `plan/06-mount-ordering-access.md`
3. `rust/src/io/uio_bridge.rs`
4. Requested verification command:
   - `cd /Users/acoliver/projects/uqm/rust && cargo test --lib --all-features 2>&1 | tail -5`

## Summary
P06 partially implements the requested groundwork:
- `MountInfo` does include `position` and `read_only`.
- `register_mount()` and `sort_mount_registry()` do implement explicit position-based ordering.
- `uio_access()` now does mode-aware checks instead of pure existence checks.
- AutoMount deferral is explicitly documented in code.

However, the phase plan for P06 explicitly required four gaps to be implemented in this phase: G4, G6, G14, and G18. G14 was not implemented, and the deferral is not acceptable as a phase completion because the plan marks overlay-aware mutation semantics as part of P06's required implementation and mandatory semantic verification. The current mutation entry points still operate directly on resolved host paths and do not use overlay-aware resolution helpers.

## Code Verification

### G6: Mount ordering foundation
Verified in `rust/src/io/uio_bridge.rs`:
- `MountInfo` has explicit ordering state:
  - `position: usize`
  - `read_only: bool`
- `register_mount()` derives positions from TOP/BOTTOM/ABOVE/BELOW semantics.
- `sort_mount_registry()` sorts active mounts first, then by ascending `position`.

Assessment:
- The foundation is directionally correct and is solid enough to support later overlay-aware mutation resolution.
- The registry lock around insertion/sorting also gives a coherent topology update story.

Caveat:
- The position scheme uses `usize` with `min_position - 1` for TOP/ABOVE. If the minimum active position ever becomes `0`, this underflows in debug and wraps in release. The current defaults avoid it initially, but the implementation is still fragile as a general ordering mechanism.
- Also, ABOVE/BELOW only compute `relative_pos ± 1`; they do not create a stable gap model or rebalance positions, so repeated insertions around the same relative mount can produce collisions and ordering ambiguity over time.

Conclusion for G6:
- Foundation present, but not robust enough to call fully complete without qualification.
- Still acceptable as groundwork, but not enough to rescue the phase because G14 is missing.

### G4: Access mode checking
Verified in `uio_access()`:
- Validates mode bits against `F_OK | R_OK | W_OK | X_OK`.
- Resolves only the topmost visible existing object by iterating registry order.
- `F_OK`: existence-only success.
- `W_OK`: fails with `EACCES` on read-only mounts; also checks host readonly bit for stdio.
- `X_OK`: directories succeed; non-stdio mounts fail; stdio defers to host execute bits on Unix.

Assessment:
- This is substantially complete for the stated P06 requirement.
- The topmost-visible-object rule is implemented for access checks.

Minor note:
- `R_OK` is effectively unconditional once the object exists. That matches the documented intent in the completion notes, though it is simplified semantics.

Conclusion for G4:
- Acceptable and complete for P06.

### G14: Overlay-aware mutation resolution
Verified by inspecting mutation entry points:
- `uio_open()` directly does `resolve_path(dir_path, &p)` and then `OpenOptions::open(&file_path)`.
- `uio_rename()` directly does `resolve_path(...)` and then `fs::rename(&old_full, &new_full)`.
- No overlay-aware helper was added for topmost-visible-object mutation resolution.
- No visible cross-mount rename detection with `EXDEV`.
- No visible no-fallthrough protection when an upper read-only mount shadows a lower writable object.
- No visible `ENOTDIR` parent shadowing logic.

Assessment:
- This is not implemented.
- The phase plan did not describe G14 as optional; it is listed under required implementation tasks and mandatory semantic verification.
- Deferring G14 to P07 changes scope and acceptance criteria for the phase.

Conclusion for G14:
- Not complete.
- Deferral is not reasonable for accepting P06 as completed.

### G18: AutoMount conditional
Verified in code comments on `MountInfo`:
- `AutoMount is DEFERRED per P00a Q2 resolution - not needed for engine/runtime paths`
- No `auto_mount_rules` field is implemented.

Assessment:
- The deferral is explicitly documented in code and aligns with the completion note.
- For the conditional branch requirement, this is sufficient if P00a indeed determined AutoMount is not required for parity.

Conclusion for G18:
- Properly documented as deferred/not required.

## Evaluation of the implementer’s deferral

### Is the mount-ordering foundation solid enough to support mutation later?
Yes, mostly.
- Explicit ordering and read-only metadata are the right prerequisites.
- Registry serialization via `Mutex<Vec<MountInfo>>` is a reasonable foundation.
- But the `usize` decrement/increment position strategy is brittle and should be hardened before too much logic depends on it.

### Is the access mode checking complete?
Yes, for P06’s access requirement.
- It is mode-aware, precedence-aware, and uses appropriate errno values for the implemented cases.

### Is AutoMount deferral properly documented?
Yes.
- It is documented both in the completion results and in code near `MountInfo`.

### Is G14 deferral reasonable?
No, not for phase acceptance.
- It may be a reasonable sequencing decision from an implementation-management perspective.
- But against the written P06 plan, it is a scope miss, not an acceptable completion.
- The plan explicitly required mutation entry points and mandatory mutation edge-case tests in this phase.

## Verification command result
```text
test threading::tests::test_condvar_wait_timeout ... ok
test threading::tests::test_condvar_wait_signal ... ok

test result: ok. 1479 passed; 0 failed; 5 ignored; 0 measured; 0 filtered out; finished in 0.10s
```

## Final Decision
REJECT

## Required follow-up before acceptance
1. Implement overlay-aware mutation resolution in the P06-required entry points or formally revise the plan.
2. Add the missing mutation semantics/tests:
   - `uio_open(O_WRONLY)` on read-only top layer fails without fallthrough
   - parent-visible-only-in-readonly-layers failure
   - upper read-only shadow prevents lower writable mutation
   - upper non-directory parent shadow returns `ENOTDIR`
   - cross-mount rename returns `EXDEV`
3. Harden mount ordering position assignment to avoid `usize` underflow/collision issues if this foundation will be relied on by later overlay logic.
