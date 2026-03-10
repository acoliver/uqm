// Planet Information Management
// Handles planet scan masks and star information storage.

use super::state_file::{
    read_u32_array, read_u32_le, write_u32_array, write_u32_le, FileMode, SeekWhence,
    StateFileError, StateFileManager, STARINFO_FILE,
};

/// Scan types for planet surveys, matching the C enum order.
pub const MINERAL_SCAN: usize = 0;
pub const ENERGY_SCAN: usize = 1;
pub const BIOLOGICAL_SCAN: usize = 2;
pub const NUM_SCAN_TYPES: usize = 3;

const OFFSET_SIZE: usize = std::mem::size_of::<u32>();
const SCAN_RECORD_SIZE: usize = NUM_SCAN_TYPES * OFFSET_SIZE;

/// Scan retrieval mask for a planet or moon.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ScanRetrieveMask {
    pub mineral: u32,
    pub energy: u32,
    pub biological: u32,
}

impl ScanRetrieveMask {
    pub const fn new() -> Self {
        Self {
            mineral: 0,
            energy: 0,
            biological: 0,
        }
    }

    pub fn from_array(values: &[u32; NUM_SCAN_TYPES]) -> Self {
        Self {
            mineral: values[MINERAL_SCAN],
            energy: values[ENERGY_SCAN],
            biological: values[BIOLOGICAL_SCAN],
        }
    }

    pub fn to_array(self) -> [u32; NUM_SCAN_TYPES] {
        [self.mineral, self.energy, self.biological]
    }

    pub fn has_data(&self) -> bool {
        self.mineral != 0 || self.energy != 0 || self.biological != 0
    }

    pub fn clear(&mut self) {
        self.mineral = 0;
        self.energy = 0;
        self.biological = 0;
    }
}

pub struct PlanetInfoManager<'a> {
    state_manager: &'a mut StateFileManager,
}

impl<'a> PlanetInfoManager<'a> {
    pub fn new(state_manager: &'a mut StateFileManager) -> Self {
        Self { state_manager }
    }

    /// Initialize planet info by setting all star offsets to 0.
    pub fn init_planet_info(&mut self, num_stars: usize) -> Result<(), StateFileError> {
        self.state_manager
            .with_open_file(STARINFO_FILE, FileMode::Write, |file| {
                for _ in 0..num_stars {
                    write_u32_le(file, 0)?;
                }
                Ok(())
            })
    }

    /// Read scan information for the current planet or moon.
    pub fn get_planet_info(
        &mut self,
        star_index: usize,
        planet_index: usize,
        moon_index: usize,
        planet_num_moons: &[u8],
    ) -> Result<ScanRetrieveMask, StateFileError> {
        validate_target(planet_index, moon_index, planet_num_moons)?;

        self.state_manager
            .with_open_file(STARINFO_FILE, FileMode::Read, |file| {
                file.seek((star_index * OFFSET_SIZE) as i64, SeekWhence::Set)?;
                let offset = read_u32_le(file)? as usize;
                if offset == 0 {
                    return Ok(ScanRetrieveMask::new());
                }

                let record_offset = offset + scan_record_offset(planet_index, moon_index, planet_num_moons);
                file.seek(record_offset as i64, SeekWhence::Set)?;

                let mut values = [0u32; NUM_SCAN_TYPES];
                read_u32_array(file, &mut values)?;
                Ok(ScanRetrieveMask::from_array(&values))
            })
    }

    /// Update scan information for the current planet or moon.
    pub fn put_planet_info(
        &mut self,
        star_index: usize,
        planet_index: usize,
        moon_index: usize,
        mask: &ScanRetrieveMask,
        planet_num_moons: &[u8],
    ) -> Result<(), StateFileError> {
        validate_target(planet_index, moon_index, planet_num_moons)?;

        self.state_manager
            .with_open_file(STARINFO_FILE, FileMode::ReadWrite, |file| {
                file.seek((star_index * OFFSET_SIZE) as i64, SeekWhence::Set)?;
                let mut offset = read_u32_le(file)? as usize;

                if offset == 0 {
                    offset = file.length();

                    file.seek((star_index * OFFSET_SIZE) as i64, SeekWhence::Set)?;
                    write_u32_le(file, offset as u32)?;

                    file.seek(offset as i64, SeekWhence::Set)?;
                    let empty_mask = ScanRetrieveMask::new().to_array();
                    for &num_moons in planet_num_moons {
                        write_u32_array(file, &empty_mask)?;
                        for _ in 0..usize::from(num_moons) {
                            write_u32_array(file, &empty_mask)?;
                        }
                    }
                }

                let record_offset = offset + scan_record_offset(planet_index, moon_index, planet_num_moons);
                file.seek(record_offset as i64, SeekWhence::Set)?;
                write_u32_array(file, &mask.to_array())
            })
    }

