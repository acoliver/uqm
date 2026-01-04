// FFI bindings for Game Initialization module
// Provides C-compatible interface for game initialization

use std::ffi::CString;
use std::os::raw::{c_char, c_int};
use std::ptr;

use super::init::{init_ships, init_space, uninit_ships, uninit_space};
use super::master::{free_master_ship_list, load_master_ship_list};
use super::setup::{init_contexts, init_game_kernel, uninit_contexts, uninit_game_kernel};

// Space initialization

/// Initialize space (FFI wrapper)
#[no_mangle]
pub extern "C" fn rust_init_space() -> c_int {
    match init_space() {
        Ok(()) => 1,
        Err(_) => 0,
    }
}

/// Uninitialize space (FFI wrapper)
#[no_mangle]
pub extern "C" fn rust_uninit_space() -> c_int {
    match uninit_space() {
        Ok(()) => 1,
        Err(_) => 0,
    }
}

// Ship initialization

/// Initialize ships (FFI wrapper)
#[no_mangle]
pub extern "C" fn rust_init_ships() -> c_int {
    match init_ships() {
        Ok(_num_players) => 1,
        Err(_) => 0,
    }
}

/// Uninitialize ships (FFI wrapper)
#[no_mangle]
pub extern "C" fn rust_uninit_ships() -> c_int {
    match uninit_ships() {
        Ok(()) => 1,
        Err(_) => 0,
    }
}

// Game kernel

/// Initialize the game kernel (FFI wrapper)
#[no_mangle]
pub extern "C" fn rust_init_game_kernel() -> c_int {
    match init_game_kernel() {
        Ok(()) => 1,
        Err(_) => 0,
    }
}

/// Uninitialize the game kernel (FFI wrapper)
#[no_mangle]
pub extern "C" fn rust_uninit_game_kernel() -> c_int {
    match uninit_game_kernel() {
        Ok(()) => 1,
        Err(_) => 0,
    }
}

// Contexts

/// Initialize all contexts (FFI wrapper)
#[no_mangle]
pub extern "C" fn rust_init_contexts() -> c_int {
    match init_contexts() {
        Ok(()) => 1,
        Err(_) => 0,
    }
}

/// Uninitialize all contexts (FFI wrapper)
#[no_mangle]
pub extern "C" fn rust_uninit_contexts() -> c_int {
    match uninit_contexts() {
        Ok(()) => 1,
        Err(_) => 0,
    }
}

// Master ship list

/// Load the master ship list (FFI wrapper)
#[no_mangle]
pub extern "C" fn rust_load_master_ship_list() -> c_int {
    match load_master_ship_list() {
        Ok(()) => 1,
        Err(_) => 0,
    }
}

/// Free the master ship list (FFI wrapper)
#[no_mangle]
pub extern "C" fn rust_free_master_ship_list() -> c_int {
    match free_master_ship_list() {
        Ok(()) => 1,
        Err(_) => 0,
    }
}

/// Get ship name by species ID
#[no_mangle]
pub extern "C" fn rust_get_ship_name(species_id: c_int) -> *mut c_char {
    use super::master::find_master_ship;

    match find_master_ship(species_id) {
        Some(ship) => CString::new(ship.name.as_str())
            .map(|s| s.into_raw())
            .unwrap_or(ptr::null_mut()),
        None => ptr::null_mut(),
    }
}

/// Get ship crew count by species ID
#[no_mangle]
pub extern "C" fn rust_get_ship_crew(species_id: c_int) -> u16 {
    use super::master::find_master_ship;

    match find_master_ship(species_id) {
        Some(ship) => ship.max_crew,
        None => 0,
    }
}

/// Get ship energy count by species ID
#[no_mangle]
pub extern "C" fn rust_get_ship_energy(species_id: c_int) -> u16 {
    use super::master::find_master_ship;

    match find_master_ship(species_id) {
        Some(ship) => ship.max_energy,
        None => 0,
    }
}

/// Check if master ship list is loaded
#[no_mangle]
pub extern "C" fn rust_is_master_ship_list_loaded() -> c_int {
    use super::master::is_master_ship_list_loaded;

    if is_master_ship_list_loaded() {
        1
    } else {
        0
    }
}

/// Get number of ships in master list
#[no_mangle]
pub extern "C" fn rust_get_master_ship_count() -> c_int {
    use super::master::get_master_ship_count;

    get_master_ship_count() as c_int
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    fn test_rust_init_uninit_space() {
        let result = rust_init_space();
        assert_eq!(result, 1);

        let result = rust_uninit_space();
        assert_eq!(result, 1);
    }

    #[test]
    fn test_rust_init_uninit_ships() {
        let result = rust_init_ships();
        assert_eq!(result, 1);

        let result = rust_uninit_ships();
        assert_eq!(result, 1);
    }

    #[test]
    fn test_rust_init_uninit_game_kernel() {
        let result = rust_init_game_kernel();
        assert_eq!(result, 1);

        let result = rust_uninit_game_kernel();
        assert_eq!(result, 1);
    }

    #[test]
    fn test_rust_init_uninit_contexts() {
        let result = rust_init_contexts();
        assert_eq!(result, 1);

        let result = rust_uninit_contexts();
        assert_eq!(result, 1);
    }

    #[test]
    fn test_rust_load_free_master_ship_list() {
        // Ensure clean state
        rust_free_master_ship_list();

        assert_eq!(rust_is_master_ship_list_loaded(), 0);

        let result = rust_load_master_ship_list();
        assert_eq!(result, 1);
        assert_eq!(rust_is_master_ship_list_loaded(), 1);

        let result = rust_free_master_ship_list();
        assert_eq!(result, 1);
        assert_eq!(rust_is_master_ship_list_loaded(), 0);
    }

    #[test]
    #[serial]
    fn test_rust_get_ship_name() {
        use std::ffi::CStr;

        // Ensure clean state
        rust_free_master_ship_list();

        rust_load_master_ship_list();

        let name_ptr = rust_get_ship_name(0);
        assert!(!name_ptr.is_null());

        unsafe {
            let name = CStr::from_ptr(name_ptr);
            assert_eq!(name.to_str().unwrap(), "VUX Intruder");
        }

        let name_ptr = rust_get_ship_name(999);
        assert!(name_ptr.is_null());

        rust_free_master_ship_list();
    }

    #[test]
    #[serial]
    fn test_rust_get_ship_crew() {
        // Ensure clean state
        rust_free_master_ship_list();

        rust_load_master_ship_list();

        let crew = rust_get_ship_crew(0);
        assert_eq!(crew, 12);

        let crew = rust_get_ship_crew(999);
        assert_eq!(crew, 0);

        rust_free_master_ship_list();
    }

    #[test]
    #[serial]
    fn test_rust_get_ship_energy() {
        rust_load_master_ship_list();

        let energy = rust_get_ship_energy(0);
        assert_eq!(energy, 20);

        let energy = rust_get_ship_energy(999);
        assert_eq!(energy, 0);

        rust_free_master_ship_list();
    }

    #[test]
    #[serial]
    fn test_rust_get_master_ship_count() {
        // Ensure clean state
        rust_free_master_ship_list();

        let count = rust_get_master_ship_count();
        assert_eq!(count, 0);

        rust_load_master_ship_list();
        let count = rust_get_master_ship_count();
        assert!(count > 0);

        rust_free_master_ship_list();
    }
}
