# Migration plan: C-shaped Rust to idiomatic Rust

Nine phases (0-8), ordered by dependency and risk. Each phase is a series of small
PRs; `main` stays playable and green after every one. Target design:
[architecture.md](architecture.md). Checks: [quality-gates.md](quality-gates.md).

## 1. Starting numbers

Production code at 0ba2845ad, measured with line regexes (method in architecture.md
section 1, which also gives crate totals for the other counters). The Phase 0
baseline replaces these with syn-based counts.

| subsystem | prod lines | unsafe % | static mut | extern "C" fn defs | no_mangle |
|---|---|---|---|---|---|
| comm | 32,597 | 8 | 70 | 68 | 65 |
| automation | 28,017 | 3 | 0 | 16 | 6 |
| graphics | 20,438 | 22 | 43 | 295 | 243 |
| ships | 18,914 | 34 | 32 | 259 | 47 |
| planets | 16,857 | 32 | 46 | 199 | 135 |
| sound | 11,482 | 16 | 6 | 66 | 34 |
| abi | 11,240 | 3 | 0 | 0 | 0 |
| battle | 10,657 | 40 | 22 | 160 | 120 |
| supermelee | 6,676 | 49 | 16 | 267 | 221 |
| planet_side | 6,234 | 13 | 1 | 9 | 2 |
| front_end | 5,924 | 74 | 48 | 46 | 13 |
| starbase | 5,781 | 3 | 0 | 1 | 0 |
| ui | 5,604 | 71 | 17 | 73 | 67 |
| savegame | 5,322 | 11 | 6 | 11 | 5 |
| mainloop | 4,482 | 25 | 20 | 75 | 56 |
| io | 4,016 | 27 | 0 | 18 | 18 |
| navigation | 3,061 | 79 | 8 | 43 | 32 |
| input | 2,992 | 29 | 1 | 24 | 13 |
| video | 2,759 | 28 | 6 | 59 | 3 |
| state | 2,269 | 12 | 1 | 11 | 11 |
| resource | 2,040 | 34 | 1 | 48 | 36 |
| threading | 1,360 | 30 | 0 | 7 | 5 |
| network | 1,103 | 59 | 0 | 39 | 33 |
| game_init | 958 | 50 | 30 | 41 | 32 |
| strings | 958 | 42 | 0 | 24 | 19 |
| collections | 684 | 64 | 0 | 9 | 9 |
| others (bin, root, pickship, time, callback, math) | 5,308 | n/a | 1 | 54 | 32 |
| **total** | **217,733** | **23** | **375** | **1,922** | **1,257** |

All of `ui/` and `navigation/hyper_*` are `#![cfg(not(test))]`.

## 2. Verification kit

Each phase states which of these it needs. V1, V2, V3 and V5 run on every PR in CI.

**V1. Deterministic replay** (`cargo xtask parity record|check`, built in Phase 0).
Fixed seeds (`automation/trace.rs:49`) and SDL dummy drivers leave real time
(`time/counter_abi.rs:27-29`, `comm/commanim_abi.rs:258-297`), speech completion
(`comm/talk_segue.rs:411-451`) and pacing (`threading/pacing.rs:122-146`) variable.
Replay mode (`uqm --replay <scenario>`) replaces exactly these inputs:

1. **Clock.** `time::source` is chosen once at start: `Real` (SDL ticks) or `Replay`.
   `GetTimeCounter` and the replay `PacingEnv` (`pacing.rs:100`) read it; the
   choice moves into `Platform` in Phase 7. Replay time
   moves only when the game sleeps: `sleep_until(wake)` runs one `process_async` and
   `pump_and_flush` pass, then sets now to `max(now, wake)`; no real sleep happens.
   `TaskSwitch` and `Async_process` leave time unchanged, so alarms fire at fixed
   replay times. A loop that reads the clock 10,000 times without sleeping panics
   with its caller instead of hanging.
2. **Audio and speech.** The rodio thread is not started. `ReplayAudio` takes the same
   command enum (`sound/rodio_audio.rs:20`) on the game thread and answers
   `PlayingTrack`, channel and position queries from replay time and each stream's
   decoded length, so speech ends on the same step every run. Scenarios can still
   skip speech with input.
