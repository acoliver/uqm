// Initialization Functions
// Handles space init/uninit and ship init/uninit

use std::sync::Mutex;

/// Space initialization state
static SPACE_INIT_COUNT: Mutex<u32> = Mutex::new(0);

/// Ship initialization state
static SHIPS_INIT_COUNT: Mutex<u32> = Mutex::new(0);

/// Initialize space (load graphics, animations, etc.)
pub fn init_space() -> Result<(), InitError> {
    let mut count = SPACE_INIT_COUNT.lock().unwrap();

    if *count == 0 {
        // Perform actual initialization
        // In a real implementation, this would:
        // - Load star graphics
        // - Load explosion animations
        // - Load blast animations
        // - Load asteroid animations
    }

    *count += 1;
    Ok(())
}

/// Uninitialize space
pub fn uninit_space() -> Result<(), InitError> {
    let mut count = SPACE_INIT_COUNT.lock().unwrap();

    if *count > 0 {
        *count -= 1;

        if *count == 0 {
            // Perform actual cleanup
            // In a real implementation, this would:
            // - Free star graphics
            // - Free animations
        }
    }

    Ok(())
}

/// Initialize ships
pub fn init_ships() -> Result<u32, InitError> {
    init_space()?;

    let mut count = SHIPS_INIT_COUNT.lock().unwrap();

    if *count == 0 {
        // Perform actual ship initialization
        // In a real implementation, this would:
        // - Initialize display list
        // - Initialize galaxy
        // - Build ship queues
        // - Load ships for current mode
    }

    *count += 1;
    Ok(2) // NUM_SIDES
}

/// Uninitialize ships
pub fn uninit_ships() -> Result<(), InitError> {
    let mut count = SHIPS_INIT_COUNT.lock().unwrap();

    if *count > 0 {
        *count -= 1;

        if *count == 0 {
            uninit_space()?;

            // Perform actual ship cleanup
            // In a real implementation, this would:
            // - Count and retrieve crew
            // - Update ship fragment crew
            // - Free ship data
            // - Uninit queues
        }
    }

    Ok(())
}

/// Check if space is initialized
pub fn is_space_initialized() -> bool {
    let count = SPACE_INIT_COUNT.lock().unwrap();
    *count > 0
}

/// Check if ships are initialized
pub fn are_ships_initialized() -> bool {
    let count = SHIPS_INIT_COUNT.lock().unwrap();
    *count > 0
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InitError {
    AlreadyInitialized,
    NotInitialized,
    LoadFailed,
    InvalidState,
}

impl std::fmt::Display for InitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InitError::AlreadyInitialized => write!(f, "Already initialized"),
            InitError::NotInitialized => write!(f, "Not initialized"),
            InitError::LoadFailed => write!(f, "Failed to load resources"),
            InitError::InvalidState => write!(f, "Invalid state"),
        }
    }
}

impl std::error::Error for InitError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_space() {
        // Ensure clean state
        while is_space_initialized() {
            uninit_space().ok();
        }

        assert!(!is_space_initialized());

        init_space().unwrap();
        assert!(is_space_initialized());

        // Second init should succeed (reference counting)
        init_space().unwrap();
        assert!(is_space_initialized());

        uninit_space().unwrap();
        uninit_space().unwrap();
        assert!(!is_space_initialized());
    }

    #[test]
    fn test_init_ships() {
        assert!(!are_ships_initialized());

        let num_players = init_ships().unwrap();
        assert_eq!(num_players, 2);
        assert!(are_ships_initialized());

        uninit_ships().unwrap();
        assert!(!are_ships_initialized());
    }

    #[test]
    fn test_init_space_before_ships() {
        // Ensure clean state
        while is_space_initialized() || are_ships_initialized() {
            uninit_ships().ok();
            uninit_space().ok();
        }

        assert!(!is_space_initialized());
        assert!(!are_ships_initialized());

        init_space().unwrap();
        assert!(is_space_initialized());

        // Ships init should also init space but not duplicate
        let num_players = init_ships().unwrap();
        assert_eq!(num_players, 2);

        // Uninit ships should also uninit space
        uninit_ships().unwrap();
        assert!(!are_ships_initialized());
        // Space should still be initialized from the first init_space()
        assert!(is_space_initialized());

        uninit_space().unwrap();
        assert!(!is_space_initialized());
    }

    #[test]
    fn test_init_error_display() {
        let err = InitError::AlreadyInitialized;
        assert!(format!("{}", err).contains("initialized"));
    }
}
