# Phase 12: End-to-End Integration Verification

## Phase ID
`PLAN-20260314-COMM.P12`

## Prerequisites
- Required: Phase 11a completed
- Expected: All Rust-side subsystems implemented (P03–P10), all C-side wiring complete (P11)

## Requirements Implemented (Expanded)

This is a verification-only phase. It validates end-to-end integration across all requirements.

### CV-REQ-001–018: Comprehensive compatibility validation

All compatibility and validation requirements must be verified in integrated context.

## Verification Approach

### Required evidence artifacts

P12 is not satisfied by free-form manual confirmation alone. Every high-risk scenario below must produce reproducible evidence captured under `project-plans/20260311/comm/artifacts/p12/` (or a phase-local equivalent path recorded in the final notes):

- `scenario-results.md` — one row per scenario with pass/fail, build mode, commit/hash if relevant, and artifact links
- `callback-traces/` — ordered event traces for callback/lifecycle sequencing scenarios
- `routing-traces/` — entry-point and replay-target traces
- `leak-checks/` — repeated-encounter resource/leak evidence
- `build-mode-comparison/` — C-mode vs Rust-mode comparison notes with stable filenames for screenshots/logs

Minimum trace format for callback/lifecycle logs must include a monotonic step index and event name, e.g.:

```text
001 init_encounter
002 phrase_callback:<id>
003 post_encounter
004 uninit_encounter
```

### Level 1: Automated Tests

```bash
# Full Rust test suite
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features

# Full C build
cd /Users/acoliver/projects/uqm/sc2 && make clean && make
```

### Level 2: Integration Scenarios

Each scenario represents a specific end-to-end verification path.

#### Scenario 1: Simple Encounter (Arilou)
**Validates**: EC-REQ-001–016, DS-REQ-001–012, RS-REQ-001–016
**Required evidence**:
- callback trace covering init/post/uninit order
- screenshot or equivalent capture of subtitle + response state
- scenario record entry in `scenario-results.md`

- Launch game, navigate to Arilou space
- Encounter triggers `RaceCommunication()` / `InitCommunication()` → `c_init_race(ARILOU_CONVERSATION)` → `init_arilou_comm()`
- Verify: alien portrait displayed, ambient animations running
- Verify: NPC speech plays with subtitles synchronized
- Verify: player responses rendered and selectable
- Verify: selecting response calls callback with correct RESPONSE_REF
- Verify: conversation continues through multiple rounds
- Verify: exit via setSegue(Peace), no combat

#### Scenario 2: Hostile Encounter (Ur-Quan)
**Validates**: EC-REQ-004, EC-REQ-005, SB-REQ-001–005
**Required evidence**:
- attack-without-hail trace proving post+uninit and absence of init callback
- hail path trace proving init→dialogue→post→uninit order
- scenario record entry

- Encounter with Ur-Quan where combat follows
- Verify: hail-or-attack choice presented
- Choosing Talk: verify dialogue proceeds normally, ends with setSegue(Hostile), combat begins
- Choosing Attack: verify init_encounter_func NOT called, post+uninit called, BATTLE_SEGUE=1
- Verify: EC-REQ-015 callback ordering for attack-without-hail

#### Scenario 3: Starbase Commander
**Validates**: DS-REQ-002 (game state dispatch)
**Required evidence**:
- routing trace showing resolved conversation path
- scenario record entry

- Visit starbase
- Verify: race/context resolution chooses the expected commander/starbase conversation path
- Verify: full starbase dialogue works

#### Scenario 4: Number Speech (ZoqFot)
**Validates**: DS-REQ-005–006 (dynamic text), number speech synthesis
**Required evidence**:
- trace or logged decomposition path for number-speech clip selection
- scenario record entry

- Encounter ZoqFotPik with fleet count scenario
- Verify: NPCNumber produces audible number speech with correct digit decomposition
- Verify: GLOBAL_PLAYER_NAME substitution in dialogue

#### Scenario 5: Long Conversation (Melnorme)
**Validates**: CV-REQ-004 (long conversations), SS-REQ-013–017 (summary)
**Required evidence**:
- summary pagination capture showing carry-over across page boundaries
- scenario record entry

- Extended Melnorme dialogue buying multiple items
- Verify: conversation summary accessible via Cancel during response
- Verify: all previous NPC dialogue in summary, correct order
- Verify: pagination works, text carries across page boundaries
- Verify: exiting summary returns to response selection

#### Scenario 6: Phrase Disable (typical race)
**Validates**: PS-REQ-001–007, CV-REQ-015
**Required evidence**:
- reference to the P00.5 all-27-script audit artifact used for expected behavior
- scenario record entry

