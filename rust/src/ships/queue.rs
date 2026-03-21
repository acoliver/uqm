//! Queue and build primitives for ship management.
//!
//! This module provides **pure helper functions** that operate on Rust-side data types
//! passed as parameters. **All queue storage is owned by C** (avail_race_q, built_ship_q,
//! npc_built_ship_q). This module only provides utilities to manipulate those queues
//! when they are passed in as Rust parameters.
//!
//! @plan PLAN-20260314-SHIPS.P07
//! @requirement REQ-QUEUE-MODEL, REQ-QUEUE-OWNER-BOUNDARY, REQ-FRAGMENT-MODEL, REQ-FRAGMENT-CLONE, REQ-FLEET-INFO

use super::registry::descriptor_template_for_species;
use super::types::{AlliedState, FleetInfo, ShipFragment, ShipsError, SpeciesId, Starship};

/// Maximum number of ships in a built ship queue.
pub const MAX_BUILT_SHIPS: usize = 12;

/// Number of available captain names.
pub const NUM_CAPTAINS_NAMES: u8 = 16;

/// Offset for captain name indices.
pub const NAME_OFFSET: u8 = 5;

/// Infinite fleet strength marker (matches C INFINITE_FLEET).
pub const INFINITE_FLEET: u16 = !0;

/// Sa-Matra ship index in the fleet queue (C: `SAMATRA_SHIP = URQUAN_DRONE_SHIP`).
/// Used for special-case captain naming: the Sa-Matra always gets captain index 0.
pub const SAMATRA_SHIP_INDEX: usize = SpeciesId::SaMatra as usize - 1;

/// Trait for types that have both a species ID and a captain name index.
/// Used by `name_captain()` to avoid duplicates across different queue types.
pub trait HasSpeciesAndCaptain {
    /// Returns the species ID.
    fn species_id(&self) -> SpeciesId;

    /// Returns the captain's name index.
    fn captains_name_index(&self) -> u8;
}

impl HasSpeciesAndCaptain for Starship {
    fn species_id(&self) -> SpeciesId {
        self.species_id
    }

    fn captains_name_index(&self) -> u8 {
        self.captains_name_index
    }
}

impl HasSpeciesAndCaptain for ShipFragment {
    fn species_id(&self) -> SpeciesId {
        self.species_id
    }

    fn captains_name_index(&self) -> u8 {
        self.captains_name_index
    }
}

/// Builds a new ship in the queue.
///
/// Creates a zero-initialized `Starship` with the given species ID and appends it to the queue.
///
/// # Arguments
///
/// * `queue` - The ship queue to append to
/// * `species_id` - The species ID for the new ship
///
/// # Returns
///
/// The index of the newly created ship, or an error if the species ID is invalid.
///
/// # Errors
///
/// Returns `ShipsError::InvalidSpecies` if `species_id == SpeciesId::NoId`.
pub fn build_ship(queue: &mut Vec<Starship>, species_id: SpeciesId) -> Result<usize, ShipsError> {
    if species_id == SpeciesId::NoId {
        return Err(ShipsError::UnknownSpecies(0));
    }

    let ship = Starship {
        species_id,
        ..Default::default()
    };

    queue.push(ship);
    Ok(queue.len() - 1)
}

/// Gets a starship from the queue by index.
///
/// # Arguments
///
/// * `queue` - The ship queue to search
/// * `index` - The index to look up
///
/// # Returns
///
/// A reference to the starship if found, or `None` if the index is out of bounds.
pub fn get_starship_from_index(queue: &[Starship], index: usize) -> Option<&Starship> {
    queue.get(index)
}

/// Gets a mutable starship from the queue by index.
///
/// # Arguments
///
/// * `queue` - The ship queue to search
/// * `index` - The index to look up
///
/// # Returns
///
/// A mutable reference to the starship if found, or `None` if the index is out of bounds.
pub fn get_starship_from_index_mut(queue: &mut [Starship], index: usize) -> Option<&mut Starship> {
    queue.get_mut(index)
}

/// Clones a ship fragment from a fleet info template.
///
/// Creates a new `ShipFragment` based on the fleet info, optionally overriding the crew level.
///
/// # Arguments
///
/// * `ship_index` - The ship's index in the fleet queue (becomes race_id)
/// * `fleet_info` - The fleet template to clone from
/// * `crew_level` - Crew level to set (0 means use template's crew_level)
/// * `captains_name_index` - Captain name index to assign. Callers must pass `0`
///   for the Sa-Matra (`SAMATRA_SHIP_INDEX`) per C convention.
///
/// # Returns
///
/// A new `ShipFragment` with copied attributes and initialized state.
///
/// # Reference
///
/// C implementation: `CloneShipFragment()` in build.c lines 463-507
pub fn clone_ship_fragment(
    ship_index: usize,
    fleet_info: &FleetInfo,
    crew_level: u16,
    captains_name_index: u8,
) -> ShipFragment {
    let actual_crew_level = if crew_level == 0 {
        fleet_info.crew_level
    } else {
        crew_level
    };

    ShipFragment {
        species_id: fleet_info.species_id,
        captains_name_index,
        race_id: ship_index as u8,
        index: 0,
        crew_level: actual_crew_level,
        max_crew: fleet_info.max_crew,
        energy_level: 0,
        max_energy: fleet_info.max_energy,
        race_strings: fleet_info.race_strings,
        icons: fleet_info.icons,
        melee_icon: fleet_info.melee_icon,
    }
}