3. **Input.** The scenario is read before start and there is no automation I/O
   thread. Injected input is consumed only at input admission, by count.

Native verification (V2, V4) keeps the real clock and the audio thread.

The trace has one line per boundary (architecture.md section 7, `Observer`):

```
I <input#> <activity> <keys>
S <step#> <activity> <crc32> <state-digest> [<event>...]
P <present#> <capture-generation> <canvas-digest>
```

- `crc32` is the netplay checksum, unchanged.
- `state-digest` is the migration trace. Per element in link order: slot, generation,
  pred and succ slots, flags, every scalar field, logical parent (ship side and slot,
  or IP group id), target slot, hook names, frame-set owner and frame index. Per ship:
  species, side, crew, energy, the weapon, special and energy counters
  (`abi/ship_abi.rs:1787-1801`), status flags, characteristics, race state. At mode
  checkpoints: story bits, game clock, event queue, activity. Hook names come from one
  registry listing each hook, legacy or converted, under a stable name. No addresses
  or padding are hashed. `record` keeps the field values in a side file, so `check`
  prints the differing fields at the first differing step.
- Events: `spawn <side> <species>`, `hook <species> <hook>`, `death <side> <species>`.
- `canvas-digest` is the first 8 bytes of SHA-256 over the internal RGBA frame.
- `TraceRecord.elapsed_ms` (`automation/trace.rs:166-173`) is replay time; pacing
  reports and other host-timing fields are not part of the trace.

Acceptance, before any golden is committed and for each new entry: three recordings
are byte-identical: one on macOS, one on ubuntu CI, and one with
`UQM_REPLAY_JITTER=<seed>`, which adds random real sleeps of 0-20 ms at each boundary
while a CPU-load process runs.

### 2.1 Corpus

| Name | Basis | Covers |
|---|---|---|
| `melee-cyborg-a`..`d` | `super-melee-v1` flow, both sides Cyborg, `SuperMeleeBattle` seed | four fleets of 7 cover the 25 melee ships; `check` asserts from events that each ship spawned and fought two different opponent species, and, once its race is converted, ran its weapon and special hooks |
| `melee-input` | `super-melee-v1` with its recorded player input | human-input path |
| `campaign-battle` | `lost-battle-v1` | SIS flagship, campaign battle setup and teardown |
| `samatra` | `dev-docs/testing/samatra.md` | Sa-Matra, final battle path |
| `hyperspace` | `out-of-fuel-v1` | hyperspace element loop, fuel, Melnorme approach |
| `solar-flight` | `real-sol-starbase-to-mars` | IP flight, orbit, landing, takeoff |
| `lander` | `lander-destroyed-v1` | planet side, hazards, lander loss |
| `comm` | `comm-encounter-v1` | dialogue phrase sequence, speech completion |
| `new-game` | `main-menu-v1` then New Game | kernel load, menus, `NewGame` seeding |

Focused fixtures use `automation/melee_fixture.rs` setups with scripted input and run
a few hundred steps:

| Fixture | Exercises |
|---|---|
| `androsynth-blazer` | collision hook saved, replaced and restored (`androsynth_native.rs:389-397`) |
| `pkunk-rebirth` | hook restore, reincarnation, the `hTarget` hack (`pkunk_native.rs:401-442`) |
| `chmmr-satellites` | delayed point defence walking `pred` links (`chmmr_native.rs:406-460,494-507`) |
| `death-spawn` | a collision kills a ship whose death hook spawns elements mid-traversal |
| `mixed-collision` | converted VUX limpet hits an unconverted ship; unconverted Orz howitzer hits converted VUX |
| `pool-full` | fighters until `AllocElement` fails at 150 |
| `frame-sets` | Arilou threat test against a weapon still flying after its ship died and a same-species ship entered; blast after ship teardown; trilinear resolution index |
| `star-contact` | hyperspace star contact, repeat suppression, system exit, save and reload |

Goldens are recorded at the end of Phase 0. Refactor PRs may not change them. A
deliberate behaviour fix is a separate PR with the `parity-golden-update` label and
the reason in the commit.

