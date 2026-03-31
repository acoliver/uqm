//! Ship Spawn & Battle Lifecycle
//!
//! @plan PLAN-20260314-SHIPS.P09
//! @requirement REQ-SPAWN-SEQUENCE, REQ-SPAWN-IDEMPOTENT, REQ-BATTLE-INIT, REQ-BATTLE-TEARDOWN, REQ-SPAWN-FAILURE, REQ-FAILURE-ISOLATION
//!
//! Implements the ship spawn and battle lifecycle, corresponding to C `ship.c` spawn_ship(),
//! `init.c` InitShips()/UninitShips()/InitSpace()/UninitSpace().
//!
//! ## Boundary Ownership Split
//!
//! ### Moved to Rust (ship-runtime dependencies)
//! - Space ref-count management (init_space/uninit_space)
//! - Shared ship-runtime asset tracking (explosion, blast, asteroid handles)
//! - Ship spawn sequence (load descriptor, patch crew, configure element)
//! - Ship teardown (free descriptors, write back crew)
//! - Ship selection (get_next_starship)
//!
//! ### Remains in C (battle/environment orchestration)
//! - Display list initialization (InitDisplayList)
//! - Galaxy/background rendering (InitGalaxy)
//! - Graphics context management (SetContext)
//! - Planet/asteroid spawning (spawn_planet, spawn_asteroid)
//! - Gravity well management (free_gravity_well)
//! - Hyperspace loading (LoadHyperspace, FreeHyperspace)
//! - Display array / PrimType management
//! - Element allocation (AllocElement, InsertElement) — called via FFI in P14
//! - Global activity state management (GLOBAL(CurrentActivity))
//! - Input flushing
//! - Super Melee ship selection UI (GetInitialMeleeStarShips)
//! - Player order (GetPlayerOrder)

use super::loader::{load_ship, LoadTier};
use super::types::{ShipsError, Starship};
use std::sync::Mutex;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

pub const NUM_SIDES: u32 = 2;
pub const NUM_PLAYERS: usize = 2;
pub const RPG_PLAYER_NUM: i16 = 0;
pub const NPC_PLAYER_NUM: i16 = 1;
pub const IN_ENCOUNTER: u8 = 2;
pub const IN_LAST_BATTLE: u8 = 3;
pub const SUPER_MELEE: u8 = 1;
pub const CREW_OBJECT: u16 = 1 << 9; // element state flag

// ---------------------------------------------------------------------------
// BattleState
// ---------------------------------------------------------------------------

/// Global battle state tracking space initialization and shared assets.
#[derive(Default)]
struct BattleState {
    space_init_count: u32,
    ships_initialized: bool,
    /// Loaded shared assets (explosion, blast, asteroid resource handles)
    explosion_assets: [usize; 3], // big, med, sml
    blast_assets: [usize; 3],
    asteroid_assets: [usize; 3],
    stars_in_space: usize,
}

static BATTLE_STATE: Mutex<BattleState> = Mutex::new(BattleState {
    space_init_count: 0,
    ships_initialized: false,
    explosion_assets: [0; 3],
    blast_assets: [0; 3],
    asteroid_assets: [0; 3],
    stars_in_space: 0,
});

// ---------------------------------------------------------------------------
// Result Types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpawnResult {
    Spawned,
    AlreadySpawned,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UninitResult {
    pub crew_retrieved: u16,
}

// ---------------------------------------------------------------------------
// Core Functions
// ---------------------------------------------------------------------------

