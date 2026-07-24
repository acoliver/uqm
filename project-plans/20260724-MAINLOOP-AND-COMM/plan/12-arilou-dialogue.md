# P12: Port Arilou dialogue state machine to Rust

## Worker scope

Port the Arilou race dialogue from C (`comm/arilou/arilouc.c`, 855 lines)
to Rust as the reference implementation for per-race dialogue porting.
Now that Rust owns CommData (P10), the dialogue populates Rust data directly.

### What to port

1. **`arilou_desc` LOCDATA struct** (lines 31-160)
   - Animation descriptors (20 ambient animations, transition, talk)
   - Resource keys: `ARILOU_PMAP_ANIM`, `ARILOU_FONT`, `ARILOU_COLOR_MAP`, `ARILOU_MUSIC`
   - Color init values, text alignment, text baseline/width
   - Function pointers: `init_encounter_func`, `post_encounter_func`, `uninit_encounter_func`

2. **`init_arilou_comm()`** (lines 830-855)
   - Sets function pointers on arilou_desc
   - Sets text baseline/width
   - Checks game state to determine segue (peace vs hostile)

3. **Dialogue state machine functions** (lines 231-825)
   - `Intro()` — initial greeting, branches on game state
   - `ArilouHome()`, `AngryHomeArilou()`, `AngrySpaceArilou()`, `FriendlySpaceArilou()`
   - `ExitConversation()` — cleanup
   - `post_arilou_enc()` — post-encounter processing
   - `uninit_arilou()` — uninit cleanup

### Key insight: text is already externalized

All dialogue text lives in resource files, loaded by string index at runtime.
The C files contain only:
- String indices (enum values from `strings.h`)
- Resource keys (from `resinst.h`)
- Branching logic (if/else on game state → call NPCPhrase(index))

This means the port is mechanical: translate C state machine to Rust match arms,
keeping the same string indices and resource keys.

### Approach

- Create `rust/src/comm/races/mod.rs` with `RaceDialogue` trait
- Create `rust/src/comm/races/arilou.rs` implementing `RaceDialogue`
- Translate C state machine to Rust match arms
- `NPCPhrase(index)` → `CommState::npc_phrase(index)` (already exists in ffi.rs)
- Response handling → `ResponseSystem::do_response(index, callback)` (already exists)
- Update `init_race` dispatch in `dispatch.rs` to use Rust Arilou impl

### Test plan

**Unit tests** (in `arilou.rs`):
- `init()` returns correct CommData with Arilou resource keys
- `init()` sets correct segue based on game state (reads from Rust-owned state)
- `intro()` speaks correct opening line based on game state
- State transitions match C behavior for each response path

**Automation proof** (extend `comm-encounter-v1.json`):
- After reaching IN_ENCOUNTER, wait for dialogue
- Capture dialogue screen
- Tap through first response
- Capture after response
- Finish

### Dependencies
- P11 (comm dispatch must be in Rust to wire the race dialogue)

### Files to create/modify
- CREATE: `rust/src/comm/races/mod.rs`
- CREATE: `rust/src/comm/races/arilou.rs`
- MODIFY: `rust/src/comm/mod.rs` (add `pub mod races`)
- MODIFY: `rust/src/comm/dispatch.rs` (wire Arilou init to Rust instead of C FFI)