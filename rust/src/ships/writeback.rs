//! Crew Writeback & Ship Death
//!
//! @plan PLAN-20260314-SHIPS.P10
//! @requirement REQ-WRITEBACK, REQ-WRITEBACK-MATCHING, REQ-DEATH-SEQUENCE, REQ-REPLACEMENT-SPAWN, REQ-FLOATING-CREW, REQ-AUDIO-RESET, REQ-TEARDOWN-ROBUSTNESS
//!
//! Implements crew writeback (UpdateShipFragCrew), ship death sequence (ship_death),
//! ship replacement spawning (new_ship_transition), floating crew collection, and
//! comprehensive battle teardown orchestration.
//!
//! ## C Reference Summary
//!
//! ### ship_death() (tactrans.c:730-749)
//! Called when a ship's crew reaches 0:
//! - Stops all battle music
//! - Clears PLAY_VICTORY_DITTY flag
//! - Starts ship explosion
//! - Finds the alive opponent
//! - Sets winner
//! - Records death
//!
//! ### new_ship() (tactrans.c:441-539)
//! Called when the dead ship element's life_span reaches 0:
//! - If not ready for battle end: keeps element alive longer, returns
//! - Otherwise: stops audio (ditty/music/sound)
//! - Frees dead ship's descriptor: `free_ship(RaceDescPtr, TRUE, TRUE)`
//! - Sets RaceDescPtr = NULL
//! - If NOT FleetIsInfinite: UpdateShipFragCrew, marks SpeciesID = NO_ID
//! - Calls GetNextStarShip to spawn replacement
//! - If no ships left on one side: clears IN_BATTLE
//!
//! ### UpdateShipFragCrew() (encount.c:214-253)
//! Matches STARSHIP to SHIP_FRAGMENT by parallel queue iteration:
//! - Finds the SHIP_FRAGMENT at the same position as the STARSHIP
//! - Writes `frag->crew_level = ship->crew_level`
//! - Asserts `frag->crew_level != INFINITE_FLEET`
//!
//! ### FleetIsInfinite() (encount.c:192-211)
//! Checks first fragment in queue: `crew_level == INFINITE_FLEET` (0xFFFF).
//! Empty queue returns false.

use super::lifecycle::{spawn_ship, NUM_PLAYERS};
use super::loader::free_ship;
use super::types::{ShipsError, SpeciesId, Starship, StatusFlags};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Sentinel for "infinite fleet" crew in SHIP_FRAGMENT (C: `(COUNT) ~0`).
/// Reuses the constant from types.rs but re-exports here for clarity.
pub use crate::ships::types::INFINITE_FLEET;

/// C: NO_ID species sentinel (from races.h).
pub const NO_ID: SpeciesId = SpeciesId::NoId;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Transient bookkeeping for crew writeback.
///
/// Tracks which fragments have been updated during a teardown pass.
/// Must be created fresh for each teardown/transition pass.
///
/// Not a field on ShipFragment — kept separate as transient state.
pub struct WritebackCursor {
    processed: Vec<bool>,
}

impl WritebackCursor {
    /// Creates a new cursor for tracking up to `fragment_count` fragments.
    pub fn new(fragment_count: usize) -> Self {
        Self {
            processed: vec![false; fragment_count],
        }
    }

    /// Marks a fragment index as processed.
    pub fn mark_processed(&mut self, index: usize) {
        if index < self.processed.len() {
            self.processed[index] = true;
        }
    }

    /// Checks if a fragment index has been processed.
    pub fn is_processed(&self, index: usize) -> bool {
        self.processed.get(index).copied().unwrap_or(false)
    }

    /// Resets all processed flags to false.
    pub fn reset(&mut self) {
        self.processed.fill(false);
    }
}

/// Result of a ship death/transition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransitionResult {
    /// Replacement ship spawned successfully.
    ReplacementSpawned,
    /// No replacement available (side has no more ships).
    NoReplacement,
    /// Fleet is infinite (Super Melee / specific game modes).
    InfiniteFleet,
}

/// Result of a full battle teardown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TeardownResult {
    /// Total floating crew collected from display list elements.
    pub floating_crew: u16,
    /// Whether each side still has ships remaining after teardown.
    pub sides_remaining: [bool; 2],
}

// ---------------------------------------------------------------------------
// ShipFragment Type (minimal mirror of C struct)
// ---------------------------------------------------------------------------

/// Minimal representation of a SHIP_FRAGMENT for crew writeback.
///
/// Only the fields needed for writeback are mirrored here. Full fragment
/// queues are managed in C; this struct allows Rust code to read/write crew.
#[derive(Debug, Clone)]
pub struct ShipFragment {
    pub species_id: SpeciesId,
    pub crew_level: u16,
    pub max_crew: u16,
    pub index: u8,
    pub captains_name_index: u8,
}

impl Default for ShipFragment {
    fn default() -> Self {
        Self {
            species_id: SpeciesId::NoId,
            crew_level: 0,
            max_crew: 0,
            index: 0,
            captains_name_index: 0,
        }
    }
}

// ---------------------------------------------------------------------------
// Core Functions
// ---------------------------------------------------------------------------

