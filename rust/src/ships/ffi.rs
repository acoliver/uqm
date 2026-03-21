//! C-Side Bridge Wiring for Ships Subsystem
//!
//! @plan PLAN-20260314-SHIPS.P14
//! @requirement REQ-QUEUE-MODEL, REQ-QUEUE-OWNER-BOUNDARY, REQ-SPAWN-ENTRYPOINT
//!
//! This module provides the FFI export surface that connects the Rust ships subsystem
//! to C. All functions are panic-safe (use `catch_unwind`) and validate pointer
//! arguments before delegation.
//!
//! ## Ownership Model
//! - C owns queue storage (`STARSHIP`, `SHIP_FRAGMENT`, `FLEET_INFO`)
//! - Rust owns `RACE_DESC` returned by `rust_ships_load()`
//! - Rust owns master catalog entries until `rust_ships_free_catalog()`

use super::catalog::{free_master_ship_list, get_ship_cost_from_index, load_master_ship_list};
use super::ffi_contract::*;
use super::lifecycle::{init_ships, spawn_ship as lifecycle_spawn};
use super::loader::{free_ship as loader_free_ship, load_ship, LoadTier};
use super::types::{RaceDesc, SpeciesId, Starship};
use crate::battle::element::{Element, FrameHandle};
use std::mem::ManuallyDrop;
use std::os::raw::c_void;
use std::panic::catch_unwind;
use std::ptr;

// ===========================================================================
// C Function Imports
// ===========================================================================

#[cfg(not(test))]
extern "C" {
    // Queue management functions
    fn AllocLink(queue: *mut c_void) -> *mut c_void;
    fn PutQueue(queue: *mut c_void, link: *mut c_void);
    fn LockLink(queue: *mut c_void, link: *mut c_void) -> *mut c_void;
    fn UnlockLink(queue: *mut c_void, link: *mut c_void);
    fn GetLinkSize(queue: *mut c_void) -> usize;

    // Activity getter
    fn uqm_get_current_activity_lobyte() -> u8;
}

// ===========================================================================
// Catalog FFI
// ===========================================================================

/// Loads the master ship catalog.
///
/// C: `BOOLEAN rust_ships_load_catalog(void);`
///
/// Returns 1 on success, 0 on failure.
#[no_mangle]
pub extern "C" fn rust_ships_load_catalog() -> CBoolean {
    catch_unwind(|| match load_master_ship_list() {
        Ok(()) => 1,
        Err(_) => 0,
    })
    .unwrap_or_default()
}

/// Frees the master ship catalog.
///
/// C: `void rust_ships_free_catalog(void);`
#[no_mangle]
pub extern "C" fn rust_ships_free_catalog() {
    let _ = catch_unwind(free_master_ship_list);
}

/// Gets the ship cost by catalog index.
///
/// C: `COUNT rust_ships_get_cost_by_index(COUNT index);`
///
/// Returns 0 if the index is invalid or catalog is not loaded.
#[no_mangle]
pub extern "C" fn rust_ships_get_cost_by_index(index: CCount) -> CCount {
    catch_unwind(|| get_ship_cost_from_index(index as usize).unwrap_or_default())
        .unwrap_or_default()
}

// ===========================================================================
// Loader FFI
// ===========================================================================

/// Loads a ship descriptor at the specified tier.
///
/// C: `RACE_DESC *rust_ships_load(SPECIES_ID species, BOOLEAN battle_ready);`
///
/// Returns a Rust-owned `*mut CRaceDesc` on success, or null on failure.
/// Caller must free with `rust_ships_free()`.
#[no_mangle]
pub extern "C" fn rust_ships_load(species: CSpeciesId, battle_ready: CBoolean) -> *mut CRaceDesc {
    match catch_unwind(|| {
        let species_enum = match SpeciesId::from_i32(species) {
            Some(s) => s,
            None => return ptr::null_mut(),
        };

        let tier = if c_bool(battle_ready) {
            LoadTier::BattleReady
        } else {
            LoadTier::MetadataOnly
        };

        match load_ship(species_enum, tier) {
            Ok(desc) => Box::into_raw(Box::new(desc)) as *mut CRaceDesc,
            Err(_) => ptr::null_mut(),
        }
    }) {
        Ok(result) => result,
        Err(_) => ptr::null_mut(),
    }
}