/// Adds escort ships to the built queue.
///
/// Clones ship fragments from the fleet queue and adds them to the built queue,
/// up to the maximum capacity. Each ship gets a unique captain name and window index.
/// Fragments are inserted in sorted order by window index, matching C's
/// `InsertQueue` behavior.
///
/// # Arguments
///
/// * `fleet_queue` - The fleet info templates
/// * `built_queue` - The built ship queue to append to
/// * `race` - The race index in the fleet queue
/// * `count` - The number of ships to add
/// * `rng` - RNG function for captain naming
///
/// # Returns
///
/// The number of ships actually added (may be less than requested due to capacity).
///
/// # Reference
///
/// C implementation: `AddEscortShips()` in build.c lines 101-149
pub fn add_escort_ships(
    fleet_queue: &[FleetInfo],
    built_queue: &mut Vec<ShipFragment>,
    race: usize,
    count: usize,
    rng: &mut dyn FnMut() -> u32,
) -> usize {
    let fleet_info = match fleet_queue.get(race) {
        Some(info) => info,
        None => return 0,
    };

    let available_slots = MAX_BUILT_SHIPS.saturating_sub(built_queue.len());
    let to_add = count.min(available_slots);

    if to_add == 0 {
        return 0;
    }

    let mut added = 0;
    for _ in 0..to_add {
        // C: if (shipIndex == SAMATRA_SHIP) captains_name_index = 0;
        let captain = if race == SAMATRA_SHIP_INDEX {
            0
        } else {
            name_captain(built_queue.as_slice(), fleet_info.species_id, rng)
        };
        let mut fragment = clone_ship_fragment(race, fleet_info, 0, captain);

        // Find first available window index (C: which_window scanning)
        let mut used_indices = [false; MAX_BUILT_SHIPS];
        for f in built_queue.iter() {
            if (f.index as usize) < MAX_BUILT_SHIPS {
                used_indices[f.index as usize] = true;
            }
        }
        if let Some(window_idx) = used_indices.iter().position(|&used| !used) {
            fragment.index = window_idx as u8;
        }

        // Insert in sorted position by window index (C: InsertQueue before hOldShip)
        let insert_pos = built_queue
            .iter()
            .position(|f| f.index >= fragment.index)
            .unwrap_or(built_queue.len());
        built_queue.insert(insert_pos, fragment);
        added += 1;
    }

    added
}

/// Counts escort ships of a specific race in the built queue.
///
/// # Arguments
///
/// * `fleet_queue` - The fleet info templates (used for validation)
/// * `built_queue` - The built ship queue to count in
/// * `race` - The race to count
///
/// # Returns
///
/// The number of ships of the specified race, or 0 if the race is invalid.
///
/// # Reference
///
/// C implementation: `CountEscortShips()` lines 275-301
pub fn count_escort_ships(
    fleet_queue: &[FleetInfo],
    built_queue: &[ShipFragment],
    race: usize,
) -> usize {
    if fleet_queue.get(race).is_none() {
        return 0;
    }

    built_queue
        .iter()
        .filter(|fragment| fragment.race_id == race as u8)
        .count()
}

/// Checks if there is at least one escort ship of a specific race.
///
/// # Arguments
///
/// * `fleet_queue` - The fleet info templates
/// * `built_queue` - The built ship queue to check
/// * `race` - The race to check for
///
/// # Returns
///
/// `true` if at least one ship of the race exists, `false` otherwise.
pub fn have_escort_ship(
    fleet_queue: &[FleetInfo],
    built_queue: &[ShipFragment],
    race: usize,
) -> bool {
    count_escort_ships(fleet_queue, built_queue, race) > 0
}