- Encounter that uses DISABLE_PHRASE extensively
- Verify: disabled phrases don't appear as response options
- Verify: PHRASE_ENABLED returns false after disable
- Verify: NPCPhrase on disabled phrase still plays audio/text
- Verify: phrase state reset on new encounter
- Verify: behavior matches the P00.5 all-27-script audit conclusions

#### Scenario 7: Save/Load During Encounter
**Validates**: CV-REQ-002, CV-REQ-007, PS-REQ-006, EC-REQ-010
**Required evidence**:
- callback/lifecycle trace covering load interruption and resumed setup ordering
- scenario record entry

- Enter encounter, save mid-conversation
- Load save
- Verify: no stale state, encounter re-initializes cleanly
- Verify: phrase state freshly initialized
- Verify: no resource leaks
- Verify: saved-game SIS display update step occurs before resumed encounter setup

#### Scenario 8: Repeated Encounters
**Validates**: CV-REQ-001 (resource leak check)
**Required evidence**:
- leak-check artifact from a repeatable tool/command
- scenario record entry

- Enter and exit 10+ encounters in sequence
- Collect a required leak artifact, e.g. `leaks`, `heaptrack`, platform-appropriate allocator diagnostics, or another repeatable command recorded with invocation details
- Verify: each encounter cleanup complete

#### Scenario 9: Abort/Skip Behavior
**Validates**: CV-REQ-009, CV-REQ-010, CV-REQ-012
**Required evidence**:
- ordered trace proving callback timing for skipped phrases
- replay-target trace proving replay lands on the correct phrase without duplicating history
- scenario record entry

- Skip phrases with cancel during talk segue
- Verify: phrase callbacks fire for skipped phrases
- Verify: summary shows all phrases including skipped
- Verify: replay after skip replays correct phrase

#### Scenario 10: Instant Victory/Defeat
**Validates**: SB-REQ-003, SB-REQ-004
**Required evidence**:
- segue outcome trace or log capture
- scenario record entry

- Encounter where setSegue(Victory) used
- Verify: BATTLE_SEGUE=1, instant_victory flag set, encounter resolves as victory
- Encounter where setSegue(Defeat) used
- Verify: crew sentinel, restart check, game-over flow

#### Scenario 11: Final Battle Restrictions
**Validates**: CV-REQ-006
**Required evidence**:
- capture/log showing summary access blocked
- scenario record entry

- Enter final battle conversation
- Verify: conversation summary (Cancel) is blocked

#### Scenario 12: Animation Behavior
**Validates**: AO-REQ-001–010, AO-REQ-016
**Required evidence**:
- capture/log of ambient/talk/transit transitions or equivalent deterministic trace
- scenario record entry

- Enter encounter with complex animations (multiple ambient + talk)
- Verify: ambient animations running during idle
- Verify: talk animation activates during speech
- Verify: transition animation plays between states
- Verify: BlockMask prevents animation conflicts
- Verify: WAIT_TALKING settles ambient during speech

### Level 3: Build-Mode and Entry-Point Routing Comparison

Compare Rust-mode behavior against C-mode behavior for the same encounter and for both public entry points. Results must be captured in a stable per-scenario record, not just described ad hoc.

```bash
# Build in C-only mode (disable USE_RUST_COMM)
# Run representative encounters, recording:
# - RaceCommunication() routing behavior
# - InitCommunication() routing behavior
# - Frame screenshots at key points
# - Subtitle text and timing
# - Response list contents
# - Animation frame sequences

# Build in Rust mode (enable USE_RUST_COMM)
# Run the same encounters, compare:
# - Same high-level routing outcomes for RaceCommunication() and InitCommunication()
# - Same subtitles at same times
# - Same response options
# - Same segue outcome
# - Visually similar animation (exact frame sequences may differ due to random timing)
```

Required artifact per compared encounter:
- one stable note/log file naming both build modes
- linked captures/screenshots if visual comparison is used
- explicit pass/fail statement for routing, subtitles, responses, segue, and animation parity

### Level 4: Stress/Edge Cases

- **Rapid skip**: skip through all phrases as fast as possible — no crash, all callbacks fire
- **Rapid encounter cycle**: enter/exit encounters rapidly — no resource leak
- **Maximum responses**: 8 responses registered — all visible and selectable
- **Oversized text**: very long subtitle text — wraps correctly, no overflow
- **Empty encounter**: encounter with no phrases, no responses — exits cleanly

Each stress/edge case exercised must also be logged in the scenario record or an auxiliary artifact with pass/fail and evidence location.

## Verification Commands Summary

