# Battle Engine Phase 2/3 Requirements — Addendum

## Purpose

This document defines additional EARS-format requirements specific to the Phase 2/3 full-logic port of the battle engine. The primary behavioral requirements for the battle engine are defined in `battle/requirements.md` (602 lines, covering element system, display list, coordinates, velocity, collision, weapons, process loop, battle lifecycle, tactical transitions, AI, netplay, and error handling). Those requirements are language-agnostic and apply across all phases.

This addendum covers requirements that arise specifically from the Phase 2/3 migration — porting orchestration logic from C to Rust while maintaining behavioral parity, preserving external ABI surfaces, and supporting a guard/toggle coexistence model.

## Scope boundaries

- Behavioral requirements for the battle simulation itself (element lifecycle, collision, velocity, weapons, process loop ordering, tactical transitions, AI dispatch, netplay checksums) are defined in `battle/requirements.md` and are not duplicated here.
- This addendum covers only requirements that are unique to the Phase 2/3 porting context: branch parity, build-mode coexistence, ABI preservation, callback-slot safety, and cross-language determinism.

## Branch-parity obligations

- **Ubiquitous:** The Rust-owned battle logic shall preserve behavioral parity for every compile-time and runtime branch present in the C reference implementation. The following branch families shall produce identical observable behavior in the Rust path as in the C path:
  - `NETPLAY` / `NETPLAY_CHECKSUM` — netplay frame synchronization, CRC computation, battle-end readiness protocol
  - `DEMO_MODE` / `CREATE_JOURNAL` — demo recording/playback paths
  - `SUPER_MELEE` — SuperMelee-specific abort, cleanup, and ship-death notification behavior
  - `CHECK_ABORT` / `CHECK_LOAD` — abort and load-game cleanup paths reachable from lifecycle code
  - `IN_ENCOUNTER` / `IN_LAST_BATTLE` — encounter-specific and final-battle-specific environment setup, teardown, and flee-eligibility behavior
  - `inHyperSpace()` / `inQuasiSpace()` — hyperspace and quasispace navigation-mode battle initialization, music selection, and ship spawn behavior
  - Max-speed rendering skip — the rendering-skip behavior when the player holds the max-speed key

- **When** a compile-time configuration enables a branch family (e.g., `NETPLAY` is defined), **the Rust-owned logic shall** execute the same branch-family behavior as the C reference for that configuration, producing identical observable side effects (frame checksums, protocol transitions, audio state, display state, crew writeback outcomes).

- **When** a compile-time configuration disables a branch family, **the Rust-owned logic shall** omit that branch's behavior entirely, matching the C reference's disabled-branch behavior.

## Build-mode coexistence

- **Ubiquitous:** The battle engine shall support at least two build modes simultaneously: a C-only baseline build (with `USE_RUST_BATTLE_LOOP` disabled or undefined) and a Rust-enabled build (with `USE_RUST_BATTLE_LOOP` enabled).

- **When** `USE_RUST_BATTLE_LOOP` is disabled, **the battle engine shall** compile and execute the original C function bodies for all battle-scope functions without modification. The presence of Rust battle code in the build shall not alter the C-only code path.

- **When** `USE_RUST_BATTLE_LOOP` is enabled, **the battle engine shall** compile Rust-owned replacements for all ported battle functions and guard out the corresponding C function bodies. The Rust-owned logic shall produce behavior identical to the C path for all observable outputs.

- **Ubiquitous:** Functions explicitly designated as permanent C boundary surfaces shall remain compiled in C in all build modes. Their bodies shall not be guarded out by `USE_RUST_BATTLE_LOOP`.

## External symbol ABI preservation

- **Ubiquitous:** Every non-static battle function that has external C callers in the current source tree shall preserve its original C symbol name and calling convention in all build modes.

- **When** `USE_RUST_BATTLE_LOOP` is enabled, **the battle engine shall** provide a C-linkage wrapper or preserved symbol that delegates to the Rust implementation, so that external C callers require no source changes.

- **Ubiquitous:** The `DoBattle()` symbol shall remain a C function in all build modes because the `DoInput()` cooperative polling framework requires an existing C callback ABI. In Rust-enabled builds, the `DoBattle()` body shall be a thin shell that delegates frame semantics to a Rust FFI export.

## Callback-slot safety

- **Ubiquitous:** Callback-bearing fields stored in C structs (`Element.preprocess_func`, `Element.postprocess_func`, `Element.collision_func`, `Element.death_func`) shall remain C-ABI function pointers in all build modes. Rust closures, trait objects, or fat pointers shall not be stored directly in these fields.

- **When** Rust-owned behavior is installed in a callback slot, **the battle engine shall** use either a Rust `extern "C"` function or a documented boundary shim as the installed callback target.

- **When** an element or queue entry is reused, freed, or rebound, **the battle engine shall** clear or reinstall all callback-bearing fields and back-references before the object can be observed again through foreign storage.

- **Ubiquitous:** Stale callback dispatch shall be prevented. When Rust-owned behavior is installed in a callback slot, the battle engine shall verify before dispatch that: (a) the callback function pointer is non-null, (b) the element's owner/parent back-reference is valid (non-null and generation-matched if applicable), and (c) the element has not been marked DISAPPEARING. Dispatch to a stale or null callback shall be treated as a no-op rather than undefined behavior.

## Cross-language frame determinism
> *Cross-reference:* This requirement extends the determinism obligations in `battle/requirements.md` §Collision system / §Process loop / §Netplay integration to the Phase 2/3 Rust-owned code path specifically. The shared requirements define the behavioral determinism contract; this section adds the cross-language parity obligation.



- **Ubiquitous:** The Rust-owned battle logic shall produce bit-identical frame checksums (netplay CRC-32) as the C reference for the same input sequence and initial state. All arithmetic shall remain integer-only with no floating-point substitution.

- **Ubiquitous:** Element processing order within the display list shall be identical in the Rust path and the C path for the same frame state.

- **Ubiquitous:** RNG calls shall occur in the same order in the Rust path as in the C reference for the same frame and input state.

## FFI boundary safety
> *Cross-reference:* This requirement extends the pointer safety and FFI obligations in `battle/requirements.md` §Cross-language boundary considerations / §Error handling and invariants to the Phase 2/3 Rust-owned code path specifically. The shared requirements define general cross-language safety; this section adds Phase 2/3-specific FFI boundary constraints.



- **Ubiquitous:** No Rust panic shall cross an FFI boundary. All Rust-callable entry points exposed to C shall contain panic-catching mechanisms that convert failures to deterministic error or abort behavior.

- **Ubiquitous:** Foreign pointers received from C shall be validated (non-null check at minimum) before dereferencing in Rust code.

- **Ubiquitous:** Borrowed C pointers shall not be cached across frame boundaries unless the backing allocation is explicitly documented as stable for that duration.

- **When** a callback or FFI wrapper can trigger re-entrant C code that mutates battle state, **the Rust code shall** use handle-based traversal and staged re-lookups after the callback returns, rather than relying on cached pointer values from before the callback.