/// Checks how many more escort ships can be added.
///
/// # Arguments
///
/// * `fleet_queue` - The fleet info templates (used for validation)
/// * `built_queue` - The built ship queue to check
/// * `race` - The race to check for (must be valid)
///
/// # Returns
///
/// The number of available slots, or 0 if the race is invalid.
///
/// # Reference
///
/// C implementation: `EscortFeasibilityStudy()` lines 318-328
pub fn escort_feasibility_study(
    fleet_queue: &[FleetInfo],
    built_queue: &[ShipFragment],
    race: usize,
) -> usize {
    if fleet_queue.get(race).is_none() {
        return 0;
    }

    MAX_BUILT_SHIPS.saturating_sub(built_queue.len())
}

/// Sets the allied state of a fleet.
///
/// # Arguments
///
/// * `fleet_info` - The fleet to modify
/// * `allied` - Whether the fleet should be allied
///
/// # Notes
///
/// Silently ignores the request if the fleet is marked as `DeadGuy`.
///
/// # Reference
///
/// C implementation: `SetRaceAllied()` lines 209-231
pub fn set_race_allied(fleet_info: &mut FleetInfo, allied: bool) {
    if fleet_info.allied_state == AlliedState::DeadGuy {
        return;
    }

    fleet_info.allied_state = if allied {
        AlliedState::GoodGuy
    } else {
        AlliedState::BadGuy
    };
}

/// Checks the alliance state of a fleet.
///
/// # Arguments
///
/// * `fleet_info` - The fleet to check
///
/// # Returns
///
/// The current allied state.
pub fn check_alliance(fleet_info: &FleetInfo) -> AlliedState {
    fleet_info.allied_state
}

/// Starts tracking a sphere of influence for a fleet.
///
/// Updates the known strength and location when a fleet becomes known to the player.
///
/// # Arguments
///
/// * `fleet_info` - The fleet to start tracking
/// * `race` - The race index (returned on success to match C contract)
///
/// # Returns
///
/// `Some(race)` on success, `None` if the fleet is extinct (actual_strength == 0
/// and allied_state == DeadGuy). Matches C `StartSphereTracking()` which returns
/// the race index on success and 0 on failure.
///
/// # Reference
///
/// C implementation: `StartSphereTracking()` lines 239-269
pub fn start_sphere_tracking(fleet_info: &mut FleetInfo, race: usize) -> Option<usize> {
    if fleet_info.actual_strength == 0 && fleet_info.allied_state == AlliedState::DeadGuy {
        return None;
    }

    if fleet_info.actual_strength != 0
        && fleet_info.known_strength == 0
        && fleet_info.actual_strength != INFINITE_FLEET
    {
        fleet_info.known_strength = 1;
        fleet_info.known_loc = fleet_info.loc;
    }

    Some(race)
}

/// Finds an escort ship fragment by its starship index.
///
/// Searches the built queue for a fragment whose `index` field matches
/// the given starship index.
///
/// # Arguments
///
/// * `built_queue` - The built ship queue to search
/// * `index` - The starship index to search for
///
/// # Returns
///
/// A reference to the matching fragment, or `None` if not found.
///
/// # Reference
///
/// C implementation: `GetEscortByStarShipIndex()` lines 71-94
pub fn get_escort_by_starship_index(
    built_queue: &[ShipFragment],
    index: u8,
) -> Option<&ShipFragment> {
    built_queue.iter().find(|f| f.index == index)
}

/// Mutable version of `get_escort_by_starship_index`.
pub fn get_escort_by_starship_index_mut(
    built_queue: &mut [ShipFragment],
    index: u8,
) -> Option<&mut ShipFragment> {
    built_queue.iter_mut().find(|f| f.index == index)
}

/// Picks a random captain name index using the provided RNG.
///
/// The RNG function should return a random `u32`; in production this will be
/// the C engine's `TFB_Random()`, in tests a simple deterministic source.
///
/// # Returns
///
/// A captain name index in the range `[NAME_OFFSET, NAME_OFFSET + NUM_CAPTAINS_NAMES)`.
///
/// # Reference
///
/// C implementation: `PickCaptainName()` macro in build.h
pub fn pick_captain_name(rng: &mut dyn FnMut() -> u32) -> u8 {
    let val = rng();
    ((val as u8) % NUM_CAPTAINS_NAMES) + NAME_OFFSET
}

/// Names a captain, ensuring uniqueness within the same species.
///
/// Picks captain names until finding one not already used by the same species in the queue.
///
/// # Arguments
///
/// * `queue` - The queue to check for existing names
/// * `species_id` - The species to name a captain for
/// * `rng` - RNG function (C: `TFB_Random()`)
///
/// # Returns
///
/// A unique captain name index for the species.
///
/// # Reference
///
/// C implementation: `NameCaptain()` lines 428-458
pub fn name_captain<T: HasSpeciesAndCaptain>(
    queue: &[T],
    species_id: SpeciesId,
    rng: &mut dyn FnMut() -> u32,
) -> u8 {
    loop {
        let name = pick_captain_name(rng);

        let is_unique = !queue
            .iter()
            .any(|item| item.species_id() == species_id && item.captains_name_index() == name);

        if is_unique {
            return name;
        }
    }
}