**V2. Native failure scenarios.** `lander-destroyed-v1`, `out-of-fuel-v1`,
`lost-battle-v1`, `super-melee-v1` on a real macOS window (CI `failure-scenarios`).

**V3. Save compatibility** (`cargo test`, added in Phase 0). Loading and saving are
not inverse operations: load returns the saved activity as `NextActivity`
(`savegame/live/load.rs:33-35`), and save prepares location and group state, derives
the summary from live solar state and clears `in_orbit` (`live/save.rs:16-34`).
- **V3a, codec.** For the 73 saves in `rust/scripts/saves/` and
  `tests/fixtures/savegame/legacy-save.00`, decode then encode gives the input bytes.
  A separate writer fixture covers the element-amount fill (`live/save.rs:65-69`).
- **V3b, runtime.** Per save, under replay mode with the clock frozen: fresh game,
  load, main loop takes `NextActivity` and enters the saved mode, stop at its first
  input admission, capture a `SaveFile` through the live path. Phase 0 commits these
  as `rust/tests/golden/saves/<name>.capture`; later phases compare their capture,
  through `snapshot` with the live `SessionView`, at the same boundary. The test
  lists the normalisations of the current path as data (activity, `in_orbit`,
  summary fields, IP group queue), so a capture may differ from its input file.

**V4. Campaign save-chain reload** (macOS, run natively at phase exit). All 45
`load-*-v1` scenarios, the smoke set in `rust/xtask/smoke.json`, and the producer
segments that touch the phase's subsystem (for example `campaign-s06c-utwig-bomb` for
battle and lander work). OS screenshots are inspected, per standards section 7.

**V5. Gates.** `cargo xtask quality`, `clippy`, `cargo test`, `deny`, `coverage`, five
release builds.

## 3. How main stays green

1. **One unit per PR, old path deleted in the same PR.** A unit is one race, one
   queue, one module or one session. No dual implementation survives a merge.
2. **One transition pattern.** The staged authority of architecture.md section 4.1,
   with `Hook::Legacy` and `app::bridge`. The counters `legacy_hooks` and
   `bridge_calls` get finite budgets in the Phase 1 opening PR (quality-gates.md
   section 4). Budgets only shrink; `legacy_hooks` reaches 0 at Phase 2 exit,
   `bridge_calls` at Phase 8.
3. **Mechanical and semantic changes in separate PRs.** Moving `GlobData.x` reads to
   `game.world.x`, or CamelCase renames, are mechanical; V1 is their oracle.
4. **Size.** Aim for at most about 1,500 changed non-mechanical lines per PR.
5. **Tests move with the code.** 34 of the 61 files in `rust/tests/` declare `extern
   "C"` symbols (for example `tests/arilou_native.rs:8-14`). A PR that removes a
   symbol rewrites its test against the Rust API and keeps the golden data (for
   example `tests/fixtures/planets/*`, captured from the C originals).
6. **No long-lived branches.** At most five worktrees under `tmp/wt/`, each removed
   when its PR merges.

## 4. Pilot choice: battle core

| Candidate | Size | Depended on by | Oracle | What it proves |
|---|---|---|---|---|
| **Battle core** (element queue, process loop, collisions, ship/weapon helpers) | 10.7k lines, 40% unsafe | ships, Super Melee, hyperspace, IP flight, netplay checksum, hyperspace save step | netplay CRC plus migration trace; melee runs from a menu with a fixed seed | staged authority, re-entrant safe hooks, session ownership, bridge |
| Super Melee (setup, teams, netplay) | 6.7k, 49% | nothing | `super-melee-v1`, packet fixtures | session ownership (already shown by `planet_side`), packet codec |
| Planet generators | 16.9k, 32% | solar system | `tests/planets_parity.rs` | little: the logic is already safe (`planets/generate/orz.rs`) |
| World state (`GlobData`) | 689 references in 111 files | everything | V3, V4 | an access pattern, before any pattern is settled |

