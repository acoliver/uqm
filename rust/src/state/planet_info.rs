// Planet Information Management
// Handles planet scan masks and star information storage

use super::state_file::{
    read_u32_le, write_u32_le, StateFile, StateFileError, StateFileManager, STARINFO_FILE,
};
use crate::state::state_file::FileMode;

/// Scan types for planet surveys
pub const NUM_SCAN_TYPES: usize = 3;
pub const BIOLOGICAL_SCAN: usize = 0;
pub const MINERAL_SCAN: usize = 1;
pub const ENERGY_SCAN: usize = 2;

/// Size of DWORD (32-bit) for offsets
const OFFSET_SIZE: usize = 4;
const SCAN_RECORD_SIZE: usize = NUM_SCAN_TYPES * OFFSET_SIZE;

/// Scan retrieval mask for a planet or moon
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ScanRetrieveMask {
    /// Biological scan data
    pub biological: u32,
    /// Mineral scan data
    pub mineral: u32,
    /// Energy scan data
    pub energy: u32,
}

impl ScanRetrieveMask {
    /// Create a new zero-initialized scan mask
    pub fn new() -> Self {
        ScanRetrieveMask {
            biological: 0,
            mineral: 0,
            energy: 0,
        }
    }

    /// Create a scan mask from an array
    pub fn from_array(arr: &[u32; NUM_SCAN_TYPES]) -> Self {
        ScanRetrieveMask {
            biological: arr[BIOLOGICAL_SCAN],
            mineral: arr[MINERAL_SCAN],
            energy: arr[ENERGY_SCAN],
        }
    }

    /// Convert scan mask to an array
    pub fn to_array(&self) -> [u32; NUM_SCAN_TYPES] {
        [self.biological, self.mineral, self.energy]
    }

    /// Check if any scans have data
    pub fn has_data(&self) -> bool {
        self.biological != 0 || self.mineral != 0 || self.energy != 0
    }

    /// Clear all scan data
    pub fn clear(&mut self) {
        self.biological = 0;
        self.mineral = 0;
        self.energy = 0;
    }
}

/// Planet information manager
pub struct PlanetInfoManager<'a> {
    state_manager: &'a mut StateFileManager,
}

impl<'a> PlanetInfoManager<'a> {
    /// Create a new planet info manager
    pub fn new(state_manager: &'a mut StateFileManager) -> Self {
        PlanetInfoManager { state_manager }
    }

    /// Initialize planet info - set all star offsets to 0
    pub fn init_planet_info(&mut self, num_stars: usize) -> Result<(), StateFileError> {
        let file = self.state_manager.open(STARINFO_FILE, FileMode::Write)?;

        // Write zero offset for each star
        for _ in 0..num_stars {
            write_u32_le(
                &mut *unsafe { &mut *(file as *const _ as *mut StateFile) },
                0,
            )?;
        }

        self.state_manager.close(STARINFO_FILE)?;
        Ok(())
    }

    /// Get planet scan information for the current planet/moon
    ///
    /// # Arguments
    /// * `star_index` - Index of the star in the star array
    /// * `planet_index` - Index of the planet in the solar system
    /// * `moon_index` - Index of the moon (0 for planet itself)
    ///
    /// # Returns
    /// Scan retrieve mask for the target planet/moon
    pub fn get_planet_info(
        &mut self,
        star_index: usize,
        planet_index: usize,
        moon_index: usize,
    ) -> Result<ScanRetrieveMask, StateFileError> {
        let file = self.state_manager.open(STARINFO_FILE, FileMode::Read)?;
        let file_mut = unsafe { &mut *(file as *const _ as *mut StateFile) };

        // Read star offset
        file_mut.seek(
            (star_index * OFFSET_SIZE) as i64,
            crate::state::state_file::SeekWhence::Set,
        )?;
        let offset = read_u32_le(file_mut)?;

        let mut mask = ScanRetrieveMask::new();

        if offset != 0 {
            // Skip scan records for preceding planets
            let mut current_offset = offset as usize;
            for _ in 0..planet_index {
                // Need to know number of moons for each planet - for now, assume 0
                // In a real implementation, this would come from the solar system state
                current_offset += SCAN_RECORD_SIZE;
            }

            // Skip scan records for preceding moons
            current_offset += moon_index * SCAN_RECORD_SIZE;

            // Read the scan mask
            file_mut.seek(
                current_offset as i64,
                crate::state::state_file::SeekWhence::Set,
            )?;
            let mut scan_values = [0u32; NUM_SCAN_TYPES];
            for v in scan_values.iter_mut() {
                *v = read_u32_le(file_mut)?;
            }

            mask = ScanRetrieveMask::from_array(&scan_values);
        }

        self.state_manager.close(STARINFO_FILE)?;
        Ok(mask)
    }

