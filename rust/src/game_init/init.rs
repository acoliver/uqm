// Initialization Functions
// Handles space init/uninit and ship init/uninit
// Delegates to ships::lifecycle module

use crate::ships;
use crate::ships::types::Starship;

/// Initialize space (load graphics, animations, etc.)
pub fn init_space() -> Result<(), InitError> {
    ships::lifecycle::init_space().map_err(|e| match e {
        ships::types::ShipsError::AlreadyInitialized => InitError::AlreadyInitialized,
        ships::types::ShipsError::NotInitialized => InitError::NotInitialized,
        ships::types::ShipsError::LoadFailed(_) => InitError::LoadFailed,
        _ => InitError::InvalidState,
    })
}

/// Uninitialize space
pub fn uninit_space() -> Result<(), InitError> {
    ships::lifecycle::uninit_space().map_err(|e| match e {
        ships::types::ShipsError::AlreadyInitialized => InitError::AlreadyInitialized,
        ships::types::ShipsError::NotInitialized => InitError::NotInitialized,
        ships::types::ShipsError::LoadFailed(_) => InitError::LoadFailed,
        _ => InitError::InvalidState,
    })
}

/// Initialize ships
pub fn init_ships() -> Result<u32, InitError> {
    // Default activity: IN_ENCOUNTER (2)
    ships::lifecycle::init_ships(2).map_err(|e| match e {
        ships::types::ShipsError::AlreadyInitialized => InitError::AlreadyInitialized,
        ships::types::ShipsError::NotInitialized => InitError::NotInitialized,
        ships::types::ShipsError::LoadFailed(_) => InitError::LoadFailed,
        _ => InitError::InvalidState,
    })
}

/// Uninitialize ships.
///
/// This zero-argument entry point handles BattleState cleanup and space
/// uninit. Crew writeback and descriptor teardown over C-owned queues
/// requires `ships::lifecycle::uninit_ships()` called directly with the
/// canonical race queues — that path is wired in P14 when the C bridge
/// provides queue access.
pub fn uninit_ships() -> Result<(), InitError> {
    // Handle BattleState + space ref-count cleanup. Pass empty queues/fragments
    // since canonical queue writeback is wired via C bridge in P14.
    let mut queues: [Vec<Starship>; ships::lifecycle::NUM_PLAYERS] = [Vec::new(), Vec::new()];
    let mut frags: [Vec<ships::writeback::ShipFragment>; ships::lifecycle::NUM_PLAYERS] =
        [Vec::new(), Vec::new()];
    ships::lifecycle::uninit_ships(&mut queues, &mut frags, 2, 0, None)
        .map(|_| ())
        .map_err(|e| match e {
            ships::types::ShipsError::AlreadyInitialized => InitError::AlreadyInitialized,
            ships::types::ShipsError::NotInitialized => InitError::NotInitialized,
            ships::types::ShipsError::LoadFailed(_) => InitError::LoadFailed,
            _ => InitError::InvalidState,
        })
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
    use serial_test::serial;

    fn cleanup() {
        // Reset battle state via lifecycle module
        crate::ships::lifecycle::reset_battle_state();
    }

    #[test]
    #[serial]
    fn test_init_space() {
        cleanup();

        init_space().unwrap();
        // Second init should succeed (reference counting)
        init_space().unwrap();

        uninit_space().unwrap();
        uninit_space().unwrap();
    }

    #[test]
    #[serial]
    fn test_init_ships() {
        cleanup();

        let num_players = init_ships().unwrap();
        assert_eq!(num_players, 2);

        uninit_ships().unwrap();
    }

    #[test]
    #[serial]
    fn test_init_space_before_ships() {
        cleanup();

        init_space().unwrap();

        // Ships init should also init space but not duplicate
        let num_players = init_ships().unwrap();
        assert_eq!(num_players, 2);

        // Uninit ships should also uninit space
        uninit_ships().unwrap();

        uninit_space().unwrap();
    }

    #[test]
    fn test_init_error_display() {
        let err = InitError::AlreadyInitialized;
        assert!(format!("{}", err).contains("initialized"));
    }
}
