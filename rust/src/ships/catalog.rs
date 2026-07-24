// Master Ship Catalog
// @plan PLAN-20260314-SHIPS.P06
// @requirement REQ-CATALOG, REQ-CATALOG-SORT, REQ-CATALOG-STARTUP, REQ-CATALOG-SHUTDOWN, REQ-CATALOG-EXCLUSION, REQ-CATALOG-LOOKUP

use super::loader::{load_ship, LoadTier};
use super::types::{FleetStuff, ShipInfo, ShipsError, SpeciesId};
use std::mem;
use std::sync::Mutex;

// ---------------------------------------------------------------------------
// MasterShipInfo
// ---------------------------------------------------------------------------

/// Entry in the master ship catalog.
///
/// Each catalog entry owns its metadata handles (icons, melee_icon, race_strings).
/// These handles are transferred from the temporary descriptor during loading
/// and freed when the catalog is freed.
#[derive(Debug, Clone)]
pub struct MasterShipInfo {
    pub species_id: SpeciesId,
    pub ship_info: ShipInfo,
    pub fleet: FleetStuff,
    pub race_name: &'static str,
}

// ---------------------------------------------------------------------------
// Race Name Lookup Table
// ---------------------------------------------------------------------------

/// Returns the canonical race name for sorting and display.
///
/// These names match the string at index 2 in each race's string table.
/// Used for alphabetical sorting since actual string tables aren't loaded yet
/// (all resource IDs are currently 0).
pub fn race_name_for_species(species: SpeciesId) -> &'static str {
    match species {
        SpeciesId::Androsynth => "Andro.",
        SpeciesId::Arilou => "Arilou",
        SpeciesId::Chenjesu => "Chenje.",
        SpeciesId::Chmmr => "Chmmr",
        SpeciesId::Druuge => "Druuge",
        SpeciesId::Earthling => "Earth.",
        SpeciesId::Ilwrath => "Ilwrath",
        SpeciesId::KohrAh => "Kohr-Ah",
        SpeciesId::Melnorme => "Melnorme",
        SpeciesId::Mmrnmhrm => "Mmrn.",
        SpeciesId::Mycon => "Mycon",
        SpeciesId::Orz => "Orz",
        SpeciesId::Pkunk => "Pkunk",
        SpeciesId::Shofixti => "Shofix.",
        SpeciesId::Slylandro => "Slylan.",
        SpeciesId::Spathi => "Spathi",
        SpeciesId::Supox => "Supox",
        SpeciesId::Syreen => "Syreen",
        SpeciesId::Thraddash => "Thradd.",
        SpeciesId::Umgah => "Umgah",
        SpeciesId::UrQuan => "Ur-Quan",
        SpeciesId::Utwig => "Utwig",
        SpeciesId::Vux => "VUX",
        SpeciesId::Yehat => "Yehat",
        SpeciesId::Zoqfotpik => "ZoqFot",
        _ => "(Unknown)",
    }
}

// ---------------------------------------------------------------------------
// Global Catalog Storage
// ---------------------------------------------------------------------------