Battle core is chosen. The hardest part of the design (exact C ordering with
re-entrant callbacks) lives there, ships and Super Melee depend on its interfaces,
and it has the strongest oracle. Piloting Super Melee first would mean converting its
battle calls twice. The risk is that unconverted races, hyperspace and IP flight read
the records directly. Stage A keeps one representation for all of them, and the pilot
proves a mixed collision and a navigation callback on it before it expands.

## 5. Phases

Sizes are changed lines (added plus deleted) and files touched.

### Phase 0: soundness, gates, replay harness

Scope:
- xtask: `quality` (ports of llxprt-code-rs `loc.rs` and `complexity.rs`, plus ffi,
  unsafe, allows, layers, native-files, deps), `clippy`, `coverage`, `deny`, `parity`;
  `abi-check` extended to foreign statics. `[workspace.lints]`, `rust/clippy.toml`,
  `rust/deny.toml`, the `xtask` Cargo alias, `ci.yaml` (quality-gates.md section 5).
- Replay mode, migration trace, corpus and fixtures (V1), fed by the existing
  coordinator's input and presentation callbacks plus a checkpoint call at
  `rust_battle_frame_advance` and each mode's loop end; V3a and V3b goldens.
- Soundness fixes:
  - the 20 `clippy::not_unsafe_ptr_arg_deref` allows in 8 files become `unsafe fn`,
    starting with safe `InitQueue` (`collections/queue.rs:84-117`);
  - each foreign static gets its defining type: `DisplayArray`
    (`planet_side/orbit_scan.rs:79` declares 150 entries, the definition has 330),
    `CurStarDescPtr` (six declarations), `GlobData` (`CGlobData` vs `GLOBDATA`, which
    is Stage A for `GlobData`); this removes the 7 `clashing_extern_declarations`
    allows;
  - the ignored `_shadow` parameter of `getGameState`/`setGameState`
    (`navigation/globals.rs:19-27`) and the dead `GameState[155]` field go, leaving
    one authority for story bits (`state/ffi.rs:15`);
  - `ShipState`'s `unsafe impl Send` over raw pointers (`ships/traits.rs:61`) goes
    with the dormant model.
- Remove dormant parallel models after confirming them dead with `pub(crate)` and
  `dead_code`: `ships/traits.rs`, the non-`_native` halves of `ships/races/*.rs` and
  their registry path (`ships/registry.rs:1012-1090`), `battle/process_loop.rs`,
  `battle/display_list.rs`, `battle/tactical.rs`, `battle/integration.rs`.

Removes: about 5.5k production lines; 20 + 7 allows; the shell `c-ratchet` job.

Exit criteria:
- baseline committed; `quality --base` and `clippy --base` green on a PR and on a
  push to `main`; `quality deps` passes on the current lockfile with `bundled-sdl`;
- `native-files` = 0; `not_unsafe_ptr_arg_deref` = 0; `clashing_extern_declarations` = 0;
- `abi-check` passes for functions and statics;
- V1 acceptance (three identical recordings) for every corpus entry and fixture;
- V3a and V3b pass for 74 files; coverage floor recorded; `cargo xtask deny` passes.

Verification: V1, V2, V3, V5. Size: xtask +4k lines; src about -5k; about 60 files.

### Phase 1 (pilot): battle core, Stage A and first Stage B consumers

Scope: `battle/{process_native, battle_native, world_elements, weapon_native,
ship_native, velocity, gravity, status_native, tactrans_native}.rs`, queue globals in
`game_init/kernel_abi.rs:36,43`, the hook writes in all race files, and the readers
`supermelee/netplay/checksum.rs:55`, `savegame/runtime/hyper_save.rs:129`,
`automation/battle_fight.rs:456`. PRs in this order:

1. Stage A (architecture.md section 4.1): records and queue statics move to
   `engine::battle::{record, legacy}`; the ABI copies become re-exports; hook fields
   become `ElementHook`/`CollisionHook` (mechanical edits in the race files).
