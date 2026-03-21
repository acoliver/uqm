// Master Ship List Management
// Delegates to ships::catalog for actual implementation

use crate::ships::catalog;
use crate::ships::types::{ShipsError, SpeciesId};

/// Error type for master ship list operations.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MasterError {
    AlreadyLoaded,
    NotLoaded,
    LoadFailed,
    NotFound,
}

impl std::fmt::Display for MasterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MasterError::AlreadyLoaded => write!(f, "Master ship list already loaded"),
            MasterError::NotLoaded => write!(f, "Master ship list not loaded"),
            MasterError::LoadFailed => write!(f, "Failed to load master ship list"),
            MasterError::NotFound => write!(f, "Ship not found in master list"),
        }
    }
}

impl std::error::Error for MasterError {}

impl From<ShipsError> for MasterError {
    fn from(e: ShipsError) -> Self {
        match e {
            ShipsError::AlreadyInitialized => MasterError::AlreadyLoaded,
            ShipsError::NotInitialized => MasterError::NotLoaded,
            _ => MasterError::LoadFailed,
        }
    }
}

/// Load the master ship list.
///
/// Delegates to `ships::catalog::load_master_ship_list()`.
pub fn load_master_ship_list() -> Result<(), MasterError> {
    catalog::load_master_ship_list().map_err(|e| e.into())
}

/// Free the master ship list.
///
/// Delegates to `ships::catalog::free_master_ship_list()`.
pub fn free_master_ship_list() -> Result<(), MasterError> {
    catalog::free_master_ship_list();
    Ok(())
}

/// Check if the master ship list is loaded.
///
/// Delegates to `ships::catalog::is_catalog_loaded()`.
pub fn is_master_ship_list_loaded() -> bool {
    catalog::is_catalog_loaded()
}

/// Find a ship by species ID and return its catalog index.
///
/// Delegates to `ships::catalog::find_master_ship()`.
pub fn find_master_ship(species_id: i32) -> Option<usize> {
    SpeciesId::from_i32(species_id).and_then(catalog::find_master_ship)
}

/// Get the number of ships in the master list.
///
/// Delegates to `ships::catalog::catalog_count()`.
pub fn get_master_ship_count() -> usize {
    catalog::catalog_count()
}

/// Get the ship cost for a given catalog index.
///
/// Delegates to `ships::catalog::get_ship_cost_from_index()`.
pub fn get_ship_cost_from_index(index: usize) -> Option<u16> {
    catalog::get_ship_cost_from_index(index)
}

/// Get the ship icons handle for a given catalog index.
///
/// Delegates to `ships::catalog::get_ship_icons_from_index()`.
pub fn get_ship_icons_from_index(index: usize) -> Option<usize> {
    catalog::get_ship_icons_from_index(index)
}

/// Get the ship melee icons handle for a given catalog index.
///
/// Delegates to `ships::catalog::get_ship_melee_icons_from_index()`.
pub fn get_ship_melee_icons_from_index(index: usize) -> Option<usize> {
    catalog::get_ship_melee_icons_from_index(index)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    // Helper to ensure clean state
    fn reset() {
        free_master_ship_list().ok();
    }

    #[test]
    #[serial]
    fn test_load_master_ship_list() {
        reset();

        assert!(!is_master_ship_list_loaded());

        load_master_ship_list().unwrap();
        assert!(is_master_ship_list_loaded());

        // Second load should fail
        assert_eq!(load_master_ship_list(), Err(MasterError::AlreadyLoaded));

        reset();
    }

    #[test]
    #[serial]
    fn test_free_master_ship_list() {
        reset();

        load_master_ship_list().unwrap();
        assert!(is_master_ship_list_loaded());

        free_master_ship_list().unwrap();
        assert!(!is_master_ship_list_loaded());
    }

    #[test]
    #[serial]
    fn test_get_master_ship_count() {
        reset();

        let count = get_master_ship_count();
        assert_eq!(count, 0);

        load_master_ship_list().unwrap();

        let count = get_master_ship_count();
        assert_eq!(count, 25); // 25 melee-eligible species

        reset();
    }

    #[test]
    #[serial]
    fn test_find_master_ship() {
        reset();

        load_master_ship_list().unwrap();

        // Arilou (species ID 1) should be in the catalog
        let idx = find_master_ship(1);
        assert!(idx.is_some());

        // Invalid species ID
        let not_found = find_master_ship(999);
        assert!(not_found.is_none());

        // Non-melee species (SisShip = 26)
        let non_melee = find_master_ship(26);
        assert!(non_melee.is_none());

        reset();
    }

    #[test]
    #[serial]
    fn test_get_ship_cost_from_index() {
        reset();

        load_master_ship_list().unwrap();

        // Find Arilou's index
        let idx = find_master_ship(1).unwrap();
        let cost = get_ship_cost_from_index(idx);

        // Arilou's cost is 16
        assert_eq!(cost, Some(16));

        // Out of bounds
        assert!(get_ship_cost_from_index(999).is_none());

        reset();
    }

    #[test]
    #[serial]
    fn test_get_ship_icons_from_index() {
        reset();

        load_master_ship_list().unwrap();

        let idx = find_master_ship(1).unwrap();
        let icons = get_ship_icons_from_index(idx);

        // Currently all resource IDs are 0, so handles will be 0
        assert_eq!(icons, Some(0));

        // Out of bounds
        assert!(get_ship_icons_from_index(999).is_none());

        reset();
    }

    #[test]
    #[serial]
    fn test_get_ship_melee_icons_from_index() {
        reset();

        load_master_ship_list().unwrap();

        let idx = find_master_ship(1).unwrap();
        let melee_icons = get_ship_melee_icons_from_index(idx);

        // Currently all resource IDs are 0, so handles will be 0
        assert_eq!(melee_icons, Some(0));

        // Out of bounds
        assert!(get_ship_melee_icons_from_index(999).is_none());

        reset();
    }

    #[test]
    fn test_master_error_display() {
        let err = MasterError::AlreadyLoaded;
        assert!(format!("{}", err).contains("already loaded"));

        let err = MasterError::NotLoaded;
        assert!(format!("{}", err).contains("not loaded"));
    }

    #[test]
    fn test_master_error_from_ships_error() {
        let err: MasterError = ShipsError::AlreadyInitialized.into();
        assert_eq!(err, MasterError::AlreadyLoaded);

        let err: MasterError = ShipsError::NotInitialized.into();
        assert_eq!(err, MasterError::NotLoaded);

        let err: MasterError = ShipsError::LoadFailed("test".to_string()).into();
        assert_eq!(err, MasterError::LoadFailed);
    }
}