/// Gets the index of a starship in the queue.
///
/// Finds the position by pointer equality.
///
/// # Arguments
///
/// * `queue` - The queue to search
/// * `target` - The starship to find
///
/// # Returns
///
/// The index if found, or `None` if not in the queue.
pub fn get_index_from_starship(queue: &[Starship], target: &Starship) -> Option<usize> {
    queue.iter().position(|s| std::ptr::eq(s, target))
}

/// Removes some escort ships of a specific race from the built queue.
///
/// # Arguments
///
/// * `built_queue` - The built ship queue to remove from
/// * `race` - The race ID to remove
/// * `count` - Maximum number of ships to remove
///
/// # Returns
///
/// The number of ships actually removed.
///
/// # Reference
///
/// C implementation: `RemoveSomeEscortShips()` lines 357-394
pub fn remove_some_escort_ships(
    built_queue: &mut Vec<ShipFragment>,
    race: u8,
    count: usize,
) -> usize {
    let mut removed = 0;

    built_queue.retain(|fragment| {
        if removed < count && fragment.race_id == race {
            removed += 1;
            false
        } else {
            true
        }
    });

    removed
}

/// Removes all escort ships of a specific race from the built queue.
///
/// # Arguments
///
/// * `built_queue` - The built ship queue to remove from
/// * `race` - The race ID to remove
pub fn remove_escort_ships(built_queue: &mut Vec<ShipFragment>, race: u8) {
    remove_some_escort_ships(built_queue, race, usize::MAX);
}

/// Calculates the total worth of all escorts in the built queue.
///
/// Sums up ship costs for all fragments using the species descriptor templates.
///
/// # Arguments
///
/// * `built_queue` - The built ship queue to calculate worth for
///
/// # Returns
///
/// The total cost of all ships in the queue.
///
/// # Reference
///
/// C implementation: `CalculateEscortsWorth()` lines 154-175
pub fn calculate_escorts_worth(built_queue: &[ShipFragment]) -> u16 {
    let mut total: u32 = 0;

    for fragment in built_queue {
        if let Ok(descriptor) = descriptor_template_for_species(fragment.species_id) {
            total = total.saturating_add(descriptor.ship_info.ship_cost as u32);
        }
    }

    total.min(u16::MAX as u32) as u16
}