2. `ElementTable`, `ElementId`/`ShipId`, `Cx`, `invoke_*`, `app::bridge`.
3. Process loop and collisions on ids.
4. Proof: VUX and Androsynth converted, and the hyperspace `star_collision` and
   `star_death` callbacks (`navigation/hyper_transition.rs:191-229`) moved to `Cx`;
   `mixed-collision`, `death-spawn`, `androsynth-blazer`, `star-contact` and
   `chmmr-satellites` (unconverted, walking raw `pred` links on the shared storage)
   pass. The `Race`, `ElementHooks` and `Cx` APIs are approved here; no further
   conversion merges before this PR.
5. Arilou; then the three readers on ids.

Removes: the `ELEMENT`/`STARSHIP`/`RACE_DESC` copies in `navigation_abi.rs`,
`netplay_abi.rs` and `global_state_abi.rs` with their layout assertions and
`link_name` aliases; statics `zoom_out`, `nth_frame`, `OPT_MAX_ZOOM_OUT` and
`battle_counter`, which move into `Battle` (readers in `supermelee`, `pickship` and
`automation` use counted bridge accessors). Kept until Stage C: the authority
statics (`disp_q`, `race_q`, `DisplayArray`, `DisplayLinks`, `DisplayFreeList`),
raw links, `pParent`, `hTarget`, unions, `farray`.

Exit criteria:
- one definition of each record and queue static; no alias for them in `abi-check`;
- battle `static_mut` holds only the Stage A authority statics; the other battle
  statics are in `Battle` or `Kernel`; `legacy_hooks` and `bridge_calls` within budget;
- VUX, Androsynth, Arilou: 0 `unsafe`, 0 raw-pointer parameters; battle `extern "C"
  fn` definitions are only entry points called by unconverted code;
- stale-id lookups across the corpus = 0.

Verification: V1 (corpus and fixtures identical); V2; V3; V4 for the `load-*` saves
made in hyperspace, plus `smoke.json`; `tests/netplay_packets.rs`.
Size: about 45 files, 8-10k lines.

### Phase 2: ships, combat AI, remaining record readers, Stage C

Scope: the other 25 `ships/races/*_native.rs`, `ships/{code_resources, loader_native,
master_native, native, native_adapters, battle_bridge, ffi_contract, catalog,
lifecycle, writeback, queue, runtime, types}.rs`, `battle/cyborg_native.rs` (walks the
queue at `:170`), `battle/ai*.rs`; then the element callbacks and record readers in
`navigation/{hyperspace, hyper_radar, hyper_transition, hyper_encounters,
interplanetary}.rs`, `planet_side/collision_adapter.rs`, `front_end/uqmdebug.rs` and
`savegame/runtime/queues.rs`. One PR per race (two for small races). When no record
reader remains, the Stage C series (architecture.md section 4.1).

Removes: 28 `static mut <RACE>_DESC` templates and `INITIALIZERS`
(`ships/code_resources.rs:75`); `RaceDescPtr`; `data: *mut c_void`; `Hook::Legacy`;
the battle part of `app::bridge`; the Stage A authority statics;
the record types and `ElementProcessFunc`/`ElementCollisionFunc` in `ship_abi.rs`.

Exit criteria: ships and battle have no `ffi` or `unsafe` baseline entries;
`legacy_hooks` = 0 and its budget deleted; the corpus assertions of section 2.1 hold
for every race. Verification: V1, V2, V3, V4 subset (`campaign-s06c-utwig-bomb`,
`samatra`, `load-connected-final-fleet-v1`), V5.
Size: about 75 files, 14-18k lines.

### Phase 3: Super Melee and netplay

Scope: `supermelee/` (setup, team files, picker, netplay), `pickship/`, `network/` and
`network.rs`, `battle/netplay.rs`.

Removes: `packetTypeData` (`supermelee/netplay/packet.rs:29`) and `#[repr(C)]` packet
structs (become `enum Packet` with explicit codec); `netConnections` and other
netplay statics; supermelee `extern_c_fn` 267 → 0, `no_mangle` 221 → 0, `static_mut`
16 → 0. Sockets move to `platform::net` (std::net, or libc/WinSock kept as an audited
module if non-blocking connect needs it).