static MASTER_CATALOG: Mutex<Option<Vec<MasterShipInfo>>> = Mutex::new(None);

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Loads the master ship catalog with all melee-eligible ships.
///
/// Iterates all 25 melee-eligible species (Arilou through Mmrnmhrm),
/// loads metadata-only descriptors, transfers handle ownership to catalog entries,
/// and sorts by race name.
///
/// # Ownership Model
/// For each species:
/// 1. Load descriptor via `load_ship(species, MetadataOnly)`
/// 2. Transfer ship_info and fleet via `mem::replace` (atomically moves data
///    and zeroes the source descriptor's fields)
/// 3. Drop the descriptor (safe because handles have been moved out)
/// 4. MasterShipInfo now owns the handles and will free them when catalog is freed
///
/// Since all resource IDs are currently 0, all handles will be 0 (null).
/// But the code is structurally correct for when real resources exist.
///
/// # Errors
/// Returns `ShipsError::AlreadyInitialized` if catalog is already loaded.
/// Returns `ShipsError::LoadFailed` if any species fails to load.
pub fn load_master_ship_list() -> Result<(), ShipsError> {
    let mut catalog_guard = MASTER_CATALOG
        .lock()
        .map_err(|_| ShipsError::InvalidState("master catalog mutex poisoned".into()))?;

    if catalog_guard.is_some() {
        return Err(ShipsError::AlreadyInitialized);
    }

    let mut entries = Vec::with_capacity(25);

    // All melee-eligible species (SpeciesId::Arilou = 1 through SpeciesId::Mmrnmhrm = 25)
    for species_val in SpeciesId::Arilou as i32..=SpeciesId::LAST_MELEE_ID {
        let species = SpeciesId::from_i32(species_val)
            .ok_or_else(|| ShipsError::LoadFailed(format!("Invalid species ID {}", species_val)))?;

        let mut desc = load_ship(species, LoadTier::MetadataOnly)?;

        // Transfer ownership of metadata handles using mem::replace.
        // This structurally ensures the source descriptor's handles are zeroed
        // before it is dropped, preventing any double-free.
        let ship_info = mem::take(&mut desc.ship_info);
        let fleet = mem::take(&mut desc.fleet);

        entries.push(MasterShipInfo {
            species_id: species,
            ship_info,
            fleet,
            race_name: race_name_for_species(species),
        });
    }

    // Sort by race name (C master.c sorts by the race name string at index 2)
    entries.sort_by(|a, b| a.race_name.cmp(b.race_name));

    *catalog_guard = Some(entries);

    Ok(())
}

/// Frees the master ship catalog and all owned resources.
///
/// Frees icons, melee_icon, race_strings for each entry.
/// Safe to call multiple times (no-op if catalog not loaded).
pub fn free_master_ship_list() {
    let mut catalog_guard = match MASTER_CATALOG.lock() {
        Ok(guard) => guard,
        Err(_) => return, // Poisoned mutex — nothing we can safely do
    };

    if let Some(mut entries) = catalog_guard.take() {
        for entry in &mut entries {
            // Free the metadata handles owned by the catalog entry
            if entry.ship_info.icons != 0 {
                super::c_bridge::free_graphic(entry.ship_info.icons);
                entry.ship_info.icons = 0;
            }

            if entry.ship_info.melee_icon != 0 {
                super::c_bridge::free_graphic(entry.ship_info.melee_icon);
                entry.ship_info.melee_icon = 0;
            }

            if entry.ship_info.race_strings != 0 {
                super::c_bridge::free_string_table(entry.ship_info.race_strings);
                entry.ship_info.race_strings = 0;
            }
        }
    }
}

/// Returns the catalog index for the given species.
///
/// # Errors
/// Returns `None` if catalog not loaded or species not found.
pub fn find_master_ship(species_id: SpeciesId) -> Option<usize> {
    with_catalog(|entries| {
        entries
            .iter()
            .position(|entry| entry.species_id == species_id)
    })
    .ok()
    .flatten()
}

/// Returns the ship cost for the given catalog index.
///
/// # Errors
/// Returns `None` if catalog not loaded or index out of bounds.
pub fn get_ship_cost_from_index(index: usize) -> Option<u16> {
    with_catalog(|entries| entries.get(index).map(|e| e.ship_info.ship_cost as u16))
        .ok()
        .flatten()
}

/// Returns the icons handle for the given catalog index.
///
/// # Errors
/// Returns `None` if catalog not loaded or index out of bounds.
pub fn get_ship_icons_from_index(index: usize) -> Option<usize> {
    with_catalog(|entries| entries.get(index).map(|e| e.ship_info.icons))
        .ok()
        .flatten()
}

