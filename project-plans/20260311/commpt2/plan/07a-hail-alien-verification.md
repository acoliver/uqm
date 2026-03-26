# Phase 07a: HailAlien Verification

## Phase ID
`PLAN-20260326-COMMPT2.P07a`

## Prerequisites
- Required: Phase 07 (HailAlien) completed
- Phase completion marker exists: `project-plans/20260311/commpt2/.completed/P07.md`

## Structural Verification Checklist

- [ ] `rust/src/comm/hail.rs` exists and contains `hail_alien()` function
- [ ] `rust/src/comm/mod.rs` has `pub mod hail;` declaration
- [ ] `rust_HailAlien` in `ffi.rs` calls `hail::hail_alien()` (not a stub)
- [ ] `hail.rs` has complete c_bridge extern block with all needed C functions
- [ ] `hail_alien()` has resource loading phase (7 resources)
- [ ] `hail_alien()` has TextCacheContext setup phase
- [ ] `hail_alien()` has AnimContext setup phase
- [ ] `hail_alien()` has SIS drawing phase with WON_LAST_BATTLE branch
- [ ] `hail_alien()` has encounter execution phase (init → DoInput → post → uninit)
- [ ] `hail_alien()` has cleanup phase (all resources destroyed)
- [ ] `@plan PLAN-20260326-COMMPT2.P07` markers present
- [ ] `@requirement REQ-HL-*` markers present
- [ ] No `P11: Stub` markers in `rust_HailAlien` function
- [ ] All existing tests compile and pass

## Semantic Verification Checklist

### Resource Loading (REQ-HL-002)
- [ ] PlayerFont loaded via `c_LoadFont(c_GetPlayerFontRes())`
- [ ] AlienFrame loaded via `c_CaptureDrawable(c_LoadGraphic(c_GetCommDataAlienFrameRes()))`
- [ ] AlienFont loaded via `c_LoadFont(c_GetCommDataAlienFontRes())`
- [ ] AlienColorMap loaded via `c_CaptureColorMap(c_LoadColorMap(c_GetCommDataAlienColorMapRes()))`
- [ ] AlienSong has alt-song fallback: check flags, try alt res, fallback to primary
- [ ] ConversationPhrases loaded via `c_CaptureStringTable(c_LoadStringTable(...))`
- [ ] Each loaded resource is stored in CommData via setter bridge

### Context Setup (REQ-HL-003)
- [ ] TextCacheContext created with `c_CreateContext("TextCacheContext")`
- [ ] TextCacheFrame created as pixmap with correct dimensions
- [ ] TextCacheContext configured: FG frame, background color (0,0,0x10), cleared, transparent color
- [ ] AnimContext created with `c_CreateContext("AnimContext")`
- [ ] AnimContext configured: FG frame = Screen, frame rect obtained

### SIS Drawing (REQ-HL-007)
- [ ] SetTransitionSource(NULL) called
- [ ] BatchGraphics called
- [ ] WON_LAST_BATTLE branch: clip rect set to CommWndRect corner
- [ ] Normal branch: clip rect set to SIS_ORG, DrawSISFrame, DrawSISMessage, DrawSISTitle
- [ ] Starbase special case: specific message and title strings
- [ ] Default case: DrawSISMessage(NULL), DrawSISTitle(PlanetName)
- [ ] DrawSISComWindow called

### Encounter Execution (REQ-HL-004, REQ-DI-001–004)
- [ ] CHECK_LOAD set before init_encounter_func (REQ-HL-006)
- [ ] init_encounter_func called
- [ ] DoInput called with exclusive=FALSE
- [ ] Post-encounter check: `!(CHECK_ABORT | CHECK_LOAD)` before calling post_encounter_func
- [ ] uninit_encounter_func called unconditionally
- [ ] Frame timing controlled by DoInput (REQ-DI-004)

### Cleanup (REQ-HL-005)
- [ ] SpaceContext restored
- [ ] Old font restored
- [ ] ConversationPhrases destroyed (DestroyStringTable)
- [ ] AlienSong destroyed (DestroyMusic)
- [ ] AlienColorMap destroyed (DestroyColorMap via ReleaseColorMap)
- [ ] AlienFont destroyed (DestroyFont)
- [ ] AlienFrame destroyed (DestroyDrawable via ReleaseDrawable)
- [ ] TextCacheContext destroyed
- [ ] TextCacheFrame destroyed
- [ ] PlayerFont destroyed
- [ ] CommData.ConversationPhrasesRes cleared to 0
- [ ] CommData.ConversationPhrases cleared to 0
- [ ] pCurInputState cleared to NULL
- [ ] Cleanup runs regardless of exit path

### Integration
- [ ] Call path: comm.c:1458 → rust_HailAlien() → hail_alien()
- [ ] hail_alien is reachable from a real game flow (player hails alien ship)
- [ ] No unused code or dead paths in hail.rs

## Verification Commands

```bash
# All tests pass
cargo test --workspace --all-features

# Lint gates
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings

# Verify hail module exists and is connected
grep "pub mod hail" rust/src/comm/mod.rs
grep -c "hail_alien\|hail::" rust/src/comm/ffi.rs
# Expected: at least 1

# Verify no stubs remain
grep "P11: Stub" rust/src/comm/ffi.rs | grep -i "hail"
# Expected: 0

# Verify resource lifecycle completeness
grep -c "c_Load\|c_Capture\|c_Destroy\|c_Release" rust/src/comm/hail.rs
# Expected: substantial count (load + capture + destroy for each resource)

# Verify encounter func calls
grep "c_Call.*EncounterFunc" rust/src/comm/hail.rs
# Expected: 3 (init, post, uninit)

# Deferred implementation check
grep -n "TODO\|FIXME\|HACK\|placeholder\|stub" rust/src/comm/hail.rs
# Expected: 0

# C build
# (project-specific build with USE_RUST_COMM=on)
# (project-specific build with USE_RUST_COMM=off)
```

## Pass/Fail Gate Criteria

**PASS if**:
- All structural checks pass
- All semantic checks pass (resource lifecycle, context setup, encounter sequence)
- All 267+ comm tests pass
- Both build modes compile and link
- No deferred implementation markers in hail.rs
- `cargo fmt`, `cargo clippy`, `cargo test` all green

**FAIL if**:
- `hail_alien()` is missing any phase of the encounter sequence
- Any resource is loaded but not freed
- Encounter function call sequence deviates from C
- WON_LAST_BATTLE branch is missing
- Starbase special case is missing
- Alt-song fallback logic is missing
- Cleanup doesn't run on abort/load exit paths
- Any test regression