/// Spawns a ship by loading its descriptor and preparing for battle.
///
/// # C Reference
/// `spawn_ship()` in ship.c lines 380-497
///
/// # Steps
/// 1. Load descriptor: `loader::load_ship(starship.species_id, LoadTier::BattleReady)?`
/// 2. Clear input/status: `ship_input_state = 0, cur_status_flags = empty, old_status_flags = empty`
/// 3. Patch crew: If activity is IN_ENCOUNTER or IN_LAST_BATTLE and starship.crew_level > 0,
///    set `desc.ship_info.crew_level = starship.crew_level`. Clamp to max_crew.
/// 4. Clear counters: `energy_counter = 0, weapon_counter = 0, special_counter = 0`
/// 5. Store descriptor in starship: `starship.race_desc = Some(Box::new(desc))`
///
/// Element allocation and field setup (mass, state_flags, position, callbacks)
/// are handled by C's `rust_bridge_spawn_element()`. See P00 for the C helper
/// implementation.
///
/// # Returns
/// - `Ok(SpawnResult::Spawned)` on success
/// - `Ok(SpawnResult::AlreadySpawned)` if starship already has a race_desc (idempotent)
/// - `Err(ShipsError::LoadFailed)` on load failure (no partial state)
pub fn spawn_ship(starship: &mut Starship, activity: u8) -> Result<SpawnResult, ShipsError> {
    // Early return if already spawned (idempotent)
    if starship.race_desc.is_some() {
        return Ok(SpawnResult::AlreadySpawned);
    }

    // Load descriptor
    let mut desc = load_ship(starship.species_id, LoadTier::BattleReady)?;

    // Clear input/status
    starship.ship_input_state = 0;
    starship.cur_status_flags = super::types::StatusFlags::empty();
    starship.old_status_flags = super::types::StatusFlags::empty();

    // Patch crew (C: ship.c lines 395-408)
    if activity == IN_ENCOUNTER || activity == IN_LAST_BATTLE {
        if starship.crew_level == 0 {
            // SIS/flagship: crew is already set by sis_ship.c via
            // GLOBAL_SIS(CrewEnlisted). The descriptor keeps its loaded default.
            // C: ship.c lines 398-402 — the block is commented out in C as well.
        } else {
            let max_crew = desc.ship_info.max_crew;
            desc.ship_info.crew_level = starship.crew_level.min(max_crew);
        }
    }

    // Clear counters
    starship.energy_counter = 0;
    starship.weapon_counter = 0;
    starship.special_counter = 0;

    // Element allocation and field setup (mass, state_flags, position,
    // callbacks) are handled by C's rust_bridge_spawn_element().
    // See P00 for the C helper implementation.

    // Store descriptor in starship
    starship.race_desc = Some(Box::new(desc));

    Ok(SpawnResult::Spawned)
}

/// Initializes space resources (explosion/blast/asteroid animations).
///
/// # C Reference
/// `InitSpace()` in init.c lines 114-144
///
/// C gates on `LOBYTE(GLOBAL(CurrentActivity)) <= IN_ENCOUNTER`; the `activity`
/// parameter mirrors that check. Asset handles are zeroed here; actual resource
/// loading occurs through the C bridge once wired in P14.
///
/// # Behavior
/// - Reference-counted (increment space_init_count)
/// - On first init: resets asset handles (real loading is done by C via P14 FFI)
/// - Safe to call multiple times
pub fn init_space() -> Result<(), ShipsError> {
    let mut state = BATTLE_STATE
        .lock()
        .map_err(|_| ShipsError::InvalidState("Failed to lock battle state".to_string()))?;

    state.space_init_count += 1;

    if state.space_init_count == 1 {
        // C: init.c lines 119-141 — loads star graphic, explosion, blast,
        // asteroid animations. Actual resource handles are provided by C
        // bridge (P14); Rust tracks them here for ref-counted free.
        state.explosion_assets = [0; 3];
        state.blast_assets = [0; 3];
        state.asteroid_assets = [0; 3];
        state.stars_in_space = 0;
    }

    Ok(())
}

/// Uninitializes space resources.
///
/// # C Reference
/// `UninitSpace()` in init.c lines 147-158
///
/// # Behavior
/// - Decrements ref count
/// - On reaching 0: frees assets
/// - Safe to call multiple times
///
/// # Returns
/// `Ok(())` on success
pub fn uninit_space() -> Result<(), ShipsError> {
    let mut state = BATTLE_STATE
        .lock()
        .map_err(|_| ShipsError::InvalidState("Failed to lock battle state".to_string()))?;

    if state.space_init_count > 0 {
        state.space_init_count -= 1;

        if state.space_init_count == 0 {
            // C: init.c lines 151-157 — free_image(blast/explosion/asteroid),
            // DestroyDrawable(stars_in_space). Actual freeing through C bridge
            // (P14); Rust zeroes handles to mark them released.
            state.explosion_assets = [0; 3];
            state.blast_assets = [0; 3];
            state.asteroid_assets = [0; 3];
            state.stars_in_space = 0;
        }
    }

    Ok(())
}

/// Initializes ships for battle.
///
/// # C Reference
/// `InitShips()` in init.c lines 178-242
///
/// # Behavior
/// - Calls `init_space()`
/// - Sets ships_initialized = true
/// - Returns NUM_SIDES for normal battle, 1 for hyperspace
/// - Does NOT do display list/galaxy/context setup (those stay in C)
///
/// # Returns
/// `Ok(NUM_SIDES)` on success (number of battle sides)
pub fn init_ships(activity: u8) -> Result<u32, ShipsError> {
    init_space()?;

    let mut state = BATTLE_STATE
        .lock()
        .map_err(|_| ShipsError::InvalidState("Failed to lock battle state".to_string()))?;

    state.ships_initialized = true;

    // C: init.c lines 189-193
    // Returns NUM_SIDES for normal battle, 1 for hyperspace
    if activity == SUPER_MELEE || activity == IN_ENCOUNTER || activity == IN_LAST_BATTLE {
        Ok(NUM_SIDES)
    } else {
        Ok(1) // Hyperspace
    }
}