Exit criteria: supermelee and pickship unsafe = 0; network unsafe only under
`platform::net`; packet byte fixtures unchanged; a two-peer loopback battle agrees on
checksums every interval and matches the `melee-input` golden; `.mle` team-file
fixtures round-trip. Verification: V1, V2 (`super-melee-v1`), V3, V5.
Size: about 45 files, 6-8k lines.

### Phase 4: world state and main loop

Scope: `navigation/globals.rs`, `state/`, `time/` (game clock and `event_q`),
`mainloop/`, `game_init/`, `c_bindings.rs`, `comm/locdata.rs` (`CGlobData`),
`savegame/runtime/*` live readers, `automation/campaign_state.rs` and the
`Coordinator` statics (become the `Observer`), `collections/queue.rs` (last users). `GlobData` goes through Stage B (readers moved to
`World` accessors over the single definition, mechanical PRs per module, 689
references in 111 files) and then Stage C (owned `World` in `Game`).
`CurStarDescPtr` becomes `World::current_star` in the same series.

Removes: `GlobData`; `CurStarDescPtr`; `GLOBAL_GAME_STATE` and both
`GLOBAL_STATE_FILES` (`state/ffi.rs:15-16`, `state/state_file.rs:321`); string-keyed
story access (`state/game_state_keys.rs:477`) and raw bit indices
(`planets/pstarmap.rs:202-213`); the `global!` kernel statics; `GameLoopOps`/`CffiOps`
(`mainloop/game_loop.rs:290`); `collections/queue.rs`; `abi/global_state_abi.rs`.

Exit criteria: `GlobData` references = 0; `static_mut` = 0 in state, time, mainloop,
game_init, navigation, collections; story lookups by string = 0; `bridge_calls`
limited to mode entry points not yet converted; before the old coordinator hooks
are deleted, every `rust/scripts/*.json` scenario gives the same `input_seen` and
`present_seen` counts and action-phase sequence as its trace from the old hooks.
Verification: V1 (with `star-contact`), V2, V3b through `snapshot`/`apply` against the Phase 0 captures,
full V4 (45 `load-*` scenarios and the connected campaign producers), V5.
Size: about 130 files, 10-14k lines, mostly mechanical.

### Phase 5: navigation and planets

Scope: the rest of `navigation/hyper_*` (UI, radar, menu, transitions, encounters),
`navigation/interplanetary.rs`, `planets/{solarsys, orbit, scan, pstarmap, starmap}`,
`planets/generate` (vtable and `*_exports.rs`), remaining FFI in `planet_side/`.

Removes: `GenerateFunctions` and its 13 hook types (`planets/generate/vtable.rs:29`);
`exports.rs`, `home_exports.rs`, `late_exports.rs`, `special_exports.rs`,
`story_exports.rs`, `sol_exports.rs`; `getGenerateFunctions` (`battle/gendef.rs:74`);
`pSolarSysState`; `abi/navigation_abi.rs`; planets `static_mut` 46 → 0 and
`extern_c_fn` 199 → 0; navigation 43 → 0; the five `#![cfg(not(test))]`
`navigation/hyper_*` modules become testable.

Exit criteria: planets, navigation and planet_side have no `ffi`/`unsafe` baseline
entries; `cfg_not_test_modules` ≤ 11. Verification: V1 (`hyperspace`,
`solar-flight`, `lander`, `new-game`, `star-contact`), `tests/planets_parity.rs`
against its C-captured fixtures through the new API, V2, V3, V4, V5.
Size: about 90 files, 10-12k lines.

### Phase 6: comm, starbase and menus

Scope: `comm/` (dialogue `Api` implementations, segues, oscilloscope, ambient
animation), `starbase/`, `ui/`, `front_end/`, `mainloop/restart_menu/`.

Removes: `COMM_STATE` (`comm/state.rs:23`), `COMM_DATA` (`comm/locdata.rs:15`),
oscilloscope statics (`comm/oscill_abi.rs:48-51`); comm `static_mut` 70 → 0;
ui and front_end `extern_c_fn` 119 → 0, `static_mut` 65 → 0; the ten `ui` modules and
`front_end/cnctdlg.rs` stop being `#![cfg(not(test))]`.