```bash
# Rust
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features

# C build
cd /Users/acoliver/projects/uqm/sc2 && make clean && make

# Runtime/manual harness support
# Store callback/routing traces and scenario evidence under project-plans/20260311/comm/artifacts/p12/
# Record leak-check command invocations and outputs in leak-checks/
```

## Structural Verification Checklist
- [ ] All Rust tests pass
- [ ] C build succeeds
- [ ] No compiler warnings in changed files
- [ ] No clippy warnings
- [ ] All plan phase completion markers exist (P03 through P11a)
- [ ] P00.5 phrase-disable audit artifact exists and is referenced in final verification notes
- [ ] `scenario-results.md` exists with pass/fail and evidence links for every scenario
- [ ] callback/lifecycle traces captured for scenarios that exercise callback ordering
- [ ] leak-check artifact captured for repeated-encounter validation

## Semantic Verification Checklist (Mandatory)
- [ ] Scenario 1: Simple encounter works end-to-end
- [ ] Scenario 2: Hostile encounter with combat transition
- [ ] Scenario 3: Game-state-dependent dispatch
- [ ] Scenario 4: Number speech synthesis
- [ ] Scenario 5: Long conversation with summary
- [ ] Scenario 6: Phrase disable behavior correct
- [ ] Scenario 7: Save/load during encounter
- [ ] Scenario 8: Repeated encounters no leak
- [ ] Scenario 9: Skip/seek/replay behavior
- [ ] Scenario 10: Instant victory/defeat segues
- [ ] Scenario 11: Final battle restrictions
- [ ] Scenario 12: Animation correctness
- [ ] Both build modes preserve high-level `RaceCommunication()` behavior
- [ ] Both build modes preserve high-level `InitCommunication()` behavior
- [ ] Ordered evidence proves callback/lifecycle sequencing for normal exit, abort/load, and attack-without-hail paths
- [ ] Ordered evidence proves replay-target correctness after callback chains and skip/replay flows

## Final Requirement Closeout Matrix

Before signing off P12, produce a concise traceability artifact mapping each requirement family to its verification evidence:

- [ ] Encounter lifecycle (EC-REQ-*) → automated tests + scenarios + build-mode routing checks
- [ ] Dialogue script / dispatch (DS-REQ-*) → unit tests + script compilation + scenario coverage
- [ ] Phrase state (PS-REQ-*) → unit tests + P00.5 audit + scenario 6/7
- [ ] Responses (RS-REQ-*) → unit tests + scenarios 1/5/9
- [ ] Trackplayer / subtitles (TP-REQ-*, SS-REQ-*) → unit tests + scenarios 5/9 + summary verification
- [ ] Animation / speech graphics (AO-REQ-*) → unit tests + scenario 12
- [ ] Segue / battle outcomes (SB-REQ-*) → unit tests + scenarios 2/10
- [ ] Ownership / lifecycle / integration (OL-REQ-*, IN-REQ-*, CB-REQ-*) → unit tests + lifecycle scenarios + both-build-mode routing checks
- [ ] Compatibility validation (CV-REQ-*) → full scenario matrix + C-vs-Rust regression comparison

## Success Criteria
- [ ] All automated tests pass
- [ ] All 12 integration scenarios verified
- [ ] No regressions compared to C-only mode
- [ ] All 27 race encounters accessible and functional
- [ ] Resource lifecycle clean (no leaks after 10+ encounters)
- [ ] Verification evidence is reproducible and archived, not only described narratively

## Failure Recovery
- If any scenario fails: identify the specific phase where the bug was introduced, fix in that phase's files, re-verify from that phase forward
- If regression found: compare C-mode behavior to identify the divergence point

## Definition of Done

The communication subsystem port is complete when:

1. **Automated**: `cargo fmt`, `cargo clippy`, `cargo test` all pass
2. **Build**: Both `USE_RUST_COMM` and non-`USE_RUST_COMM` builds succeed
3. **Scripts**: All 27 race scripts compile without modification
4. **Runtime**: All 12 integration scenarios verified
5. **Entry points**: Both `RaceCommunication()` and `InitCommunication()` are verified in Rust mode and C fallback mode
6. **Regression**: No externally visible behavior difference from C-only mode
7. **Resources**: No leaks after repeated encounter cycles
8. **Callbacks**: Correct ordering verified for all exit paths (normal, abort, attack)
9. **Traceability**: Final requirement-family closeout matrix completed
10. **Evidence**: Scenario records, callback/routing traces, and leak-check artifacts are archived and referenced

## Phase Completion Marker
Create: `project-plans/20260311/comm/.completed/P12.md`