/// Returns the melee icons handle for the given catalog index.
///
/// # Errors
/// Returns `None` if catalog not loaded or index out of bounds.
pub fn get_ship_melee_icons_from_index(index: usize) -> Option<usize> {
    with_catalog(|entries| entries.get(index).map(|e| e.ship_info.melee_icon))
        .ok()
        .flatten()
}

/// Returns whether the catalog is currently loaded.
pub fn is_catalog_loaded() -> bool {
    MASTER_CATALOG
        .lock()
        .map(|guard| guard.is_some())
        .unwrap_or(false)
}

/// Returns the number of entries in the catalog.
///
/// Returns 0 if catalog not loaded.
pub fn catalog_count() -> usize {
    with_catalog(|entries| entries.len()).unwrap_or(0)
}

/// Accesses a single catalog entry by index via callback.
///
/// Returns `None` if catalog not loaded or index out of bounds.
pub fn with_master_ship_by_index<T>(
    index: usize,
    f: impl FnOnce(&MasterShipInfo) -> T,
) -> Option<T> {
    with_catalog(|entries| entries.get(index).map(f))
        .ok()
        .flatten()
}

/// Accesses the catalog with a callback.
///
/// # Errors
/// Returns `ShipsError::NotInitialized` if catalog not loaded.
pub fn with_catalog<F, T>(f: F) -> Result<T, ShipsError>
where
    F: FnOnce(&[MasterShipInfo]) -> T,
{
    let catalog_guard = MASTER_CATALOG
        .lock()
        .map_err(|_| ShipsError::InvalidState("master catalog mutex poisoned".into()))?;

    match catalog_guard.as_ref() {
        Some(entries) => Ok(f(entries)),
        None => Err(ShipsError::NotInitialized),
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    // -- Helper for clean test state ----------------------------------------

    fn reset_catalog() {
        free_master_ship_list();
    }

    // -- Race name lookup tests ---------------------------------------------

    #[test]
    fn race_name_for_all_melee_species() {
        assert_eq!(race_name_for_species(SpeciesId::Arilou), "Arilou");
        assert_eq!(race_name_for_species(SpeciesId::Androsynth), "Andro.");
        assert_eq!(race_name_for_species(SpeciesId::Chenjesu), "Chenje.");
        assert_eq!(race_name_for_species(SpeciesId::Chmmr), "Chmmr");
        assert_eq!(race_name_for_species(SpeciesId::Druuge), "Druuge");
        assert_eq!(race_name_for_species(SpeciesId::Earthling), "Earth.");
        assert_eq!(race_name_for_species(SpeciesId::Ilwrath), "Ilwrath");
        assert_eq!(race_name_for_species(SpeciesId::KohrAh), "Kohr-Ah");
        assert_eq!(race_name_for_species(SpeciesId::Melnorme), "Melnorme");
        assert_eq!(race_name_for_species(SpeciesId::Mmrnmhrm), "Mmrn.");
        assert_eq!(race_name_for_species(SpeciesId::Mycon), "Mycon");
        assert_eq!(race_name_for_species(SpeciesId::Orz), "Orz");
        assert_eq!(race_name_for_species(SpeciesId::Pkunk), "Pkunk");
        assert_eq!(race_name_for_species(SpeciesId::Shofixti), "Shofix.");
        assert_eq!(race_name_for_species(SpeciesId::Slylandro), "Slylan.");
        assert_eq!(race_name_for_species(SpeciesId::Spathi), "Spathi");
        assert_eq!(race_name_for_species(SpeciesId::Supox), "Supox");
        assert_eq!(race_name_for_species(SpeciesId::Syreen), "Syreen");
        assert_eq!(race_name_for_species(SpeciesId::Thraddash), "Thradd.");
        assert_eq!(race_name_for_species(SpeciesId::Umgah), "Umgah");
        assert_eq!(race_name_for_species(SpeciesId::UrQuan), "Ur-Quan");
        assert_eq!(race_name_for_species(SpeciesId::Utwig), "Utwig");
        assert_eq!(race_name_for_species(SpeciesId::Vux), "VUX");
        assert_eq!(race_name_for_species(SpeciesId::Yehat), "Yehat");
        assert_eq!(race_name_for_species(SpeciesId::Zoqfotpik), "ZoqFot");
    }

    #[test]
    fn race_name_for_non_melee_returns_unknown() {
        assert_eq!(race_name_for_species(SpeciesId::SisShip), "(Unknown)");
        assert_eq!(race_name_for_species(SpeciesId::SaMatra), "(Unknown)");
        assert_eq!(race_name_for_species(SpeciesId::UrQuanProbe), "(Unknown)");
        assert_eq!(race_name_for_species(SpeciesId::NoId), "(Unknown)");
    }

    // -- Catalog lifecycle tests --------------------------------------------

    #[test]
    #[serial]
    fn catalog_initially_not_loaded() {
        reset_catalog();
        assert!(!is_catalog_loaded());
        assert_eq!(catalog_count(), 0);
    }

    #[test]
    #[serial]
    fn load_master_ship_list_succeeds() {
        reset_catalog();
        let result = load_master_ship_list();
        assert!(result.is_ok());
        assert!(is_catalog_loaded());
        assert_eq!(catalog_count(), 25);
        reset_catalog();
    }

    #[test]
    #[serial]
    fn load_master_ship_list_twice_returns_error() {
        reset_catalog();
        load_master_ship_list().unwrap();

        let result = load_master_ship_list();
        assert!(result.is_err());
        match result {
            Err(ShipsError::AlreadyInitialized) => {}
            _ => panic!("Expected AlreadyInitialized error"),
        }

        reset_catalog();
    }

    #[test]
    #[serial]
    fn free_master_ship_list_clears_catalog() {
        reset_catalog();
        load_master_ship_list().unwrap();
        assert!(is_catalog_loaded());

        free_master_ship_list();
        assert!(!is_catalog_loaded());
        assert_eq!(catalog_count(), 0);
    }

    #[test]
    #[serial]
    fn free_master_ship_list_safe_when_not_loaded() {
        reset_catalog();
        free_master_ship_list(); // Should not panic
        assert!(!is_catalog_loaded());
    }

    #[test]
    #[serial]
    fn free_master_ship_list_safe_multiple_calls() {
        reset_catalog();
        load_master_ship_list().unwrap();
        free_master_ship_list();
        free_master_ship_list(); // Second call should be safe
        assert!(!is_catalog_loaded());
    }

    // -- Catalog content tests ----------------------------------------------

    #[test]
    #[serial]
    fn catalog_contains_exactly_25_melee_species() {
        reset_catalog();
        load_master_ship_list().unwrap();

        let count = with_catalog(|entries| entries.len()).unwrap();
        assert_eq!(count, 25);

        reset_catalog();
    }

    #[test]
    #[serial]
    fn catalog_excludes_non_melee_ships() {
        reset_catalog();
        load_master_ship_list().unwrap();

        assert!(find_master_ship(SpeciesId::SisShip).is_none());
        assert!(find_master_ship(SpeciesId::SaMatra).is_none());
        assert!(find_master_ship(SpeciesId::UrQuanProbe).is_none());

        reset_catalog();
    }

    #[test]
    #[serial]
    fn catalog_is_sorted_by_race_name() {
        reset_catalog();
        load_master_ship_list().unwrap();

        let names = with_catalog(|entries| entries.iter().map(|e| e.race_name).collect::<Vec<_>>())
            .unwrap();

        // Expected sorted order
        let expected = vec![
            "Andro.", "Arilou", "Chenje.", "Chmmr", "Druuge", "Earth.", "Ilwrath", "Kohr-Ah",
            "Melnorme", "Mmrn.", "Mycon", "Orz", "Pkunk", "Shofix.", "Slylan.", "Spathi", "Supox",
            "Syreen", "Thradd.", "Umgah", "Ur-Quan", "Utwig", "VUX", "Yehat", "ZoqFot",
        ];

        assert_eq!(names, expected);

        reset_catalog();
    }

    // -- Lookup tests -------------------------------------------------------

    #[test]
    #[serial]
    fn find_master_ship_returns_correct_index() {
        reset_catalog();
        load_master_ship_list().unwrap();

        // Arilou should be at index 1 (after Andro. at 0)
        let idx = find_master_ship(SpeciesId::Arilou);
        assert_eq!(idx, Some(1));

        // Verify it's actually Arilou
        let verified = with_catalog(|entries| entries.get(1).map(|e| e.species_id)).unwrap();
        assert_eq!(verified, Some(SpeciesId::Arilou));

        reset_catalog();
    }

    #[test]
    #[serial]
    fn find_master_ship_invalid_species_returns_none() {
        reset_catalog();
        load_master_ship_list().unwrap();

        assert!(find_master_ship(SpeciesId::NoId).is_none());
        assert!(find_master_ship(SpeciesId::SisShip).is_none());

        reset_catalog();
    }

    #[test]
    #[serial]
    fn find_master_ship_when_not_loaded_returns_none() {
        reset_catalog();
        assert!(find_master_ship(SpeciesId::Arilou).is_none());
    }

    #[test]
    #[serial]
    fn get_ship_cost_from_index_returns_correct_value() {
        reset_catalog();
        load_master_ship_list().unwrap();

        // Find Arilou's index
        let idx = find_master_ship(SpeciesId::Arilou).unwrap();
        let cost = get_ship_cost_from_index(idx);

        // Arilou's cost is 16 (from registry.rs template)
        assert_eq!(cost, Some(16));

        reset_catalog();
    }

    #[test]
    #[serial]
    fn get_ship_cost_from_index_out_of_bounds_returns_none() {
        reset_catalog();
        load_master_ship_list().unwrap();

        assert!(get_ship_cost_from_index(999).is_none());

        reset_catalog();
    }

    #[test]
    #[serial]
    fn get_ship_cost_from_index_when_not_loaded_returns_none() {
        reset_catalog();
        assert!(get_ship_cost_from_index(0).is_none());
    }

    #[test]
    #[serial]
    fn get_ship_icons_from_index_returns_handle() {
        reset_catalog();
        load_master_ship_list().unwrap();

        let idx = find_master_ship(SpeciesId::Chmmr).unwrap();
        let handle = get_ship_icons_from_index(idx);

        // Currently all resource IDs are 0, so handles will be 0
        assert_eq!(handle, Some(0));

        reset_catalog();
    }

    #[test]
    #[serial]
    fn get_ship_icons_from_index_out_of_bounds_returns_none() {
        reset_catalog();
        load_master_ship_list().unwrap();

        assert!(get_ship_icons_from_index(999).is_none());

        reset_catalog();
    }

    #[test]
    #[serial]
    fn get_ship_melee_icons_from_index_returns_handle() {
        reset_catalog();
        load_master_ship_list().unwrap();

        let idx = find_master_ship(SpeciesId::Earthling).unwrap();
        let handle = get_ship_melee_icons_from_index(idx);

        // Currently all resource IDs are 0, so handles will be 0
        assert_eq!(handle, Some(0));

        reset_catalog();
    }

    #[test]
    #[serial]
    fn get_ship_melee_icons_from_index_out_of_bounds_returns_none() {
        reset_catalog();
        load_master_ship_list().unwrap();

        assert!(get_ship_melee_icons_from_index(999).is_none());

        reset_catalog();
    }

    // -- with_catalog tests -------------------------------------------------

    #[test]
    #[serial]
    fn with_catalog_allows_complex_access() {
        reset_catalog();
        load_master_ship_list().unwrap();

        let result = with_catalog(|entries| {
            entries
                .iter()
                .filter(|e| e.ship_info.ship_cost > 20)
                .count()
        });

        assert!(result.is_ok());
        // Several ships have cost > 20
        assert!(result.unwrap() > 0);

        reset_catalog();
    }

    #[test]
    #[serial]
    fn with_catalog_when_not_loaded_returns_error() {
        reset_catalog();

        let result = with_catalog(|entries| entries.len());
        assert!(result.is_err());
        match result {
            Err(ShipsError::NotInitialized) => {}
            _ => panic!("Expected NotInitialized error"),
        }
    }

    // -- Handle ownership tests ---------------------------------------------

    #[test]
    #[serial]
    fn catalog_owns_metadata_handles() {
        use crate::ships::c_bridge::{mock_allocated_count, mock_reset};

        reset_catalog();
        mock_reset();

        // Load catalog (currently all resource IDs are 0, so no actual allocations)
        load_master_ship_list().unwrap();

        // With real resources, handles would be allocated here
        // Currently this just verifies the structure is correct
        assert_eq!(catalog_count(), 25);

        // Free catalog
        free_master_ship_list();

        // All handles should be freed (currently 0 since no real resources)
        assert_eq!(mock_allocated_count(), 0);

        mock_reset();
    }

    // -- Integration tests --------------------------------------------------

    #[test]
    #[serial]
    fn catalog_provides_all_expected_species() {
        reset_catalog();
        load_master_ship_list().unwrap();

        let melee_species = [
            SpeciesId::Arilou,
            SpeciesId::Chmmr,
            SpeciesId::Earthling,
            SpeciesId::Orz,
            SpeciesId::Pkunk,
            SpeciesId::Shofixti,
            SpeciesId::Spathi,
            SpeciesId::Supox,
            SpeciesId::Thraddash,
            SpeciesId::Utwig,
            SpeciesId::Vux,
            SpeciesId::Yehat,
            SpeciesId::Melnorme,
            SpeciesId::Druuge,
            SpeciesId::Ilwrath,
            SpeciesId::Mycon,
            SpeciesId::Slylandro,
            SpeciesId::Umgah,
            SpeciesId::UrQuan,
            SpeciesId::Zoqfotpik,
            SpeciesId::Syreen,
            SpeciesId::KohrAh,
            SpeciesId::Androsynth,
            SpeciesId::Chenjesu,
            SpeciesId::Mmrnmhrm,
        ];

        for species in &melee_species {
            let idx = find_master_ship(*species);
            assert!(idx.is_some(), "Species {:?} not found in catalog", species);
        }

        reset_catalog();
    }

    #[test]
    #[serial]
    fn with_master_ship_by_index_returns_entry() {
        reset_catalog();
        load_master_ship_list().unwrap();

        // Index 0 should be "Andro." (Androsynth)
        let species = with_master_ship_by_index(0, |e| e.species_id);
        assert_eq!(species, Some(SpeciesId::Androsynth));

        let name = with_master_ship_by_index(0, |e| e.race_name);
        assert_eq!(name, Some("Andro."));

        // Out of bounds returns None
        assert!(with_master_ship_by_index(999, |e| e.species_id).is_none());

        reset_catalog();
    }

    #[test]
    #[serial]
    fn with_master_ship_by_index_when_not_loaded_returns_none() {
        reset_catalog();
        assert!(with_master_ship_by_index(0, |e| e.species_id).is_none());
    }

    #[test]
    #[serial]
    fn catalog_lookup_by_index_matches_find() {
        reset_catalog();
        load_master_ship_list().unwrap();

        let species = SpeciesId::Spathi;
        let idx = find_master_ship(species).unwrap();

        let verified_species =
            with_catalog(|entries| entries.get(idx).map(|e| e.species_id)).unwrap();

        assert_eq!(verified_species, Some(species));

        reset_catalog();
    }

    #[test]
    #[serial]
    fn catalog_lifecycle_isolated_from_other_catalog_tests() {
        for _ in 0..64 {
            reset_catalog();
            load_master_ship_list().unwrap();
            assert_eq!(
                with_master_ship_by_index(0, |entry| entry.species_id),
                Some(SpeciesId::Androsynth)
            );
            reset_catalog();
        }
    }
}
