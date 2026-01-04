// Master Ship List Management
// Handles loading and management of ship data

use std::collections::HashMap;
use std::sync::Mutex;

/// Ship configuration data
#[derive(Debug, Clone)]
pub struct ShipConfig {
    pub species_id: i32,
    pub name: String,
    pub crew: u16,
    pub max_crew: u16,
    pub energy: u16,
    pub max_energy: u16,
}

impl ShipConfig {
    pub fn new(species_id: i32, name: &str) -> Self {
        ShipConfig {
            species_id,
            name: name.to_string(),
            crew: 0,
            max_crew: 12,
            energy: 20,
            max_energy: 20,
        }
    }
}

/// Master ship list containing all available ships
#[derive(Debug)]
pub struct MasterShipList {
    ships: HashMap<i32, ShipConfig>,
    loaded: bool,
}

impl MasterShipList {
    pub fn new() -> Self {
        MasterShipList {
            ships: HashMap::new(),
            loaded: false,
        }
    }

    /// Load the master ship list
    pub fn load(&mut self) -> Result<(), MasterError> {
        // In a real implementation, this would:
        // - Load ship data from disk
        // - Parse the master ship configuration
        // - Initialize all ship entries

        // For now, we'll add some placeholder ships
        self.add_ship(ShipConfig::new(0, "VUX Intruder"));
        self.add_ship(ShipConfig::new(1, "Chmmr Avatar"));
        self.add_ship(ShipConfig::new(2, "Ur-Quan Dreadnought"));
        self.add_ship(ShipConfig::new(3, "Earthling Cruiser"));

        self.loaded = true;
        Ok(())
    }

    /// Free the master ship list
    pub fn free(&mut self) {
        self.ships.clear();
        self.loaded = false;
    }

    /// Check if the list is loaded
    pub fn is_loaded(&self) -> bool {
        self.loaded
    }

    /// Add a ship configuration
    pub fn add_ship(&mut self, ship: ShipConfig) {
        self.ships.insert(ship.species_id, ship);
    }

    /// Get a ship by species ID
    pub fn get_ship(&self, species_id: i32) -> Option<&ShipConfig> {
        self.ships.get(&species_id)
    }

    /// Get all ship IDs
    pub fn get_ship_ids(&self) -> Vec<i32> {
        self.ships.keys().copied().collect()
    }

    /// Get the number of ships
    pub fn count(&self) -> usize {
        self.ships.len()
    }

    /// Check if a ship exists
    pub fn has_ship(&self, species_id: i32) -> bool {
        self.ships.contains_key(&species_id)
    }
}

impl Default for MasterShipList {
    fn default() -> Self {
        Self::new()
    }
}

/// Global master ship list
static GLOBAL_MASTER_LIST: Mutex<Option<MasterShipList>> = Mutex::new(None);

/// Load the master ship list
pub fn load_master_ship_list() -> Result<(), MasterError> {
    let mut master = GLOBAL_MASTER_LIST.lock().unwrap();

    if master.is_none() {
        *master = Some(MasterShipList::new());
    }

    if let Some(ref mut m) = *master {
        if m.is_loaded() {
            return Err(MasterError::AlreadyLoaded);
        }
        m.load()?;
    }

    Ok(())
}

/// Free the master ship list
pub fn free_master_ship_list() -> Result<(), MasterError> {
    let mut master = GLOBAL_MASTER_LIST.lock().unwrap();

    if let Some(ref mut m) = *master {
        m.free();
    }

    Ok(())
}

/// Check if the master ship list is loaded
pub fn is_master_ship_list_loaded() -> bool {
    let master = GLOBAL_MASTER_LIST.lock().unwrap();
    master.as_ref().map(|m| m.is_loaded()).unwrap_or(false)
}

/// Find a ship in the master list
pub fn find_master_ship(species_id: i32) -> Option<ShipConfig> {
    let master = GLOBAL_MASTER_LIST.lock().unwrap();
    master.as_ref()?.get_ship(species_id).cloned()
}