    /// Update planet scan information for the current planet/moon
    ///
    /// # Arguments
    /// * `star_index` - Index of the star in the star array
    /// * `planet_index` - Index of the planet in the solar system
    /// * `moon_index` - Index of the moon (0 for planet itself)
    /// * `mask` - Scan retrieve mask to write
    /// * `num_planets` - Number of planets in the system
    /// * `planet_num_moons` - Function to get number of moons for each planet
    pub fn put_planet_info<F>(
        &mut self,
        star_index: usize,
        planet_index: usize,
        moon_index: usize,
        mask: &ScanRetrieveMask,
        num_planets: usize,
        planet_num_moons: F,
    ) -> Result<(), StateFileError>
    where
        F: Fn(usize) -> usize,
    {
        let file = self
            .state_manager
            .open(STARINFO_FILE, FileMode::ReadWrite)?;
        let file_mut = unsafe { &mut *(file as *const _ as *mut StateFile) };

        // Read star offset
        file_mut.seek(
            (star_index * OFFSET_SIZE) as i64,
            crate::state::state_file::SeekWhence::Set,
        )?;
        let offset = read_u32_le(file_mut)?;

        let final_offset = if offset == 0 {
            // Create new scan record
            let new_offset = file_mut.length() as u32;

            // Write the record offset
            file_mut.seek(
                (star_index * OFFSET_SIZE) as i64,
                crate::state::state_file::SeekWhence::Set,
            )?;
            write_u32_le(file_mut, new_offset)?;

            // Initialize scan records for all planets and moons in the system
            file_mut.seek(new_offset as i64, crate::state::state_file::SeekWhence::Set)?;
            let empty_mask = ScanRetrieveMask::new();
            for i in 0..num_planets {
                write_u32_le(file_mut, empty_mask.biological)?;
                write_u32_le(file_mut, empty_mask.mineral)?;
                write_u32_le(file_mut, empty_mask.energy)?;

                // Initialize moons for this planet
                let num_moons = planet_num_moons(i);
                for _ in 0..num_moons {
                    write_u32_le(file_mut, empty_mask.biological)?;
                    write_u32_le(file_mut, empty_mask.mineral)?;
                    write_u32_le(file_mut, empty_mask.energy)?;
                }
            }

            new_offset as usize
        } else {
            offset as usize
        };

        // Skip scan records for preceding planets
        let mut current_offset = final_offset;
        for planet_idx in 0..planet_index {
            current_offset += SCAN_RECORD_SIZE * (1 + planet_num_moons(planet_idx));
        }

        // Skip scan records for preceding moons
        current_offset += moon_index * SCAN_RECORD_SIZE;

        // Write the scan mask
        file_mut.seek(
            current_offset as i64,
            crate::state::state_file::SeekWhence::Set,
        )?;
        write_u32_le(file_mut, mask.biological)?;
        write_u32_le(file_mut, mask.mineral)?;
        write_u32_le(file_mut, mask.energy)?;

        self.state_manager.close(STARINFO_FILE)?;
        Ok(())
    }

