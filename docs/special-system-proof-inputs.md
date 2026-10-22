# Pluto and Earth proof inputs are ready for a first gameplay run

These are unplayed scenario definitions, not live acceptance results. Both start
through New Game, handle the opening probe broadcast, and use the existing
feedback-driven player controls. Neither requires a save, a start scene, a
story-state write, a teleport, a clock override or a new action implementation.
The definitions add inventory and matrix entries without changing any existing
scenario, assertion, seed, deadline policy or admission rule.

## Executable subset

| Scenario | Destination and required state | Limit |
| --- | --- | --- |
| `real-sol-pluto-orbit-scan` | Planet 8 inner-system arrival, planet 8 orbit, orbit menu, auto-scan followed by Dispatch readiness, Cancel out of the landing-site picker, orbit mode again | No landing, Fwiffo pickup, conversation or recruitment claim |
| `real-sol-earth-orbit` | Planet 2 inner-system arrival, planet 2 orbit and orbit menu | No shield-pixel, surface landing or Starbase claim |

Both require interplanetary activity before navigation, encounter/dialogue
24/18 for the actual probe, one completed communication, and 1000 raw fuel / 0 RU
before and after the destination orbit. Pluto repeats the resource and orbit
assertions after scanning. Arrivals, menu transitions, assertions and captures
retain the existing ordered native checkpoint obligations. The captures are
requested evidence for a future run; no capture was produced for this change.

The scripts retain the opening nine actions of `explore-planet-v1` exactly.
They declare 25000 input ticks, 50000 presentations and a 300 s wall ceiling.
Both use the existing default outer deadline. These are failure ceilings, not
measured successful runtimes. A navigation or wall-budget failure must be
investigated from its trace, not hidden by a skipped step or a changed assertion.

Pluto belongs to Sol. `sol::generate_orbital` loads its Fwiffo report scene while
`FOUND_PLUTO_SPATHI` is zero; the energy pickup later writes that flag and
launches the lander. This proposal reaches orbit and scans but never performs
that pickup. It does not prove the flag's value or that an energy node appeared.
It does not exercise Spathi homeworld generator index 5.

Earth orbit drives the newly ported Earth resource/surface load path. The
existing headless additive-pixel fixture is separate evidence. Orbit mode alone
cannot establish correct red shield pixels, so this script makes no such claim.

## What the existing inputs already attempt

The current inventory has real-path inputs for the probe, Commander and allied
Starbase, Mercury radioactives, Luna moonbase departure, and Mars orbit/scan and
lander return. Several later scripts load `saves/allied-starbase.sav`, then
re-dock through Earth moon 0, the Starbase. That committed save's provenance
records the alliance producer, interplanetary flight near Earth, 896 raw fuel and
2500 RU. It supplies no distant-system location or late-story state.

There is no existing typed action that flies to a named hyperspace star. The
hyperspace action leaves a solar system, while planet/moon/orbit navigation
addresses bodies within the current system. The flight-leg actions prove
movement and facing changes but do not prove entry into a particular distant
special system. No legitimate save for a special-system destination was added
here. No save-dependent placeholder was admitted to the inventory.

Consequently these special-system indexes remain without gameplay inputs in
this delivery: Utwig 22/36, Mycon 24/31/32/33/34, VUX 3/28/39, Sa-Matra 29 and
Spathi homeworld 5. Their reachable story branches also require gameplay-produced
state, not just arriving at the corresponding star. A future Fwiffo landing
needs a real lander route to the Pluto energy site and assertions for the
resulting encounter/state; this orbit/scan input does not substitute for it.

## Run after lane025 is released

Do not launch, enqueue, kill or replace a game while lane025 is retained. No
native command in this section was executed for this change.

After that lane is released, use a clean checkout at the full proposal commit,
the established iTerm native-window execution route, the pinned content package
and a fresh evidence directory. The native runner builds the production game
and validates its source inputs before launch. Run the smaller Earth path first,
then Pluto in a separate fresh bundle if Earth completes:

```sh
unset SDKROOT MACOSX_DEPLOYMENT_TARGET
export CARGO_BUILD_JOBS=4
: "${UQM_CONTENT_DIR:?directory holding the pinned content package is required}"
: "${UQM_PROOF_ROOT:?fresh external evidence directory is required}"
: "${UQM_TARGET_DIR:?isolated build target directory is required}"
export CARGO_TARGET_DIR="$UQM_TARGET_DIR"
cargo +1.97.1 run --locked --manifest-path rust/xtask/Cargo.toml -- \
  native run --scenario real-sol-earth-orbit \
  --content "$UQM_CONTENT_DIR" --artifacts "$UQM_PROOF_ROOT/earth"
cargo +1.97.1 run --locked --manifest-path rust/xtask/Cargo.toml -- \
  native validate "$UQM_PROOF_ROOT/earth" --scenario real-sol-earth-orbit
cargo +1.97.1 run --locked --manifest-path rust/xtask/Cargo.toml -- \
  native run --scenario real-sol-pluto-orbit-scan \
  --content "$UQM_CONTENT_DIR" --artifacts "$UQM_PROOF_ROOT/pluto"
cargo +1.97.1 run --locked --manifest-path rust/xtask/Cargo.toml -- \
  native validate "$UQM_PROOF_ROOT/pluto" --scenario real-sol-pluto-orbit-scan
```