/// Sets the crew complement and captain for a specific escort ship.
///
/// Finds the first ship of the specified race with full crew and updates its
/// crew level and captain name.
///
/// # Arguments
///
/// * `fleet_queue` - The fleet info templates
/// * `built_queue` - The built ship queue to modify
/// * `which_ship` - The race to find
/// * `crew_level` - The crew level to set
/// * `captain` - The captain name index to set
///
/// # Returns
///
/// The index of the modified ship, or `None` if no matching ship was found.
///
/// # Reference
///
/// C implementation: `SetEscortCrewComplement()` lines 511-547
pub fn set_escort_crew_complement(
    fleet_queue: &[FleetInfo],
    built_queue: &mut [ShipFragment],
    which_ship: usize,
    crew_level: u16,
    captain: u8,
) -> Option<usize> {
    let fleet_info = fleet_queue.get(which_ship)?;

    for (idx, fragment) in built_queue.iter_mut().enumerate() {
        // C: StarShipPtr->crew_level == TemplatePtr->crew_level (finds "full crew" ships)
        if fragment.race_id == which_ship as u8 && fragment.crew_level == fleet_info.crew_level {
            fragment.crew_level = crew_level;
            fragment.captains_name_index = captain;
            return Some(idx);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Deterministic test RNG: returns sequential values starting from seed.
    fn test_rng(seed: u32) -> impl FnMut() -> u32 {
        let mut counter = seed;
        move || {
            counter = counter.wrapping_add(7);
            counter
        }
    }

    fn create_test_fleet_info(species_id: SpeciesId) -> FleetInfo {
        FleetInfo {
            species_id,
            allied_state: AlliedState::BadGuy,
            days_left: 0,
            growth_fract: 0,
            crew_level: 10,
            max_crew: 20,
            growth: 0,
            max_energy: 30,
            loc: (100, 200),
            race_strings: 0,
            icons: 0,
            melee_icon: 0,
            actual_strength: 5,
            known_strength: 0,
            known_loc: (0, 0),
            growth_err_term: 0,
            func_index: 0xFF,
            dest_loc: (0, 0),
        }
    }

    #[test]
    fn test_build_ship_success() {
        let mut queue = Vec::new();
        let result = build_ship(&mut queue, SpeciesId::Earthling);

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
        assert_eq!(queue.len(), 1);
        assert_eq!(queue[0].species_id, SpeciesId::Earthling);
    }

    #[test]
    fn test_build_ship_invalid_species() {
        let mut queue = Vec::new();
        let result = build_ship(&mut queue, SpeciesId::NoId);

        assert!(result.is_err());
        assert_eq!(queue.len(), 0);
    }

    #[test]
    fn test_build_ship_multiple() {
        let mut queue = Vec::new();

        let idx1 = build_ship(&mut queue, SpeciesId::Earthling).unwrap();
        let idx2 = build_ship(&mut queue, SpeciesId::Androsynth).unwrap();
        let idx3 = build_ship(&mut queue, SpeciesId::Arilou).unwrap();

        assert_eq!(idx1, 0);
        assert_eq!(idx2, 1);
        assert_eq!(idx3, 2);
        assert_eq!(queue.len(), 3);
    }

    #[test]
    fn test_get_starship_from_index() {
        let mut queue = Vec::new();
        build_ship(&mut queue, SpeciesId::Earthling).unwrap();
        build_ship(&mut queue, SpeciesId::Androsynth).unwrap();

        let ship = get_starship_from_index(&queue, 1);
        assert!(ship.is_some());
        assert_eq!(ship.unwrap().species_id, SpeciesId::Androsynth);

        let none = get_starship_from_index(&queue, 10);
        assert!(none.is_none());
    }

    #[test]
    fn test_get_starship_from_index_mut() {
        let mut queue = Vec::new();
        build_ship(&mut queue, SpeciesId::Earthling).unwrap();

        let ship = get_starship_from_index_mut(&mut queue, 0);
        assert!(ship.is_some());

        if let Some(s) = ship {
            s.crew_level = 42;
        }

        assert_eq!(queue[0].crew_level, 42);
    }

    #[test]
    fn test_clone_ship_fragment_default_crew() {
        let fleet_info = create_test_fleet_info(SpeciesId::Earthling);
        let fragment = clone_ship_fragment(0, &fleet_info, 0, 7);

        assert_eq!(fragment.species_id, SpeciesId::Earthling);
        assert_eq!(fragment.race_id, 0);
        assert_eq!(fragment.captains_name_index, 7);
        assert_eq!(fragment.crew_level, 10);
        assert_eq!(fragment.max_crew, 20);
        assert_eq!(fragment.max_energy, 30);
        assert_eq!(fragment.energy_level, 0);
        assert_eq!(fragment.index, 0);
    }

    #[test]
    fn test_clone_ship_fragment_custom_crew() {
        let fleet_info = create_test_fleet_info(SpeciesId::Earthling);
        let fragment = clone_ship_fragment(1, &fleet_info, 15, 8);

        assert_eq!(fragment.crew_level, 15);
        assert_eq!(fragment.race_id, 1);
        assert_eq!(fragment.captains_name_index, 8);
    }

    #[test]
    fn test_add_escort_ships_basic() {
        let fleet_queue = vec![create_test_fleet_info(SpeciesId::Earthling)];
        let mut built_queue = Vec::new();
        let mut rng = test_rng(42);

        let added = add_escort_ships(&fleet_queue, &mut built_queue, 0, 3, &mut rng);

        assert_eq!(added, 3);
        assert_eq!(built_queue.len(), 3);

        for fragment in &built_queue {
            assert_eq!(fragment.species_id, SpeciesId::Earthling);
            assert_eq!(fragment.race_id, 0);
        }
    }

    #[test]
    fn test_add_escort_ships_capacity_limit() {
        let fleet_queue = vec![create_test_fleet_info(SpeciesId::Earthling)];
        let mut built_queue = Vec::new();
        let mut rng = test_rng(42);

        add_escort_ships(&fleet_queue, &mut built_queue, 0, 10, &mut rng);
        assert_eq!(built_queue.len(), 10);

        let added = add_escort_ships(&fleet_queue, &mut built_queue, 0, 5, &mut rng);
        assert_eq!(added, 2);
        assert_eq!(built_queue.len(), MAX_BUILT_SHIPS);
    }

    #[test]
    fn test_add_escort_ships_invalid_race() {
        let fleet_queue = vec![create_test_fleet_info(SpeciesId::Earthling)];
        let mut built_queue = Vec::new();
        let mut rng = test_rng(42);

        let added = add_escort_ships(&fleet_queue, &mut built_queue, 5, 3, &mut rng);
        assert_eq!(added, 0);
        assert_eq!(built_queue.len(), 0);
    }

    #[test]
    fn test_add_escort_ships_window_indices_sorted() {
        let fleet_queue = vec![create_test_fleet_info(SpeciesId::Earthling)];
        let mut built_queue = Vec::new();
        let mut rng = test_rng(42);

        add_escort_ships(&fleet_queue, &mut built_queue, 0, 5, &mut rng);

        // Window indices should be sequential
        let indices: Vec<u8> = built_queue.iter().map(|f| f.index).collect();
        assert_eq!(indices, vec![0, 1, 2, 3, 4]);

        // Queue should be sorted by window index
        for i in 1..built_queue.len() {
            assert!(built_queue[i].index >= built_queue[i - 1].index);
        }
    }

    #[test]
    fn test_add_escort_ships_fills_gaps() {
        let fleet_queue = vec![
            create_test_fleet_info(SpeciesId::Earthling),
            create_test_fleet_info(SpeciesId::Androsynth),
        ];
        let mut built_queue = Vec::new();
        let mut rng = test_rng(42);

        // Add 3 earthlings (indices 0,1,2)
        add_escort_ships(&fleet_queue, &mut built_queue, 0, 3, &mut rng);
        assert_eq!(
            built_queue.iter().map(|f| f.index).collect::<Vec<_>>(),
            vec![0, 1, 2]
        );

        // Remove the middle one (index 1)
        built_queue.retain(|f| f.index != 1);
        assert_eq!(built_queue.len(), 2);

        // Add an androsynth — should fill the gap at index 1
        add_escort_ships(&fleet_queue, &mut built_queue, 1, 1, &mut rng);
        assert_eq!(built_queue.len(), 3);

        // Queue should be sorted by window index with gap filled
        let indices: Vec<u8> = built_queue.iter().map(|f| f.index).collect();
        assert_eq!(indices, vec![0, 1, 2]);

        // The gap-filler should be Androsynth
        assert_eq!(built_queue[1].race_id, 1);
    }

    #[test]
    fn test_count_escort_ships() {
        let fleet_queue = vec![
            create_test_fleet_info(SpeciesId::Earthling),
            create_test_fleet_info(SpeciesId::Androsynth),
        ];
        let mut built_queue = Vec::new();
        let mut rng = test_rng(42);

        add_escort_ships(&fleet_queue, &mut built_queue, 0, 3, &mut rng);
        add_escort_ships(&fleet_queue, &mut built_queue, 1, 2, &mut rng);

        assert_eq!(count_escort_ships(&fleet_queue, &built_queue, 0), 3);
        assert_eq!(count_escort_ships(&fleet_queue, &built_queue, 1), 2);
        assert_eq!(count_escort_ships(&fleet_queue, &built_queue, 2), 0);
    }

    #[test]
    fn test_have_escort_ship() {
        let fleet_queue = vec![create_test_fleet_info(SpeciesId::Earthling)];
        let mut built_queue = Vec::new();
        let mut rng = test_rng(42);

        assert!(!have_escort_ship(&fleet_queue, &built_queue, 0));

        add_escort_ships(&fleet_queue, &mut built_queue, 0, 1, &mut rng);

        assert!(have_escort_ship(&fleet_queue, &built_queue, 0));
    }

    #[test]
    fn test_escort_feasibility_study() {
        let fleet_queue = vec![create_test_fleet_info(SpeciesId::Earthling)];
        let mut built_queue = Vec::new();
        let mut rng = test_rng(42);

        assert_eq!(escort_feasibility_study(&fleet_queue, &built_queue, 0), 12);

        add_escort_ships(&fleet_queue, &mut built_queue, 0, 5, &mut rng);
        assert_eq!(escort_feasibility_study(&fleet_queue, &built_queue, 0), 7);

        assert_eq!(escort_feasibility_study(&fleet_queue, &built_queue, 99), 0);
    }

    #[test]
    fn test_set_race_allied() {
        let mut fleet_info = create_test_fleet_info(SpeciesId::Earthling);

        assert_eq!(fleet_info.allied_state, AlliedState::BadGuy);

        set_race_allied(&mut fleet_info, true);
        assert_eq!(fleet_info.allied_state, AlliedState::GoodGuy);

        set_race_allied(&mut fleet_info, false);
        assert_eq!(fleet_info.allied_state, AlliedState::BadGuy);
    }

    #[test]
    fn test_set_race_allied_dead_guy() {
        let mut fleet_info = create_test_fleet_info(SpeciesId::Earthling);
        fleet_info.allied_state = AlliedState::DeadGuy;

        set_race_allied(&mut fleet_info, true);
        assert_eq!(fleet_info.allied_state, AlliedState::DeadGuy);
    }

    #[test]
    fn test_check_alliance() {
        let mut fleet_info = create_test_fleet_info(SpeciesId::Earthling);

        assert_eq!(check_alliance(&fleet_info), AlliedState::BadGuy);

        fleet_info.allied_state = AlliedState::GoodGuy;
        assert_eq!(check_alliance(&fleet_info), AlliedState::GoodGuy);
    }

    #[test]
    fn test_start_sphere_tracking_success() {
        let mut fleet_info = create_test_fleet_info(SpeciesId::Earthling);
        fleet_info.actual_strength = 10;
        fleet_info.known_strength = 0;

        let result = start_sphere_tracking(&mut fleet_info, 5);

        assert_eq!(result, Some(5));
        assert_eq!(fleet_info.known_strength, 1);
        assert_eq!(fleet_info.known_loc.0, 100);
        assert_eq!(fleet_info.known_loc.1, 200);
    }

    #[test]
    fn test_start_sphere_tracking_extinct() {
        let mut fleet_info = create_test_fleet_info(SpeciesId::Earthling);
        fleet_info.actual_strength = 0;
        fleet_info.allied_state = AlliedState::DeadGuy;

        let result = start_sphere_tracking(&mut fleet_info, 5);
        assert_eq!(result, None);
    }

    #[test]
    fn test_start_sphere_tracking_already_known() {
        let mut fleet_info = create_test_fleet_info(SpeciesId::Earthling);
        fleet_info.actual_strength = 10;
        fleet_info.known_strength = 5;

        let original_known = fleet_info.known_strength;
        let result = start_sphere_tracking(&mut fleet_info, 3);

        assert_eq!(result, Some(3));
        assert_eq!(fleet_info.known_strength, original_known);
    }

    #[test]
    fn test_start_sphere_tracking_infinite_fleet() {
        let mut fleet_info = create_test_fleet_info(SpeciesId::Earthling);
        fleet_info.actual_strength = INFINITE_FLEET;
        fleet_info.known_strength = 0;

        let result = start_sphere_tracking(&mut fleet_info, 7);

        assert_eq!(result, Some(7));
        assert_eq!(fleet_info.known_strength, 0);
    }

    #[test]
    fn test_pick_captain_name_range() {
        let mut rng = test_rng(42);
        for _ in 0..100 {
            let name = pick_captain_name(&mut rng);
            assert!(name >= NAME_OFFSET);
            assert!(name < NAME_OFFSET + NUM_CAPTAINS_NAMES);
        }
    }

    #[test]
    fn test_name_captain_uniqueness() {
        let mut starship_queue = Vec::new();
        let mut rng = test_rng(42);

        for _ in 0..5 {
            let name = name_captain(&starship_queue, SpeciesId::Earthling, &mut rng);
            let mut ship = Starship::default();
            ship.species_id = SpeciesId::Earthling;
            ship.captains_name_index = name;
            starship_queue.push(ship);
        }

        let mut names: Vec<u8> = starship_queue
            .iter()
            .map(|s| s.captains_name_index)
            .collect();
        names.sort_unstable();
        names.dedup();

        assert_eq!(names.len(), 5);
    }

    #[test]
    fn test_name_captain_different_species() {
        let mut starship_queue = Vec::new();
        let mut rng = test_rng(42);

        let name1 = name_captain(&starship_queue, SpeciesId::Earthling, &mut rng);
        let mut ship1 = Starship::default();
        ship1.species_id = SpeciesId::Earthling;
        ship1.captains_name_index = name1;
        starship_queue.push(ship1);

        let name2 = name_captain(&starship_queue, SpeciesId::Androsynth, &mut rng);
        let mut ship2 = Starship::default();
        ship2.species_id = SpeciesId::Androsynth;
        ship2.captains_name_index = name2;
        starship_queue.push(ship2);

        assert_eq!(starship_queue.len(), 2);
    }

    #[test]
    fn test_get_index_from_starship() {
        let mut queue = Vec::new();
        build_ship(&mut queue, SpeciesId::Earthling).unwrap();
        build_ship(&mut queue, SpeciesId::Androsynth).unwrap();
        build_ship(&mut queue, SpeciesId::Arilou).unwrap();

        let target = &queue[1];
        let idx = get_index_from_starship(&queue, target);

        assert_eq!(idx, Some(1));
    }

    #[test]
    fn test_get_index_from_starship_not_found() {
        let mut queue = Vec::new();
        build_ship(&mut queue, SpeciesId::Earthling).unwrap();

        let other_ship = Starship {
            species_id: SpeciesId::Androsynth,
            ..Default::default()
        };

        let idx = get_index_from_starship(&queue, &other_ship);
        assert_eq!(idx, None);
    }

    #[test]
    fn test_remove_some_escort_ships() {
        let fleet_queue = vec![
            create_test_fleet_info(SpeciesId::Earthling),
            create_test_fleet_info(SpeciesId::Androsynth),
        ];
        let mut built_queue = Vec::new();
        let mut rng = test_rng(42);

        add_escort_ships(&fleet_queue, &mut built_queue, 0, 4, &mut rng);
        add_escort_ships(&fleet_queue, &mut built_queue, 1, 3, &mut rng);

        assert_eq!(built_queue.len(), 7);

        let removed = remove_some_escort_ships(&mut built_queue, 0, 2);

        assert_eq!(removed, 2);
        assert_eq!(built_queue.len(), 5);
        assert_eq!(count_escort_ships(&fleet_queue, &built_queue, 0), 2);
    }

    #[test]
    fn test_remove_some_escort_ships_more_than_available() {
        let fleet_queue = vec![create_test_fleet_info(SpeciesId::Earthling)];
        let mut built_queue = Vec::new();
        let mut rng = test_rng(42);

        add_escort_ships(&fleet_queue, &mut built_queue, 0, 3, &mut rng);

        let removed = remove_some_escort_ships(&mut built_queue, 0, 10);

        assert_eq!(removed, 3);
        assert_eq!(built_queue.len(), 0);
    }

    #[test]
    fn test_remove_escort_ships() {
        let fleet_queue = vec![
            create_test_fleet_info(SpeciesId::Earthling),
            create_test_fleet_info(SpeciesId::Androsynth),
        ];
        let mut built_queue = Vec::new();
        let mut rng = test_rng(42);

        add_escort_ships(&fleet_queue, &mut built_queue, 0, 4, &mut rng);
        add_escort_ships(&fleet_queue, &mut built_queue, 1, 3, &mut rng);

        remove_escort_ships(&mut built_queue, 0);

        assert_eq!(built_queue.len(), 3);
        assert_eq!(count_escort_ships(&fleet_queue, &built_queue, 0), 0);
        assert_eq!(count_escort_ships(&fleet_queue, &built_queue, 1), 3);
    }

    #[test]
    fn test_calculate_escorts_worth_empty() {
        let built_queue = Vec::new();
        let worth = calculate_escorts_worth(&built_queue);
        assert_eq!(worth, 0);
    }

    #[test]
    fn test_set_escort_crew_complement_success() {
        let fleet_queue = vec![create_test_fleet_info(SpeciesId::Earthling)];
        let mut built_queue = Vec::new();
        let mut rng = test_rng(42);

        add_escort_ships(&fleet_queue, &mut built_queue, 0, 3, &mut rng);

        // Fragments start with crew_level matching fleet_info.crew_level (10)
        let idx = set_escort_crew_complement(&fleet_queue, &mut built_queue, 0, 15, 10);

        assert!(idx.is_some());

        let modified_idx = idx.unwrap();
        assert_eq!(built_queue[modified_idx].crew_level, 15);
        assert_eq!(built_queue[modified_idx].captains_name_index, 10);
    }

    #[test]
    fn test_set_escort_crew_complement_no_match() {
        let fleet_queue = vec![create_test_fleet_info(SpeciesId::Earthling)];
        let mut built_queue = Vec::new();
        let mut rng = test_rng(42);

        add_escort_ships(&fleet_queue, &mut built_queue, 0, 3, &mut rng);

        // Set all to different crew level than template
        for fragment in &mut built_queue {
            fragment.crew_level = 5;
        }

        let idx = set_escort_crew_complement(&fleet_queue, &mut built_queue, 0, 15, 10);
        assert!(idx.is_none());
    }

    #[test]
    fn test_set_escort_crew_complement_invalid_race() {
        let fleet_queue = vec![create_test_fleet_info(SpeciesId::Earthling)];
        let mut built_queue = Vec::new();

        let idx = set_escort_crew_complement(&fleet_queue, &mut built_queue, 99, 15, 10);
        assert!(idx.is_none());
    }

    #[test]
    fn test_get_escort_by_starship_index() {
        let fleet_queue = vec![create_test_fleet_info(SpeciesId::Earthling)];
        let mut built_queue = Vec::new();
        let mut rng = test_rng(42);

        add_escort_ships(&fleet_queue, &mut built_queue, 0, 3, &mut rng);

        let found = get_escort_by_starship_index(&built_queue, 1);
        assert!(found.is_some());
        assert_eq!(found.unwrap().index, 1);

        let not_found = get_escort_by_starship_index(&built_queue, 10);
        assert!(not_found.is_none());
    }

    #[test]
    fn test_get_escort_by_starship_index_mut() {
        let fleet_queue = vec![create_test_fleet_info(SpeciesId::Earthling)];
        let mut built_queue = Vec::new();
        let mut rng = test_rng(42);

        add_escort_ships(&fleet_queue, &mut built_queue, 0, 3, &mut rng);

        if let Some(frag) = get_escort_by_starship_index_mut(&mut built_queue, 0) {
            frag.crew_level = 99;
        }

        assert_eq!(built_queue[0].crew_level, 99);
    }
}
