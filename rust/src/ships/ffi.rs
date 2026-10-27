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
use super::lifecycle::spawn_ship as lifecycle_spawn;
use super::loader::{free_ship as loader_free_ship, load_ship, LoadTier};
#[cfg(not(test))]
use super::types::{Characteristics, ShipData, ShipInfo};
use super::types::{RaceDesc, SpeciesId, Starship};
use crate::collections::queue::{AllocLink, PutQueue, Queue};
use std::os::raw::c_void;
use std::panic::catch_unwind;
use std::ptr;

// ===========================================================================
// Layout Verification (P05)
// ===========================================================================

/// Whether the RaceDesc/RACE_DESC layout check has been performed.
/// Set to `true` after the first call to `verify_race_desc_layout()`.
// Transitional: layout verification runs but the accessor-mode dispatch path
// is not yet wired into all field-access sites. Retained so the FFI shape is
// stable while the accessors are rolled out.
#[cfg(not(test))]
static mut LAYOUT_ACCESSOR_MODE: bool = false;

/// Returns `true` if accessor functions must be used instead of direct
/// struct casts for RaceDesc/RACE_DESC cross-language field access.
///
/// This is `true` when Rust's `RaceDesc` layout does not match C's
/// `RACE_DESC` layout — which is the expected case because RaceDesc
/// uses owned Rust data instead of native callback pointers.
#[inline]
#[cfg(not(test))]
#[expect(
    dead_code,
    reason = "transitional layout-accessor dispatch check, not yet wired into all field-access sites"
)]
fn is_accessor_mode() -> bool {
    // SAFETY: read-only, written once during init before any concurrent access.
    unsafe { LAYOUT_ACCESSOR_MODE }
}

/// Verify that Rust `RaceDesc` and C `RACE_DESC` have matching field offsets.
///
/// Called once during `rust_ships_init()`. Since RaceDesc is NOT `#[repr(C)]`
/// and owns Rust metadata rather than native callback pointers, the layouts are expected to differ. When they differ, this sets
/// `LAYOUT_ACCESSOR_MODE = true` so that accessor functions are used instead
/// of direct struct casts.
///
/// This is NOT debug-only — it runs in all builds because a layout mismatch
/// without accessor functions would mean silent memory corruption.
#[cfg(not(test))]
fn verify_race_desc_layout() {
    use crate::ships::ffi_contract::{rust_bridge_get_race_desc_layout, RaceDescLayout};

    let mut c_layout = RaceDescLayout {
        race_desc_size: 0,
        ship_data_offset: 0,
        ship_info_offset: 0,
        characteristics_offset: 0,
        ship_data_ship_offset: 0,
        ship_info_crew_offset: 0,
        ship_info_max_crew_offset: 0,
        characteristics_mass_offset: 0,
    };
    // SAFETY: the provider initializes this live writable layout record.
    unsafe {
        rust_bridge_get_race_desc_layout(&mut c_layout);
    }

    let mismatches: Vec<String> = [
        (
            "RACE_DESC size",
            c_layout.race_desc_size,
            std::mem::size_of::<RaceDesc>(),
        ),
        (
            "ship_info",
            c_layout.ship_info_offset,
            std::mem::offset_of!(RaceDesc, ship_info),
        ),
        (
            "characteristics",
            c_layout.characteristics_offset,
            std::mem::offset_of!(RaceDesc, characteristics),
        ),
        (
            "ship_data",
            c_layout.ship_data_offset,
            std::mem::offset_of!(RaceDesc, ship_data),
        ),
        (
            "ship_data.ship",
            c_layout.ship_data_ship_offset,
            std::mem::offset_of!(ShipData, ship),
        ),
        (
            "ship_info.crew_level",
            c_layout.ship_info_crew_offset,
            std::mem::offset_of!(ShipInfo, crew_level),
        ),
        (
            "ship_info.max_crew",
            c_layout.ship_info_max_crew_offset,
            std::mem::offset_of!(ShipInfo, max_crew),
        ),
        (
            "characteristics.ship_mass",
            c_layout.characteristics_mass_offset,
            std::mem::offset_of!(Characteristics, ship_mass),
        ),
    ]
    .into_iter()
    .filter(|(_, native, owned)| native != owned)
    .map(|(name, native, owned)| format!("{name}: C={native} Rust={owned}"))
    .collect();

    if mismatches.is_empty() {
        eprintln!(
            "rust_ships: RaceDesc/RACE_DESC layout verified — \
             direct cast is safe."
        );
    } else {
        // Layouts differ (expected) — switch to accessor mode.
        // Do NOT abort: accessor functions handle field access safely.
        // SAFETY: initialization runs before callbacks or concurrent access.
        unsafe {
            LAYOUT_ACCESSOR_MODE = true;
        }
        eprintln!(
            "rust_ships: RaceDesc/RACE_DESC layout mismatch detected \
             (expected — RaceDesc is not #[repr(C)]). \
             Switching to accessor-function mode.
\
             Mismatches:
  {}
\
             Accessor functions will be used for all cross-language \
             RaceDesc field access.",
            mismatches.join(
                "
  "
            )
        );
    }
}