/// Uninitializes ships after battle.
///
/// # C Reference
/// `UninitShips()` in init.c lines 268-349
///
/// Delegates crew writeback, floating crew distribution, and descriptor
/// freeing to [`writeback::battle_teardown_writeback()`], which faithfully
/// follows C sequencing:
/// 1. Add floating crew to survivor's descriptor (before writeback)
/// 2. Copy descriptor crew → starship.crew_level
/// 3. Write back starship crew → fragment via queue-position matching
/// 4. Free all descriptors
///
/// `floating_crew` is the count of crew-object elements still in the display
/// list (C: `CountCrewElements()`). The caller obtains this from C before
/// invoking uninit. When no display list is available (pure Rust tests),
/// pass 0.
///
/// `fragment_queues` carries the persistent fleet fragments for crew
/// writeback. Pass empty vecs when fragments are managed in C (P14).
///
/// `survivor_side` identifies which side's first spawned ship (if any)
/// receives the floating crew. Pass `None` when there is no survivor.
pub fn uninit_ships(
    race_queues: &mut [Vec<Starship>; NUM_PLAYERS],
    fragment_queues: &mut [Vec<super::writeback::ShipFragment>; NUM_PLAYERS],
    _activity: u8,
    floating_crew: u16,
    survivor_side: Option<usize>,
) -> Result<UninitResult, ShipsError> {
    // Delegate to writeback module for C-faithful teardown sequencing
    let teardown_result = super::writeback::battle_teardown_writeback(
        race_queues,
        fragment_queues,
        floating_crew,
        survivor_side,
    )?;

    // Mark ships as uninitialized
    let mut state = BATTLE_STATE
        .lock()
        .map_err(|_| ShipsError::InvalidState("Failed to lock battle state".to_string()))?;
    state.ships_initialized = false;

    drop(state); // Release lock before calling uninit_space
    uninit_space()?;

    Ok(UninitResult {
        crew_retrieved: teardown_result.floating_crew,
    })
}

/// Finds the first unspawned alive starship in the queue.
///
/// # C Reference
/// Part of `GetNextStarShip()` in ship.c
///
/// # Behavior
/// - Finds first entry in queue that: has no race_desc (not spawned), crew_level > 0 (not dead),
///   and species_id != NO_ID (not deactivated)
/// - Returns the index or None
///
/// # Returns
/// - `Some(index)` if a valid starship is found
/// - `None` if all starships are spawned, dead, or deactivated
pub fn get_next_starship(queue: &[Starship]) -> Option<usize> {
    use super::types::SpeciesId;
    queue
        .iter()
        .position(|s| s.race_desc.is_none() && s.crew_level > 0 && s.species_id != SpeciesId::NoId)
}