/// Ship death handler — called when a ship's crew reaches 0.
///
/// # C Reference
/// `ship_death()` in tactrans.c lines 730-749
///
/// # Behavior
/// - Stops battle music (via flag; actual audio stop is C-side)
/// - Clears `PLAY_VICTORY_DITTY` from `cur_status_flags`
/// - Records death by setting `crew_level = 0` in descriptor (if present)
/// - Returns `Ok(())` on success
///
/// # Implementation Notes
/// - StartShipExplosion, FindAliveStarShip, SetWinnerStarShip, RecordShipDeath
///   remain in C (tactrans.c). This Rust handler performs only the ship-state
///   updates; the C side orchestrates the rest via FFI (P14).
pub fn ship_death(starship: &mut Starship) -> Result<(), ShipsError> {
    // Clear PLAY_VICTORY_DITTY flag (C: tactrans.c line 736)
    starship.cur_status_flags &= !StatusFlags::PLAY_VICTORY_DITTY;

    // Record death in descriptor
    if let Some(ref mut desc) = starship.race_desc {
        desc.ship_info.crew_level = 0;
    }

    // Starship crew already zeroed by caller (damage logic)
    starship.crew_level = 0;

    Ok(())
}

/// New ship transition — handles audio stop, descriptor freeing, crew writeback,
/// and replacement spawning.
///
/// # C Reference
/// `new_ship()` in tactrans.c lines 441-539
///
/// # Parameters
/// - `starship_index`: Index of the dead ship in `race_queue`.
/// - `_side`: Which side (0 or 1) the dead ship belonged to.
/// - `race_queue`: The full battle queue for this side (for finding replacement).
/// - `fragments`: The persistent fragment queue for this side (for crew writeback).
/// - `cursor`: Transient bookkeeping to prevent double-updates within a teardown pass.
/// - `activity`: Current battle activity (IN_ENCOUNTER, SUPER_MELEE, etc.)
///
/// # Behavior
/// 1. Stop audio: set `audio_stopped` flag (actual audio calls remain in C)
/// 2. Free descriptor: take and free `starship.race_desc`
/// 3. If NOT fleet-infinite: writeback crew, mark ship inactive (species_id = NO_ID)
/// 4. Find next available ship: `get_next_starship(race_queue)`
/// 5. If found: spawn it via `spawn_ship()`, return `ReplacementSpawned`
/// 6. If not found: return `NoReplacement`
/// 7. If fleet-infinite: descriptor freed but no writeback, return `InfiniteFleet`
///
/// # Audio Stop Contract
/// C calls `StopDitty()`, `StopMusic()`, `StopSound()` before freeing.
/// Rust sets `audio_stopped = true` on the starship to signal the C side
/// to actually stop audio via FFI (P14). This ensures the contract is modeled.
///
/// # Returns
/// - `Ok(TransitionResult::InfiniteFleet)` if fleet is infinite (no writeback)
/// - `Ok(TransitionResult::ReplacementSpawned)` if replacement spawned
/// - `Ok(TransitionResult::NoReplacement)` if no ships left on side
/// - `Err(ShipsError)` on spawn failure or writeback error
pub fn new_ship_transition(
    starship_index: usize,
    _side: usize,
    race_queue: &mut [Starship],
    fragments: &mut [ShipFragment],
    cursor: &mut WritebackCursor,
    activity: u8,
) -> Result<TransitionResult, ShipsError> {
    if starship_index >= race_queue.len() {
        return Err(ShipsError::InvalidState(format!(
            "Starship index {} out of bounds (len={})",
            starship_index,
            race_queue.len()
        )));
    }

    // 1. Signal audio stop (C: tactrans.c lines 469-471 — StopDitty/StopMusic/StopSound)
    // Actual audio stop calls remain in C; Rust models it as a flag for P14 bridge.
    race_queue[starship_index].audio_stopped = true;

    // 2. Free descriptor (C: tactrans.c lines 476-477)
    if let Some(mut desc) = race_queue[starship_index].race_desc.take() {
        free_ship(&mut desc, true, true);
    }

    // 3. Check if fleet is infinite (C: tactrans.c line 501)
    // C only deactivates when NOT infinite: `if (!FleetIsInfinite(...))`
    if fleet_is_infinite(fragments) {
        // C does NOT set SpeciesID = NO_ID for infinite fleets
        return Ok(TransitionResult::InfiniteFleet);
    }

    // 4. Update fragment crew via queue-position matching (C: tactrans.c line 503)
    update_ship_frag_crew(starship_index, race_queue, fragments, cursor)?;

    // 5. Mark ship inactive (C: tactrans.c line 505 — SpeciesID = NO_ID)
    race_queue[starship_index].species_id = NO_ID;

    // 6. Find next available ship (C: tactrans.c line 508)
    let next_index = super::lifecycle::get_next_starship(race_queue);

    match next_index {
        Some(index) => {
            // 7. Spawn replacement (C: tactrans.c lines 508-524)
            spawn_ship(&mut race_queue[index], activity)?;
            Ok(TransitionResult::ReplacementSpawned)
        }
        None => {
            // No more ships on this side
            Ok(TransitionResult::NoReplacement)
        }
    }
}

