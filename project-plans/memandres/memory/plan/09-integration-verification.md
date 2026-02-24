# Phase 09: Integration Verification

## Phase ID
`PLAN-20260224-MEM-SWAP.P09`

## Prerequisites
- Required: Phase 08a (Implementation Verification) completed
- Build succeeds with `USE_RUST_MEM` enabled
- Binary produced and launchable
- Expected files: all C-side files modified, `USE_RUST_MEM` active

## Requirements Implemented (Expanded)

### REQ-MEM-006: Behavioral Equivalence (Integration)
**Requirement text**: Verify end-to-end that the game functions correctly with Rust memory allocation.

Behavior contract:
- GIVEN: The game binary is built with `USE_RUST_MEM` enabled
- WHEN: The game is launched and exercised (menus, gameplay, shutdown)
- THEN: The game behaves identically to the C-memory build — no crashes, no corruption, no leaks

Why it matters:
- This is the final proof that the swap works correctly in production

### REQ-MEM-007: Build Both Paths (Integration)
**Requirement text**: Confirm both paths still compile for reversibility.

Behavior contract:
- GIVEN: All changes are in place
- WHEN: `USE_RUST_MEM` is commented out and the project is rebuilt
- THEN: The C path compiles and links correctly (regression check)

Why it matters:
- Ensures the swap is reversible if issues are found later

## Implementation Tasks

### End-to-End Runtime Verification

1. **Game Launch Test**
   - Launch the game with Rust memory active
   - Verify: game reaches main menu without crash
   - Verify: log output shows "Rust memory management initialized."

2. **Menu Navigation**
   - Navigate through main menu options
   - Enter and exit settings/options screens
   - These exercise allocation paths in `setupmenu.c`, `getchar.c`, etc.

3. **New Game / Content Loading**
   - Start a new game (if possible in automated fashion)
   - Verify: content loads (resources, graphics, sound)
   - These exercise `resinit.c`, `gfxload.c`, sound decoders, etc.

4. **Super Melee (if applicable)**
   - Enter Super Melee mode
   - Load teams
   - Exercises `loadmele.c`, `meleesetup.c`

5. **Clean Exit**
   - Exit the game cleanly
   - Verify: no crash on shutdown
   - Verify: log output shows "Rust memory management deinitialized."

### Build Regression Check

6. **C path regression**
   - Comment out `#define USE_RUST_MEM` in `config_unix.h`
   - Clean build
   - Verify success
   - Re-enable `#define USE_RUST_MEM`

### Automated Verification

7. **Cargo tests**
   - `cargo test --workspace --all-features`

8. **Lint/format**
   - `cargo fmt --all --check`
   - `cargo clippy --workspace --all-targets --all-features -- -D warnings`

## Integration Contract

### Existing Callers
- 322+ call sites across 55+ files — all redirected via `memlib.h` macros
- `uqm.c` → `mem_init()` / `mem_uninit()` — lifecycle entry points

### Existing Code Replaced/Removed
- `w_memlib.c` — excluded from build by Makeinfo conditional, guarded by `#error`
- NOT deleted — remains for C-path fallback

### User Access Path
- Every game feature that allocates memory (all of them) exercises the Rust path

### End-to-End Verification
- Game launch + menu navigation + content loading + clean exit

## Verification Commands

```bash
# Full Rust checks
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features

# C build (Rust path active)
cd sc2 && ./build.sh uqm

# Launch game (manual or scripted)
# ./uqm  (verify log output, menu navigation, clean exit)

# C path regression test
# (temporarily comment out USE_RUST_MEM, rebuild, re-enable)
```

## Structural Verification Checklist
- [ ] All plan phases completed (P00a through P09)
- [ ] All files modified as specified
- [ ] No skipped phases
- [ ] Plan/requirement traceability present throughout

## Semantic Verification Checklist (Mandatory)
- [ ] Game launches and reaches main menu (Rust memory active)
- [ ] Log shows Rust memory init/deinit messages
- [ ] Menu navigation works without crashes
- [ ] Content loading works (graphics, sound, resources)
- [ ] Clean exit without crash
- [ ] C path regression build succeeds
- [ ] All cargo tests pass
- [ ] All lint checks pass

## Deferred Implementation Detection (Mandatory)

```bash
# Check all modified files for deferred patterns
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" \
  rust/src/memory.rs \
  rust/src/logging.rs \
  sc2/src/libs/memlib.h \
  sc2/src/libs/memory/w_memlib.c \
  sc2/src/libs/memory/Makeinfo \
  sc2/config_unix.h
```

Pre-existing comments in `memory.rs` about "later phases" for custom allocators are out of scope.

## Success Criteria
- [ ] Game runs correctly with Rust memory allocation
- [ ] C path regression succeeds
- [ ] All automated checks pass
- [ ] No crashes, corruption, or leaks observed

## Failure Recovery
- Rollback: Comment out `#define USE_RUST_MEM` in `config_unix.h` → rebuild
- If crash on launch: check log for allocation failure, verify `rust_hmalloc` is being called
- If linker error: verify `libuqm_rust.a` exports the `rust_*` symbols
- If Makeinfo issue: verify `USE_RUST_MEM` propagation through build system

## Phase Completion Marker
Create: `project-plans/memandres/memory/.completed/P09.md`

Contents:
- phase ID
- timestamp
- files changed: summary of all files across all phases
- tests added/updated: `test_fatal_alias` in logging.rs
- verification outputs (build log, test output, runtime verification notes)
- semantic verification summary
- final PASS/FAIL decision for entire plan