/// Frees a ship descriptor.
///
/// C: `void rust_ships_free(RACE_DESC *desc, BOOLEAN free_battle, BOOLEAN free_metadata);`
///
/// # Safety
/// `desc` must be a valid pointer returned by `rust_ships_load()` and not yet freed.
#[no_mangle]
pub extern "C" fn rust_ships_free(
    desc: *mut CRaceDesc,
    free_battle: CBoolean,
    free_metadata: CBoolean,
) {
    let _ = catch_unwind(|| {
        if desc.is_null() {
            return;
        }

        // Safety: we trust C to pass a valid pointer from rust_ships_load()
        unsafe {
            let mut desc_box = Box::from_raw(desc as *mut RaceDesc);
            loader_free_ship(&mut desc_box, c_bool(free_battle), c_bool(free_metadata));
        }
    });
}

// ===========================================================================
// Queue/Build FFI
// ===========================================================================

/// Builds a new ship in a C-owned queue.
///
/// C: `HSTARSHIP rust_ships_build(QUEUE *queue, SPECIES_ID species);`
///
/// Allocates a new STARSHIP or SHIP_FRAGMENT entry in the C-owned queue,
/// initializes it with the species ID, and adds it to the queue.
#[no_mangle]
pub extern "C" fn rust_ships_build(
    queue: *mut std::os::raw::c_void,
    species: CSpeciesId,
) -> HStarship {
    #[cfg(test)]
    {
        let _ = (queue, species);
        return ptr::null_mut();
    }

    #[cfg(not(test))]
    catch_unwind(|| {
        if queue.is_null() {
            return ptr::null_mut();
        }

        // Safety: We trust C to pass a valid queue pointer
        unsafe {
            // Allocate a new link in the queue
            let h_new_ship = AllocLink(queue);
            if h_new_ship.is_null() {
                return ptr::null_mut();
            }

            // Lock the link to access the struct
            let ship_ptr = LockLink(queue, h_new_ship);
            if ship_ptr.is_null() {
                // Allocation failed
                return ptr::null_mut();
            }

            // Zero out the entire struct
            let link_size = GetLinkSize(queue);
            ptr::write_bytes(ship_ptr as *mut u8, 0, link_size);

            // Cast to CStarship (STARSHIP and SHIP_FRAGMENT both start with SHIP_BASE_COMMON)
            let ship_base = ship_ptr as *mut CStarship;
            (*ship_base).species_id = species;

            // Unlock the link
            UnlockLink(queue, h_new_ship);

            // Add to the queue
            PutQueue(queue, h_new_ship);

            h_new_ship
        }
    })
    .unwrap_or(ptr::null_mut())
}

/// Clones a ship fragment from source to destination.
///
/// C: `BOOLEAN rust_ships_clone_fragment(const SHIP_FRAGMENT *src, SHIP_FRAGMENT *dst);`
///
/// Returns 1 on success, 0 on failure.
///
/// # Safety
/// Both `src` and `dst` must be valid pointers to C-owned `SHIP_FRAGMENT` structs.
#[no_mangle]
pub extern "C" fn rust_ships_clone_fragment(
    src: *const std::os::raw::c_void,
    dst: *mut std::os::raw::c_void,
) -> CBoolean {
    catch_unwind(|| {
        if src.is_null() || dst.is_null() {
            return 0;
        }

        // Safety: we trust C to pass valid pointers
        unsafe {
            let src_frag = &*(src as *const CShipFragment);
            let dst_frag = &mut *(dst as *mut CShipFragment);

            // Copy all fields from CShipFragment
            dst_frag.species_id = src_frag.species_id;
            dst_frag.captains_name_index = src_frag.captains_name_index;
            dst_frag.race_id = src_frag.race_id;
            dst_frag.index = src_frag.index;
            dst_frag.crew_level = src_frag.crew_level;
            dst_frag.max_crew = src_frag.max_crew;
            dst_frag.energy_level = src_frag.energy_level;
            dst_frag.max_energy = src_frag.max_energy;
            dst_frag.race_strings = src_frag.race_strings;
            dst_frag.icons = src_frag.icons;
            dst_frag.melee_icon = src_frag.melee_icon;
            1
        }
    })
    .unwrap_or_default()
}

