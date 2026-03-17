# Phase 11: Integration Verification

## Phase ID
`PLAN-20260314-RESOURCE.P11`

## Prerequisites
- Required: All previous phases (P00.5 through P10a) completed and verified
- Required: Phase 0.5 artifacts include the authoritative engine build command, any authoritative launch/test command, the concrete sentinel-validation path/fixture, and the concrete config round-trip verification path/harness to use here

## Purpose

Final integration verification ensuring all gap-closure work functions correctly in the full engine context. This phase produces no new code — it validates the cumulative effect of all changes and confirms that any dead-code cleanup decisions were actually safe in the real build/runtime path.

## Integration Contract

### Existing Callers (unchanged, must still work)
- `sc2/src/uqm/setup.c` → `InitResourceSystem()`
- `sc2/src/uqm.c` → `LoadResourceIndex(configDir, "uqm.cfg", "config.")`
- `sc2/src/options.c` → `LoadResourceIndex()` for each `.rmp` content index
- `sc2/src/libs/input/sdl/input.c` → `res_GetString`, `res_PutString`, `res_IsString`, `res_Remove`
- `sc2/src/libs/graphics/resgfx.c` → `InstallResTypeVectors`, `res_GetResource`, `res_DetachResource`
- `sc2/src/libs/strings/sresins.c` → `InstallResTypeVectors`, `res_GetResource`, `res_DetachResource`
- `sc2/src/libs/sound/resinst.c` → `InstallResTypeVectors`, `res_GetResource`, `res_DetachResource`
- `sc2/src/libs/video/vresins.c` → `InstallResTypeVectors`, `res_GetResource`, `res_DetachResource`
- `sc2/src/uqm/dummy.c` → `InstallCodeResType`, `res_GetResource`, `res_DetachResource`
- `sc2/src/uqm/cleanup.c` → `UninitResourceSystem()`

### Existing Code Replaced/Removed
- C `resinit.c`, `getres.c`, `filecntl.c`, `propfile.c`, `loadres.c` — already compiled out by `USE_RUST_RESOURCE`
- Rust dead modules: `ffi.rs`, `resource_system.rs`, `loader.rs`, `cache.rs`, `index.rs`, `config_api.rs` — removed or explicitly retained with justification in Phase 10
- C wrappers `sc2/src/libs/resource/rust_resource.h` / `rust_resource.c` — disposition must be confirmed here if Phase 10 left them conditional or deferred

### User Access Path
- Boot game → resources load → menus render → audio plays → game runs
- Change settings → save config → restart → settings persist

### Data/State Migration
- None — no config format changes, no index format changes

## End-to-End Verification Steps

### 1. Full Rust test suite

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

### 2. Full engine build

Run the authoritative engine build command recorded in Phase 0.5. Do not substitute a newly discovered command at execution time unless the preflight artifact is first corrected.

```bash
cd sc2
./build.sh uqm
```

If Phase 0.5 recorded a different authoritative build invocation, record and use that exact command here.

Verify: builds successfully with `USE_RUST_RESOURCE` enabled.

### 3. Boot test

- Launch the game using the authoritative launch/test command recorded in Phase 0.5
- Verify: main menu appears (requires resource loading: graphics, strings, audio)
- Verify: no crashes, no error logs related to resource loading
- Record the exact launch command and factual result

### 4. Config round-trip test

Use the concrete config round-trip verification path/harness recorded in Phase 0.5.

1. Boot game
2. Change a setting (e.g., sound volume)
3. Exit game (triggers `SaveResourceIndex` for config)
4. Re-launch game
5. Verify: setting persists (was saved and reloaded correctly)
6. Record the exact setting changed, the config file/path or harness used, and the factual before/after result

### 5. Resource load/detach cycle

- Enter a game (loads ship resources, combat graphics, etc.)
- Navigate through menus that trigger resource load/detach patterns
- Verify: no crashes, no visible graphical/audio glitches
- Record the exact navigation path used as evidence

### 6. Sentinel-path verification

- Exercise the exact concrete directory-backed resource path or fixture recorded during Phase 0.5 and used in Phase 08/08a verification
- Verify: `res_OpenResFile` directory handling works and `LoadResourceFromPath` does not forward sentinel handles into callbacks
- Record the exact path/fixture, command/harness, and factual result

### 7. Shutdown verification

- Exit the game cleanly
- Verify: no crash during `UninitResourceSystem`
- If running under a memory checker (valgrind, AddressSanitizer, or project-supported equivalent): verify no leaks from C-allocated resources
- Record the exact tool used, if any, and the factual result

### 8. Explicit-verification requirement revalidation

Revalidate every requirement marked "Explicit verification" in `01a-analysis-verification.md` using the execution matrix defined there. This step is mandatory and must not be collapsed into a short representative subset.

For each explicit-verification requirement:
- identify the exact test, command, runtime path, or evidence artifact used
- record a short factual result
- note the corresponding requirement ID in the phase completion marker

