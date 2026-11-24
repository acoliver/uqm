# Interaction subsystems design audit: comm, content comm, front_end, ui, starbase

Pin: `678f8c11defe28a85cfccac29899410209e9963c` (acoliver/uqm). Read-only audit. No builds, no runtime edits, no tests run.

Path convention: `path:line` is relative to `rust/crates/`. A pinned link is
`https://github.com/acoliver/uqm/blob/678f8c11defe28a85cfccac29899410209e9963c/rust/crates/<path>#L<line>`.
Examples:
[modes/src/comm/ffi.rs#L155](https://github.com/acoliver/uqm/blob/678f8c11defe28a85cfccac29899410209e9963c/rust/crates/modes/src/comm/ffi.rs#L155),
[modes/src/comm/talk_segue.rs#L139](https://github.com/acoliver/uqm/blob/678f8c11defe28a85cfccac29899410209e9963c/rust/crates/modes/src/comm/talk_segue.rs#L139),
[modes/src/starbase/mod.rs#L9](https://github.com/acoliver/uqm/blob/678f8c11defe28a85cfccac29899410209e9963c/rust/crates/modes/src/starbase/mod.rs#L9).

C behavior references below cite `sc2/src/uqm/comm.c` as it stood at
`90079522487c7b06dd4339230f91aedde95c4e79^` (the commit before it was deleted).

## 1. Verdict

The interaction code is Rust that still thinks in C. The modes crate forbids
`unsafe` (`modes/src/lib.rs:3`) and holds no process globals of its own, which is
good. The shape is the problem. The conversation loop exists twice, once for
production and once for `cfg(test)`, and the unit tests exercise the copy the game
never runs. The starbase family has a second, test-only implementation of all
three screens. Dialogue scripts reach the engine through 28 hand-written
trampoline files and a function-pointer vtable indexed by enum discriminant.
Integers stand in for booleans, menus, segues, activities and intro modes
nearly everywhere, even where typed versions already exist in the engine.

Reading the production path against the C source turned up four likely
behavior defects. Each one sits in a place the cfg split keeps unit tests away from (section 4).

## 2. Coverage

Every tracked `.rs` file under these roots was listed and classified. Files were
read in full where they sit on the production path, or skimmed down to their
signatures where they are leaf data or tests.

| Family | Files | Lines | `#[test]` | Read depth |
|---|---|---|---|---|
| `modes/src/comm` (incl. `live/`, `ffi/`, `dispatch/`, `host/`, `talk_segue/`, `lifecycle/`) | 78 | 18,054 | 322 | Production path read line by line (dispatch, hail, ffi, speech, response_commands, talk_segue, comm_abi, commglue_abi, commanim_abi signatures, oscill_abi signatures, sis_graphics, state, locdata, types, segue, phrase_state, input, assets, speech_graphics, response_ui, subtitle_display, track, host, lifecycle, two live adapters). Other live adapters and tests skimmed. |
| `content/src/comm/races` | 141 | 33,131 | 268 | `script.rs`, `mod.rs`, `encounter.rs`, `descriptor.rs`, `starbas/api.rs`, `ilwrath.rs` read. Every race checked by pattern counts (handler signatures, API traits, C-translation idioms). Test harness `encounter/testing.rs` read. |
| `modes/src/front_end` | 47 | 10,440 | 86 | `intro.rs`, `intro/records.rs`, `intro/commands.rs` head, `credits.rs`, `credits/render.rs` hook, `gameopt.rs` runtime, `setupmenu.rs` head, `cnctdlg.rs` and `uqmdebug.rs` signatures. |
| `modes/src/ui` | 39 | 8,512 | 58 | `mod.rs`, `menu_state.rs`, `menu.rs`, `settings.rs`, `session.rs`, `confirm.rs`, `util.rs`, `getchar/editor.rs` head, `starbase.rs`, `outfit.rs` fuel path, `shipyard.rs` head, `sis*` signatures, capacity math. |
| `modes/src/starbase` | 23 | 10,903 | 213 | `mod.rs`, `starbase_menu.rs`, `visit.rs`, `platform.rs` head, `screen.rs` entry points, `sis_modules.rs` capacity, reachability of every module from outside the directory. |

Out of scope but touched as dependencies: `engine/src/sound/trackplayer.rs`
(process-global `TRACK_STATE`, line 136), `engine/src/sound/heart_ffi.rs`,
`engine/src/input.rs` (`Frame`, `run_input_frame`), `src/mainloop/input_driver.rs`
(the production `InputRuntime`), `modes/src/mainloop/restart_menu` (main menu
caller of `fmv` and `setupmenu`).

## 3. Production traces

### 3.1 Encounter to conversation

1. `comm::dispatch::race_communication` (`modes/src/comm/dispatch.rs:194`) picks a
   conversation id from `u32` tables (`RACE_COMMUNICATION`, `RACE_SHIP_FOR_COMM`,
   `dispatch.rs:106-164`) and calls `init_communication` (`:288`), which returns `u16`
   0/1 for "combat selected".
2. `run_encounter` (`:374`) builds `Comm::with_options` (`:399`), loads the
   descriptor through `lifecycle::session` (`dispatch.rs:516-534`,
   `lifecycle.rs:308`), then either `hail::hail_alien` or `finish_callbacks`.
   `lifecycle::for_conversation(..).unwrap_or(DialogueKind::Chmmr)`
   (`dispatch.rs:531`) silently treats an unknown id as Chmmr.
3. `hail::hail_alien` (`modes/src/comm/hail.rs:127`) calls `prepare` (`:22`, panics
   on error at `:131-132`), runs the race intro callback, then
   `comm.run_input(.. rust_do_communication(..) != 0)` (`:148-150`), then `finish`
   (`:78`) and completion admission.

### 3.2 One frame of the dialogue loop (production)

`ffi::rust_do_communication` (`modes/src/comm/ffi.rs:155-293`) returns `c_int`:

- Speech phase: `ffi/speech.rs:10-76`. First call runs
  `talk_segue::alien_talk_first_call_init` (`talk_segue.rs:761`). Then
  `talk_segue::dinput::run_talk_segue_dinput` (`talk_segue.rs:242-310`), a nested
  input loop that ports C `TALKING_STATE` with `c_int` booleans
  (`talk_segue.rs:155-161`) and paces itself with `sleep_until` plus
  `ONE_SECOND/60` (`:286-287`).
- Response phase: abort check, observer checkpoints, then direct
  `pulsed().menu[...]` reads for select, cancel, up, down and left
  (`ffi.rs:215-219`), each branch calling a `c_bridge::call_*` wrapper
  (`talk_segue.rs:59-137`). Selection goes through
  `response_commands::deliver_selected` (`ffi/response_commands.rs:55`), which
  calls the shared `talk_segue::select_response` (`talk_segue.rs:660`) and then
  invokes the stored `ResponseFunc` (`fn(&mut Comm, &mut Cx, u16)`,
  `comm/response.rs:7`).
- No responses: `run_last_replay_bridge` into `dinput::run_last_replay_dinput`
  (`talk_segue.rs:371`).

Per-frame `eprintln!("[DBG] ...")` remain in the production loop
(`ffi.rs:170`, `ffi/speech.rs:19`, `ffi/response_commands.rs:14,29,74`).

### 3.3 Branching dialogue

A race script is a `Dialogue<'a, A>` struct in `uqm-content`
(e.g. `content/src/comm/races/ilwrath.rs:18-31`) with a `Handler` enum and an
inherent `handle(handler, r)` method. The script talks to the game through
`DialogueHost<H>` (`content/src/comm/races/script.rs:19-42`), optional
`EncounterApi<H>` (`encounter.rs:5-14`) and one per-race API trait
(20 traits, for example `starbas/api.rs`).

The modes side bridges this with:

- `comm::host::Host` (`modes/src/comm/host.rs:18-22`), holding
  `RefCell<&mut Comm>` and `RefCell<&mut Cx>` so `&self` trait methods can
  borrow mutably (`live/starbas.rs:130-139`).
- One `live/<race>.rs` per race (28 files, 2,210 non-test lines) that converts each
  `Handler` variant into a free `fn(&mut Comm, &mut Cx, u16)` trampoline
  (`live/ilwrath.rs:9-36`), because `ResponseFunc` is a bare function pointer.
- `lifecycle::LIFECYCLES` (`modes/src/comm/lifecycle.rs:19-216`), a 27-entry array of
  function-pointer structs indexed by `DialogueKind as usize` (`:301`, `:309`),
  with `COUNT = 27` set by hand (`:251`) and a second positional table
  `CONVERSATIONS` (`:254-284`).
- `SessionData` and its copy `CommData` hold those pointers as `Option<fn>`
  (`locdata.rs:36-41`, `types.rs:150-158`). `CommData` also stores resource keys as
  `*const c_char` (`types.rs:163`, built by `locdata.rs:52-54`) inside a crate that
  forbids `unsafe`. Its only remaining readers are tests (`dispatch/encounter.rs:106,149`).

### 3.4 Presentation loops

- Intro and ending slides: `front_end::intro::show_presentation`
  (`modes/src/front_end/intro.rs:158`) runs `cx.run_input` with `presentation_frame`
  (`:91`). Each frame re-parses the next string-table entry with byte-slice
  opcode matching (`intro/commands.rs:9-34`, `:42-90`) and paces itself with the
  global `get_time_counter()` (`intro.rs:65`). State lives in a flat
  `PresentationState` with `-1` sentinels (`intro/records.rs:6-34`, `:54-58`).
- Video: `show_video` (`intro.rs:218`), `video_input` (`:180`).
- Credits: `front_end::credits::credits` (`credits.rs:70`). It plays 15 outtake
  conversations through `init_communication_with_options` (`:30-66`) while an input
  hook keeps the credits scrolling (`credits/render.rs:456-466`, shared
  `Rc<RefCell<Session>>` and `Rc<Cell<CRect>>`).
- `fmv` wraps splash, intro, victory and ship spin (`front_end/fmv.rs:20-89`).

### 3.5 Menu loops

- In-flight game menu: `gameopt::game_options` (`modes/src/front_end/gameopt.rs:245`)
  returns `u32`, keeps its menu state as `&mut u8` with C item ids
  (`:52-68`), and steps through `ui::menu::choose_menu_state`
  (`modes/src/ui/menu.rs:20-49`, returns `u32`) over the `PM_*` frame-index
  constants (`ui/menu_state.rs:9-64`) with arithmetic stepping (`:103-120`).
- Setup menu: `front_end::setupmenu` with `menu.choices[20]`-style magic
  indices (`setupmenu.rs:74-79`, `:113`).
- Starbase: `ui::starbase::run_starbase_visit` (`modes/src/ui/starbase.rs:261`),
  `ui::outfit::run` (`ui/outfit.rs:669`), `ui::shipyard::run` (`ui/shipyard.rs:782`),
  injected through `ui::MenuServices` (`ui/session.rs:57-71`).
- Confirm and pause: `ui::confirm::do_confirm_exit` (`ui/confirm.rs:117`) and
  `ui::util::pause_game` (`ui/util.rs:122`) run their own sleep loops outside
  `run_input`. `pause_game` spins on `begin_input_frame()` and
  `yield_game_thread()` (`util.rs:147-156`).

## 4. Likely behavior defects on the production path

These come from reading Rust against `comm.c`. Each needs one focused headless
check before anyone fixes it. The parity goldens passing does not rule them out,
because the goldens are Rust-generated and do not cover these branches.

1. **Response list never scrolls.** C `PlayerResponseInput` sets
   `top_response = response` when the highlighted line falls below
   `SIS_SCREEN_HEIGHT`, and resets it to 0 when moving above the top
   (comm.c:1036-1045). In production Rust the only write is
   `comm.top_response = Some(0)` (`modes/src/comm/ffi.rs:200`). Up and down keep the
   old top (`ffi.rs:267-287`). A list taller than the window leaves the selection
   off screen. The test-only path admits it only "approximates" this
   (`talk_segue.rs:857-861`).
2. **Player statement feedback shows the wrong text.** C keeps the last chosen
   phrase in `phrase_buf` (comm.c:943) and shows it under "(In response to your
   statement)" on replay and in the summary (comm.c:965, 980). Rust replay passes
   `None` (`talk_segue.rs:120-123`, called from `ffi.rs:248`), so no statement is
   drawn. Rust summary shows the currently highlighted option of the current list
   (`sis_graphics.rs:483-487`, using `response_display`), not the phrase the player
   said. No Rust field holds the last chosen phrase.
3. **Fade durations shorter than C.** `select_response` calls
   `fade_music_to_background` with `60` ticks labelled "ONE_SECOND"
   (`talk_segue.rs:1269-1282`). C uses `ONE_SECOND` = 840 (comm.c:950). Replay fades
   to background in 0 ticks (`fade_music_to_background_bridge`,
   `talk_segue.rs:958-960`, called from `ffi.rs:247`), where C uses 840 (comm.c:978).
4. **Response phase has no frame pacing.** C `PlayerResponseInput` ends with
   `SleepThreadUntil(NextTime)` at `COMM_ANIM_RATE` (comm.c:1050-1051). The Rust
   response branch never sleeps (`ffi.rs:197-292`), and the production `run_loop`
   only yields (`src/mainloop/input_driver.rs:5-40`). The loop spins as fast as it
   can. Animation stays time-based, but CPU use and presentation cadence differ.

Lower-confidence observations, worth a test while the code is open:

- Speech widgets rate-limit on host wall-clock milliseconds except during
  replay (`speech_graphics.rs:12-18`, `ffi.rs:311-316`). C rate-limits on game ticks
  (`OSCILLOSCOPE_RATE`). Live and replay therefore follow different clocks.
- `response_display` (`state.rs:30`) is never reset between response rounds
  (`state.rs:215-236` leaves it alone), so the no-response summary path can redraw
  a stale list (`sis_graphics.rs:499-502`).
- The test-only scroll-mode helper treats `smooth_scroll == 0` as Page
  (`talk_segue.rs:1060-1067`). Production treats `== OPT_PC (2)` as Page
  (`talk_segue.rs:149,195`). The unit tests check the opposite meaning of the setting
  from the one the game uses.
- `CommIntroMode` variant names do not match their values. 0 is CIM_CROSSFADE_SPACE but is
  named `Default` ("fade in"), 3 is FADE_IN_SCREEN but named `Immediate`
  (`types.rs:116-138`). Behavior is correct only because `intro_transition` matches
  the raw `u32` (`talk_segue.rs:1323-1334`).

## 5. Duplicate paths: production versus test

| Concern | Production path | Second path | Second path reachable from production? |
|---|---|---|---|
| Talk segue | `talk_segue::dinput::{run_talk_segue_dinput, do_talk_segue_frame}` (`talk_segue.rs:139-310`) | `talk_segue::{talk_segue, do_talk_segue, alien_talk_segue}` (`:478-644`) with `#[cfg(test)]` simulation helpers (`:977-1415`) | No. `ffi::rust_alien_talk_segue` picks by cfg (`ffi.rs:82-108`). |
| Response phase | `ffi::rust_do_communication` (`ffi.rs:155`) | `talk_segue::{do_communication, do_communication_responses, player_response_input}` (`:695-842`) | No. Only `select_response` is shared. |
| Conversation summary | `sis_graphics::select_conversation_summary` over `trackplayer::subtitle_history()` (`sis_graphics.rs:478-505`) | `ffi::rust_show_conversation_summary` `cfg(test)` over `summary::SummaryView` (`ffi.rs:367-400`); `Comm::rebuild_summary` from `TrackManager` (`state.rs:555-563`) | `rebuild_summary` runs in production on every `npc_phrase` (`ffi.rs:488,513`) and rebuilds from an empty `TrackManager`. Dead work. |
| Track model | Engine `trackplayer` global via `heart_ffi` | `comm::track::TrackManager` with `f32` seconds (`track.rs:177`); `CTrackBridge` (`track.rs:21-106`) has no callers | No. |
| Subtitles | `comm_abi::{check_subtitles, comm_redraw_subtitles}` (`comm_abi.rs:130-166`) | `subtitle::SubtitleTracker` (`subtitle.rs:70`), `subtitle_display::SubtitleDisplay` (`subtitle_display.rs:14`), `Comm::prev_subtitle` (`state.rs:103`) | `SubtitleDisplay::redraw` forwards to `comm_redraw_subtitles` and ignores its own state (`subtitle_display.rs:61-65`). |
| Oscilloscope | `oscill_abi::runtime` (`oscill_abi.rs:7-155`) | `oscilloscope::Oscilloscope` (`oscilloscope.rs:13`) | No. |
| Speech widget rate | `SpeechGraphics::update` with wall-clock ms (`speech_graphics.rs:107`) | `talk_segue::update_speech_graphics` with ticks and `next_speech_time` (`talk_segue.rs:1094-1109`) | No. Two rate limiters with two clocks. |
| Response scroll state | `Comm::top_response: Option<u8>` and `response_display: [u8; 3]` | `ResponseUI` with a hard-coded `MAX_VISIBLE = 4` (`response_ui.rs:93-108`) | `refresh_response_ui` is test-only (`state.rs:517-529`). |
| Encounter lifecycle | `dispatch::encounter::finish_callbacks` (`dispatch/encounter.rs:12`) | `comm::encounter::{EncounterState, CallbackTracker, begin_encounter, end_encounter_*}` (`encounter.rs:31-188`) | No callers outside the file. |
| Segue | World `BATTLE_SEGUE` + `kernel.instant_victory` + crew sentinel + activity bit `0x2000` (`commglue_abi.rs:60-88`) | `Comm::segue: Segue` (`state.rs:69`, `ffi.rs:62`) | Written, never read in production. |
| Comm state flags | `talking_finished`, `first_talk_call` | `talking`, `initialized`, `input_paused`, `Comm::init/uninit/update(f32)` (`state.rs:181-196`, `:566-585`) | `update(f32)` and `init` are test-only. |
| Starbase menu | `ui::starbase::update_starbase_screen` (`ui/starbase.rs:83`) | `starbase::starbase_menu::StarbaseMenu::step` (`starbase_menu.rs:140`), `starbase::screen::run_starbase_menu` (`screen.rs:584`) | Only `StarbaseState` is shared. |
| Outfit and shipyard screens | `ui::outfit`, `ui::shipyard` (+ `ui/shipyard/*`, `ui/outfit_state.rs`) | `starbase::screen::{run_outfit, run_shipyard}`, `starbase::shipyard_screen`, `starbase::outfit_menu`, `starbase::module_screen` over a mocked `starbase::platform::Platform` | No. `starbase/mod.rs:9-16` says so in its own doc. |
| Fuel purchase | `ui::outfit::change_fuel_quantity` (`ui/outfit.rs:423`) | `starbase::fuel::change_fuel_quantity` (`starbase/fuel.rs:117`) | No. Same rule written twice. |
| Arrival time passage and cleanup | `ui::starbase::{do_time_passage, cleanup_after_star_base}` (`ui/starbase.rs:158-172`) | `starbase::visit::{entry_time_passage, cleanup_after_starbase}` (`visit.rs:37,63`), `screen::do_time_passage` (`screen.rs:593`) | `cleanup_after_starbase` is used only by a test in `src/mainloop/state_machine.rs:378-392`. |
| SIS capacities | `ui::sis::capacity` over raw `u8` slots (`ui/sis/capacity.rs:14,61-85`), `ui::sis_capacity_math` with literal 50/500 (`sis_capacity_math.rs:5-17`) | `starbase::sis_modules::ModuleSlots::fuel_tank_capacity` (`sis_modules.rs:189`) | Partially. Both use `ModuleType::fuel_capacity`, but aggregation is written twice. |
| Starbase error type | none | `starbase::StarbaseError` wired into `src/mainloop/mod.rs:57` | Never constructed in production. |

Size of the second paths: comm shadow files (`track`, `subtitle`, `oscilloscope`,
`summary`, `encounter`, `subtitle_display`, `response_ui`, `ffi_behavior_tests`)
total about 3,100 lines, plus about 1,000 lines of test-path functions and source-scanning tests inside
`talk_segue.rs` and `ffi.rs`. The starbase shadow (`screen`, `shipyard_screen`,
`outfit_menu`, `module_screen`, `platform`, `fuel`, `input`, `test_support`, `tests`,
`shipyard_tests`, `module_route_tests`, `new_game_reference`) totals about 6,800 lines.

Tests that read their own source with `include_str!` and assert which text
appears inside `cfg` blocks (`talk_segue.rs:2148-2430`) exist only to police the
split. They go when the split goes.

## 6. Findings by family

### 6.1 `modes/src/comm`

- **God object.** `Comm` (`state.rs:21-104`) has 30 fields mixing three generations:
  C-port runtime blocks named `*_abi::RuntimeState` (`comm_abi`, `oscill_abi`,
  `commanim_abi`), plan-era trackers (`TrackManager`, `SubtitleTracker`,
  `Oscilloscope`, `SpeechGraphics`, `ResponseUI`, `SubtitleDisplay`,
  `ConversationSummary`, `EncounterState`), and loose flags. Accessors for most of
  them exist only for tests (`state.rs:264-400`, `:474-553`).
- **C file names on Rust modules.** `ffi.rs`, `comm_abi.rs`, `commanim_abi.rs`,
  `commglue_abi.rs`, `oscill_abi.rs`, `glue.rs`, `c_bridge` (`talk_segue.rs:59`),
  `rust_*` function prefixes (15 of them, e.g. `ffi.rs:34,62,75,155,303,311,326`),
  `extern "C" {}` with no body (`host.rs:15`). There is no C left to bridge to.
- **Stale concurrency story.** Module docs and comments describe a `COMM_STATE`
  lock and dropping it before C calls (`mod.rs:15-18`, `talk_segue.rs:8-12`,
  `ffi.rs:148-151`, `:196-201`, `talk_segue.rs:913-915`). No such lock exists. Phrase
  callbacks queue through `Arc<parking_lot::Mutex<Vec<fn(&mut Comm)>>>`
  (`state.rs:18`, `:150-165`) because the engine trackplayer callback is
  `Box<dyn Fn(i32) + Send>`. That boundary is the only real cross-thread point.
- **Integer contracts.** `c_int` 0/1 returns (`ffi.rs:75-81`, `:155-160`,
  `response_commands.rs:13`), `c_uint` segue codes (`ffi.rs:62`), `i32` from
  `clear_subtitles_pending` (`comm_abi.rs:173`), `i32` `full` and `paused` arguments
  (`commanim_abi.rs:280`, `:426`), `ResponseSystem::selected() -> i32` with `-1`
  for none (`response.rs:54-65`), phrase sentinels `-1_000_000` and `-999_999`
  (`ffi.rs:408-411`, `content/.../script.rs:9-10`), raw activity bits
  (`talk_segue.rs:150,182,329,917-943`; `hail.rs:61,134`), `glob_flags & 8` for music
  (`talk_segue.rs:621,762`). The engine already has `ActivityKind`,
  `ActivityValue` and named `activity_flags` (`engine/src/world/activity.rs:64,171,236-263`).
- **Four copies of animation flags.** `types::anim_flags` (`types.rs:8-21`), private
  consts in `commanim_abi.rs:6-13`, `script::ANIM_DISABLED` (`content/.../script.rs:6`),
  and bare literals 8, 16, 32, 64 (`talk_segue.rs:1195-1225`, `state.rs:417-423`).
- **Three copies of intro mode.** `CommIntroMode` (misnamed, `types.rs:116`),
  `dispatch::cim` (`dispatch.rs:540-545`), local consts in `ui/starbase.rs:21-22`, plus
  `CommOptions::intro: (u32, u32)` (`comm/mod.rs:87`).
- **C strings in Rust.** Response text is built as `CString` with
  `NonZeroU8` filtering and a 2048-byte assertion that copies the old C shared
  buffer (`host.rs:38-71`). The alliance name goes through a fixed 400-byte
  NUL-terminated buffer (`ffi.rs:411-470`). Subtitles go through `CString` per line
  (`comm_abi.rs:68-77`). The engine text API takes `&CStr`, which pushes this onto
  every caller.
- **Out-parameter through shared cell.** `CommOptions::window:
  Option<Rc<Cell<CRect>>>` (`mod.rs:86`) exists so `hail::prepare` can write the
  window rect back for the credits (`hail.rs:56-58`).
- **Errors.** `CommError` (`types.rs:71-108`) is hand-rolled and used only by the
  test-path API (`start_track`, `init`, `encounter.rs`). Production uses
  `panic!` for asset and invariant failures (`hail.rs:132`, `:84-102`,
  `talk_segue.rs:1364-1371`) and `Result<(), String>` (`hail.rs:27`). Panicking on
  broken invariants matches the fail-fast preference. The `String` error and
  the unused enum do not.
- **Kept facilities.** `comm::input::ConversationFrame` and `Comm::run_input`
  (`input.rs:10-55`) put every conversation loop through the engine `Frame` trait
  and the observer. `commanim_abi::Sequence` is a clean, tested animation state
  machine (`commanim_abi.rs:16-180`) with an injected RNG. `fresh_press::FreshPressLatch`
  (`talk_segue/fresh_press.rs`) is a typed edge detector. `hail::finish` keeps the
  resource release order explicit.

### 6.2 `content/src/comm` (race scripts)

- **Shape worth keeping.** Scripts are pure Rust over a capability trait, with a
  per-race `Handler` enum and an effect-recording `Fake` host for tests
  (`encounter/testing.rs:6-35`). That is the right seam between content and modes.
- **C translation idioms left in.** 112 post-increment blocks
  (`let res = { let old = x; x += 1; old }`, e.g. `ilwrath.rs:68-74`), 27
  declare-then-assign pairs (`ilwrath.rs:113-114`), 14 triple-paren boolean chains
  (`ilwrath.rs:63-66`), 917 `npc(i32::from(PHRASE))` calls, 159 `segue(<digit>)` calls
  with raw codes, 34 `activity()` magic comparisons (`ilwrath.rs:35`).
- **Phrases are untyped.** 1,926 `pub(super) const NAME: u16` phrase ids across the
  races. `npc` takes `i32` so it can also carry name sentinels. Response fragments
  are `&[i32]` (`script.rs:23`).
- **Inconsistent entry points.** `handle` takes `r: i32` in 14 races and `r: u16` in
  13. Post hooks are named per race (`post_ilwrath_enc`, `post_starbase_enc`).
  Commander is a tuple struct (`comandr.rs`). There is no shared script trait, so
  modes needs the trampolines and the vtable from 3.3.
- **Scripts own frame timing.** `StarbaseApi` exposes `now`, `sleep`,
  `sleep_until`, `task_switch` and `skip_sale` (`starbas/api.rs:36-53`). The mineral
  sale animation blocks inside a response callback. The engine `random()` global
  feeds `random` (`modes/src/comm/live/starbas.rs:143-145`).
- **Capability overlap.** `date` appears in 7 API traits, `number` in 5,
  `intro_mode` in 4, `random` in 3. These belong in `DialogueHost`.
- Stale module docs describe C FFI (`races/mod.rs:9-14`, `script.rs:1`).

### 6.3 `modes/src/front_end`

- **Intro interpreter.** The slide script is an external format (justified),
  but it is parsed again on every pass, opcode by opcode, from byte slices
  (`intro/commands.rs:9-34`). A Rust version parses the table once into
  `Vec<Command>` with parse errors reported, then runs it.
  `PresentationState` (`intro/records.rs:6-34`) should split into a parsed
  script, a cursor, and a render state with `Option` in place of `-1`.
- **Game options.** Menu items are `u8` consts (`gameopt.rs:52-68`), outcome codes
  `MENU_SUCCESS/FAILURE: u16` (`:49-50`), `pick_game(.., u32, u32) -> u32`
  (`:217`, `:260`), `game_options -> u32` (`:249`). Callers write `!= 0`
  (`src/app/mod.rs:32`). `gameopt/picker.rs` already has a typed `PickerAction` that
  shows how the rest should look.
- **Credits.** The outtake reel and the scroll hook are justified behavior. The
  mechanism is an input callback plus `Rc<RefCell<Session>>` and `Rc<Cell<CRect>>`
  (`credits/render.rs:451-473`, `credits.rs:85-104`). Comm should accept a
  background presenter in `CommOptions`, and the credits should stop sharing
  mutable cells with it.
- **Setup menu.** Positional `choices[20]`, `options.res == 6`, `counts[0] = 6 or 2`
  (`setupmenu.rs:74-79`, `:113`). `widgets::{Event, Action, MenuState}` is the typed
  part to grow.
- **`cnctdlg`** already uses `Event` and `Action` enums (`cnctdlg.rs:64-135`) and
  works as the in-repo example of the target style. `uqmdebug` is gated behind a feature
  (`uqmdebug.rs:2`) and uses `io::Write`. Low priority.
- Repeated helpers: `rgb15` is defined in `ui/mod.rs:44` and again in `cnctdlg.rs:153`,
  and `bounded` CString truncation appears in `intro.rs:39`.

### 6.4 `modes/src/ui`

- **Menu identity is a frame index.** `PM_*` (`menu_state.rs:9-64`) are indices into
  the menu art, which is an external representation. Using them as the menu
  state model, with arithmetic stepping (`menu_state.rs:103-120`), is the C part.
  `navigation::MenuStep` is the typed seed.
- **Bool-as-int APIs.** `choose_menu_state -> u32` (`menu.rs:20-49`),
  `do_confirm_exit -> u32` (`confirm.rs:117`), `pause_game -> u32` (`util.rs:122`),
  `wait_for_*(.., new_button: u32, .., reset: u32) -> u32` (`util.rs:178-233`),
  `play_loaded_music(.., continuous: u32, ..)` (`settings.rs:14-28`),
  `clear_sis_rect(.., flags: u8)` with `1 << 3` at call sites
  (`live/starbas.rs:171`). `bitflags` is already a dependency of the crate
  (`modes/Cargo.toml`).
- **Constants copied around.** `OPT_PC` appears in `settings.rs:3`, `menu.rs:10` and
  `talk_segue.rs:149`, `SOUND_DISABLED` in `settings.rs:4` although `world::SOUND_DISABLED`
  exists. Activity bits `0x4000`, `0x100`, `0x1000`, `0x2000` are spelled out in
  `util.rs:124-126` and `confirm.rs:153-156`.
- **NUL inside `&str`.** `session::text` requires the caller to append `"\0"`
  (`session.rs:32-36`, callers `ui/shipyard.rs:126`, `ui/outfit.rs:88`). There are 24
  such literals.
- **Loops outside the frame driver.** `confirm::poll_confirmation` and
  `util::pause_game` run their own sleep or spin loops (`confirm.rs:79-114`,
  `util.rs:147-156`), bypassing `run_input` and the observer. Automation can't
  see those frames.
- **Two drawing styles.** `uqm_engine::graphics::menu` free functions with
  explicit `select`, `batch` and `unbatch` pairs (`ui/starbase.rs:105-125`, `:139-152`)
  next to `ContextLease::with_selected` and `with_batch` closures (comm). The
  closure form is the one to keep.
- **Module layout.** `ui/mod.rs:23-40` mounts `ui/shipyard/*.rs` as siblings through
  `#[path]` attributes. Plain nested modules are the normal form.
- **Kept facilities.** `MenuServices` injection (`session.rs:57-71`) removes ambient
  lookups between starbase and comm and options. It needs typed returns, not fn
  pointers to `u16`/`u32`. `getchar::TextEntryCallbacks` is a usable seam, but the
  editor works on a fixed `&mut [u8]` (`getchar/editor.rs:72-80`).

### 6.5 `modes/src/starbase`

- The directory is two things at once. Its rule kernels (`module_guard`,
  `module_chooser`, `sis_modules`, `starbase_menu::StarbaseState`, `draw`,
  `graphics`, `gfx`, `ids`) are used by production `ui::*`. Its screens
  (`screen`, `shipyard_screen`, `outfit_menu`, `module_screen`) run only against
  the mocked `Platform` trait (`platform.rs:1-80`) in tests. The module doc says
  so (`mod.rs:9-16`).
- The test-only model is better typed than production (`StarbaseError`,
  `ModuleSlot`, `FuelError`, `StarbaseStep`, `CleanupOutcome`). Production
  re-implements the same rules with raw integers (`ui/outfit.rs:423-470` against
  `starbase/fuel.rs:117`; `ui/starbase.rs:158-172` against `visit.rs:37,63`).
- `ids.rs` is a grab bag of string bases and layout constants imported by about 40
  files across the crate, including comm and planets. It belongs with the
  owners of those strings and layouts.

## 7. Target design

### 7.1 Conversation state as types

```text
enum Phase { Speaking(Playback), Choosing(ResponseMenu), Replaying(Playback), Summary(SummaryPager), LastReplay(Deadline), Done(Outcome) }
struct Playback { wait: WaitTrack, seeking: Option<SeekDir>, slider: SliderImage }
struct ResponseMenu { entries: ArrayVec<Response, 8>, selected: usize, top: usize, last_said: Option<String> }
enum Segue { Peace, Hostile, Victory, Defeat }   // single source, written to World at teardown
enum IntroTransition { CrossfadeSpace, CrossfadeWindow(Rect), CrossfadeScreen, FadeInScreen(Ticks) }
```

`Comm` shrinks to: loaded assets and display caches, the descriptor, the active
script, `Phase`, phrase enablement, and the animation sequences. All
`*_abi::RuntimeState` blocks fold into named owners (`SubtitleView`,
`SpeechWidgets`, `AnimationSet`).

### 7.2 Dialogue scripts

One trait in `uqm-content`:

```text
trait Script { type Handler: Copy; fn init(&mut self, host, desc); fn intro(&mut self, host); fn respond(&mut self, host, Self::Handler, PhraseId); fn post(&mut self, host); }
enum ActiveScript { Arilou(arilou::Script), ... }   // one match replaces LIFECYCLES and live/*
struct Response { phrase: PhraseId, text: String, handler: ActiveHandler }
enum Speech { Phrase(PhraseId), PlayerName, ShipName, Number(i32) }   // replaces i32 sentinels
```

The response entry stores `ActiveHandler` (an enum of per-race `Handler`s), not
a function pointer, so dispatch is one `match`. The 28 `live/*.rs` trampolines,
`lifecycle.rs` tables, `SessionData`/`CommData` function pointers and
`Host`'s `RefCell` wrappers go. `Host` can take `&mut Comm` and `&mut Cx` once the
trait methods take `&mut self` where they mutate. Shared capabilities (`date`,
`number`, `random`, `intro_mode`) move into the base host trait, with `random`
coming from `Cx::rng`.

### 7.3 Frame scheduling

- Every interaction loop runs through `cx.run_input(io, reset, frame)` and ends
  each frame with one pacing call on the game tick clock: comm speech
  (`ONE_SECOND/60`), comm response and last replay (`COMM_ANIM_RATE`), summary
  (`ONE_SECOND/20`), menus (`ONE_SECOND/20` or `/30`), confirm (`/30`), pause. Confirm
  and pause move onto `run_input` so automation can see their frames.
- One clock. `speech_graphics` uses game ticks in live and replay alike. No direct
  `get_time_counter()` or `unix_millis()` in modes; pass `now` from `Cx` or the
  runtime.
- Content scripts do not sleep. The starbase mineral sale returns a
  `SaleAnimation` plan (amounts, per-step delay, skip rule) that the mode plays as
  frames.

### 7.4 API and error rules

- `bool` or an outcome enum wherever a value is a truth or a choice. No
  `c_int`/`c_uint` in modes or content.
- Typed `Activity` (engine `ActivityValue`, `ActivityKind`, flag methods)
  everywhere; delete local re-declarations (`dispatch.rs:48-97`).
- `ConversationId` and `ShipKind` enums in place of `u32`/`u8` tables
  (`dispatch.rs:106-164`); unknown ids fail fast instead of falling back to Chmmr.
- `bitflags` for `AnimFlags`, `SisRedraw`, `MenuSounds`.
- Text as `&str`/`String` in modes; convert at the engine font boundary until
  the engine takes `&str` (a cross-family change, flagged in section 10).
- `thiserror` enums at load boundaries (`hail::prepare`, asset loads, slide
  script parse). `panic!` stays for violated invariants.
- Remove all `eprintln!("[DBG]")`; use `log::debug!` where a trace is worth keeping.

## 8. Contract tests

Keep, because they test the real path or pure rules the real path uses:

- `comm_abi::owner_tests::subtitle_bridge_renders_and_reuses_the_real_cache_without_test_substitutes` (`comm_abi.rs`, real font, real cache, pixel checks).
- `host::tests::supplied_names_are_copied_into_responses_and_admitted_to_real_speech` (`host.rs:221-254`, real trackplayer).
- `commanim_abi` sequence tests (`commanim_abi.rs:475-660`), `fresh_press` tests, `segue` and `phrase_state` tests (once collapsed into the typed versions).
- Content per-race `tests.rs` and `parity_tests.rs` over `Fake` (268 tests). These
  are the contract for branching dialogue and stay as they are, with signatures
  updated.
- `starbase::module_guard`, `module_chooser`, `sis_modules`, `starbase_menu`,
  `visit` unit tests, once production calls those kernels.
- `hail::tests::completion_admission_preserves_the_current_input_owner_contract`.

Delete, because they test code the game does not run: `ffi_behavior_tests.rs`, the
`state.rs` legacy-API tests, `ffi.rs` `test_ffi_*`, the `talk_segue.rs` test-path
tests and source-scanning tests (`:2148-2430`), `encounter.rs` tests, `track.rs`,
`subtitle.rs`, `oscilloscope.rs`, `summary.rs` tests, `live/dialogue_tests.rs` cases
that drive `talk_segue::alien_talk_segue`, and the starbase screen tests
(`tests.rs`, `shipyard_tests.rs`, `module_route_tests.rs`, `test_support.rs`,
`new_game_reference.rs`) unless they are ported onto the production screens.

Add (each one drives the production loop through the `input::fixture::Driver`
runtime and a recording observer, with no `cfg(test)` branch in the code under test):

1. Dialogue loop contract: speech to choose to select to callback to next speech,
   asserting observer checkpoints, the callback order and `Phase` transitions.
2. Response scroll: eight long responses, move down until the selection passes
   the window bottom; assert `top` follows C (comm.c:1036-1045).
3. Player statement feedback: after a selection, replay and summary show
   "(In response to your statement)" with the said phrase.
4. Pacing: the response phase and last replay request one sleep per frame at
   `COMM_ANIM_RATE`; speech at `ONE_SECOND/60`; speech widgets on game ticks.
5. Fade durations for select and replay equal `ONE_SECOND`.
6. Script dispatch: every `ConversationId` maps to a script and
   unknown ids fail.
7. Starbase screens: the production outfit fuel step, module purchase, shipyard
   crew transaction and visit cleanup, each against the existing pure-kernel
   expectations.
8. Slide script parse: every shipped presentation table parses into commands
   with no unknown opcode.

## 9. Phased units

Each unit introduces the new path, moves every caller, and deletes the old code
in the same PR. Per-PR gates: fmt, quality, clippy, tests, coverage, release build,
headless exact parity. Golden changes are expected only in units 2 and 3, and
only on the P/presentation lines the fixes touch.

**Unit 1. One conversation loop.**
Replace `ffi.rs`, `ffi/*`, `talk_segue.rs` (both halves), `comm_abi`'s loop
functions and the `c_bridge` module with a `comm::session` module holding `Phase`
and one `Frame` implementation. Delete `track.rs`, `subtitle.rs`, `oscilloscope.rs`,
`summary.rs`, `encounter.rs`, `subtitle_display.rs`, `response_ui.rs`, `glue.rs`, the
dead `Comm` fields and accessors, `Comm::segue`, `CommError`, `CTrackBridge`, all
`#[cfg(test)]`/`#[cfg(not(test))]` pairs in comm (45 sites), the source-scanning
tests and `[DBG]` prints. Rename `*_abi` modules to owners (`subtitles`, `widgets`,
`animation`, `numbers`, `segue`). Add contract tests 1 and 6. Scale: about 4,000
lines removed, about 1,000 added. Behavior identical; parity unchanged.

**Unit 2. Fix the four defects on the new loop.**
Response scroll, last-said phrase feedback, `ONE_SECOND` fades, response-phase
pacing, tick-based widget clock, `response_display` reset. Add contract tests 2
through 5. Expected parity deltas: presentation counts in comm scripts
(`repro-comm-menu-initial`, `comm-encounter-v1`, `real-sol-*`). Apply the golden
label once the diff is confined to these frames.

**Unit 3. Typed script dispatch.**
Add the `Script` trait and `ActiveScript`/`ActiveHandler` enums in content; make
`Response` store `ActiveHandler`; replace `Speech` sentinels; unify `handle` to
`PhraseId`. Delete `live/*.rs` trampolines (2,210 lines), `lifecycle.rs` tables,
`SessionData`/`CommData` and `locdata.rs` raw pointers, `Host` `RefCell`s, the
duplicated capability methods. Move the starbase sale animation out of the
script into a returned plan. Content tests keep their `Fake`.

**Unit 4. Content cleanup across all races.**
Phrase ids as a per-race `#[repr(u16)] enum` or `PhraseId` newtype generated from
the dialogue tables; `Segue` enum in content; typed activity checks; remove
post-increment blocks, declare-then-assign and paren chains. Mechanical across
141 files; behavior covered by the 268 content tests. Can run in parallel with
unit 5 once unit 3 lands.

**Unit 5. One starbase implementation.**
Move production `ui::{starbase, outfit, shipyard}` onto the pure kernels
(`starbase_menu::StarbaseMenu::step`, `fuel::change_fuel_quantity`,
`visit::{entry_time_passage, cleanup_after_starbase}`, `ModuleSlots` capacities).
Delete `starbase::{screen, shipyard_screen, outfit_menu, module_screen, platform,
input, test_support, tests, shipyard_tests, module_route_tests,
new_game_reference}`, `ui/sis_capacity_math.rs`, the duplicate
`ui/starbase.rs:158-172` functions, the `#[path]` mounts in `ui/mod.rs`, and the
unused `StarbaseError` variant in `src/mainloop/mod.rs:57`. Split `starbase/ids.rs`
into the owners of each constant. About 6,000 lines removed. Add contract test 7.

**Unit 6. Menus and front end.**
Typed menus per screen (`GameMenuItem`, `SettingsItem`, `OutfitItem`, ...) mapped to
`PM_*` frame indices at draw time; `choose_menu_state` returns `MenuStep`;
`game_options` and `pick_game` return outcome enums; `MenuServices` returns typed
outcomes; `confirm` and `pause` run on `run_input`; `bool` parameters in `util`;
`bitflags` for SIS redraw and menu sounds; drop `"\0"` text and local constant
copies (`OPT_PC`, `SOUND_DISABLED`, `rgb15`, intro modes). Setup menu choices by
named field. Intro slides parsed once into `Vec<Command>` (contract test 8).
Credits pass a background presenter through `CommOptions` and stop sharing
`Rc<Cell>`.

Ordering: 1, then 2 and 3 (3 may start after 1 lands), then 4 and 5 and 6 in
parallel slots, since 5 and 6 only share `ui::menu` and `MenuServices`. Native
real-window runs and screenshot review once after unit 6, per the current process
rule.

## 10. Open questions and limits

- The four defects in section 4 come from reading the code against C, not from
  runs. Unit 2 should start with failing headless checks for each.
- Engine dependencies limit how far these units can go. The trackplayer is a process
  global (`engine/src/sound/trackplayer.rs:136`). Text drawing takes `&CStr`.
  `graphics::menu` keeps a selected-context global. Unit 1 can wrap the trackplayer
  behind a handle owned by `Comm`, but removing the global and moving text to
  `&str` belong to an engine audit.
- `modes/src/mainloop/restart_menu` (main menu) is a caller of `fmv` and
  `setupmenu` and was not audited here.
- Line counts come from `wc -l` at the pin and include comments and tests.