/// Get all ship IDs from the master list
pub fn get_all_ship_ids() -> Vec<i32> {
    let master = GLOBAL_MASTER_LIST.lock().unwrap();
    master
        .as_ref()
        .map(|m| m.get_ship_ids())
        .unwrap_or_default()
}

/// Get the number of ships in the master list
pub fn get_master_ship_count() -> usize {
    let master = GLOBAL_MASTER_LIST.lock().unwrap();
    master.as_ref().map(|m| m.count()).unwrap_or(0)
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_master_ship_list_new() {
        let list = MasterShipList::new();
        assert!(!list.is_loaded());
        assert_eq!(list.count(), 0);
    }

    #[test]
    fn test_master_ship_list_load() {
        let mut list = MasterShipList::new();

        assert!(!list.is_loaded());
        list.load().unwrap();
        assert!(list.is_loaded());
        assert!(list.count() > 0);
    }

    #[test]
    fn test_master_ship_list_free() {
        let mut list = MasterShipList::new();
        list.load().unwrap();
        assert!(list.is_loaded());

        list.free();
        assert!(!list.is_loaded());
        assert_eq!(list.count(), 0);
    }

    #[test]
    fn test_add_ship() {
        let mut list = MasterShipList::new();

        let ship = ShipConfig::new(10, "Test Ship");
        list.add_ship(ship);

        assert_eq!(list.count(), 1);
        assert!(list.has_ship(10));
    }

    #[test]
    fn test_get_ship() {
        let mut list = MasterShipList::new();

        let ship = ShipConfig::new(10, "Test Ship");
        list.add_ship(ship.clone());

        let retrieved = list.get_ship(10);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "Test Ship");
    }

    #[test]
    fn test_find_master_ship() {
        // Ensure clean state
        free_master_ship_list().ok();

        load_master_ship_list().unwrap();

        let ship = find_master_ship(0);
        assert!(ship.is_some());
        assert_eq!(ship.unwrap().name, "VUX Intruder");

        let not_found = find_master_ship(999);
        assert!(not_found.is_none());

        free_master_ship_list().unwrap();
    }

    #[test]
    fn test_load_master_ship_list() {
        // Ensure clean state
        free_master_ship_list().ok();

        assert!(!is_master_ship_list_loaded());

        load_master_ship_list().unwrap();
        assert!(is_master_ship_list_loaded());

        // Second load should fail
        assert_eq!(load_master_ship_list(), Err(MasterError::AlreadyLoaded));

        free_master_ship_list().unwrap();
        assert!(!is_master_ship_list_loaded());
    }

    #[test]
    fn test_free_master_ship_list() {
        // Ensure clean state
        free_master_ship_list().ok();

        load_master_ship_list().unwrap();
        assert!(is_master_ship_list_loaded());

        free_master_ship_list().unwrap();
        assert!(!is_master_ship_list_loaded());
    }

    #[test]
    fn test_get_all_ship_ids() {
        // Ensure clean state
        free_master_ship_list().ok();

        load_master_ship_list().unwrap();
        let ids = get_all_ship_ids();
        assert!(ids.len() > 0);
        assert!(ids.contains(&0));
        assert!(ids.contains(&1));

        free_master_ship_list().unwrap();
    }

    #[test]
    fn test_get_master_ship_count() {
        let count = get_master_ship_count();
        assert_eq!(count, 0);

        load_master_ship_list().unwrap();

        let count = get_master_ship_count();
        assert!(count > 0);

        free_master_ship_list().unwrap();
    }

    #[test]
    fn test_default() {
        let list: MasterShipList = Default::default();
        assert!(!list.is_loaded());
    }

    #[test]
    fn test_ship_config_new() {
        let config = ShipConfig::new(5, "Test Ship");

        assert_eq!(config.species_id, 5);
        assert_eq!(config.name, "Test Ship");
        assert_eq!(config.max_crew, 12);
        assert_eq!(config.energy, 20);
    }

    #[test]
    fn test_master_error_display() {
        let err = MasterError::AlreadyLoaded;
        assert!(format!("{}", err).contains("already loaded"));
    }
}
