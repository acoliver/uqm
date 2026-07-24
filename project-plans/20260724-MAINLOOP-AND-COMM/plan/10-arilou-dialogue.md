# P10: Port Arilou dialogue state machine to Rust

## Worker scope

Port the Arilou race dialogue from C (`comm/arilou/arilouc.c`, 855 lines)
to Rust as the reference implementation for per-race dialogue porting.

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
   - `ArilouHome()` — homeworld dialogue
   - `AngryHomeArilou()`, `AngrySpaceArilou()`, `FriendlySpaceArilou()` — branching responses
   - `ExitConversation()` — cleanup
   - `post_arilou_enc()` — post-encounter processing
   - `uninit_arilou()` — uninit cleanup

### Approach

- Create `rust/src/comm/races/arilou.rs`
- Define `ArilouDialogue` implementing a `RaceDialogue` trait:
  ```rust
  pub trait RaceDialogue {
      fn init(&self) -> CommData;
      fn intro(&self, response: ResponseRef);
      fn exit(&self, response: ResponseRef);
      fn post_encounter(&self);
      fn uninit(&self);
  }
  ```
- Translate the C state machine to Rust match arms
- Keep the same string indices (from `strings.h` enum) and resource keys
- The `NPCPhrase(index)` calls become `CommState::npc_phrase(index)`
- Response handling becomes `ResponseSystem::do_response(index, callback)`

### Test plan

**Unit tests** (in `arilou.rs`):
- `init()` returns correct CommData with Arilou resource keys
- `init()` sets correct segue based on game state
- `intro()` speaks correct opening line based on game state
- State transitions match C behavior for each response path
- `exit()` cleans up correctly

**Automation proof** (extend `comm-encounter-v1.json`):
- After reaching IN_ENCOUNTER, wait for dialogue to start
- Capture the dialogue screen
- Tap through first response
- Capture after response
- Finish

### Dependencies
- P09 (comm dispatch must be in Rust to wire the race dialogue)
- Existing `comm/state.rs`, `comm/response.rs`, `comm/animation.rs`

### Files to create/modify
- CREATE: `rust/src/comm/races/mod.rs`
- CREATE: `rust/src/comm/races/arilou.rs`
- MODIFY: `rust/src/comm/mod.rs` (add `pub mod races`)
- MODIFY: `rust/src/comm/dispatch.rs` (wire Arilou init to Rust instead of C FFI)