// ===========================================================================
// Spawn/Lifecycle FFI
// ===========================================================================

/// Spawns a ship in battle.
///
/// C: `BOOLEAN rust_ships_spawn(STARSHIP *starship);`
///
/// Returns 1 on success, 0 on failure.
///
/// # Safety
/// `starship` must be a valid pointer to a C-owned `STARSHIP` struct.
#[no_mangle]
pub extern "C" fn rust_ships_spawn(starship: *mut std::os::raw::c_void) -> CBoolean {
    catch_unwind(|| {
        if starship.is_null() {
            return 0;
        }

        // Safety: we trust C to pass a valid pointer
        unsafe {
            let starship_c = &mut *(starship as *mut CStarship);

            let mut starship_rust = Starship {
                species_id: match SpeciesId::from_i32(starship_c.species_id) {
                    Some(s) => s,
                    None => return 0,
                },
                captains_name_index: starship_c.captains_name_index,
                race_desc: None,
                crew_level: starship_c.crew_level,
                max_crew: starship_c.max_crew,
                ship_cost: starship_c.ship_cost,
                index: starship_c.index,
                race_strings: starship_c.race_strings as usize,
                icons: starship_c.icons as usize,
                weapon_counter: starship_c.weapon_counter,
                special_counter: starship_c.special_counter,
                energy_counter: starship_c.energy_counter,
                ship_input_state: starship_c.ship_input_state,
                cur_status_flags: super::types::StatusFlags(starship_c.cur_status_flags),
                old_status_flags: super::types::StatusFlags(starship_c.old_status_flags),
                h_ship: starship_c.h_ship as usize,
                ship_facing: starship_c.ship_facing,
                player_nr: starship_c.player_nr,
                control: starship_c.control,
                audio_stopped: false,
            };

            // Get activity from C
            #[cfg(test)]
            let activity = 2u8; // IN_ENCOUNTER for tests
            #[cfg(not(test))]
            let activity = uqm_get_current_activity_lobyte();

            match lifecycle_spawn(&mut starship_rust, activity) {
                Ok(_) => {
                    starship_c.race_desc_ptr = match starship_rust.race_desc {
                        Some(desc) => Box::into_raw(desc) as *mut std::os::raw::c_void,
                        None => ptr::null_mut(),
                    };
                    1
                }
                Err(_) => 0,
            }
        }
    })
    .unwrap_or_default()
}

/// Initializes the battle subsystem.
///
/// C: `COUNT rust_ships_init(void);`
///
/// Returns the number of players (2) on success, 0 on failure.
#[no_mangle]
pub extern "C" fn rust_ships_init() -> CCount {
    catch_unwind(|| {
        // Get activity from C
        #[cfg(test)]
        let activity = 2u8; // IN_ENCOUNTER for tests
        #[cfg(not(test))]
        let activity = unsafe { uqm_get_current_activity_lobyte() };

        match init_ships(activity) {
            Ok(num_players) => num_players as CCount,
            Err(_) => 0,
        }
    })
    .unwrap_or_default()
}