Exit criteria: comm, starbase, ui and front_end have no `ffi`/`unsafe` baseline
entries; `cfg_not_test_modules` = 0; dialogue scripts in `comm/races/*` change only
in their `Api` impls. Verification: V1 (`comm`, `new-game`), dialogue unit tests,
`starbase-visit-v1`, `main-menu-v1`, V2, V3, V4, V5.
Size: about 120 files, 12-15k lines.

### Phase 7: platform and content

Scope: `graphics/` (drawables, contexts, frames, fonts, colour maps, DCQ, canvases,
scalers), `resource/`, `io/` (`uio_bridge.rs`, 2.9k lines), `strings/`, `sound/`
(decoder vtables such as `rust_ova_DecoderVtbl`, `lib.rs:51`), `video/`, `input/`,
`threading/`, `callback/`, the OS part of `time/`.

Removes: `abi/graphics_abi.rs` and the graphics `*_abi.rs` shims; `SDL_Surface`
canvases (become Rust pixel buffers, SDL used only to present); the `uio_*` C-style
API (becomes `content::Vfs`); `res_*` (becomes `ResourceCache`); decoder vtables
(become `trait Decoder`); `threading/legacy_abi.rs`. Graphics `no_mangle` 243 → 0,
`extern_c_fn` 295 → 0.

Exit criteria: `unsafe` only in `UNSAFE_ALLOWED` modules; `FrameSetRef` handles
backed by `ResourceCache`; no `include!` of `abi/*` except under `platform`.
Verification: V1 with canvas digests for every corpus entry, V2, V3, V4 including the
intro presentation (`tests/intro_presentation.rs`) and real-window screenshots
inspected for every smoke scenario, five release builds, V5.
Size: about 150 files, 15-20k lines.

### Phase 8: closeout

Removes: `rust/src/abi/` (12.1k lines today), `app::bridge`, `xtask abi-check`, all
`non_snake_case`/`non_upper_case_globals`/`non_camel_case_types` allows. Adds crate
root `#![deny(unsafe_code)]` with `#[allow(unsafe_code)]` only on `UNSAFE_ALLOWED`
modules, and moves the debt lints to `deny`.

Exit criteria: crate-wide `static_mut` = 0, `extern_c_fn` = 0 outside platform,
`no_mangle` = 0, foreign items, `repr(C)` and raw-pointer parameters only in platform,
`bridge_calls` = 0, `allow` attributes only the platform `unsafe_code` allows,
`unwrap`/`expect` in production = 0, coverage ≥ 70%. Remaining baseline entries are
LOC and complexity only. Verification: full V1-V5. Size: about 40 files, net deletion.

## 6. Review response

Each finding of the design review was checked against the cited code at 0ba2845ad.
All are accepted; none is rejected.

| # | Verdict | Change |
|---|---|---|
| 1 | accepted | ARCH 4.1 staged authority and its rules; Phases 1-2 reordered, Stage C at Phase 2 exit |
| 2 | accepted | V1 replay clock, `ReplayAudio`, boundaries, trace projection, three-recording acceptance |
| 3 | accepted | QG 3.4: bans first-party native builds and vendoring, allows the external graph |
| 4 | accepted | ARCH 2, 5.1, 5.3: engine `Cx`, content host traits, copy-call-reread, two sequences |
| 5 | accepted | V3a and V3b; ARCH 7 `SessionView`, `apply` returns the activity |
| 6 | accepted | V1 state digest with field reports; ARCH 7 checksum description corrected |
| 7 | accepted | V1 event assertions, eight fixtures, demanding hooks in the Phase 1 proof PR |
| 8 | accepted | ARCH 7 three `Observer` hooks and their order; Phase 4 trace-count check |
| 9 | accepted | QG 4 budgets, debt transfers, reported headroom, per-file coverage |
| 10 | accepted | ARCH 3 `World::current_star`; `star-contact`; removal moved to Phase 4 |
| 11 | accepted | ARCH 4 `FrameSetRef`; `frame-sets` fixture |
| 12 | accepted | QG 2.3 `--force-warn`, 3.2 walk and identity, allow scopes |
| 13 | accepted | QG 5 base fetch, push handling, Cargo alias |
