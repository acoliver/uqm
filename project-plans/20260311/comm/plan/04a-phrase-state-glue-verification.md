# Phase 04a: Phrase State & Glue Layer Verification

## Phase ID
`PLAN-20260314-COMM.P04a`

## Prerequisites
- Required: Phase 04 completed

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] `rust/src/comm/phrase_state.rs` exists
- [ ] `rust/src/comm/glue.rs` exists
- [ ] `rust/src/comm/segue.rs` exists
- [ ] All three modules registered in `mod.rs`
- [ ] CommState extended with phrase_state and segue fields
- [ ] FFI exports in ffi.rs for phrase state, glue, and segue functions

## Semantic Verification Checklist

### Phrase State Tests
- [ ] `test_phrase_state_initial_all_enabled` — fresh state, all phrases enabled
- [ ] `test_phrase_state_disable_single` — disable one phrase, verify disabled
- [ ] `test_phrase_state_disable_multiple` — disable several, others still enabled
- [ ] `test_phrase_state_reset_clears` — reset after disable, all re-enabled
- [ ] `test_phrase_state_no_reenable` — no API to re-enable single phrase
- [ ] `test_phrase_state_encounter_local` — verify clear() resets phrase state

### Glue Layer Tests
- [ ] `test_npc_phrase_zero_noop` — index 0 produces no output
- [ ] `test_npc_phrase_normal_resolves` — normal index resolves text from phrases
- [ ] `test_npc_phrase_player_name` — GLOBAL_PLAYER_NAME substitution
- [ ] `test_npc_phrase_ship_name` — GLOBAL_SHIP_NAME substitution
- [ ] `test_npc_phrase_negative_alliance` — negative index alliance name
- [ ] `test_npc_phrase_cb_with_callback` — callback stored for later dispatch
- [ ] `test_npc_phrase_splice_no_page_break` — verify no page break
- [ ] `test_npc_number_with_table` — number decomposition with speech table
- [ ] `test_npc_number_text_only` — no speech table, text-only output
- [ ] `test_construct_response_fragments` — multiple fragments concatenated
- [ ] `test_construct_response_zero_skip` — fragment 0 skipped

### Segue Tests
- [ ] `test_segue_peace` — Peace sets BATTLE_SEGUE=0
- [ ] `test_segue_hostile` — Hostile sets BATTLE_SEGUE=1
- [ ] `test_segue_victory` — Victory sets BATTLE_SEGUE=1, instantVictory=true
- [ ] `test_segue_defeat` — Defeat applies crew sentinel and restart
- [ ] `test_get_segue_default` — default is Peace
- [ ] `test_segue_round_trip` — set then get matches

## Deferred Implementation Detection

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/comm/phrase_state.rs rust/src/comm/glue.rs rust/src/comm/segue.rs
```

## Phase Completion Marker
Create: `project-plans/20260311/comm/.completed/P04a.md`