/// Uninitializes the battle subsystem.
///
/// C: `void rust_ships_uninit(void);`
///
/// Frees Rust-owned resources (catalog). Since C owns the queues, crew writeback
/// and descriptor cleanup are handled by C's UninitShips() calling rust_ships_free()
/// on each descriptor.
#[no_mangle]
pub extern "C" fn rust_ships_uninit() {
    let _ = catch_unwind(|| {
        // Free the master catalog if loaded
        free_master_ship_list();

        // Note: C handles queue iteration, crew writeback, and calling rust_ships_free()
        // for each RACE_DESC. Rust uninit only needs to release global resources.
    });
}

// ===========================================================================
// Runtime Callbacks FFI
// ===========================================================================

/// Builds a Rust Starship from C fields WITHOUT taking ownership of race_desc.
///
/// Returns `ManuallyDrop<Starship>` so that if a panic occurs during the
/// callback, the Starship's Drop (which would free the Box<RaceDesc> inside)
/// is never executed. The caller uses `&mut *starship` to work with it.
/// When the ManuallyDrop goes out of scope, no destructors run and C's
/// race_desc pointer remains valid.
///
/// # Safety
/// `starship_c` must be a valid reference. `race_desc_ptr` must point to a
/// Rust-allocated RaceDesc (from `rust_ships_load`).
#[cfg(not(test))]
unsafe fn borrow_starship_from_c(starship_c: &CStarship) -> Option<ManuallyDrop<Starship>> {
    let species_id = SpeciesId::from_i32(starship_c.species_id)?;
    // Box::from_raw reconstitutes the Box but ManuallyDrop prevents Drop
    let race_desc_box = Box::from_raw(starship_c.race_desc_ptr as *mut RaceDesc);
    Some(ManuallyDrop::new(Starship {
        species_id,
        captains_name_index: starship_c.captains_name_index,
        race_desc: Some(race_desc_box),
        crew_level: starship_c.crew_level,
        max_crew: starship_c.max_crew,
        ship_cost: starship_c.ship_cost,
        index: starship_c.index,
        race_strings: starship_c.race_strings as usize,
        icons: starship_c.icons as usize,
        weapon_counter: starship_c.weapon_counter,
        special_counter: starship_c.special_counter,
        energy_counter: starship_c.energy_counter,
        ship_input_state: starship_c.ship_input_state,
        cur_status_flags: super::types::StatusFlags(starship_c.cur_status_flags),
        old_status_flags: super::types::StatusFlags(starship_c.old_status_flags),
        h_ship: starship_c.h_ship as usize,
        ship_facing: starship_c.ship_facing,
        player_nr: starship_c.player_nr,
        control: starship_c.control,
        audio_stopped: false,
    }))
}

/// Builds an ElementState from a C Element pointer.
///
/// Maps battle::element::Element fields to ships::runtime::ElementState,
/// handling field naming differences (crew_or_hp→crew_level, thrust_or_blast,
/// ElementFlags→u16, FrameHandle→u16).
///
/// # Safety
/// `elem_ptr` must be a valid pointer to a C-owned ELEMENT struct.
#[cfg(not(test))]
unsafe fn build_element_state(elem_ptr: *const Element) -> super::runtime::ElementState {
    super::runtime::ElementState {
        state_flags: (*elem_ptr).state_flags.bits(),
        life_span: (*elem_ptr).life_span,
        crew_level: (*elem_ptr).crew_or_hp,
        mass_points: (*elem_ptr).mass_points,
        turn_wait: (*elem_ptr).turn_wait,
        thrust_wait: (*elem_ptr).thrust_or_blast,
        next_turn: 0, // next_turn shares storage with thrust_or_blast in C union
        color_cycle_index: (*elem_ptr).color_cycle_index,
        player_nr: (*elem_ptr).player_nr,
        position: (
            (*elem_ptr).current.location.x as i32,
            (*elem_ptr).current.location.y as i32,
        ),
        next_position: (
            (*elem_ptr).next.location.x as i32,
            (*elem_ptr).next.location.y as i32,
        ),
        velocity: super::runtime::VelocityState::default(),
        image_frame: (*elem_ptr).current.frame as u16,
        prim_index: (*elem_ptr).prim_index,
        h_target: (*elem_ptr).h_target as usize,
    }
}