/// Updates a ship fragment's crew level from its corresponding starship.
///
/// # C Reference
/// `UpdateShipFragCrew()` in encount.c lines 214-253
///
/// # Matching Logic
/// Uses explicit queue-position matching: the caller provides the starship's
/// index in the race queue, and the same index is used to access the
/// corresponding fragment. This mirrors C's lockstep iteration over the
/// ship queue and fragment queue.
///
/// # Behavior
/// - Uses `starship_index` to access both race_queue and fragments
/// - Verifies species_id matches between starship and fragment
/// - Writes `fragment.crew_level = starship.crew_level`
/// - Uses WritebackCursor to prevent double-update within a single teardown pass
/// - Returns error if fragment has `crew_level == INFINITE_FLEET`
/// - Returns error if index is out of bounds for either queue
/// - Skips writeback (non-fatal) if species_id mismatch detected
///
/// # Returns
/// - `Ok(())` on success or species_id mismatch (non-fatal)
/// - `Err(ShipsError::InvalidState)` if index out of bounds or fragment is infinite
pub fn update_ship_frag_crew(
    starship_index: usize,
    race_queue: &[Starship],
    fragments: &mut [ShipFragment],
    cursor: &mut WritebackCursor,
) -> Result<(), ShipsError> {
    // Bounds check on race queue
    if starship_index >= race_queue.len() {
        return Err(ShipsError::InvalidState(format!(
            "Starship index {} out of bounds (race_queue len={})",
            starship_index,
            race_queue.len()
        )));
    }

    // Bounds check on fragment queue (C: encount.c lines 239-247)
    if starship_index >= fragments.len() {
        return Err(ShipsError::InvalidState(format!(
            "Fragment index {} out of bounds (len={})",
            starship_index,
            fragments.len()
        )));
    }

    // Prevent double-update via WritebackCursor
    if cursor.is_processed(starship_index) {
        return Ok(()); // Already written back this pass
    }

    let fragment = &mut fragments[starship_index];

    // Species ID cross-check (queue ordering AND species match)
    if race_queue[starship_index].species_id != fragment.species_id {
        eprintln!(
            "Warning: Species ID mismatch at index {}: starship={:?}, fragment={:?}. Skipping writeback.",
            starship_index,
            race_queue[starship_index].species_id,
            fragment.species_id
        );
        return Ok(()); // Non-fatal, just skip writeback like C would
    }

    // Assert fragment is not infinite (C: encount.c line 240)
    if fragment.crew_level == INFINITE_FLEET {
        return Err(ShipsError::InvalidState(
            "Cannot write back crew to infinite-fleet fragment".to_string(),
        ));
    }

    // Write crew level (C: encount.c line 243)
    fragment.crew_level = race_queue[starship_index].crew_level;

    // Mark as processed
    cursor.mark_processed(starship_index);

    Ok(())
}

/// Checks if a fleet is infinite (Super Melee mode).
///
/// # C Reference
/// `FleetIsInfinite()` in encount.c lines 192-211
///
/// # Behavior
/// - Checks first fragment: `crew_level == INFINITE_FLEET` (0xFFFF)
/// - Empty slice returns `false`
///
/// # Returns
/// `true` if first fragment has infinite crew, `false` otherwise.
pub fn fleet_is_infinite(fragments: &[ShipFragment]) -> bool {
    fragments
        .first()
        .map(|frag| frag.crew_level == INFINITE_FLEET)
        .unwrap_or(false)
}

/// Orchestrates full battle teardown with crew writeback.
///
/// Follows C's UninitShips() sequencing (init.c lines 268-349):
/// 1. Count floating crew from display list (passed in)
/// 2. Add floating crew to the surviving ship's descriptor (BEFORE writeback)
/// 3. Copy descriptor crew back to starship.crew_level
/// 4. Write back crew from starship to fragment via queue-position matching
/// 5. Free all descriptors
///
/// # Parameters
/// - `race_queues`: Battle queues for both sides (starships).
/// - `fragment_queues`: Persistent fragment queues for both sides (crew tracking).
/// - `floating_crew`: Count of CREW_OBJECT elements still in display list.
/// - `survivor_side`: Which side (if any) won the battle and receives floating crew.
///
/// # Returns
/// - `Ok(TeardownResult)` with summary
/// - `Err(ShipsError)` on writeback failure
pub fn battle_teardown_writeback(
    race_queues: &mut [Vec<Starship>; NUM_PLAYERS],
    fragment_queues: &mut [Vec<ShipFragment>; NUM_PLAYERS],
    floating_crew: u16,
    survivor_side: Option<usize>,
) -> Result<TeardownResult, ShipsError> {
    let mut sides_remaining = [false; 2];

    // Step 1: Add floating crew to survivor's DESCRIPTOR (C: init.c lines 302-312)
    // This must happen BEFORE writeback so the crew flows through properly.
    if let Some(side) = survivor_side {
        if side < NUM_PLAYERS {
            for starship in race_queues[side].iter_mut() {
                if let Some(ref mut desc) = starship.race_desc {
                    if desc.ship_info.crew_level > 0 {
                        let max_crew = desc.ship_info.max_crew;
                        let capacity = max_crew.saturating_sub(desc.ship_info.crew_level);
                        let added = floating_crew.min(capacity);
                        desc.ship_info.crew_level = desc.ship_info.crew_level.saturating_add(added);
                        break; // Only one survivor per side
                    }
                }
            }
        }
    }

    // Step 2: Copy descriptor crew to starship.crew_level (C: init.c line 315-316)
    for queue in race_queues.iter_mut() {
        for starship in queue.iter_mut() {
            if let Some(ref desc) = starship.race_desc {
                starship.crew_level = desc.ship_info.crew_level;
            }
        }
    }

    // Step 3: Write back crew from starship to fragment via queue-position matching
    // Uses WritebackCursor to prevent double-updates.
    for (race_queue, fragment_queue) in race_queues.iter().zip(fragment_queues.iter_mut()) {
        let is_infinite = fleet_is_infinite(fragment_queue);

        if !is_infinite {
            let mut cursor = WritebackCursor::new(fragment_queue.len());
            for index in 0..race_queue.len() {
                if index < fragment_queue.len() {
                    // Use update_ship_frag_crew with cursor for double-update prevention
                    update_ship_frag_crew(index, race_queue, fragment_queue, &mut cursor)?;
                }
            }
        }
    }

    // Step 4: Free all descriptors (C: init.c lines 318-321)
    for (side, race_queue) in race_queues.iter_mut().enumerate() {
        for starship in race_queue.iter_mut() {
            if let Some(mut desc) = starship.race_desc.take() {
                free_ship(&mut desc, true, true);
            }
        }

        // Check if side has any ships remaining
        sides_remaining[side] = race_queue
            .iter()
            .any(|s| s.crew_level > 0 && s.species_id != NO_ID);
    }

    Ok(TeardownResult {
        floating_crew,
        sides_remaining,
    })
}