    /// Uninitialize planet info - delete the state file
    pub fn uninit_planet_info(&mut self) -> Result<(), StateFileError> {
        self.state_manager.delete(STARINFO_FILE)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::state_file::SeekWhence;

    fn create_test_manager() -> StateFileManager {
        let mut manager = StateFileManager::new();
        manager.open(STARINFO_FILE, FileMode::Write).unwrap();
        manager.close(STARINFO_FILE).unwrap();
        manager
    }

    #[test]
    fn test_scan_retrieve_mask_new() {
        let mask = ScanRetrieveMask::new();
        assert_eq!(mask.biological, 0);
        assert_eq!(mask.mineral, 0);
        assert_eq!(mask.energy, 0);
        assert!(!mask.has_data());
    }

    #[test]
    fn test_scan_retrieve_mask_from_array() {
        let arr = [100, 200, 300];
        let mask = ScanRetrieveMask::from_array(&arr);
        assert_eq!(mask.biological, 100);
        assert_eq!(mask.mineral, 200);
        assert_eq!(mask.energy, 300);
        assert!(mask.has_data());
    }

    #[test]
    fn test_scan_retrieve_mask_to_array() {
        let mask = ScanRetrieveMask {
            biological: 10,
            mineral: 20,
            energy: 30,
        };
        let arr = mask.to_array();
        assert_eq!(arr, [10, 20, 30]);
    }

    #[test]
    fn test_scan_retrieve_mask_clear() {
        let mut mask = ScanRetrieveMask {
            biological: 10,
            mineral: 20,
            energy: 30,
        };
        assert!(mask.has_data());

        mask.clear();
        assert_eq!(mask.biological, 0);
        assert_eq!(mask.mineral, 0);
        assert_eq!(mask.energy, 0);
        assert!(!mask.has_data());
    }

    #[test]
    fn test_scan_retrieve_mask_default() {
        let mask: ScanRetrieveMask = Default::default();
        assert_eq!(mask.biological, 0);
        assert_eq!(mask.mineral, 0);
        assert_eq!(mask.energy, 0);
    }

    #[test]
    fn test_scan_retrieve_mask_clone() {
        let mask1 = ScanRetrieveMask {
            biological: 10,
            mineral: 20,
            energy: 30,
        };
        let mask2 = mask1;
        assert_eq!(mask2.biological, 10);
        assert_eq!(mask2.mineral, 20);
        assert_eq!(mask2.energy, 30);
    }

    #[test]
    fn test_planet_info_manager_init() {
        let mut manager = create_test_manager();
        let mut planet_mgr = PlanetInfoManager::new(&mut manager);

        // Initialize with 100 stars
        planet_mgr.init_planet_info(100).unwrap();

        // Verify all offsets are 0
        {
            let file = manager.open(STARINFO_FILE, FileMode::Read).unwrap();
            for i in 0..100 {
                unsafe {
                    let file_mut = &mut *(file as *const _ as *mut StateFile);
                    file_mut
                        .seek((i * OFFSET_SIZE) as i64, SeekWhence::Set)
                        .unwrap();
                    let offset = read_u32_le(file_mut).unwrap();
                    assert_eq!(offset, 0);
                }
            }
            manager.close(STARINFO_FILE).unwrap();
        }
    }

    #[test]
    fn test_planet_info_manager_get_no_data() {
        let mut manager = create_test_manager();
        let mut planet_mgr = PlanetInfoManager::new(&mut manager);

        planet_mgr.init_planet_info(10).unwrap();

        // Get info for unsaved planet
        let mask = planet_mgr.get_planet_info(0, 0, 0).unwrap();
        assert!(!mask.has_data());
    }

    #[test]
    fn test_planet_info_manager_put_get() {
        let mut manager = create_test_manager();
        let mut planet_mgr = PlanetInfoManager::new(&mut manager);

        planet_mgr.init_planet_info(10).unwrap();

        // Put scan data for star 0, planet 0
        let mask = ScanRetrieveMask {
            biological: 100,
            mineral: 200,
            energy: 300,
        };
        planet_mgr
            .put_planet_info(0, 0, 0, &mask, 1, |_| 0)
            .unwrap();

        // Get it back
        let retrieved = planet_mgr.get_planet_info(0, 0, 0).unwrap();
        assert_eq!(retrieved, mask);
    }

    #[test]
    fn test_planet_info_manager_multiple_planets() {
        let mut manager = create_test_manager();
        let mut planet_mgr = PlanetInfoManager::new(&mut manager);

        planet_mgr.init_planet_info(10).unwrap();

        // Put data for different planets
        let mask1 = ScanRetrieveMask {
            biological: 1,
            mineral: 2,
            energy: 3,
        };
        let mask2 = ScanRetrieveMask {
            biological: 10,
            mineral: 20,
            energy: 30,
        };

        planet_mgr
            .put_planet_info(0, 0, 0, &mask1, 3, |_| 0)
            .unwrap();
        planet_mgr
            .put_planet_info(0, 1, 0, &mask2, 3, |_| 0)
            .unwrap();

        // Verify both are retrievable
        let retrieved1 = planet_mgr.get_planet_info(0, 0, 0).unwrap();
        let retrieved2 = planet_mgr.get_planet_info(0, 1, 0).unwrap();

        assert_eq!(retrieved1, mask1);
        assert_eq!(retrieved2, mask2);
    }

    #[test]
    fn test_planet_info_manager_moons() {
        let mut manager = create_test_manager();
        let mut planet_mgr = PlanetInfoManager::new(&mut manager);

        planet_mgr.init_planet_info(10).unwrap();

        let planet_mask = ScanRetrieveMask {
            biological: 100,
            mineral: 200,
            energy: 300,
        };
        let moon_mask = ScanRetrieveMask {
            biological: 10,
            mineral: 20,
            energy: 30,
        };

        // Put data for planet and its moon
        planet_mgr
            .put_planet_info(0, 0, 0, &planet_mask, 1, |i| if i == 0 { 2 } else { 0 })
            .unwrap();
        planet_mgr
            .put_planet_info(0, 0, 1, &moon_mask, 1, |i| if i == 0 { 2 } else { 0 })
            .unwrap();

        // Verify both are retrievable
        let retrieved_planet = planet_mgr.get_planet_info(0, 0, 0).unwrap();
        let retrieved_moon = planet_mgr.get_planet_info(0, 0, 1).unwrap();

        assert_eq!(retrieved_planet, planet_mask);
        assert_eq!(retrieved_moon, moon_mask);
    }

    #[test]
    fn test_planet_info_manager_different_stars() {
        let mut manager = create_test_manager();
        let mut planet_mgr = PlanetInfoManager::new(&mut manager);

        planet_mgr.init_planet_info(10).unwrap();

        let mask0 = ScanRetrieveMask {
            biological: 1,
            mineral: 2,
            energy: 3,
        };
        let mask1 = ScanRetrieveMask {
            biological: 4,
            mineral: 5,
            energy: 6,
        };

        planet_mgr
            .put_planet_info(0, 0, 0, &mask0, 1, |_| 0)
            .unwrap();
        planet_mgr
            .put_planet_info(1, 0, 0, &mask1, 1, |_| 0)
            .unwrap();

        let retrieved0 = planet_mgr.get_planet_info(0, 0, 0).unwrap();
        let retrieved1 = planet_mgr.get_planet_info(1, 0, 0).unwrap();

        assert_eq!(retrieved0, mask0);
        assert_eq!(retrieved1, mask1);
    }

    #[test]
    fn test_planet_info_manager_uninit() {
        let mut manager = create_test_manager();
        let mut planet_mgr = PlanetInfoManager::new(&mut manager);

        // Put some data
        planet_mgr.init_planet_info(10).unwrap();
        let mask = ScanRetrieveMask {
            biological: 100,
            mineral: 200,
            energy: 300,
        };
        planet_mgr
            .put_planet_info(0, 0, 0, &mask, 1, |_| 0)
            .unwrap();

        // Uninit
        planet_mgr.uninit_planet_info().unwrap();

        // Verify data is gone
        let retrieved = planet_mgr.get_planet_info(0, 0, 0).unwrap();
        assert!(!retrieved.has_data());
    }

    #[test]
    fn test_scan_retrieve_mask_partial_eq() {
        let mask1 = ScanRetrieveMask {
            biological: 10,
            mineral: 20,
            energy: 30,
        };
        let mask2 = ScanRetrieveMask {
            biological: 10,
            mineral: 20,
            energy: 30,
        };
        let mask3 = ScanRetrieveMask {
            biological: 10,
            mineral: 20,
            energy: 31,
        };

        assert_eq!(mask1, mask2);
        assert_ne!(mask1, mask3);
    }

    #[test]
    fn test_scan_retrieve_mask_debug() {
        let mask = ScanRetrieveMask {
            biological: 100,
            mineral: 200,
            energy: 300,
        };
        let debug_str = format!("{:?}", mask);
        assert!(debug_str.contains("biological: 100"));
        assert!(debug_str.contains("mineral: 200"));
        assert!(debug_str.contains("energy: 300"));
    }
}
