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

`campaign-s06c-utwig-bomb` loads the preserved pre-guard save, approaches
Zeta Hyades VI-b, challenges the Druuge and defeats their five-ship fleet with
normal-speed Cyborg combat. The lander then collects the Precursor bomb through
a surface collision. Native run `pt-p6-17` passed and saved slot 1 with
`BOMB_UNPROTECTED=1`, `UTWIG_BOMB=1` and `UTWIG_BOMB_ON_SHIP=1`. The earned
`utwig-bomb.sav` retains the returned Ultron, 32 crew and 6192 fuel, with no
escorts remaining. `load-utwig-bomb-v1` independently reloads that checkpoint.


### Parallel transit resupply

The development route `campaign-s08b-crew-ready` starts from `dev-refitted`,
sells fuel at Sol, installs a second crew pod and hires 100 crew through
Shipyard. `campaign-s08c-pkunk-resupply` returns to Gamma Krueger, handles
migrating patrols peacefully and makes a second home visit after the
Kohr-Ah warning so the annual gift can be delivered. The resulting six
Furies and 100 crew are saved in `dev-pkunk-resupply`.

`campaign-s08d-fueled-crew` returns to Sol, scraps three Furies through
Shipyard, installs a third crew pod, fills the fuel tanks through Outfit
and hires crew to 150. Native run pmid-44 saved `dev-fueled-crew` with
21000 raw fuel, 150 crew, 534 RU and three Furies. These are development
checkpoints, not the connected campaign's S08 milestone. No Arilou visit
or Talking Pet acquisition is implied by the resupply scripts.

All three scenarios accept a different earned ancestor through
`prepare_mid_segment.py --start-save`; their initial inventory and resource
assertions still apply. Shipyard sales and purchases use the ordinary menus.

### Funded shuttle and vault replay

`campaign-s09f-funded-shuttle` repeats the egg-casing and Talana proof route
from `dev-funded-knowledge`, using peaceful departure from the Druuge trade
world and opt-in escape during interstellar travel. Native run `pmid-51`
returned the casing, accepted Talana's proof dialogue, acquired the shuttle,
and saved `dev-funded-shuttle` with 145 crew and three Furies.

`campaign-s09g-funded-vault` continues from that development save. It approaches
the vault planet before navigating its moon, lands, unlocks the vault with the
shuttle, returns the lander, and saves `dev-funded-vault`. Native run `pmid-56`
proved `SHIP_VAULT_UNLOCKED=1` and `SYREEN_SHUTTLE_ON_SHIP=0`, with 145 crew,
4915 raw fuel, and three Furies. Neither save is a connected campaign milestone.

`campaign-s09d-syreen-ambush` returns the recovered ships to Talana from
`dev-funded-vault`. Native `pmid-61` accepted her post-vault conversation and
proved `MYCON_AMBUSH=1`, then saved `dev-syreen-ambush`. This development save
has only 2086 raw fuel; the lure leg needs resupply first.

`campaign-s09h-rainbow-refuel` branches earlier, from `dev-funded-shuttle`,
visits the unreported rainbow world at catalog 187, then trades its location
for credits at catalog 153. Native `pmid-62` bought a full tank through the
Melnorme dialogue and saved `dev-rainbow-refueled` with 21000 raw fuel,
70 crew and three Furies. Travel casualties are retained, not replaced.

### Arilou prerequisite and Talking Pet development