/// Writes back changed Starship fields to the C struct.
///
/// # Safety
/// `starship_c` must be a valid mutable reference.
#[cfg(not(test))]
unsafe fn writeback_starship(starship_c: &mut CStarship, starship_rust: &Starship) {
    starship_c.cur_status_flags = starship_rust.cur_status_flags.0;
    starship_c.old_status_flags = starship_rust.old_status_flags.0;
    starship_c.weapon_counter = starship_rust.weapon_counter;
    starship_c.special_counter = starship_rust.special_counter;
    starship_c.energy_counter = starship_rust.energy_counter;
    starship_c.ship_input_state = starship_rust.ship_input_state;
    starship_c.ship_facing = starship_rust.ship_facing;
}

/// Extracts a mutable CStarship ref from an element's pParent, validating
/// both the element pointer and the starship/race_desc pointers.
///
/// # Safety
/// `element` must be a valid pointer to a C-owned ELEMENT struct.
#[cfg(not(test))]
unsafe fn extract_starship_from_element(
    element: *mut c_void,
) -> Option<(*mut Element, &'static mut CStarship)> {
    let elem_ptr = element as *mut Element;
    let starship_ptr = (*elem_ptr).p_parent as *mut CStarship;
    if starship_ptr.is_null() {
        return None;
    }
    let starship_c = &mut *starship_ptr;
    if starship_c.race_desc_ptr.is_null() {
        return None;
    }
    Some((elem_ptr, starship_c))
}

/// Ship preprocess callback (called once per frame before physics).
///
/// C: `void rust_ships_preprocess(ELEMENT *element);`
///
/// # Safety
/// `element` must be a valid pointer to a C-owned `ELEMENT` struct.
#[no_mangle]
pub extern "C" fn rust_ships_preprocess(element: *mut c_void) {
    let _ = catch_unwind(|| {
        if element.is_null() {
            return;
        }

        #[cfg(test)]
        {
            return;
        }

        #[cfg(not(test))]
        unsafe {
            let (elem_ptr, starship_c) = match extract_starship_from_element(element) {
                Some(pair) => pair,
                None => return,
            };

            let mut starship_md = match borrow_starship_from_c(starship_c) {
                Some(s) => s,
                None => return,
            };

            let mut element_state = build_element_state(elem_ptr);

            // Deref ManuallyDrop to get &mut Starship — panic-safe because
            // ManuallyDrop prevents Drop of the inner Starship (and its Box<RaceDesc>)
            if super::runtime::ship_preprocess(&mut starship_md, &mut element_state).is_ok() {
                writeback_starship(starship_c, &starship_md);
                (*elem_ptr).state_flags = crate::battle::element::ElementFlags::from_bits_truncate(
                    element_state.state_flags,
                );
                (*elem_ptr).turn_wait = element_state.turn_wait;
                (*elem_ptr).thrust_or_blast = element_state.thrust_wait;
                (*elem_ptr).crew_or_hp = element_state.crew_level;
                (*elem_ptr).current.frame = element_state.image_frame as FrameHandle;
            }
            // starship_md drops here — ManuallyDrop prevents Box<RaceDesc> free
        }
    });
}