At minimum, the completion marker must include explicit evidence entries for:
- REQ-RES-LIFE-001, REQ-RES-LIFE-002, REQ-RES-LIFE-003, REQ-RES-LIFE-006, REQ-RES-LIFE-007
- REQ-RES-TYPE-001, REQ-RES-TYPE-002, REQ-RES-TYPE-003, REQ-RES-TYPE-005, REQ-RES-TYPE-006, REQ-RES-TYPE-007, REQ-RES-TYPE-008
- REQ-RES-IDX-001, REQ-RES-IDX-002, REQ-RES-IDX-003, REQ-RES-IDX-004, REQ-RES-IDX-007, REQ-RES-IDX-008
- REQ-RES-UNK-004
- REQ-RES-CONF-001, REQ-RES-CONF-002, REQ-RES-CONF-004, REQ-RES-CONF-005, REQ-RES-CONF-006, REQ-RES-CONF-007, REQ-RES-CONF-008
- REQ-RES-LOAD-001, REQ-RES-LOAD-002, REQ-RES-LOAD-004, REQ-RES-LOAD-005, REQ-RES-LOAD-006, REQ-RES-LOAD-009, REQ-RES-LOAD-010
- REQ-RES-FILE-001, REQ-RES-FILE-004, REQ-RES-FILE-007
- REQ-RES-OWN-001, REQ-RES-OWN-002, REQ-RES-OWN-003, REQ-RES-OWN-004, REQ-RES-OWN-006, REQ-RES-OWN-007, REQ-RES-OWN-008
- REQ-RES-ERR-001, REQ-RES-ERR-002, REQ-RES-ERR-004, REQ-RES-ERR-005, REQ-RES-ERR-006
- REQ-RES-INT-001, REQ-RES-INT-002, REQ-RES-INT-003, REQ-RES-INT-004, REQ-RES-INT-005, REQ-RES-INT-007, REQ-RES-INT-009

### 9. Final dead-code cleanup confirmation

- Confirm the real build/runtime path still succeeds after any Rust-side removals
- Confirm the recorded disposition of `rust_resource.h` / `rust_resource.c` is correct in the real build
- If those C-side files were deferred in Phase 10, either prove them unused now or leave them intentionally retained with explicit rationale

## Gap Closure Confirmation

| Gap | Fix Phase | Verification |
|-----|-----------|-------------|
| GAP-1: res_OpenResFile sentinel | P08 | Directory resources / sentinel-producing path works |
| GAP-2: res_GetString type check | P05 | Config string reads return correct values; non-string queries return "" |
| GAP-3: UNKNOWNRES as value type | P04 | Unknown type entries stored with str_ptr; res_GetResource returns descriptor |
| GAP-4: get_resource value types | P04 | Value-type entries (STRING, INT32, BOOLEAN, COLOR) return correct data through res_GetResource |
| GAP-5: UninitResourceSystem cleanup | P06 | Clean shutdown with no C-resource leaks |
| GAP-6: Entry replacement freeFun | P06 | Index reload doesn't leak old loaded resources |
| GAP-7: SaveResourceIndex filtering | P07 | Config save doesn't emit GFXRES/SNDRES entries; UNKNOWNRES entries skipped |
| GAP-8: CountResourceTypes u32 | P09 | ABI matches C declaration |
| GAP-9: LoadResourceFromPath invalid-open/zero-length guards | P08 | Sentinel and zero-length files handled gracefully before callback dispatch |
| GAP-10: GetResourceData doc fix | P09 | Comment matches code behavior |
| GAP-11: Dead code removal | P10 + P11 | Only authoritative modules remain, with evidence-backed cleanup decisions |

## Structural Verification Checklist
- [ ] All 11 gaps addressed
- [ ] All phases completed (P00.5 through P10a)
- [ ] All tests pass
- [ ] Full engine builds using the preflight-recorded authoritative command
- [ ] No regressions
- [ ] Deferred dead-code decisions, if any, are explicitly resolved or intentionally retained with rationale
- [ ] Every explicit-verification requirement from Phase 01a has a recorded evidence entry

## Semantic Verification Checklist (Mandatory)
- [ ] Engine boots and runs correctly
- [ ] Config persistence works (save/load round-trip)
- [ ] Resource loading works for all types (graphics, sound, strings, video, code)
- [ ] Sentinel directory path is handled correctly end-to-end using the recorded concrete path/fixture
- [ ] Clean shutdown with no leaks
- [ ] Already-implemented requirements listed in Phase 01a have explicit revalidation evidence
- [ ] Only one authoritative resource module stack exists in practice, not just by assertion

## Success Criteria
- [ ] All verification steps pass
- [ ] All substantive review findings are closed by evidence
- [ ] Plan is complete

## Failure Recovery
- rollback steps: none for this verification-only phase; use per-phase recovery from earlier phases for code changes
- blocking issues to resolve before declaring complete: failed engine build, unresolved dead-code dependency, missing explicit evidence for revalidated requirements, or mismatch between Phase 0.5 recorded commands/paths and actual execution evidence

## Phase Completion Marker
Create: `project-plans/20260311/resource/.completed/P11.md`

Contents:
- phase ID
- timestamp
- files changed
- tests added/updated
- verification outputs
- semantic verification summary
- authoritative build command used
- authoritative launch/test command used
- config round-trip path/harness and factual result
- sentinel path/fixture and factual result
- explicit evidence for every already-implemented requirement revalidation entry from Phase 01a
- final dead-code cleanup disposition
