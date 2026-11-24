# UQM Rust design audit

**Start with resource ownership, from a mounted package to the player who uses it.** The index resolves files, the settings editor changes preferences, the renderer creates drawables, and a campaign retains the art it needs. Those are four different responsibilities. Today a read can recreate the process resource index, and a failed navigation load can leave enough state installed for a retry to report success without the required art. This path gives the first change a precise test: fail `graphics.miscdata`, restore the asset, retry new-game admission, then release the campaign and start another.

The [implementation plan](PLAN.md) covers the whole game and its verification tools. Resource ownership comes first because startup, menus, ships, conversations, planets and playback all consume it. Battle's bounded ordered pool, authored race behavior, file and packet codecs, raster math and production verification oracles have independent reasons to survive. Keeping those contracts does not require keeping their callers shaped like C.

**Revision:** [`678f8c11defe28a85cfccac29899410209e9963c`](https://github.com/acoliver/uqm/tree/678f8c11defe28a85cfccac29899410209e9963c). References below are paths at that pin. `E`, `M`, `C`, `P` mean `rust/crates/{engine,modes,content,platform}/src` respectively; `R` means `rust/src`. Each linked report defines its own path shorthand. **Evidence is static.** No builds, tests, replays, benchmarks, native captures or CI queries ran for this audit. A test cited here is a source witness or a proposed gate. No failure rate, performance gain or passing result follows from it.

## Where to look

| Audit | Production path and main design finding | Reading depth and limits |
|---|---|---|
| [Battle, fleets and network](battle.md) | Startup species catalog → picker → ship allocation → ordered tick → death, salvage or replacement. `E/battle/statistics.rs` delegates a stats change through `M/battle/services.rs` to the HUD; `M/battle/process_native.rs` allows one collision handler to change the next. Super Melee retains setup and peers across matches; packets and live CRC have different byte orders. | 185 tracked files in its defined roots including math, collections, species, ship content, pickship and both netplay sides of the engine port. All 28 race modules inventoried; selected behaviors followed. No peer interoperability or numerical equivalence run. |
| [World, runtime and exploration](world.md) | `R/main.rs` → application → restart → campaign dispatch → flight/orbit → lander → return. `E/world.rs` already owns campaign data; `E/kernel.rs` mixes process services and scene assets. The host clock, frame driver and campaign calendar have distinct advancement rules. | 269 tracked Rust files in its roots, including host, configuration, threading, observation, story, state files, generators and planet side. Startup, events, Sol and trip settlement traced; special plots and presentation leaves sampled. See supplementary paths below. |
| [Conversations, menus and starbase](interaction.md) | `M/comm/hail.rs` runs `ffi::rust_do_communication`; real speech uses the engine track player. Race handlers return through `live/*` trampolines. The game runs `M/ui/{starbase,outfit,shipyard}.rs`; the similar screens under `M/starbase` chiefly serve tests. | Inventory of comm, race scripts, front end, UI and starbase. Core loops traced; authored scripts, widgets and leaves sampled. Four dialogue discrepancies identified by comparison with historical `comm.c`, without playback evidence. |
| [Resources, media and platform](media.md) | Startup mounts → `.rmp` and settings → asset owner → draw, playback or input session → teardown. Graphics queues retained work; native and replay drive different sinks. `E/video/legacy.rs` lets an old token stop the current movie. `P/sdl/input.rs` supplies events without the device registry assumed by the input driver. | 118 engine files and 29 platform crate files in the stated inventory, including OS services, sockets, display, codecs, logging and fixtures. Device capability, window output, audio timing and allocation costs unmeasured. |
| [Saves, automation, proof and delivery](verification.md) | Slot → codec → staged sections → live world; restore can install story/events before a later queue error. Scheduler → observed input → ordered trace → process receipt → offline verdict. The outer `rust/xtask/src/process.rs` can claim cleanup without observing it. | Codec, savegame, automation, proof binaries, xtask, tests, CI and release discussed by family. Representative integration cases read. The report announces a file appendix that is absent in the staged copy; use the pinned inventory below. No release, provenance chain, cleanup failure or live CI configuration verified. |

### Coverage reconciliation

The unit of coverage is a subsystem with a production entry, owner, evidence and destination in the plan. File counts include tests and declarations; they do not measure semantic coverage. A reproducible census is `git ls-tree -r --name-only 678f8c11defe28a85cfccac29899410209e9963c -- rust rast`. Restricting that listing to `.rs` gives:

| Root | Tracked `.rs` | Coverage assignment |
|---|---:|---|
| `rust/crates/engine` | 251 | Battle/collections/math/species/network; world/kernel/host/time/state/observers; media and resource services; save mapping and test support. `E/lib.rs` names the module boundaries. |
| `rust/crates/content` | 258 | Ships and catalog, planets and generation tables, race conversation scripts. `C/lib.rs` declares the three families. |
| `rust/crates/modes` | 389 | Battle, pickship, Super Melee and protocol; restart, navigation, planets and surface; comm, front end, UI and starbase. See `M/lib.rs`. |
| `rust/crates/platform` / `rust/crates/save` | 28 / 18 | OS, SDL, audio, socket and replay adapters; file DTOs, UMS, MLE, statefiles and LZH. Crate manifests and non-Rust fixtures are additional inputs. |
| `rust/src` | 166 | Binary, public exports, CLI/config, app, game init, mainloop, time, savegame, automation and both proof binaries. |
| `rust/xtask` / `rust/tests` | 89 / 132 | Quality/coverage, parity, native admission, supervision, evidence, reports and integration harnesses. `rust/scripts`, `rust/native-scripts`, `rust/tests/golden`, `.github/{scripts,workflows}` and `scripts/release` connect these to fixtures, CI and delivery. |

This accounts for 1,331 Rust files under `rust`. Outside the workspace, `rast/src/io/uio_bridge.rs` is an empty tracked file at the pin. The packaged and loose data, key bindings, indices, documentation and historical material are compatibility inputs or references. The audit did not inspect every art asset, package byte, document, platform branch or external consumer. `tools/render-topography-oracle.py` expects a supplied historical source and headers; its presence establishes neither a production C renderer nor an executed comparison.

**Production details that matter beyond the slice inventories:**

* **Flight menus and devices (U06, U09, U16).** `M/planets/solarsys/flight.rs:430-459` advances the calendar before controls and location transitions. `solarsys/lifecycle.rs:271-285,626-639` routes to device use, cargo, roster and the starmap. `M/planets/device_effects.rs:19-32` changes Ultron condition and clears component and dialogue flags; `pstarmap.rs:201` admits the map. A menu rewrite needs to check story, fuel, roster order and autopilot on return as well as the highlighted item.
* **Terrain and orbit (U09).** `M/planets/orbit.rs:223-264` calls `plangen::live::generate_planet_surface`. `plangen.rs:213-305` reseeds from the planet, chooses supplied art or generated elevation/palette, conditionally scales topography with a separate seed, reads colors, builds lighting and starts sphere rotation. `orbit.rs:415-450` lends a rotation callback across the menu. Compare elevation, scaler seed, shield layer, rotated pixels and callback restoration separately. A terrain fixture alone cannot qualify an orbital visit.
* **Surface objects (U09).** `M/planet_side/runtime.rs:393` uses the lander movement reducer. `collision_adapter.rs:421` drives `world::step_world` (`world.rs:561`), which takes a list of IDs, processes creatures and shots, then expires lifetimes. This is distinct from battle's live successor walk. `generation_adapter.rs:231-242` has a test configuration that bypasses the linked generator. Pair seeded reducer checks with unassisted generation, collision, pickup and return through actual admission.
* **Special systems (U09, U14, U15).** `C/planets/generate/home_contact.rs::meet` saves and clears IP groups, talks, then checks load/abort before restoring groups. `generate/sam.rs::build_guard` populates groups 1 through 12 and 13 through 16 separately and assigns station positions in queue order. Sol's Pluto and Luna pickup dispositions also differ. Require cases by registered generator and effect family. These examples do not establish that every plot branch has an oracle.
* **Proof and delivery (U17, U18).** At the pin `rust/xtask/parity-corpus.toml` has 22 entries; `rust/tests/golden/parity/counts/` has 190 TOML files. These are different inventories. The 74 runtime save payload/metadata pairs are another obligation. `rust/xtask/src/native_smoke.rs` sequences real-window scenarios; `scripts/release/package.py` exercises an unpacked asset away from the checkout. Neither workflow proves the other's claim. No installation/uninstallation, signing-service or physical-device audit is implied.

## What has to change

The same fact often has two writers. Ship statistics are changed through drawing, a save temporarily rewrites live location, and a trace predicate infers game state from formatted labels. Other copies exist only under test: a second dialogue loop can pass while the shipped response loop behaves differently. The intended model gives one game thread authority over campaign, active mode and ordered gameplay effects. A presenter receives a delta or a retained draw command; an observer receives a value with logical identity. A callback that can change the next collision or schedule today's event still runs at that point. Domain boundaries specify when an effect takes place.

File bytes, network messages, content lookup, timing boundaries and native device behavior each need their own evidence. For example, the live save writer's repeated-first-element convention belongs to a writer projection. It gives no reason for the inventory itself to repeat that value. The plan distinguishes representation changes with zero expected parity delta from corrections that require a failing live-path test and an explained change in behavior.

## Tracking matrix

Unit IDs refer to [PLAN.md](PLAN.md). The evidence column names a discriminator for completion, not an audit result.

| Topic | Units | Evidence to retain |
|---|---|---|
| 1. Globals | U01–U05, U07, U17 | Two sessions with conflicting keys, input, RNG, playback and run state do not interfere; teardown leaves no registered work. |
| 2. Game model | U06, U08–U16 | One campaign, one active mode, one live record per fact; save/load/restart and second encounter preserve intended state. |
| 3. Ownership and borrowing | U01–U05, U08, U10 | Acquisition/release log, queued draw after source drop, child join, absent partial commit. |
| 4. Entity lifecycle | U09–U12 | Foreign/stale IDs, unlink versus free, exhaustion, same-tick insert/retire, child/parent death and survivor writeback. |
| 5. Frame states | U03–U05, U09–U15 | Entry/final present, first-hook effect on second, abort checkpoints, ordered speech/choice and peer-ready transitions. |
| 6. Algorithms and data structures | U04, U07, U09–U13, U18 | Fixed-point and pixel vectors, stable ties, next RNG draw, partial-write cursor, bytes read and allocation counts. |
| 7. Domain types | U06–U16 | Distinct slot/species/catalog IDs, `Body`, velocity/facing, phrase and tick units; checked conversion at file and wire edges. |
| 8. Resources | U01, U02, U04, U05, U11 | Required versus optional loads, addon precedence, retained mounts, assets, music voices and stale-token checks. |
| 9. Time, input, RNG and async | U03, U05–U07, U12, U13 | Wake versus read, repeat edges, calendar FIFO, seed ordinal, delayed input/CRC, callback reentry and cancellation. |
| 10. Errors and invariants | U01, U02, U08, U09, U13, U18 | Typed external errors, explicit gameplay outcomes, one settlement, old slot survives failure, truthful cleanup receipt. |
| 11. APIs | U01–U18 | Production callers, feature builds and proof readers use the replacement; obsolete exports, shadows and internal ABI relays removed. |
| 12. Permanent verification | All; especially U17, U18 | Exact replay/bytes/counts, properties, fuzz regressions, measured budgets, native milestones and release/quality checks. |