    pub fn uninit_planet_info(&mut self) -> Result<(), StateFileError> {
        self.state_manager.delete(STARINFO_FILE)
    }
}

fn validate_target(
    planet_index: usize,
    moon_index: usize,
    planet_num_moons: &[u8],
) -> Result<(), StateFileError> {
    let Some(&num_moons) = planet_num_moons.get(planet_index) else {
        return Err(StateFileError::ReadOutOfBounds);
    };

    if moon_index > usize::from(num_moons) {
        return Err(StateFileError::ReadOutOfBounds);
    }

    Ok(())
}

fn scan_record_offset(planet_index: usize, moon_index: usize, planet_num_moons: &[u8]) -> usize {
    let mut offset = 0;
    for &num_moons in &planet_num_moons[..planet_index] {
        offset += (usize::from(num_moons) + 1) * SCAN_RECORD_SIZE;
    }
    offset + moon_index * SCAN_RECORD_SIZE
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_manager() -> StateFileManager {
        StateFileManager::new()
    }

    #[test]
    fn test_scan_retrieve_mask_new() {
        let mask = ScanRetrieveMask::new();
        assert_eq!(mask.mineral, 0);
        assert_eq!(mask.energy, 0);
        assert_eq!(mask.biological, 0);
        assert!(!mask.has_data());
    }

    #[test]
    fn test_scan_retrieve_mask_from_array_uses_c_order() {
        let values = [100, 200, 300];
        let mask = ScanRetrieveMask::from_array(&values);
        assert_eq!(mask.mineral, 100);
        assert_eq!(mask.energy, 200);
        assert_eq!(mask.biological, 300);
    }

    #[test]
    fn test_scan_retrieve_mask_to_array_uses_c_order() {
        let mask = ScanRetrieveMask {
            mineral: 10,
            energy: 20,
            biological: 30,
        };
        assert_eq!(mask.to_array(), [10, 20, 30]);
    }

    #[test]
    fn test_scan_retrieve_mask_clear() {
        let mut mask = ScanRetrieveMask {
            mineral: 10,
            energy: 20,
            biological: 30,
        };
        assert!(mask.has_data());

        mask.clear();
        assert_eq!(mask.mineral, 0);
        assert_eq!(mask.energy, 0);
        assert_eq!(mask.biological, 0);
        assert!(!mask.has_data());
    }

    #[test]
    fn test_planet_info_manager_init() {
        let mut manager = create_test_manager();
        let mut planet_mgr = PlanetInfoManager::new(&mut manager);
        planet_mgr.init_planet_info(100).unwrap();

        manager
            .with_open_file(STARINFO_FILE, FileMode::Read, |file| {
                for i in 0..100 {
                    file.seek((i * OFFSET_SIZE) as i64, SeekWhence::Set)?;
                    assert_eq!(read_u32_le(file)?, 0);
                }
                Ok(())
            })
            .unwrap();
    }

    #[test]
    fn test_planet_info_manager_get_no_data() {
        let mut manager = create_test_manager();
        let mut planet_mgr = PlanetInfoManager::new(&mut manager);

        planet_mgr.init_planet_info(10).unwrap();
        let mask = planet_mgr.get_planet_info(0, 0, 0, &[0]).unwrap();
        assert!(!mask.has_data());
    }

    #[test]
    fn test_planet_info_manager_put_get() {
        let mut manager = create_test_manager();
        let mut planet_mgr = PlanetInfoManager::new(&mut manager);

        planet_mgr.init_planet_info(10).unwrap();

        let mask = ScanRetrieveMask {
            mineral: 100,
            energy: 200,
            biological: 300,
        };
        planet_mgr.put_planet_info(0, 0, 0, &mask, &[0]).unwrap();

        let retrieved = planet_mgr.get_planet_info(0, 0, 0, &[0]).unwrap();
        assert_eq!(retrieved, mask);
    }

    #[test]
    fn test_planet_info_manager_multiple_planets() {
        let mut manager = create_test_manager();
        let mut planet_mgr = PlanetInfoManager::new(&mut manager);

        planet_mgr.init_planet_info(10).unwrap();

        let mask1 = ScanRetrieveMask {
            mineral: 1,
            energy: 2,
            biological: 3,
        };
        let mask2 = ScanRetrieveMask {
            mineral: 10,
            energy: 20,
            biological: 30,
        };

        planet_mgr.put_planet_info(0, 0, 0, &mask1, &[0, 0, 0]).unwrap();
        planet_mgr.put_planet_info(0, 1, 0, &mask2, &[0, 0, 0]).unwrap();

        let retrieved1 = planet_mgr.get_planet_info(0, 0, 0, &[0, 0, 0]).unwrap();
        let retrieved2 = planet_mgr.get_planet_info(0, 1, 0, &[0, 0, 0]).unwrap();

        assert_eq!(retrieved1, mask1);
        assert_eq!(retrieved2, mask2);
    }

    #[test]
    fn test_planet_info_manager_moons() {
        let mut manager = create_test_manager();
        let mut planet_mgr = PlanetInfoManager::new(&mut manager);

        planet_mgr.init_planet_info(10).unwrap();

        let planet_mask = ScanRetrieveMask {
            mineral: 100,
            energy: 200,
            biological: 300,
        };
        let moon_mask = ScanRetrieveMask {
            mineral: 10,
            energy: 20,
            biological: 30,
        };

        planet_mgr.put_planet_info(0, 0, 0, &planet_mask, &[2]).unwrap();
        planet_mgr.put_planet_info(0, 0, 1, &moon_mask, &[2]).unwrap();

        let retrieved_planet = planet_mgr.get_planet_info(0, 0, 0, &[2]).unwrap();
        let retrieved_moon = planet_mgr.get_planet_info(0, 0, 1, &[2]).unwrap();

        assert_eq!(retrieved_planet, planet_mask);
        assert_eq!(retrieved_moon, moon_mask);
    }

    #[test]
    fn test_planet_info_manager_uses_preceding_moons_when_reading() {
        let mut manager = create_test_manager();
        let mut planet_mgr = PlanetInfoManager::new(&mut manager);

        planet_mgr.init_planet_info(10).unwrap();

        let mask = ScanRetrieveMask {
            mineral: 7,
            energy: 8,
            biological: 9,
        };
        planet_mgr.put_planet_info(0, 1, 0, &mask, &[2, 0]).unwrap();

        let retrieved = planet_mgr.get_planet_info(0, 1, 0, &[2, 0]).unwrap();
        assert_eq!(retrieved, mask);
    }

    #[test]
    fn test_planet_info_manager_different_stars() {
        let mut manager = create_test_manager();
        let mut planet_mgr = PlanetInfoManager::new(&mut manager);

        planet_mgr.init_planet_info(10).unwrap();

        let mask0 = ScanRetrieveMask {
            mineral: 1,
            energy: 2,
            biological: 3,
        };
        let mask1 = ScanRetrieveMask {
            mineral: 4,
            energy: 5,
            biological: 6,
        };

        planet_mgr.put_planet_info(0, 0, 0, &mask0, &[0]).unwrap();
        planet_mgr.put_planet_info(1, 0, 0, &mask1, &[0]).unwrap();

        let retrieved0 = planet_mgr.get_planet_info(0, 0, 0, &[0]).unwrap();
        let retrieved1 = planet_mgr.get_planet_info(1, 0, 0, &[0]).unwrap();

        assert_eq!(retrieved0, mask0);
        assert_eq!(retrieved1, mask1);
    }

    #[test]
    fn test_planet_info_manager_uninit() {
        let mut manager = create_test_manager();
        let mut planet_mgr = PlanetInfoManager::new(&mut manager);

        planet_mgr.init_planet_info(10).unwrap();
        let mask = ScanRetrieveMask {
            mineral: 100,
            energy: 200,
            biological: 300,
        };
        planet_mgr.put_planet_info(0, 0, 0, &mask, &[0]).unwrap();

        planet_mgr.uninit_planet_info().unwrap();

        let retrieved = planet_mgr.get_planet_info(0, 0, 0, &[0]).unwrap();
        assert!(!retrieved.has_data());
    }

    #[test]
    fn test_validate_target_rejects_out_of_bounds_moon() {
        assert_eq!(validate_target(0, 2, &[1]), Err(StateFileError::ReadOutOfBounds));
    }
}
