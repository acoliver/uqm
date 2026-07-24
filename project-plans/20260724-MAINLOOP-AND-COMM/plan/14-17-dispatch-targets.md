# P14-P17: Port remaining game-loop dispatch targets to Rust

## Worker scope

Port the 4 remaining C dispatch targets that `game_loop.rs` calls through FFI.

## P14: ExploreSolarSys (planets.c, 483 lines)

### What it does
- Dispatches to interplanetary exploration: orbiting planets, scanning surfaces,
  landing, collecting resources/biological data
- Called when activity = IN_INTERPLANETARY

### Approach
- Create `rust/src/mainloop/explore_solar_sys.rs`
- Port the C dispatch logic
- May need to port parts of `planets/` subsystem (scan, surface, orbit)
- Wire `CffiOps::explore_solar_sys()` to call Rust implementation

### Test plan
**Unit tests**: Mock `GameLoopOps`, verify dispatch logic
**Automation proof** (`scripts/explore-planet-v1.json`):
- Start new game, wait for hyperspace
- Navigate to a planet (may need additional menu key actions)
- Assert IN_INTERPLANETARY activity
- Capture, finish

## P15: VisitStarBase (starbase.c, 602 lines)

### What it does
- Starbase visit: outfit ship, build modules, talk to commander
- Called when activity = IN_STARBASE

### Approach
- Create `rust/src/mainloop/visit_starbase.rs`
- Port the C starbase dispatch
- Wire `CffiOps::visit_starbase()` to call Rust implementation

### Test plan
**Unit tests**: Mock dispatch, verify state transitions
**Automation proof** (`scripts/starbase-visit-v1.json`):
- Start new game, navigate to starbase
- Assert IN_STARBASE activity
- Capture, finish

## P16: InstallBombAtEarth + hyperspace (hyper.c, 1747 lines)

### What it does
- Hyperspace navigation: moving in hyperspace, encountering aliens
- Bomb installation sequence
- Called when activity = IN_HYPERSPACE with bomb flag

### Approach
- Create `rust/src/mainloop/hyperspace.rs`
- Port the C hyperspace dispatch
- This is the largest single dispatch target
- May need to port parts of `hyper.c` navigation logic

### Test plan
**Unit tests**: Mock dispatch, verify navigation state
**Automation proof** (`scripts/hyperspace-v1.json`):
- Start new game, wait for hyperspace
- Assert IN_HYPERSPACE activity
- Capture, finish

## P17: Battle dispatch (battle.c, 517 lines)

### What it does
- Combat dispatch: initializes battle, runs frame loop, cleanup
- Called when activity = IN_BATTLE

### Approach
- Rust `battle/` module already exists with process_loop, lifecycle, etc.
- Wire `CffiOps::battle_with_frame_callback()` to call Rust battle module
- The Rust battle module may need additional FFI to replace remaining C deps

### Test plan
**Unit tests**: Mock battle dispatch, verify state transitions
**Automation proof** (`scripts/battle-v1.json`):
- Start new game, encounter hostile alien
- Choose attack
- Assert IN_BATTLE activity
- Capture, finish

## Dependencies
- P09 (comm dispatch must be ported first for encounter flow)
- Existing Rust `battle/` module (for P17)
- Existing Rust `mainloop/` infrastructure