/// Ship postprocess callback (called once per frame after physics).
///
/// C: `void rust_ships_postprocess(ELEMENT *element);`
///
/// # Safety
/// `element` must be a valid pointer to a C-owned `ELEMENT` struct.
#[no_mangle]
pub extern "C" fn rust_ships_postprocess(element: *mut c_void) {
    let _ = catch_unwind(|| {
        if element.is_null() {
            return;
        }

        #[cfg(test)]
        {
            return;
        }

        #[cfg(not(test))]
        unsafe {
            let (elem_ptr, starship_c) = match extract_starship_from_element(element) {
                Some(pair) => pair,
                None => return,
            };

            let mut starship_md = match borrow_starship_from_c(starship_c) {
                Some(s) => s,
                None => return,
            };

            let mut element_state = build_element_state(elem_ptr);

            if super::runtime::ship_postprocess(&mut starship_md, &mut element_state).is_ok() {
                writeback_starship(starship_c, &starship_md);
                (*elem_ptr).crew_or_hp = element_state.crew_level;
            }
        }
    });
}

/// Ship death callback (called when crew reaches 0).
///
/// C: `void rust_ships_death(ELEMENT *element);`
///
/// # Safety
/// `element` must be a valid pointer to a C-owned `ELEMENT` struct.
#[no_mangle]
pub extern "C" fn rust_ships_death(element: *mut c_void) {
    let _ = catch_unwind(|| {
        if element.is_null() {
            return;
        }

        #[cfg(test)]
        {
            return;
        }

        #[cfg(not(test))]
        unsafe {
            let (elem_ptr, starship_c) = match extract_starship_from_element(element) {
                Some(pair) => pair,
                None => return,
            };

            let mut starship_md = match borrow_starship_from_c(starship_c) {
                Some(s) => s,
                None => return,
            };

            let _element_state = build_element_state(elem_ptr);

            // ShipBehavior has uninit() for cleanup, not a death() method.
            // Ship death orchestration (explosion, debris, winner) is battle engine scope.
            // Here we just call uninit() for race-specific resource cleanup.
            if let Some(ref mut desc) = starship_md.race_desc {
                desc.behavior.uninit();
            }

            writeback_starship(starship_c, &starship_md);
        }
    });
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_catalog_load_free() {
        // Ensure clean state
        rust_ships_free_catalog();

        let result = rust_ships_load_catalog();
        assert_eq!(result, 1);

        rust_ships_free_catalog();
    }

    #[test]
    fn test_get_cost_by_index() {
        rust_ships_load_catalog();

        // Index 0 should be valid (first melee ship after sorting)
        let cost = rust_ships_get_cost_by_index(0);
        assert!(cost > 0);

        // Invalid index should return 0
        let cost = rust_ships_get_cost_by_index(9999);
        assert_eq!(cost, 0);

        rust_ships_free_catalog();
    }

    #[test]
    fn test_load_free_ship() {
        // Load VUX (species 11) metadata-only
        let desc = rust_ships_load(11, 0);
        assert!(!desc.is_null());

        // Free it
        rust_ships_free(desc, 1, 1);
    }

    #[test]
    fn test_load_free_ship_battle_ready() {
        // Load VUX (species 11) battle-ready
        let desc = rust_ships_load(11, 1);
        assert!(!desc.is_null());

        // Free it
        rust_ships_free(desc, 1, 1);
    }

    #[test]
    fn test_load_ship_invalid_species() {
        let desc = rust_ships_load(999, 0);
        assert!(desc.is_null());
    }

    #[test]
    fn test_init_uninit_ships() {
        let result = rust_ships_init();
        assert_eq!(result, 2); // NUM_PLAYERS

        rust_ships_uninit();
    }

    #[test]
    fn test_null_pointer_safety() {
        // All functions should handle null pointers gracefully
        rust_ships_free(ptr::null_mut(), 0, 0);
        assert_eq!(rust_ships_spawn(ptr::null_mut()), 0);
        assert_eq!(
            rust_ships_clone_fragment(ptr::null_mut(), ptr::null_mut()),
            0
        );

        // These should not crash
        rust_ships_preprocess(ptr::null_mut());
        rust_ships_postprocess(ptr::null_mut());
        rust_ships_death(ptr::null_mut());
    }
}