`campaign-s08a-arilou-home` waits inside Gamma Circini for the natural portal,
flies through QuasiSpace, visits Arilou home, returns through the Umgah-side
exit, and reaches Beta Orionis before saving. Arilou's own conversation sets
`UMGAH_ZOMBIE_BLOBBIES=1`. Runs `pmid-59` and `pmid-64` completed those story
and navigation assertions, but failed final native screen comparison at the
portal-entry capture (#451). They are **not full native acceptance passes**.
`dev-arilou-home` retains the genuine `pmid-59` menu save solely for development;
its provenance and the Pet producer's parent digest refer to that exact file.
The scenario does not claim that the failed portal capture is visually valid.

`campaign-s08-talking-pet` loads that protected, post-Arilou Beta Orionis save.
Native `pmid-63` passed: normal-speed rendered Cyborg defeated all ten Umgah
guards, then the Dnyarri deal set `TALKING_PET_ON_SHIP=1`. Assertions also proved
`PLAYER_HYPNOTIZED=0`, the Protector still aboard, and the zombie flag cleared.
The real losses remain in `dev-talking-pet`: all three Furies were lost, while
flagship crew stayed 150. This is an independent development branch, not a
connected S08 campaign milestone.

`campaign-s09i-refueled-ambush` repeats vault recovery and Talana's ambush talk
from `dev-rainbow-refueled`. Native `pmid-66` passed, saving
`dev-refueled-ambush` with 15935 raw fuel, 64 crew, three Furies, the vault
unlocked and `MYCON_AMBUSH=1`. Its larger fuel reserve supports lure development.

### Connected bomb transit refuel

`campaign-s07a-bomb-refueled` starts from the connected `utwig-bomb.sav`,
visits Gamma Aquarii to record another rainbow world, then buys fuel from
the Melnorme at Alpha Aquilae. The northern route avoids taking the
32-crew, unescorted flagship through hostile space to Alpha Eridani.
`load-bomb-refueled-v1` independently reloads the extracted slot.

Campaign replay seeds now apply at every `Battle` entry, including hyperspace,
where the production clock seed previously replaced the script's seed.
The trace records each application, encounter entry, and retreat result.
An accepted Escape starts a vulnerable flee animation; it is not sufficient
for a surviving retreat assertion.

`campaign-s07b-west-refueled` continues from that save to Alpha Apodis,
reselects autopilot after a surviving retreat, and buys a full tank with the
remaining earned Melnorme credits. Native `pt-p7-6` saved the connected
`west-bomb-refueled` checkpoint with 20 crew, 21000 raw fuel, 1175 RU and
no escorts. The combat losses are retained. `load-west-bomb-refueled-v1`
independently checks its date, resources and inherited story state.

### Incidental patrols during a body approach

`navigate_to_planet` and `navigate_to_orbit` accept `patrol_policy`:
`stop` (the default), `retreat`, or `fight`. Retreat uses the real battle
Escape command; fight uses normal rendered Cyborg combat. Neither an escape
nor a battle win substitutes for reaching the requested body. Interruption
callbacks count against the original navigation action's `max_ticks`.
The policy cannot be combined with an `encounter_response` phrase.
Orbital patrol handling excludes homeworld dialogue, so a
`stop_at_encounter` homeworld approach still exposes its actual responses.
If a patrol returns the ship to outer-system flight, orbital navigation
reapproaches the target planet before applying inner-system steering.

### Crew and maneuvering preparation for the Mycon route

`campaign-s10a-crew-refill` returns from `dev-rainbow-refueled` to Sol and
hires crew through the shipyard. Native `pmid-75` passed, retaining the
shuttle and saving 150 crew, 18779 raw fuel, 294 RU and three Furies as
`dev-ambush-crew`. Direct return attempts from the later low-crew ambush
branch lost the flagship, so this preparation deliberately starts earlier.

`campaign-s10a-full-drives` starts from that Sol save, sells the Fury in
hangar slot 3, purchases the two missing drives (7 and 10), and installs
turning jets in slots 1 and 2. Native `pmid-78` passed and saved
`dev-ambush-drives`: all eleven drives installed, four turning jets,
150 crew, 18800 raw fuel, 315 RU and two Furies. The small fuel/RU refund
comes from dismissing escort crew through the ordinary shipyard. This is
an outfit-specific development route, not a general fleet-layout controller.
Its ancestor must have the asserted empty slots and the retained Furies
in hangar slots 3 through 5. Neither save is a connected campaign milestone.

`campaign-s10a-driven-ambush` replays vault recovery and Talana's ambush
conversation from `dev-ambush-drives`. Native `pmid-80` passed, saving
`dev-driven-ambush` with 144 crew, 12021 raw fuel, two Furies and
`MYCON_AMBUSH=1`. The vault consumed the shuttle and returned six of its
twelve lander crew. Its predecessor probe circled Betelgeuse indefinitely:
outer planet approaches now correct actual velocity rather than merely
pointing the bow toward the target. Wrong-inner-system departure retains
its outward steering. This save still does not prove the Mycon lure or
Sun Device guard victory.

`campaign-s10a-mycon-lure` starts from `dev-driven-ambush` and uses real
flagship Escape during departure, hyperspace transit and incidental body
patrols. `navigate_to_hyperspace` now accepts opt-in
`retreat_encounters: true`; omitted retains combat, and combining retreat
with `encounter_response` is rejected. Escape completion resumes the same
navigation action and its remaining budget, without declaring arrival or
fleet victory.

Native `pmid-82` passed the actual homeworld conversation at Epsilon
Scorpii: `i_have_a_cunning_plan`, then all three clues. It proved both
ambush flags and saved `dev-mycon-lure`. Repeated retreats cost fuel and
crew: the save has 70 crew, two Furies and zero fuel. It is a stranded
but genuinely earned development checkpoint, not a finished Sun Device
route or a connected campaign milestone. No resource loss is hidden.

`campaign-s10a-lure-refuel` instead branches before the stranded lure:
`dev-driven-ambush` to the trader at Alpha Vulpeculae (catalog153), contact,
then `buy`, `buy_fuel`, `fill_me_up`, and normal departure/save. Native
`pmid-86` passed and produced `dev-lure-refueled`: 116 crew, 21000 raw
fuel, 315 RU and two Furies. The fuel was paid from prior earned trader
credits. This route leaves the ambush arranged but not yet delivered.

`campaign-s10c-fueled-lure` repeats the homeworld lure from that refueled
ancestor. Native `pmid-87` completed with twelve crew after thirteen
patrol escapes. Its same-seed/source replay `pmid-88`, with distance-bounded
approach speed, completed with 92 crew after three escapes and 16835 raw
fuel remaining. The controller now cruises between bodies and reduces
requested speed near the target, while continuing to correct actual
velocity error. This changes only real thrust/turn inputs, not simulation
speed, ship capacity, fuel consumption or encounter rules. `pmid-88`
passed full native acceptance and produced `dev-fueled-mycon-lure`.