Retain the source/executable/content/config identities, trace, actual OS window
captures, offline validation and exact-child teardown. A success recorded by a
pure schema test is not a gameplay success. Compare expected and observed
navigation inputs at the first failure before changing the source. No native
proof or visual judgment is claimed by this document.

## Preparing saves without inventing provenance

The existing extractor accepts only a finalized passing bundle containing
exactly `real-sol-starbase-alliance`. For that save, preparation remains:

```sh
cargo +1.97.1 run --locked --manifest-path rust/xtask/Cargo.toml -- \
  native run --scenario real-sol-starbase-alliance \
  --content "$UQM_CONTENT_DIR" --artifacts "$UQM_PROOF_ROOT/alliance"
cargo +1.97.1 run --locked --manifest-path rust/xtask/Cargo.toml -- \
  native validate "$UQM_PROOF_ROOT/alliance" --scenario real-sol-starbase-alliance
cargo +1.97.1 run --locked --manifest-path rust/xtask/Cargo.toml -- \
  native extract-save --artifacts "$UQM_PROOF_ROOT/alliance" --slot 0 \
  --output rust/scripts/saves/allied-starbase.sav
```

The extractor copies the recorded save bytes and generates the provenance from
that run. Commit the save and generated provenance together if regenerating it;
do not relabel an alliance bundle as a homeworld producer. These commands are
preparation instructions only and were not run here.

For a distant special-system save, first play the route and required story
choices through normal game controls, save through the production Game menu,
and retain the original successful proof bundle. The current alliance-only
extractor cannot authenticate an arbitrary distant-system producer. It needs a
separately scoped producer contract before such a save can be published through
this path. Do not create a provenance JSON by hand, rename the alliance save,
patch save bytes or add a missing-save skip. Until real bytes and producer
evidence exist, keep that destination out of the authoritative scenario
inventory. Existing missing-input admission remains fail-fast.

## Source verification scope

`special_system_scenario_inputs` validates the complete JSON definitions,
unchanged opening probe inputs, exact destination tails, closed root/action
fields, target bounds, positive navigation bounds, sufficient static budgets,
terminal finish, ordered checkpoint classes, inventory membership and default
outer-deadline selection. Suite classification, deadline admission, save-input
regressions, strict focused Clippy, workspace formatting and workflow checks
are source-only checks. No gameplay, save creation or screenshot follows from
those checks. Review skipped per Andrew 2026-10-09.

The following checks passed on the proposal's source bytes with Rust 1.97.1,
locked dependencies, four jobs, unset SDKROOT/deployment overrides and an
isolated target directory. All 42 selected tests passed with zero failures or
ignored tests. These commands did not launch a game:

```sh
cargo +1.97.1 test --locked --manifest-path rust/Cargo.toml \
  --no-default-features --features audio_heart,debug-process \
  --test special_system_scenario_inputs
cargo +1.97.1 test --locked --manifest-path rust/Cargo.toml \
  --no-default-features --features audio_heart,debug-process \
  --lib automation::suite::tests
cargo +1.97.1 test --locked --manifest-path rust/xtask/Cargo.toml \
  --lib native_suite::outer_deadline_tests
cargo +1.97.1 test --locked --manifest-path rust/xtask/Cargo.toml \
  --lib native_save::tests
cargo +1.97.1 clippy --locked --manifest-path rust/Cargo.toml \
  --no-default-features --features audio_heart,debug-process \
  --test special_system_scenario_inputs -- -D warnings
cargo +1.97.1 fmt --manifest-path rust/Cargo.toml --all -- --check
cargo +1.97.1 run --locked --manifest-path rust/xtask/Cargo.toml -- ci workflow-check
cargo +1.97.1 run --locked --manifest-path rust/xtask/Cargo.toml -- \
  native list navigation_orbit_scan --json
git diff --check
```

The inventory check admitted both names at the unchanged default seed
1437213463. Before committing, a byte comparison confirmed that all 38 existing
scenario/save JSON and SAV inputs were unchanged, and that the two new pins
were the only changes to `gates.json` data. Existing scenario assertions and
resource thresholds were not edited.
