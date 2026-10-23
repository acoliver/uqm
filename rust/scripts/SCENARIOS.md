# Runtime scenarios

Scenarios are JSON files in this directory. The native runner drives the Rust
game through player input and checks live state assertions on a real macOS
window. Hosted CI builds and tests the game but does not run these windows.

From the repository root:

```sh
cargo run --locked --manifest-path rust/xtask/Cargo.toml -- native run \
  --scenario starbase-visit-v1 --artifacts tmp/starbase --content sc2/content/packages
cargo run --locked --manifest-path rust/xtask/Cargo.toml -- native run \
  --scenario super-melee-v1 --artifacts tmp/melee --content sc2/content/packages
```

Keep each run's output inside the checkout. Inspect the terminal result, semantic
assertions and OS screenshots; internal surface captures alone cannot establish
what a player saw. `native smoke --artifacts tmp/smoke --content
sc2/content/packages` runs the set listed in `rust/xtask/smoke.json`.

## Editing and adding scenarios

No source digest pins a scenario. Keep assertions aligned with the behaviour
claimed by its name. Update the relevant script, scheduler and integration tests
when changing an action contract. Do not remove assertions to make a run pass.

Start menu scripts wait for `wait_for_main_menu_ready`. Use menu keys for menus
and player keys (`thrust`, `left`, `right`, `weapon`, `special`, `down`, `escape`)
for flight and combat. Feedback-driven navigation actions include
`navigate_to_planet`, `navigate_to_orbit`, `navigate_to_moon`,
`navigate_to_hyperspace`, `fly_to_star` and `navigate_to_contact`.
They must reach their stated destination within their tick budgets.

`select_response` selects an offered phrase by its string-table name through
normal input. `fight_encounter` uses the player-selectable Cyborg and requires
battle teardown, enemy fleet loss and a surviving player flagship. Device and
energy-node actions require their actual story effects, not just a menu sound
or proximity to a node. Read-only campaign keys are defined in
`rust/src/automation/campaign_state.rs`.

A `start_scene` is a deterministic setup for focused tests. It activates once
after New Game; it does not prove that ordinary navigation reaches the scene.
Real-path scenarios should use navigation or an earned starting save instead.

## Starting saves and campaign checkpoints

A script may declare a starting save:

```json
{"start_save": {"path": "saves/allied-starbase.sav", "slot": 0}}
```

The path is relative to the script, limited to `saves/<name>.sav`. The runner
installs it into the isolated configuration directory before launch. The script
loads it through the normal main menu. No scenario edits save bytes or grants
resources through assertions.

`starbase-visit-v1` loads the allied Starbase checkpoint, docks, buys fuel,
dismisses and rehires a crew member, then departs with resource assertions.
`super-melee-v1` enters the fleet editor, reduces each side to one ship, plays
the battle to a result and returns to setup. The failure scenarios include
`out-of-fuel-v1`, `lost-battle-v1` and `lander-destroyed-v1`.

Campaign producer scripts save through the Game menu. Extract a checkpoint only
from a passing producer run, naming the scenario and slot explicitly:

```sh
cargo run --locked --manifest-path rust/xtask/Cargo.toml -- native save-extract \
  --scenario real-sol-starbase-alliance --artifacts tmp/alliance --slot 0 \
  --output rust/scripts/saves/allied-starbase.sav
```

Commit the save and its adjacent `.provenance.json` together. Extraction decodes
the selected file's campaign snapshot; assertion expectations do not supply its
values. Use a separate output slot from an installed parent save when producing
a new checkpoint. An unchanged parent cannot establish that Save ran.

Replay a late segment from its preceding earned checkpoint rather than repeating
the whole alliance chain. Regenerate a checkpoint when its producer's earlier
route, conversations or transactions change. A development checkpoint from a
parallel route is not a connected campaign milestone.

`prepare_mid_segment.py` can change a segment's input save without changing save
bytes or milestone assertions:

```sh
python3 rust/scripts/prepare_mid_segment.py campaign-s07-taalo-protected \
  --start-save utwig-bomb.sav
```

The named file must exist in `saves/`. Run and extract the resulting producer
before claiming that the segment connects to that ancestor.

### S09: shattered-world knowledge and Syreen shuttle

`campaign-s09a-shattered-knowledge` starts at Zeta Persei from
`repaired-ultron.sav`. Three separate peaceful Druuge homeworld conversations
raise `DRUUGE_HOME_INFO` to 3 and earn `KNOW_ABOUT_SHATTERED=2`. Native run
`pmid-11` passed and wrote slot 1. `dev-shattered-knowledge.sav` is its
extracted development checkpoint.

`campaign-s09b-syreen-shuttle` starts from that development save, lands on
Beta Copernicus I, collects an egg casing, then visits Talana at Betelgeuse.
Phrase-based dialogue presents the proof and earns `SYREEN_KNOW_ABOUT_MYCON`,
`SYREEN_SHUTTLE`, and `SYREEN_SHUTTLE_ON_SHIP`, all equal to 1. Native run
`pmid-13` passed and wrote slot 1. `dev-syreen-shuttle.sav` is its extracted
checkpoint. Neither save claims connected campaign ancestry. Use
`prepare_mid_segment.py --start-save` to substitute the campaign's earned
ancestor while retaining the scripts' story/resource preconditions.

`campaign-s09c-syreen-vault` starts from `dev-syreen-shuttle.sav` and flies to
Epsilon Camelopardalis I-a. The lander uses the shuttle at the vault, which
remains on the surface: completion requires a fresh vault unlock and shuttle
consumption on the same trip. Native `pmid-20` passed and saved slot 1;
`dev-vault-open.sav` is its development output. This subsegment stops before
returning to Talana and does not prove `MYCON_AMBUSH`.

### Utwig alliance checkpoint

`campaign-s06a-utwig-alliance` loads `repaired-ultron.sav`, flies to Beta
Aquarii and returns the completed Ultron through `got_ultron`. It saves local
flight to slot 1 with `UTWIG_HAVE_ULTRON=1`, `ULTRON_CONDITION=5` and
`UTWIG_SUPOX_MISSION=1`. The campaign retains 32 crew, 6934 fuel, 115 RU and
four escorts. `load-utwig-alliance-v1` independently reloads the extracted save.
The save round-trip tests also check that the Utwig and Supox fleets are allied,
travelling to their mission destinations, and retain the Utwig arrival callback.
This checkpoint precedes the Druuge guard battle and does not claim the bomb.

`campaign-s06b-bomb-approach` takes `utwig-alliance.sav` directly to Zeta
Hyades and saves before approaching the guard moon. Its extracted
`bomb-preguard.sav` retains the alliance and mission while requiring
`BOMB_UNPROTECTED=0` and `UTWIG_BOMB_ON_SHIP=0`. The separate
`load-bomb-preguard-v1` reloads the same state. Departure navigation now accepts
an optional `encounter_response`, using the existing input-only patrol hailing
and response selection used by planet navigation.

### Transit refit and autopilot recovery

`campaign-s08a-refitted` loads `dev-taalo-protected.sav`, visits Sol through
ordinary flight, sells fuel and buys four thrusters in Outfit, then purchases
100 fuel units from the Melnorme at Alpha Centauri. Native run `pmid-23` passed
and saved `dev-refitted.sav` with crew 32, fuel 15400 and two escorts. This is a
development branch, not a connected campaign milestone. The later Arilou and
Talking Pet segments are not proved by this refit.

An interrupting encounter clears production autopilot. After a verified fleet
win, `fly_to_star` reopens the real map and selects the original destination;
it preserves the action's tick count and original fuel baseline. The refit run
proved a fleet win followed by destination reselection and arrival. This fixes
#429 without writing the autopilot target or position.