/// Spawns the initial starships for battle (one per side).
///
/// # C Reference
/// `GetInitialStarShips()` in ship.c lines 537-573
///
/// # Behavior
/// - For each side, find the first available starship and spawn it
/// - For non-SUPER_MELEE: iterate sides in reverse order (NUM_PLAYERS-1 down to 0)
///
/// # Returns
/// `Ok(())` on success
pub fn get_initial_starships(
    race_queues: &mut [Vec<Starship>; NUM_PLAYERS],
    activity: u8,
) -> Result<(), ShipsError> {
    // C: ship.c lines 554-570
    // For non-SUPER_MELEE, iterate in reverse order
    let order: Vec<usize> = if activity == SUPER_MELEE {
        (0..NUM_PLAYERS).collect()
    } else {
        (0..NUM_PLAYERS).rev().collect()
    };

    for &side in &order {
        if let Some(index) = get_next_starship(&race_queues[side]) {
            spawn_ship(&mut race_queues[side][index], activity)?;
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Lifecycle State API (P02)
// ---------------------------------------------------------------------------

/// Mark battle ships as initialized (called after successful arena setup).
pub(crate) fn mark_ships_initialized() {
    let mut state = BATTLE_STATE
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    state.ships_initialized = true;
}

/// Mark battle ships as uninitialized (called after teardown).
pub(crate) fn mark_ships_uninitialized() {
    let mut state = BATTLE_STATE
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    state.ships_initialized = false;
}

/// Query whether ships are currently initialized.
/// Used by uninit idempotence guard (P03) — not test-only.
pub(crate) fn is_ships_initialized_for_uninit() -> bool {
    let state = BATTLE_STATE
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    state.ships_initialized
}

/// Query whether ships are currently initialized (test convenience).
#[cfg(test)]
pub(crate) fn is_ships_initialized() -> bool {
    is_ships_initialized_for_uninit()
}

// ---------------------------------------------------------------------------
// Testing Helper
// ---------------------------------------------------------------------------

#[cfg(test)]
pub(crate) fn reset_battle_state() {
    let mut state = BATTLE_STATE.lock().unwrap();
    *state = BattleState::default();
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::super::types::SpeciesId;
    use super::super::writeback::ShipFragment;
    use super::*;

    fn cleanup() {
        reset_battle_state();
    }

    // Helper to create a test starship
    fn make_test_starship(species: SpeciesId, crew: u16, player: i16) -> Starship {
        Starship {
            species_id: species,
            crew_level: crew,
            player_nr: player,
            ..Starship::default()
        }
    }

    // -- spawn_ship tests ---------------------------------------------------

    #[test]
    fn spawn_ship_successful_spawn() {
        cleanup();
        let mut starship = make_test_starship(SpeciesId::Earthling, 20, RPG_PLAYER_NUM);
        let result = spawn_ship(&mut starship, IN_ENCOUNTER);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), SpawnResult::Spawned);
        assert!(starship.race_desc.is_some());
        assert_eq!(starship.ship_input_state, 0);
        assert!(starship.cur_status_flags.is_empty());
        assert!(starship.old_status_flags.is_empty());
        assert_eq!(starship.energy_counter, 0);
        assert_eq!(starship.weapon_counter, 0);
        assert_eq!(starship.special_counter, 0);
    }

    #[test]
    fn spawn_ship_idempotent() {
        cleanup();
        let mut starship = make_test_starship(SpeciesId::Spathi, 30, NPC_PLAYER_NUM);
        spawn_ship(&mut starship, IN_ENCOUNTER).unwrap();
        let result = spawn_ship(&mut starship, IN_ENCOUNTER);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), SpawnResult::AlreadySpawned);
    }

    #[test]
    fn spawn_ship_crew_patching_in_encounter() {
        cleanup();
        // Create starship with crew_level different from species default
        let mut starship = make_test_starship(SpeciesId::Earthling, 10, RPG_PLAYER_NUM);

        // Get species default for comparison
        let default_desc = crate::ships::loader::load_ship(
            SpeciesId::Earthling,
            crate::ships::loader::LoadTier::MetadataOnly,
        )
        .unwrap();
        let default_crew = default_desc.ship_info.crew_level;

        // Spawn in encounter (should patch crew)
        spawn_ship(&mut starship, IN_ENCOUNTER).unwrap();
        let crew_in_desc = starship.race_desc.as_ref().unwrap().ship_info.crew_level;

        // In IN_ENCOUNTER activity with non-zero starship.crew_level,
        // descriptor crew should be patched from starship (not species default)
        assert_eq!(starship.crew_level, 10);
        assert_ne!(default_crew, 10); // Verify we're not testing default
        assert_eq!(crew_in_desc, 10); // Should match starship's crew
    }

    #[test]
    fn spawn_ship_crew_patching_in_last_battle() {
        cleanup();
        // Test with Spathi (30 default crew)
        let mut starship = make_test_starship(SpeciesId::Spathi, 15, RPG_PLAYER_NUM);

        spawn_ship(&mut starship, IN_LAST_BATTLE).unwrap();
        let crew_in_desc = starship.race_desc.as_ref().unwrap().ship_info.crew_level;

        // In IN_LAST_BATTLE with non-zero starship.crew_level,
        // descriptor crew should be patched from starship
        assert_eq!(starship.crew_level, 15);
        assert_eq!(crew_in_desc, 15); // Should match starship's crew (not 30)
    }

    #[test]
    fn spawn_ship_crew_not_patched_in_super_melee() {
        cleanup();
        let mut starship = make_test_starship(SpeciesId::Utwig, 5, RPG_PLAYER_NUM);

        // Get species default for comparison
        let default_desc = crate::ships::loader::load_ship(
            SpeciesId::Utwig,
            crate::ships::loader::LoadTier::MetadataOnly,
        )
        .unwrap();
        let default_crew = default_desc.ship_info.crew_level;

        starship.crew_level = 5; // Set lower than default
        spawn_ship(&mut starship, SUPER_MELEE).unwrap();
        let crew_in_desc = starship.race_desc.as_ref().unwrap().ship_info.crew_level;

        // In SUPER_MELEE, descriptor default is used (not patched from starship)
        assert_eq!(starship.crew_level, 5); // Starship value unchanged
        assert_eq!(crew_in_desc, default_crew); // Descriptor has species default
    }

    #[test]
    fn spawn_ship_crew_clamped_to_max() {
        cleanup();
        let mut starship = make_test_starship(SpeciesId::Chmmr, 10000, RPG_PLAYER_NUM);
        spawn_ship(&mut starship, IN_ENCOUNTER).unwrap();
        let desc = starship.race_desc.as_ref().unwrap();
        let crew_in_desc = desc.ship_info.crew_level;
        let max_crew = desc.ship_info.max_crew;
        assert_eq!(crew_in_desc, max_crew); // Clamped
        assert!(crew_in_desc < 10000);
    }

    #[test]
    fn spawn_ship_no_id_fails() {
        cleanup();
        let mut starship = make_test_starship(SpeciesId::NoId, 20, RPG_PLAYER_NUM);
        let result = spawn_ship(&mut starship, IN_ENCOUNTER);
        assert!(result.is_err());
        assert!(starship.race_desc.is_none()); // No partial state
    }

    #[test]
    fn spawn_ship_descriptor_loaded_with_mass() {
        cleanup();
        let mut starship = make_test_starship(SpeciesId::Orz, 20, RPG_PLAYER_NUM);
        spawn_ship(&mut starship, IN_ENCOUNTER).unwrap();
        // Verify descriptor is loaded and has valid ship mass
        // (used by C helper rust_bridge_spawn_element for element creation)
        assert!(starship.race_desc.is_some());
        let desc = starship.race_desc.as_ref().unwrap();
        assert!(desc.characteristics.ship_mass > 0);
    }

    // -- init_space / uninit_space tests ------------------------------------

    #[test]
    fn init_space_ref_counting() {
        cleanup();
        init_space().unwrap();
        {
            let state = BATTLE_STATE.lock().unwrap();
            assert_eq!(state.space_init_count, 1);
        }
        init_space().unwrap();
        {
            let state = BATTLE_STATE.lock().unwrap();
            assert_eq!(state.space_init_count, 2);
        }
        uninit_space().unwrap();
        {
            let state = BATTLE_STATE.lock().unwrap();
            assert_eq!(state.space_init_count, 1);
        }
        uninit_space().unwrap();
        {
            let state = BATTLE_STATE.lock().unwrap();
            assert_eq!(state.space_init_count, 0);
        }
    }

    #[test]
    fn uninit_space_safe_when_zero() {
        cleanup();
        uninit_space().unwrap();
        let state = BATTLE_STATE.lock().unwrap();
        assert_eq!(state.space_init_count, 0);
    }

    // -- init_ships / uninit_ships tests ------------------------------------

    #[test]
    fn init_ships_returns_num_sides_for_encounter() {
        cleanup();
        let result = init_ships(IN_ENCOUNTER).unwrap();
        assert_eq!(result, NUM_SIDES);
        let state = BATTLE_STATE.lock().unwrap();
        assert!(state.ships_initialized);
    }

    #[test]
    fn init_ships_returns_num_sides_for_super_melee() {
        cleanup();
        let result = init_ships(SUPER_MELEE).unwrap();
        assert_eq!(result, NUM_SIDES);
    }

    #[test]
    fn init_ships_returns_one_for_hyperspace() {
        cleanup();
        let result = init_ships(0).unwrap(); // activity=0 is not SUPER_MELEE/IN_ENCOUNTER/IN_LAST_BATTLE
        assert_eq!(result, 1);
    }

    #[test]
    fn uninit_ships_round_trip() {
        cleanup();
        let mut queues: [Vec<Starship>; NUM_PLAYERS] = [Vec::new(), Vec::new()];
        queues[0].push(make_test_starship(SpeciesId::Earthling, 20, RPG_PLAYER_NUM));
        queues[1].push(make_test_starship(SpeciesId::Vux, 15, NPC_PLAYER_NUM));

        init_ships(IN_ENCOUNTER).unwrap();
        spawn_ship(&mut queues[0][0], IN_ENCOUNTER).unwrap();
        spawn_ship(&mut queues[1][0], IN_ENCOUNTER).unwrap();

        let mut frags: [Vec<ShipFragment>; NUM_PLAYERS] = [Vec::new(), Vec::new()];
        let result = uninit_ships(&mut queues, &mut frags, IN_ENCOUNTER, 0, None).unwrap();
        assert!(result.crew_retrieved == 0); // no floating crew passed
        assert!(queues[0][0].race_desc.is_none());
        assert!(queues[1][0].race_desc.is_none());

        let state = BATTLE_STATE.lock().unwrap();
        assert!(!state.ships_initialized);
    }

    #[test]
    fn uninit_ships_writes_back_crew() {
        cleanup();
        let mut queues: [Vec<Starship>; NUM_PLAYERS] = [Vec::new(), Vec::new()];
        queues[0].push(make_test_starship(SpeciesId::Syreen, 20, RPG_PLAYER_NUM));

        init_ships(IN_ENCOUNTER).unwrap();
        spawn_ship(&mut queues[0][0], IN_ENCOUNTER).unwrap();

        // Modify crew in descriptor
        if let Some(desc) = queues[0][0].race_desc.as_mut() {
            desc.ship_info.crew_level = 15; // Simulate damage
        }

        let mut frags: [Vec<ShipFragment>; NUM_PLAYERS] = [Vec::new(), Vec::new()];
        uninit_ships(&mut queues, &mut frags, IN_ENCOUNTER, 0, None).unwrap();
        assert_eq!(queues[0][0].crew_level, 15); // Written back
    }

    #[test]
    fn uninit_ships_crew_retrieved_count() {
        cleanup();
        let mut queues: [Vec<Starship>; NUM_PLAYERS] = [Vec::new(), Vec::new()];
        queues[0].push(make_test_starship(SpeciesId::Earthling, 20, RPG_PLAYER_NUM));
        queues[0].push(make_test_starship(SpeciesId::Spathi, 30, RPG_PLAYER_NUM));

        init_ships(IN_ENCOUNTER).unwrap();
        spawn_ship(&mut queues[0][0], IN_ENCOUNTER).unwrap();
        spawn_ship(&mut queues[0][1], IN_ENCOUNTER).unwrap();

        let mut frags: [Vec<ShipFragment>; NUM_PLAYERS] = [Vec::new(), Vec::new()];
        let result = uninit_ships(&mut queues, &mut frags, IN_ENCOUNTER, 0, None).unwrap();
        // No floating crew passed, so crew_retrieved is just the floating_crew input (0)
        assert_eq!(result.crew_retrieved, 0);
    }

    // -- get_next_starship tests --------------------------------------------

    #[test]
    fn get_next_starship_finds_first_unspawned_alive() {
        cleanup();
        let mut queue = vec![
            make_test_starship(SpeciesId::Earthling, 20, RPG_PLAYER_NUM),
            make_test_starship(SpeciesId::Spathi, 30, RPG_PLAYER_NUM),
            make_test_starship(SpeciesId::Pkunk, 10, RPG_PLAYER_NUM),
        ];

        // All unspawned and alive
        assert_eq!(get_next_starship(&queue), Some(0));

        // Spawn first
        spawn_ship(&mut queue[0], IN_ENCOUNTER).unwrap();
        assert_eq!(get_next_starship(&queue), Some(1));

        // Spawn second
        spawn_ship(&mut queue[1], IN_ENCOUNTER).unwrap();
        assert_eq!(get_next_starship(&queue), Some(2));

        // Spawn all
        spawn_ship(&mut queue[2], IN_ENCOUNTER).unwrap();
        assert_eq!(get_next_starship(&queue), None);
    }

    #[test]
    fn get_next_starship_skips_dead_ships() {
        cleanup();
        let queue = vec![
            make_test_starship(SpeciesId::Earthling, 0, RPG_PLAYER_NUM), // Dead
            make_test_starship(SpeciesId::Spathi, 30, RPG_PLAYER_NUM),   // Alive
        ];

        assert_eq!(get_next_starship(&queue), Some(1));
    }

    #[test]
    fn get_next_starship_returns_none_for_all_dead() {
        cleanup();
        let queue = vec![
            make_test_starship(SpeciesId::Earthling, 0, RPG_PLAYER_NUM),
            make_test_starship(SpeciesId::Spathi, 0, RPG_PLAYER_NUM),
        ];

        assert_eq!(get_next_starship(&queue), None);
    }

    #[test]
    fn get_next_starship_empty_queue() {
        cleanup();
        let queue: Vec<Starship> = Vec::new();
        assert_eq!(get_next_starship(&queue), None);
    }

    // -- get_initial_starships tests ----------------------------------------

    #[test]
    fn get_initial_starships_spawns_one_per_side() {
        cleanup();
        let mut queues: [Vec<Starship>; NUM_PLAYERS] = [Vec::new(), Vec::new()];
        queues[0].push(make_test_starship(SpeciesId::Earthling, 20, RPG_PLAYER_NUM));
        queues[1].push(make_test_starship(SpeciesId::Vux, 15, NPC_PLAYER_NUM));

        get_initial_starships(&mut queues, IN_ENCOUNTER).unwrap();
        assert!(queues[0][0].race_desc.is_some());
        assert!(queues[1][0].race_desc.is_some());
    }

    #[test]
    fn get_initial_starships_super_melee_order() {
        cleanup();
        let mut queues: [Vec<Starship>; NUM_PLAYERS] = [Vec::new(), Vec::new()];
        queues[0].push(make_test_starship(SpeciesId::Earthling, 20, 0));
        queues[1].push(make_test_starship(SpeciesId::Vux, 15, 1));

        get_initial_starships(&mut queues, SUPER_MELEE).unwrap();
        // Both spawned, order doesn't matter for this test
        assert!(queues[0][0].race_desc.is_some());
        assert!(queues[1][0].race_desc.is_some());
    }

    #[test]
    fn get_initial_starships_encounter_reverse_order() {
        cleanup();
        let mut queues: [Vec<Starship>; NUM_PLAYERS] = [Vec::new(), Vec::new()];
        queues[0].push(make_test_starship(SpeciesId::Earthling, 20, 0));
        queues[1].push(make_test_starship(SpeciesId::Vux, 15, 1));

        get_initial_starships(&mut queues, IN_ENCOUNTER).unwrap();
        // Both spawned
        assert!(queues[0][0].race_desc.is_some());
        assert!(queues[1][0].race_desc.is_some());
    }

    #[test]
    fn get_initial_starships_empty_queue_side() {
        cleanup();
        let mut queues: [Vec<Starship>; NUM_PLAYERS] = [Vec::new(), Vec::new()];
        queues[0].push(make_test_starship(SpeciesId::Earthling, 20, 0));
        // queues[1] is empty

        let result = get_initial_starships(&mut queues, IN_ENCOUNTER);
        assert!(result.is_ok());
        assert!(queues[0][0].race_desc.is_some());
    }

    #[test]
    fn get_initial_starships_all_dead() {
        cleanup();
        let mut queues: [Vec<Starship>; NUM_PLAYERS] = [Vec::new(), Vec::new()];
        queues[0].push(make_test_starship(SpeciesId::Earthling, 0, 0)); // Dead
        queues[1].push(make_test_starship(SpeciesId::Vux, 0, 1)); // Dead

        let result = get_initial_starships(&mut queues, IN_ENCOUNTER);
        assert!(result.is_ok());
        assert!(queues[0][0].race_desc.is_none());
        assert!(queues[1][0].race_desc.is_none());
    }

    // -- Edge cases ---------------------------------------------------------

    #[test]
    fn spawn_ship_failure_no_partial_race_desc() {
        cleanup();
        let mut starship = make_test_starship(SpeciesId::NoId, 20, RPG_PLAYER_NUM);
        let result = spawn_ship(&mut starship, IN_ENCOUNTER);
        assert!(result.is_err());
        assert!(starship.race_desc.is_none());
    }

    #[test]
    fn init_uninit_idempotency() {
        cleanup();
        init_ships(IN_ENCOUNTER).unwrap();
        let mut queues: [Vec<Starship>; NUM_PLAYERS] = [Vec::new(), Vec::new()];
        let mut frags: [Vec<ShipFragment>; NUM_PLAYERS] = [Vec::new(), Vec::new()];
        uninit_ships(&mut queues, &mut frags, IN_ENCOUNTER, 0, None).unwrap();

        // Second uninit should be safe
        uninit_ships(&mut queues, &mut frags, IN_ENCOUNTER, 0, None).unwrap();

        let state = BATTLE_STATE.lock().unwrap();
        assert!(!state.ships_initialized);
        assert_eq!(state.space_init_count, 0);
    }

    // -- SIS zero-crew special case -----------------------------------------

    #[test]
    fn spawn_ship_sis_zero_crew_uses_descriptor_default() {
        cleanup();
        // SIS ships enter with crew_level=0 — crew is set by sis_ship.c
        // Spawn should NOT patch descriptor crew from starship
        let mut starship = make_test_starship(SpeciesId::Earthling, 0, RPG_PLAYER_NUM);
        spawn_ship(&mut starship, IN_ENCOUNTER).unwrap();

        let desc = starship.race_desc.as_ref().unwrap();
        // Descriptor should keep its species default (18 for Earthling)
        assert_eq!(desc.ship_info.crew_level, 18);
    }

    // -- Floating crew retrieval tests --------------------------------------

    #[test]
    fn uninit_ships_floating_crew_added_to_survivor() {
        cleanup();
        let mut queues: [Vec<Starship>; NUM_PLAYERS] = [Vec::new(), Vec::new()];
        queues[0].push(make_test_starship(SpeciesId::Earthling, 10, RPG_PLAYER_NUM));

        init_ships(IN_ENCOUNTER).unwrap();
        spawn_ship(&mut queues[0][0], IN_ENCOUNTER).unwrap();

        // Simulate damage: set crew to 10 in descriptor
        if let Some(ref mut desc) = queues[0][0].race_desc {
            desc.ship_info.crew_level = 10;
        }

        // 5 floating crew elements in display list, survivor on side 0
        let mut frags: [Vec<ShipFragment>; NUM_PLAYERS] = [Vec::new(), Vec::new()];
        let result = uninit_ships(&mut queues, &mut frags, IN_ENCOUNTER, 5, Some(0)).unwrap();
        assert_eq!(result.crew_retrieved, 5);
        // Survivor had 10 crew + 5 floating = 15 written back
        assert_eq!(queues[0][0].crew_level, 15);
    }

    #[test]
    fn uninit_ships_floating_crew_clamped_to_max() {
        cleanup();
        let mut queues: [Vec<Starship>; NUM_PLAYERS] = [Vec::new(), Vec::new()];
        queues[0].push(make_test_starship(SpeciesId::Earthling, 17, RPG_PLAYER_NUM));

        init_ships(IN_ENCOUNTER).unwrap();
        spawn_ship(&mut queues[0][0], IN_ENCOUNTER).unwrap();

        // Survivor has 17 crew, max is 18 — only 1 fits
        let max_crew = queues[0][0].race_desc.as_ref().unwrap().ship_info.max_crew;
        let mut frags: [Vec<ShipFragment>; NUM_PLAYERS] = [Vec::new(), Vec::new()];
        let result = uninit_ships(&mut queues, &mut frags, IN_ENCOUNTER, 10, Some(0)).unwrap();
        assert_eq!(result.crew_retrieved, 10);
        // Should be clamped: 17 + min(10, 18-17) = 17 + 1 = 18
        assert_eq!(queues[0][0].crew_level, max_crew);
    }

    #[test]
    fn uninit_ships_no_survivor_no_crew_added() {
        cleanup();
        let mut queues: [Vec<Starship>; NUM_PLAYERS] = [Vec::new(), Vec::new()];
        queues[0].push(make_test_starship(SpeciesId::Earthling, 10, RPG_PLAYER_NUM));

        init_ships(IN_ENCOUNTER).unwrap();
        spawn_ship(&mut queues[0][0], IN_ENCOUNTER).unwrap();

        if let Some(ref mut desc) = queues[0][0].race_desc {
            desc.ship_info.crew_level = 10;
        }

        // Floating crew but no survivor specified
        let mut frags: [Vec<ShipFragment>; NUM_PLAYERS] = [Vec::new(), Vec::new()];
        uninit_ships(&mut queues, &mut frags, IN_ENCOUNTER, 5, None).unwrap();
        // Crew written back is just the descriptor value (10), not 10+5
        assert_eq!(queues[0][0].crew_level, 10);
    }

    #[test]
    fn uninit_ships_zero_crew_ship_not_survivor() {
        cleanup();
        let mut queues: [Vec<Starship>; NUM_PLAYERS] = [Vec::new(), Vec::new()];
        // Ship with crew_level=0 (destroyed)
        queues[0].push(make_test_starship(SpeciesId::Earthling, 0, RPG_PLAYER_NUM));

        init_ships(IN_ENCOUNTER).unwrap();
        // SIS path: crew_level=0, uses descriptor default
        spawn_ship(&mut queues[0][0], IN_ENCOUNTER).unwrap();

        // Even with floating crew, a destroyed ship (desc crew_level=0
        // at teardown time would not receive crew). But Earthling with
        // SIS path starts with default crew (18). Simulate death:
        if let Some(ref mut desc) = queues[0][0].race_desc {
            desc.ship_info.crew_level = 0;
        }

        let mut frags: [Vec<ShipFragment>; NUM_PLAYERS] = [Vec::new(), Vec::new()];
        uninit_ships(&mut queues, &mut frags, IN_ENCOUNTER, 5, Some(0)).unwrap();
        // Ship has 0 crew in desc, so survivor search skips it (crew_level > 0 check)
        assert_eq!(queues[0][0].crew_level, 0);
    }
}