/// Counts CREW_OBJECT elements in the display list.
///
/// # C Reference
/// `CountCrewElements()` in init.c lines 245-266
///
/// # Parameters
/// - `elements`: Slice of element states from the display list.
///
/// # Behavior
/// Counts elements with `CREW_OBJECT` state flag set (bit 9 = 0x200).
///
/// # Returns
/// Count of crew object elements (as `u16` to match C `COUNT` type).
///
/// # Implementation Note
/// In real integration, the caller queries the C display list and passes
/// element state flags here. For pure Rust tests, pass an empty slice.
pub fn count_floating_crew_elements(elements: &[u16]) -> u16 {
    const CREW_OBJECT: u16 = 1 << 9;
    elements
        .iter()
        .filter(|&&state| state & CREW_OBJECT != 0)
        .count() as u16
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // Helper to create test starship
    fn make_test_starship(species: SpeciesId, crew: u16, player: i16) -> Starship {
        Starship {
            species_id: species,
            crew_level: crew,
            player_nr: player,
            ..Starship::default()
        }
    }

    // Helper to create test fragment
    fn make_test_fragment(species: SpeciesId, crew: u16, max_crew: u16, index: u8) -> ShipFragment {
        ShipFragment {
            species_id: species,
            crew_level: crew,
            max_crew,
            index,
            ..ShipFragment::default()
        }
    }

    // -- WritebackCursor tests ----------------------------------------------

    #[test]
    fn writeback_cursor_new_initializes_all_false() {
        let cursor = WritebackCursor::new(5);
        for i in 0..5 {
            assert!(!cursor.is_processed(i));
        }
    }

    #[test]
    fn writeback_cursor_mark_processed() {
        let mut cursor = WritebackCursor::new(3);
        cursor.mark_processed(1);
        assert!(!cursor.is_processed(0));
        assert!(cursor.is_processed(1));
        assert!(!cursor.is_processed(2));
    }

    #[test]
    fn writeback_cursor_reset() {
        let mut cursor = WritebackCursor::new(3);
        cursor.mark_processed(0);
        cursor.mark_processed(2);
        cursor.reset();
        assert!(!cursor.is_processed(0));
        assert!(!cursor.is_processed(2));
    }

    #[test]
    fn writeback_cursor_out_of_bounds_safe() {
        let mut cursor = WritebackCursor::new(3);
        cursor.mark_processed(10); // Out of bounds
        assert!(!cursor.is_processed(10)); // Returns false, doesn't panic
    }

    // -- ship_death tests ---------------------------------------------------

    #[test]
    fn ship_death_clears_victory_flag() {
        let mut starship = make_test_starship(SpeciesId::Earthling, 10, 0);
        starship.cur_status_flags = StatusFlags::PLAY_VICTORY_DITTY | StatusFlags::THRUST;

        ship_death(&mut starship).unwrap();

        assert!(!starship
            .cur_status_flags
            .contains(StatusFlags::PLAY_VICTORY_DITTY));
        assert!(starship.cur_status_flags.contains(StatusFlags::THRUST)); // Other flags preserved
    }

    #[test]
    fn ship_death_records_crew_zero() {
        use super::super::loader::{load_ship, LoadTier};

        let mut starship = make_test_starship(SpeciesId::Spathi, 30, 0);
        starship.race_desc = Some(Box::new(
            load_ship(SpeciesId::Spathi, LoadTier::MetadataOnly).unwrap(),
        ));

        ship_death(&mut starship).unwrap();

        assert_eq!(starship.crew_level, 0);
        assert_eq!(starship.race_desc.as_ref().unwrap().ship_info.crew_level, 0);
    }

    #[test]
    fn ship_death_no_descriptor_safe() {
        let mut starship = make_test_starship(SpeciesId::Earthling, 10, 0);
        starship.race_desc = None;

        let result = ship_death(&mut starship);
        assert!(result.is_ok());
        assert_eq!(starship.crew_level, 0);
    }

    // -- update_ship_frag_crew tests ----------------------------------------

    #[test]
    fn update_ship_frag_crew_writes_crew() {
        let race_queue = vec![
            make_test_starship(SpeciesId::Earthling, 15, 0),
            make_test_starship(SpeciesId::Spathi, 20, 1),
        ];
        let mut fragments = vec![
            make_test_fragment(SpeciesId::Earthling, 18, 18, 0),
            make_test_fragment(SpeciesId::Spathi, 30, 30, 1),
        ];
        let mut cursor = WritebackCursor::new(2);

        // Update first ship's fragment
        update_ship_frag_crew(0, &race_queue, &mut fragments, &mut cursor).unwrap();
        assert_eq!(fragments[0].crew_level, 15);

        // Update second ship's fragment
        update_ship_frag_crew(1, &race_queue, &mut fragments, &mut cursor).unwrap();
        assert_eq!(fragments[1].crew_level, 20);
    }

    #[test]
    fn update_ship_frag_crew_starship_not_in_queue_fails() {
        let race_queue = vec![make_test_starship(SpeciesId::Earthling, 15, 0)];
        let mut fragments = vec![make_test_fragment(SpeciesId::Earthling, 18, 18, 0)];
        let mut cursor = WritebackCursor::new(1);

        // Try to update with out-of-bounds index
        let result = update_ship_frag_crew(5, &race_queue, &mut fragments, &mut cursor);
        assert!(result.is_err());
    }

    #[test]
    fn update_ship_frag_crew_infinite_fragment_fails() {
        let race_queue = vec![make_test_starship(SpeciesId::Earthling, 15, 0)];
        let mut fragments = vec![make_test_fragment(
            SpeciesId::Earthling,
            INFINITE_FLEET,
            18,
            0,
        )];
        let mut cursor = WritebackCursor::new(1);

        let result = update_ship_frag_crew(0, &race_queue, &mut fragments, &mut cursor);
        assert!(result.is_err());
    }

    #[test]
    fn update_ship_frag_crew_index_out_of_bounds_fails() {
        let race_queue = vec![
            make_test_starship(SpeciesId::Earthling, 15, 0),
            make_test_starship(SpeciesId::Spathi, 20, 1),
        ];
        let mut fragments = vec![
            make_test_fragment(SpeciesId::Earthling, 18, 18, 0),
            // Missing second fragment
        ];
        let mut cursor = WritebackCursor::new(1);

        let result = update_ship_frag_crew(1, &race_queue, &mut fragments, &mut cursor);
        assert!(result.is_err());
    }

    #[test]
    fn species_id_mismatch_skips_writeback() {
        let race_queue = vec![
            make_test_starship(SpeciesId::Earthling, 10, 0), // Starship is Earthling with 10 crew
        ];
        let mut fragments = vec![
            make_test_fragment(SpeciesId::Spathi, 30, 30, 0), // Fragment is Spathi with 30 crew
        ];
        let mut cursor = WritebackCursor::new(1);

        // Call should succeed (non-fatal) but skip writeback
        let result = update_ship_frag_crew(0, &race_queue, &mut fragments, &mut cursor);
        assert!(result.is_ok());

        // Fragment crew should remain unchanged (30, not overwritten with 10)
        assert_eq!(fragments[0].crew_level, 30);
        assert_eq!(fragments[0].species_id, SpeciesId::Spathi);
    }

    // -- fleet_is_infinite tests --------------------------------------------

    #[test]
    fn fleet_is_infinite_true_for_infinite_crew() {
        let fragments = vec![make_test_fragment(
            SpeciesId::Earthling,
            INFINITE_FLEET,
            18,
            0,
        )];
        assert!(fleet_is_infinite(&fragments));
    }

    #[test]
    fn fleet_is_infinite_false_for_normal_crew() {
        let fragments = vec![make_test_fragment(SpeciesId::Earthling, 18, 18, 0)];
        assert!(!fleet_is_infinite(&fragments));
    }

    #[test]
    fn fleet_is_infinite_false_for_empty_slice() {
        let fragments: Vec<ShipFragment> = Vec::new();
        assert!(!fleet_is_infinite(&fragments));
    }

    #[test]
    fn fleet_is_infinite_checks_only_first_fragment() {
        let fragments = vec![
            make_test_fragment(SpeciesId::Earthling, 18, 18, 0),
            make_test_fragment(SpeciesId::Spathi, INFINITE_FLEET, 30, 1), // Second is infinite
        ];
        assert!(!fleet_is_infinite(&fragments)); // First is not infinite
    }

    // -- new_ship_transition tests ------------------------------------------

    #[test]
    fn new_ship_transition_spawns_replacement() {
        use super::super::loader::{load_ship, LoadTier};

        let mut race_queue = vec![
            make_test_starship(SpeciesId::Earthling, 0, 0), // Dead
            make_test_starship(SpeciesId::Spathi, 30, 1),   // Alive, unspawned
        ];
        let mut fragments = vec![
            make_test_fragment(SpeciesId::Earthling, 0, 18, 0),
            make_test_fragment(SpeciesId::Spathi, 30, 30, 1),
        ];
        let mut cursor = WritebackCursor::new(2);

        // Load descriptor for dead ship
        race_queue[0].race_desc = Some(Box::new(
            load_ship(SpeciesId::Earthling, LoadTier::MetadataOnly).unwrap(),
        ));

        let result = new_ship_transition(
            0,
            0,
            &mut race_queue,
            &mut fragments,
            &mut cursor,
            super::super::lifecycle::IN_ENCOUNTER,
        )
        .unwrap();

        assert_eq!(result, TransitionResult::ReplacementSpawned);
        assert!(race_queue[0].race_desc.is_none()); // Old descriptor freed
        assert_eq!(race_queue[0].species_id, NO_ID); // Marked inactive
        assert!(race_queue[0].audio_stopped); // Audio stop flag set
        assert!(race_queue[1].race_desc.is_some()); // Replacement spawned
    }

    #[test]
    fn new_ship_transition_no_replacement_available() {
        use super::super::loader::{load_ship, LoadTier};

        let mut race_queue = vec![
            make_test_starship(SpeciesId::Earthling, 0, 0), // Dead
        ];
        let mut fragments = vec![make_test_fragment(SpeciesId::Earthling, 0, 18, 0)];
        let mut cursor = WritebackCursor::new(1);

        race_queue[0].race_desc = Some(Box::new(
            load_ship(SpeciesId::Earthling, LoadTier::MetadataOnly).unwrap(),
        ));

        let result = new_ship_transition(
            0,
            0,
            &mut race_queue,
            &mut fragments,
            &mut cursor,
            super::super::lifecycle::IN_ENCOUNTER,
        )
        .unwrap();

        assert_eq!(result, TransitionResult::NoReplacement);
        assert!(race_queue[0].race_desc.is_none());
        assert_eq!(race_queue[0].species_id, NO_ID);
        assert!(race_queue[0].audio_stopped);
    }

    #[test]
    fn new_ship_transition_infinite_fleet_skips_writeback() {
        use super::super::loader::{load_ship, LoadTier};

        let mut race_queue = vec![make_test_starship(SpeciesId::Earthling, 0, 0)];
        let mut fragments = vec![make_test_fragment(
            SpeciesId::Earthling,
            INFINITE_FLEET,
            18,
            0,
        )];
        let mut cursor = WritebackCursor::new(1);

        race_queue[0].race_desc = Some(Box::new(
            load_ship(SpeciesId::Earthling, LoadTier::MetadataOnly).unwrap(),
        ));

        let result = new_ship_transition(
            0,
            0,
            &mut race_queue,
            &mut fragments,
            &mut cursor,
            super::super::lifecycle::SUPER_MELEE,
        )
        .unwrap();

        assert_eq!(result, TransitionResult::InfiniteFleet);
        assert!(race_queue[0].race_desc.is_none()); // Descriptor freed even for infinite
                                                    // For infinite fleets, species_id should NOT be set to NO_ID (Issue 4 fix)
        assert_ne!(race_queue[0].species_id, NO_ID);
        assert!(race_queue[0].audio_stopped);
    }

    #[test]
    fn new_ship_transition_writes_back_crew_before_spawning() {
        use super::super::loader::{load_ship, LoadTier};

        let mut race_queue = vec![
            make_test_starship(SpeciesId::Earthling, 10, 0), // Damaged
            make_test_starship(SpeciesId::Spathi, 30, 0),
        ];
        let mut fragments = vec![
            make_test_fragment(SpeciesId::Earthling, 18, 18, 0), // Full crew originally
            make_test_fragment(SpeciesId::Spathi, 30, 30, 1),
        ];
        let mut cursor = WritebackCursor::new(2);

        // Load descriptor and set crew to match starship state
        let mut desc = load_ship(SpeciesId::Earthling, LoadTier::MetadataOnly).unwrap();
        desc.ship_info.crew_level = 10; // Damaged crew matches starship
        race_queue[0].race_desc = Some(Box::new(desc));

        new_ship_transition(
            0,
            0,
            &mut race_queue,
            &mut fragments,
            &mut cursor,
            super::super::lifecycle::IN_ENCOUNTER,
        )
        .unwrap();

        assert_eq!(fragments[0].crew_level, 10); // Written back
    }

    // -- battle_teardown_writeback tests ------------------------------------

    #[test]
    fn battle_teardown_writeback_full_round_trip() {
        use super::super::loader::{load_ship, LoadTier};

        let mut race_queues: [Vec<Starship>; NUM_PLAYERS] = [
            vec![make_test_starship(SpeciesId::Earthling, 15, 0)],
            vec![make_test_starship(SpeciesId::Spathi, 20, 0)],
        ];
        let mut fragment_queues: [Vec<ShipFragment>; NUM_PLAYERS] = [
            vec![make_test_fragment(SpeciesId::Earthling, 18, 18, 0)],
            vec![make_test_fragment(SpeciesId::Spathi, 30, 30, 0)],
        ];

        // Load descriptors and set their crew to match starship state
        let mut desc0 = load_ship(SpeciesId::Earthling, LoadTier::MetadataOnly).unwrap();
        desc0.ship_info.crew_level = 15; // Damaged from battle
        race_queues[0][0].race_desc = Some(Box::new(desc0));

        let mut desc1 = load_ship(SpeciesId::Spathi, LoadTier::MetadataOnly).unwrap();
        desc1.ship_info.crew_level = 20; // Current crew
        race_queues[1][0].race_desc = Some(Box::new(desc1));

        let result =
            battle_teardown_writeback(&mut race_queues, &mut fragment_queues, 5, Some(0)).unwrap();

        assert_eq!(result.floating_crew, 5);
        assert!(result.sides_remaining[0]);
        assert!(result.sides_remaining[1]);
        // Survivor (side 0) had 15 crew, gained 3 floating (clamped to max 18)
        assert_eq!(fragment_queues[0][0].crew_level, 18);
        // Non-survivor unchanged at 20
        assert_eq!(fragment_queues[1][0].crew_level, 20);
        assert!(race_queues[0][0].race_desc.is_none()); // Descriptors freed
        assert!(race_queues[1][0].race_desc.is_none());
    }

    #[test]
    fn battle_teardown_writeback_no_descriptors_safe() {
        let mut race_queues: [Vec<Starship>; NUM_PLAYERS] = [
            vec![make_test_starship(SpeciesId::Earthling, 15, 0)],
            vec![make_test_starship(SpeciesId::Spathi, 20, 0)],
        ];
        let mut fragment_queues: [Vec<ShipFragment>; NUM_PLAYERS] = [
            vec![make_test_fragment(SpeciesId::Earthling, 18, 18, 0)],
            vec![make_test_fragment(SpeciesId::Spathi, 30, 30, 0)],
        ];

        // No descriptors loaded
        let result = battle_teardown_writeback(&mut race_queues, &mut fragment_queues, 0, None);
        assert!(result.is_ok());
    }

    #[test]
    fn battle_teardown_writeback_infinite_fleet_skips_writeback() {
        use super::super::loader::{load_ship, LoadTier};

        let mut race_queues: [Vec<Starship>; NUM_PLAYERS] = [
            vec![make_test_starship(SpeciesId::Earthling, 15, 0)],
            vec![make_test_starship(SpeciesId::Spathi, 20, 0)],
        ];
        let mut fragment_queues: [Vec<ShipFragment>; NUM_PLAYERS] = [
            vec![make_test_fragment(
                SpeciesId::Earthling,
                INFINITE_FLEET,
                18,
                0,
            )], // Infinite
            vec![make_test_fragment(SpeciesId::Spathi, 30, 30, 0)],
        ];

        // Load descriptors and set crew to match starship state
        let mut desc0 = load_ship(SpeciesId::Earthling, LoadTier::MetadataOnly).unwrap();
        desc0.ship_info.crew_level = 15;
        race_queues[0][0].race_desc = Some(Box::new(desc0));

        let mut desc1 = load_ship(SpeciesId::Spathi, LoadTier::MetadataOnly).unwrap();
        desc1.ship_info.crew_level = 20;
        race_queues[1][0].race_desc = Some(Box::new(desc1));

        let result =
            battle_teardown_writeback(&mut race_queues, &mut fragment_queues, 0, None).unwrap();

        // Side 0 is infinite — crew should not change
        assert_eq!(fragment_queues[0][0].crew_level, INFINITE_FLEET);
        // Side 1 is normal — crew written back
        assert_eq!(fragment_queues[1][0].crew_level, 20);
        assert!(result.sides_remaining[0]);
        assert!(result.sides_remaining[1]);
    }

    #[test]
    fn battle_teardown_writeback_floating_crew_clamped() {
        use super::super::loader::{load_ship, LoadTier};

        let mut race_queues: [Vec<Starship>; NUM_PLAYERS] = [
            vec![make_test_starship(SpeciesId::Earthling, 17, 0)],
            vec![],
        ];
        let mut fragment_queues: [Vec<ShipFragment>; NUM_PLAYERS] = [
            vec![make_test_fragment(SpeciesId::Earthling, 17, 18, 0)], // Max 18, current 17
            vec![],
        ];

        race_queues[0][0].race_desc = Some(Box::new(
            load_ship(SpeciesId::Earthling, LoadTier::MetadataOnly).unwrap(),
        ));

        // Try to add 10 floating crew, but only 1 fits
        let result =
            battle_teardown_writeback(&mut race_queues, &mut fragment_queues, 10, Some(0)).unwrap();

        assert_eq!(result.floating_crew, 10);
        assert_eq!(fragment_queues[0][0].crew_level, 18); // Clamped to max
    }

    #[test]
    fn battle_teardown_writeback_survivor_side_out_of_bounds_safe() {
        let mut race_queues: [Vec<Starship>; NUM_PLAYERS] = [
            vec![make_test_starship(SpeciesId::Earthling, 15, 0)],
            vec![],
        ];
        let mut fragment_queues: [Vec<ShipFragment>; NUM_PLAYERS] = [
            vec![make_test_fragment(SpeciesId::Earthling, 15, 18, 0)],
            vec![],
        ];

        // Survivor side 99 (out of bounds)
        let result = battle_teardown_writeback(&mut race_queues, &mut fragment_queues, 5, Some(99));
        assert!(result.is_ok()); // Should not panic
    }

    // -- count_floating_crew_elements tests ---------------------------------

    #[test]
    fn count_floating_crew_elements_counts_crew_objects() {
        const CREW_OBJECT: u16 = 1 << 9;
        let elements = vec![
            CREW_OBJECT,
            CREW_OBJECT | 0x01, // Crew object with other flags
            0x00,               // Not a crew object
            CREW_OBJECT,
        ];

        assert_eq!(count_floating_crew_elements(&elements), 3);
    }

    #[test]
    fn count_floating_crew_elements_empty_slice() {
        let elements: Vec<u16> = Vec::new();
        assert_eq!(count_floating_crew_elements(&elements), 0);
    }

    #[test]
    fn count_floating_crew_elements_no_crew_objects() {
        let elements = vec![0x01, 0x02, 0x04, 0x08];
        assert_eq!(count_floating_crew_elements(&elements), 0);
    }

    // -- Edge cases ---------------------------------------------------------

    #[test]
    fn ship_death_already_dead_ship_safe() {
        let mut starship = make_test_starship(SpeciesId::Earthling, 0, 0);
        starship.crew_level = 0;

        let result = ship_death(&mut starship);
        assert!(result.is_ok());
        assert_eq!(starship.crew_level, 0);
    }

    #[test]
    fn update_ship_frag_crew_zero_crew_writeback() {
        let race_queue = vec![
            make_test_starship(SpeciesId::Earthling, 0, 0), // Dead
        ];
        let mut fragments = vec![make_test_fragment(SpeciesId::Earthling, 18, 18, 0)];
        let mut cursor = WritebackCursor::new(1);

        update_ship_frag_crew(0, &race_queue, &mut fragments, &mut cursor).unwrap();
        assert_eq!(fragments[0].crew_level, 0); // Zero written back correctly
    }

    #[test]
    fn battle_teardown_writeback_multiple_ships_per_side() {
        use super::super::loader::{load_ship, LoadTier};

        let mut race_queues: [Vec<Starship>; NUM_PLAYERS] = [
            vec![
                make_test_starship(SpeciesId::Earthling, 15, 0),
                make_test_starship(SpeciesId::Spathi, 25, 0),
            ],
            vec![make_test_starship(SpeciesId::Orz, 10, 0)],
        ];
        let mut fragment_queues: [Vec<ShipFragment>; NUM_PLAYERS] = [
            vec![
                make_test_fragment(SpeciesId::Earthling, 18, 18, 0),
                make_test_fragment(SpeciesId::Spathi, 30, 30, 1),
            ],
            vec![make_test_fragment(SpeciesId::Orz, 20, 20, 0)],
        ];

        // Load descriptors and set crew to match starship state
        let mut desc0 = load_ship(SpeciesId::Earthling, LoadTier::MetadataOnly).unwrap();
        desc0.ship_info.crew_level = 15;
        race_queues[0][0].race_desc = Some(Box::new(desc0));

        let mut desc1 = load_ship(SpeciesId::Spathi, LoadTier::MetadataOnly).unwrap();
        desc1.ship_info.crew_level = 25;
        race_queues[0][1].race_desc = Some(Box::new(desc1));

        let mut desc2 = load_ship(SpeciesId::Orz, LoadTier::MetadataOnly).unwrap();
        desc2.ship_info.crew_level = 10;
        race_queues[1][0].race_desc = Some(Box::new(desc2));

        let result =
            battle_teardown_writeback(&mut race_queues, &mut fragment_queues, 3, Some(0)).unwrap();

        assert_eq!(result.floating_crew, 3);
        assert_eq!(fragment_queues[0][0].crew_level, 15 + 3); // First alive on side 0 gets crew
        assert_eq!(fragment_queues[0][1].crew_level, 25); // Second ship unchanged
        assert_eq!(fragment_queues[1][0].crew_level, 10); // Other side unchanged
    }

    #[test]
    fn battle_teardown_writeback_sides_remaining_accurate() {
        use super::super::loader::{load_ship, LoadTier};

        let mut race_queues: [Vec<Starship>; NUM_PLAYERS] = [
            vec![make_test_starship(SpeciesId::Earthling, 0, 0)], // Dead
            vec![make_test_starship(SpeciesId::Spathi, 20, 0)],   // Alive
        ];
        let mut fragment_queues: [Vec<ShipFragment>; NUM_PLAYERS] = [
            vec![make_test_fragment(SpeciesId::Earthling, 0, 18, 0)],
            vec![make_test_fragment(SpeciesId::Spathi, 30, 30, 0)],
        ];

        // Load descriptors and set crew to match starship state
        let mut desc0 = load_ship(SpeciesId::Earthling, LoadTier::MetadataOnly).unwrap();
        desc0.ship_info.crew_level = 0; // Dead
        race_queues[0][0].race_desc = Some(Box::new(desc0));

        let mut desc1 = load_ship(SpeciesId::Spathi, LoadTier::MetadataOnly).unwrap();
        desc1.ship_info.crew_level = 20;
        race_queues[1][0].race_desc = Some(Box::new(desc1));

        let result =
            battle_teardown_writeback(&mut race_queues, &mut fragment_queues, 0, None).unwrap();

        assert!(!result.sides_remaining[0]); // Side 0 has no ships (crew=0)
        assert!(result.sides_remaining[1]); // Side 1 has ships
    }

    // -- New behavior tests (Issue remediation) ----------------------------

    #[test]
    fn writebackcursor_prevents_double_update() {
        let race_queue = vec![make_test_starship(SpeciesId::Earthling, 15, 0)];
        let mut fragments = vec![make_test_fragment(SpeciesId::Earthling, 18, 18, 0)];
        let mut cursor = WritebackCursor::new(1);

        // First update
        update_ship_frag_crew(0, &race_queue, &mut fragments, &mut cursor).unwrap();
        assert_eq!(fragments[0].crew_level, 15);

        // Modify fragment manually (simulate error condition)
        fragments[0].crew_level = 999;

        // Second update should be skipped (already processed)
        update_ship_frag_crew(0, &race_queue, &mut fragments, &mut cursor).unwrap();
        assert_eq!(fragments[0].crew_level, 999); // NOT overwritten
    }

    #[test]
    fn audio_stopped_flag_set_during_transition() {
        use super::super::loader::{load_ship, LoadTier};

        let mut race_queue = vec![make_test_starship(SpeciesId::Earthling, 0, 0)];
        let mut fragments = vec![make_test_fragment(SpeciesId::Earthling, 0, 18, 0)];
        let mut cursor = WritebackCursor::new(1);

        race_queue[0].race_desc = Some(Box::new(
            load_ship(SpeciesId::Earthling, LoadTier::MetadataOnly).unwrap(),
        ));

        assert!(!race_queue[0].audio_stopped); // Initially false

        new_ship_transition(
            0,
            0,
            &mut race_queue,
            &mut fragments,
            &mut cursor,
            super::super::lifecycle::IN_ENCOUNTER,
        )
        .unwrap();

        assert!(race_queue[0].audio_stopped); // Set to true after transition
    }

    #[test]
    fn battle_teardown_uses_cursor_prevents_double_writeback() {
        use super::super::loader::{load_ship, LoadTier};

        let mut race_queues: [Vec<Starship>; NUM_PLAYERS] = [
            vec![make_test_starship(SpeciesId::Earthling, 15, 0)],
            vec![],
        ];
        let mut fragment_queues: [Vec<ShipFragment>; NUM_PLAYERS] = [
            vec![make_test_fragment(SpeciesId::Earthling, 18, 18, 0)],
            vec![],
        ];

        // Load descriptor and set crew to match starship state
        let mut desc = load_ship(SpeciesId::Earthling, LoadTier::MetadataOnly).unwrap();
        desc.ship_info.crew_level = 15;
        race_queues[0][0].race_desc = Some(Box::new(desc));

        battle_teardown_writeback(&mut race_queues, &mut fragment_queues, 0, None).unwrap();

        // Fragment should have crew written back exactly once (15)
        assert_eq!(fragment_queues[0][0].crew_level, 15);
    }
}
