# P13-P15: Port remaining race dialogues to Rust

## Worker scope

Port the remaining 17 race dialogue files from C to Rust, following the
Arilou reference pattern established in P12.

## Race list (by batch)

### P13 — Batch 1 (6 races)
| Race | C file | LoC | Conversation ID |
|---|---|---|---|
| Starbase/Commander | comm/starbas/starbas.c | ~1,200 | COMMANDER_CONVERSATION |
| Spathi (space) | comm/spathi/spathic.c | ~900 | SPATHI_CONVERSATION |
| Spathi (home) | comm/spahome/spahome.c | ~800 | SLYLANDRO_HOME_CONVERSATION |
| Orz | comm/orz/orzc.c | ~700 | ORZ_CONVERSATION |
| Ilwrath | comm/ilwrath/ilwrathc.c | ~600 | ILWRATH_CONVERSATION |
| Chmmr | comm/chmmr/chmmrc.c | ~600 | CHMMR_CONVERSATION |

### P14 — Batch 2 (6 races)
| Race | C file | LoC | Conversation ID |
|---|---|---|---|
| Melnorme | comm/melnorm/melnorm.c | ~900 | MELNORME_CONVERSATION |
| Mycon | comm/mycon/myconc.c | ~700 | MYCON_CONVERSATION |
| Pkunk | comm/pkunk/pkunkc.c | ~700 | PKUNK_CONVERSATION |
| Druuge | comm/druuge/druugec.c | ~600 | DRUUGE_CONVERSATION |
| Syreen | comm/syreen/syreenc.c | ~600 | SYREEN_CONVERSATION |
| Utwig | comm/utwig/utwigc.c | ~500 | UTWIG_CONVERSATION |

### P15 — Batch 3 (remaining races)
| Race | C file | LoC | Conversation ID |
|---|---|---|---|
| Ur-Quan | comm/urquan/urquanc.c | ~900 | URQUAN_CONVERSATION |
| Black Ur-Quan | comm/blackur/blackurc.c | ~800 | BLACKURQ_CONVERSATION |
| Vux | comm/vux/vuxc.c | ~700 | VUX_CONVERSATION |
| Yehat | comm/yehat/yehatc.c | ~700 | YEHAT_CONVERSATION |
| Yehat Rebel | comm/rebel/rebel.c | ~500 | YEHAT_REBEL_CONVERSATION |
| Shofixti | comm/shofixt/shofixt.c | ~500 | SHOFIXTI_CONVERSATION |
| Supox | comm/supox/supoxc.c | ~400 | SUPOX_CONVERSATION |
| Slylandro | comm/slyland/slyland.c | ~400 | SLYLANDRO_CONVERSATION |
| Slylandro Home | comm/slyhome/slyhome.c | ~400 | SLYLANDRO_HOME_CONVERSATION |
| Thraddash | comm/thradd/thraddc.c | ~400 | THRADD_CONVERSATION |
| Umgah | comm/umgah/umgahc.c | ~400 | UMGAH_CONVERSATION |
| Zoq-Fot-Pik | comm/zoqfot/zoqfotc.c | ~400 | ZOQFOTPIK_CONVERSATION |
| Talking Pet | comm/talkpet/talkpet.c | ~300 | TALKING_PET_CONVERSATION |

### Approach (per race)

Each race follows the P12 pattern:
1. Create `rust/src/comm/races/<race>.rs`
2. Implement `RaceDialogue` trait
3. Translate C state machine to Rust match arms
4. Keep same string indices and resource keys
5. Unit test state transitions
6. Update `init_race` dispatch in `dispatch.rs` to use Rust impl

### Test plan

**Per-race unit tests**:
- `init()` returns correct CommData
- State transitions match C behavior
- Resource keys match `resinst.h` values

### Dependencies
- P12 (Arilou reference implementation)