// ===========================================================================
// Catalog FFI
// ===========================================================================

/// Loads the master ship catalog.
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
///
/// C: `BOOLEAN rust_ships_load_catalog(void);`
///
/// Returns 1 on success, 0 on failure.
pub unsafe extern "C" fn rust_ships_load_catalog() -> CBoolean {
    catch_unwind(|| match load_master_ship_list() {
        Ok(()) => 1,
        Err(_) => 0,
    })
    .unwrap_or_default()
}

/// Frees the master ship catalog.
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
///
/// C: `void rust_ships_free_catalog(void);`
pub unsafe extern "C" fn rust_ships_free_catalog() {
    let _ = catch_unwind(free_master_ship_list);
}

/// Gets the ship cost by catalog index.
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
///
/// C: `COUNT rust_ships_get_cost_by_index(COUNT index);`
///
/// Returns 0 if the index is invalid or catalog is not loaded.
pub unsafe extern "C" fn rust_ships_get_cost_by_index(index: CCount) -> CCount {
    catch_unwind(|| get_ship_cost_from_index(index as usize).unwrap_or_default())
        .unwrap_or_default()
}

// ===========================================================================
// Loader FFI
// ===========================================================================

/// Loads a ship descriptor at the specified tier.
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
///
/// C: `RACE_DESC *rust_ships_load(SPECIES_ID species, BOOLEAN battle_ready);`
///
/// Returns a Rust-owned `*mut CRaceDesc` on success, or null on failure.
/// Caller must free with `rust_ships_free()`.
pub unsafe extern "C" fn rust_ships_load(
    species: CSpeciesId,
    battle_ready: CBoolean,
) -> *mut CRaceDesc {
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
pub unsafe extern "C" fn rust_ships_free(
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

/// Allocate, clear and append a ship record in the initialized queue.
///
/// # Safety
/// The queue must own STARSHIP or SHIP_FRAGMENT slots and be exclusively used
/// on the game thread. Returned records remain valid until queue teardown.
pub unsafe fn build_ship(queue: &mut Queue, species: CSpeciesId) -> HStarship {
    let handle = AllocLink(queue);
    if handle.is_null() {
        return handle;
    }
    ptr::write_bytes(handle.cast::<u8>(), 0, usize::from(queue.object_size));
    (*handle.cast::<CStarship>()).species_id = species;
    PutQueue(queue, handle);
    handle
}

/// Clones a ship fragment from source to destination.
///
/// C: `BOOLEAN rust_ships_clone_fragment(const SHIP_FRAGMENT *src, SHIP_FRAGMENT *dst);`
///
/// Returns 1 on success, 0 on failure.
///
/// # Safety
/// Both `src` and `dst` must be valid pointers to C-owned `SHIP_FRAGMENT` structs.
pub unsafe extern "C" fn rust_ships_clone_fragment(
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
pub unsafe extern "C" fn rust_ships_spawn(starship: *mut std::os::raw::c_void) -> CBoolean {
    catch_unwind(|| {
        if starship.is_null() {
            return 0;
        }

        // Safety: we trust C to pass a valid pointer
        unsafe {
            let starship_c = &mut *(starship as *mut CStarship);

            #[cfg(debug_assertions)]
            eprintln!(
                "LIFECYCLE: rust_ships_spawn entry (species={}, player={})",
                starship_c.species_id, starship_c.player_nr
            );

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
            let activity = (crate::mainloop::ffi::get_current_activity().0 & 0xFF) as u8;

            match lifecycle_spawn(&mut starship_rust, activity) {
                Ok(_) => {
                    // Extract race_desc_ptr — the only CStarship mutation before
                    // the C helper call (H3 rollback contract).
                    let race_desc_ptr = match starship_rust.race_desc {
                        Some(desc) => Box::into_raw(desc) as *mut c_void,
                        None => return 0,
                    };
                    starship_c.race_desc_ptr = race_desc_ptr;

                    // Call C helper to create the ELEMENT.
                    // C helper reads starship_c.hShip to decide fresh alloc vs reuse.
                    // We do NOT modify hShip before this call.
                    #[cfg(not(test))]
                    {
                        use crate::ships::ffi_contract::rust_bridge_spawn_element;

                        // Get ship mass from descriptor for element creation.
                        // Safe: race_desc_ptr is a valid *mut RaceDesc from Box::into_raw.
                        let ship_mass = (*(race_desc_ptr as *const RaceDesc))
                            .characteristics
                            .ship_mass;

                        let element_ok = rust_bridge_spawn_element(
                            starship as *mut CStarship,
                            race_desc_ptr,
                            ship_mass,
                            activity,
                        );
                        if element_ok == 0 {
                            // H3: Element creation failed — rollback.
                            // Free the descriptor we just allocated.
                            let _desc = Box::from_raw(race_desc_ptr as *mut RaceDesc);
                            // Null the pointer so C doesn't see a dangling ref.
                            starship_c.race_desc_ptr = ptr::null_mut();
                            // Do NOT write back counters/flags — CStarship
                            // remains in its pre-spawn state.
                            return 0;
                        }

                        // C helper succeeded. hShip and ShipFacing are already
                        // set by the C helper via the pointer we passed.
                    }

                    // Post-helper success: write back cleared counters/flags.
                    // These are safe to write because the element exists.
                    starship_c.ship_input_state = starship_rust.ship_input_state;
                    starship_c.cur_status_flags = starship_rust.cur_status_flags.0;
                    starship_c.old_status_flags = starship_rust.old_status_flags.0;
                    starship_c.energy_counter = starship_rust.energy_counter;
                    starship_c.weapon_counter = starship_rust.weapon_counter;
                    starship_c.special_counter = starship_rust.special_counter;

                    #[cfg(debug_assertions)]
                    eprintln!(
                        "LIFECYCLE: rust_ships_spawn exit OK (species={}, player={})",
                        starship_c.species_id, starship_c.player_nr
                    );

                    1
                }
                Err(_error) => {
                    #[cfg(debug_assertions)]
                    eprintln!("LIFECYCLE: rust_ships_spawn FAILED: {:?}", _error);
                    0
                }
            }
        }
    })
    .unwrap_or_default()
}

/// Initializes the battle subsystem.
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
///
/// C: `COUNT rust_ships_init(void);`
///
/// Returns the number of players (2 for battle, 1 for hyperspace) on success,
/// 0 on failure.
pub unsafe extern "C" fn rust_ships_init() -> CCount {
    #[cfg(debug_assertions)]
    eprintln!("LIFECYCLE: rust_ships_init entry");

    let result = catch_unwind(|| {
        #[cfg(test)]
        {
            use crate::ships::lifecycle::init_ships;
            let activity = 2u8;
            match init_ships(activity) {
                Ok(num_players) => num_players as CCount,
                Err(_) => 0,
            }
        }

        #[cfg(not(test))]
        unsafe {
            use crate::ships::ffi_contract::rust_bridge_init_battle_arena;

            // P05: One-time layout verification — logs result, does not abort.
            // Sets LAYOUT_ACCESSOR_MODE if layouts differ (expected).
            // Must run before any spawn.
            static LAYOUT_VERIFIED: std::sync::Once = std::sync::Once::new();
            LAYOUT_VERIFIED.call_once(|| {
                verify_race_desc_layout();
            });

            // Delegate arena setup entirely to C — this calls the original
            // InitShips() body which handles InitSpace, display list, galaxy,
            // asteroids, planets, hyperspace setup, etc.
            let num_ships = rust_bridge_init_battle_arena();
            if num_ships <= 0 {
                #[cfg(debug_assertions)]
                eprintln!(
                    "LIFECYCLE: rust_ships_init FAILED (arena returned {})",
                    num_ships
                );
                return 0;
            }

            // Track initialization state on the Rust side
            super::lifecycle::mark_ships_initialized();

            #[cfg(debug_assertions)]
            eprintln!("LIFECYCLE: rust_ships_init exit (num_ships={})", num_ships);

            num_ships as CCount
        }
    })
    .unwrap_or_else(|_| {
        #[cfg(debug_assertions)]
        eprintln!("LIFECYCLE: rust_ships_init PANICKED");
        0
    });

    result
}

/// Uninitializes the battle subsystem.
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
///
/// C: `void rust_ships_uninit(void);`
///
/// Performs complete battle teardown:
/// - Stops audio (StopSound)
/// - Frees space resources (UninitSpace — explosions, blasts, asteroids)
/// - Counts floating crew in display list
/// - Distributes floating crew to survivor's descriptor
/// - Writes back descriptor crew → starship.crew_level
/// - Frees each spawned ship's descriptor via free_ship() → rust_ships_free()
/// - Clears IN_BATTLE from CurrentActivity
/// - Reinits queues for non-IN_ENCOUNTER
///
/// Idempotent: calling twice is safe (idempotence guard checks Rust + C state).
pub unsafe extern "C" fn rust_ships_uninit() {
    #[cfg(debug_assertions)]
    eprintln!("LIFECYCLE: rust_ships_uninit entry");

    let _ = catch_unwind(|| {
        #[cfg(test)]
        {
            // Test mode: defensive catalog cleanup (no C arena to tear down)
            free_master_ship_list();
            super::lifecycle::mark_ships_uninitialized();
        }

        #[cfg(not(test))]
        unsafe {
            use crate::ships::ffi_contract::rust_bridge_uninit_ships;

            // H2: Reconcile Rust-side state with C-side state before
            // deciding whether to skip teardown. The Rust flag alone
            // is not authoritative — it can desync from C arena state
            // in failure/partial-init paths.
            let rust_says_initialized = super::lifecycle::is_ships_initialized_for_uninit();

            if !rust_says_initialized {
                // Check C-side: does CurrentActivity still have IN_BATTLE?
                let c_activity = (crate::mainloop::ffi::get_current_activity().0 & 0xFF) as u8;
                // IN_ENCOUNTER is 2, which implies battle context may exist
                let c_might_have_arena = c_activity == 2;

                if c_might_have_arena {
                    // H2: Desync detected — Rust says uninitialized but C
                    // may still have arena resources. Proceed with teardown
                    // to prevent resource leak. C state is authoritative.
                    #[cfg(debug_assertions)]
                    eprintln!(
                        "rust_ships_uninit: WARNING desync detected — \
                         Rust says uninitialized but C activity={:#x} \
                         suggests arena may exist. Proceeding with teardown.",
                        c_activity
                    );
                    // Fall through to teardown below
                } else {
                    // Both Rust and C agree: no arena to tear down.
                    #[cfg(debug_assertions)]
                    eprintln!(
                        "rust_ships_uninit: idempotence guard fired \
                         (Rust=uninitialized, C activity={:#x})",
                        c_activity
                    );
                    return;
                }
            }

            #[cfg(debug_assertions)]
            eprintln!("rust_ships_uninit: beginning teardown");

            // Delegate full teardown to C helper.
            // This performs:
            //   - StopSound()
            //   - UninitSpace() (free explosion/blast/asteroid resources)
            //   - CountCrewElements() (count floating crew in display list)
            //   - Iterate display list: add floating crew to survivor
            //   - Write back descriptor crew -> StarShipPtr->crew_level
            //   - free_ship() for each spawned ship (calls rust_ships_free)
            //   - Clear IN_BATTLE from CurrentActivity
            //   - UpdateShipFragCrew() for IN_ENCOUNTER
            //   - ReinitQueue / FreeHyperspace for non-IN_ENCOUNTER
            //
            // C-side null guards (C3): The C helper guards against null
            // StarShipPtr and null RaceDescPtr on each element, so
            // partial-init states are handled safely.
            rust_bridge_uninit_ships();

            // Update Rust-side state tracking
            super::lifecycle::mark_ships_uninitialized();

            #[cfg(debug_assertions)]
            {
                assert!(
                    !super::lifecycle::is_ships_initialized_for_uninit(),
                    "ships_initialized should be false after uninit"
                );
                eprintln!("rust_ships_uninit: teardown complete, state cleared");
            }
        }
    });
}

// ===========================================================================
// RaceDesc Accessor Functions (P05)
// ===========================================================================
//
// These functions allow C to read/write RaceDesc fields without depending on
// struct layout compatibility. Since Rust's RaceDesc is NOT #[repr(C)] and
// owns Rust metadata rather than native callback pointers, direct struct casting is NOT safe. These accessors are the
// correct path for all cross-language field access.
//
// C calls these via declarations in rust_bridge_ships.h.

/// C calls this to get `ship_data.ship` frame array pointer.
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
///
/// Returns a pointer to the first element of the `ship` array,
/// which C treats as `FRAME *` (i.e., `FRAME_DESC **`).
///
/// C: `void *rust_race_desc_get_ship_frames(const void *rd);`
pub unsafe extern "C" fn rust_race_desc_get_ship_frames(rd: *const c_void) -> *mut c_void {
    if rd.is_null() {
        return ptr::null_mut();
    }
    let desc = &*(rd as *const RaceDesc);
    desc.ship_data.ship.as_ptr() as *mut c_void
}

/// C calls this to get `characteristics.ship_mass`.
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
///
/// C: `BYTE rust_race_desc_get_ship_mass(const void *rd);`
pub unsafe extern "C" fn rust_race_desc_get_ship_mass(rd: *const c_void) -> CByte {
    if rd.is_null() {
        return 0;
    }
    let desc = &*(rd as *const RaceDesc);
    desc.characteristics.ship_mass
}

/// C calls this to get `ship_info.crew_level`.
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
///
/// C: `COUNT rust_race_desc_get_crew_level(const void *rd);`
pub unsafe extern "C" fn rust_race_desc_get_crew_level(rd: *const c_void) -> CCount {
    if rd.is_null() {
        return 0;
    }
    let desc = &*(rd as *const RaceDesc);
    desc.ship_info.crew_level
}

/// C calls this to set `ship_info.crew_level` (for crew writeback during uninit).
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
///
/// C: `void rust_race_desc_set_crew_level(void *rd, COUNT crew);`
pub unsafe extern "C" fn rust_race_desc_set_crew_level(rd: *mut c_void, crew: CCount) {
    if rd.is_null() {
        return;
    }
    let desc = &mut *(rd as *mut RaceDesc);
    desc.ship_info.crew_level = crew;
}

/// C calls this to get `ship_info.max_crew`.
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
///
/// C: `COUNT rust_race_desc_get_max_crew(const void *rd);`
pub unsafe extern "C" fn rust_race_desc_get_max_crew(rd: *const c_void) -> CCount {
    if rd.is_null() {
        return 0;
    }
    let desc = &*(rd as *const RaceDesc);
    desc.ship_info.max_crew
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    #[serial]
    fn test_catalog_load_free() {
        unsafe {
            // Ensure clean state
            rust_ships_free_catalog();

            let result = rust_ships_load_catalog();
            assert_eq!(result, 1);

            rust_ships_free_catalog();
        }
    }

    #[test]
    #[serial]
    fn test_get_cost_by_index() {
        unsafe {
            rust_ships_free_catalog();
            rust_ships_load_catalog();

            // Index 0 should be valid (first melee ship after sorting)
            let cost = rust_ships_get_cost_by_index(0);
            assert!(cost > 0);

            // Invalid index should return 0
            let cost = rust_ships_get_cost_by_index(9999);
            assert_eq!(cost, 0);

            rust_ships_free_catalog();
        }
    }

    #[test]
    fn test_load_free_ship() {
        unsafe {
            // Load VUX (species 11) metadata-only
            let desc = rust_ships_load(11, 0);
            assert!(!desc.is_null());

            // Free it
            rust_ships_free(desc, 1, 1);
        }
    }

    #[test]
    fn test_load_free_ship_battle_ready() {
        unsafe {
            // Load VUX (species 11) battle-ready
            let desc = rust_ships_load(11, 1);
            assert!(!desc.is_null());

            // Free it
            rust_ships_free(desc, 1, 1);
        }
    }

    #[test]
    fn test_load_ship_invalid_species() {
        unsafe {
            let desc = rust_ships_load(999, 0);
            assert!(desc.is_null());
        }
    }

    #[test]
    #[serial]
    fn test_init_uninit_ships() {
        unsafe {
            let result = rust_ships_init();
            assert_eq!(result, 2); // NUM_PLAYERS

            rust_ships_uninit();
        }
    }

    #[test]
    fn test_null_pointer_safety() {
        unsafe {
            // All functions should handle null pointers gracefully
            rust_ships_free(ptr::null_mut(), 0, 0);
            assert_eq!(rust_ships_spawn(ptr::null_mut()), 0);
            assert_eq!(
                rust_ships_clone_fragment(ptr::null_mut(), ptr::null_mut()),
                0
            );

            // These should not crash
        }
    }

    #[test]
    #[serial]
    fn full_lifecycle_roundtrip() {
        unsafe {
            // Ensure clean starting state
            super::super::lifecycle::reset_battle_state();
            assert!(!super::super::lifecycle::is_ships_initialized());

            // Init
            let result = rust_ships_init();
            assert!(
                result > 0,
                "init should return positive ship count, got {}",
                result
            );
            assert!(super::super::lifecycle::is_ships_initialized());

            // Uninit
            rust_ships_uninit();
            assert!(!super::super::lifecycle::is_ships_initialized());

            // Re-init (multi-battle scenario)
            let result2 = rust_ships_init();
            assert!(
                result2 > 0,
                "re-init should return positive ship count, got {}",
                result2
            );
            assert!(super::super::lifecycle::is_ships_initialized());

            // Final uninit
            rust_ships_uninit();
            assert!(!super::super::lifecycle::is_ships_initialized());
        }
    }

    #[test]
    #[serial]
    fn uninit_without_init_is_safe() {
        unsafe {
            // Ensure ships are not initialized
            super::super::lifecycle::reset_battle_state();
            assert!(!super::super::lifecycle::is_ships_initialized());

            // This should be a no-op — no crash
            rust_ships_uninit();
            assert!(!super::super::lifecycle::is_ships_initialized());
        }
    }

    #[test]
    #[serial]
    fn double_uninit_is_idempotent() {
        unsafe {
            // Ensure clean starting state
            super::super::lifecycle::reset_battle_state();

            // Init
            let result = rust_ships_init();
            assert!(result > 0);
            assert!(super::super::lifecycle::is_ships_initialized());

            // First uninit
            rust_ships_uninit();
            assert!(!super::super::lifecycle::is_ships_initialized());

            // Second uninit — should be no-op
            rust_ships_uninit();
            assert!(!super::super::lifecycle::is_ships_initialized());
        }
    }
}
