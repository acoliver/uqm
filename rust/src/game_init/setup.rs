// Game Setup Functions
// Handles game initialization and context setup

use crate::state::GameState;
use crate::time::GameClock;
use std::sync::Mutex;

/// Global game state
static GLOBAL_GAME_KERNEL: Mutex<Option<GameKernel>> = Mutex::new(None);

/// Game kernel containing core game structures
#[derive(Debug)]
pub struct GameKernel {
    pub game_state: Option<GameState>,
    pub game_clock: Option<GameClock>,
    pub initialized: bool,
}

impl GameKernel {
    pub fn new() -> Self {
        GameKernel {
            game_state: None,
            game_clock: None,
            initialized: false,
        }
    }

    pub fn initialize(&mut self) -> Result<(), SetupError> {
        // Initialize game state
        self.game_state = Some(GameState::new());

        // Initialize game clock
        self.game_clock = Some(GameClock::new());

        self.initialized = true;
        Ok(())
    }

    pub fn uninitialize(&mut self) {
        self.game_state = None;
        self.game_clock = None;
        self.initialized = false;
    }
}

impl Default for GameKernel {
    fn default() -> Self {
        Self::new()
    }
}

/// Initialize the game kernel
pub fn init_game_kernel() -> Result<(), SetupError> {
    let mut kernel = GLOBAL_GAME_KERNEL.lock().unwrap();

    if kernel.is_none() {
        *kernel = Some(GameKernel::new());
    }

    if let Some(ref mut k) = *kernel {
        if k.initialized {
            return Err(SetupError::AlreadyInitialized);
        }
        k.initialize()?;
    }

    Ok(())
}

/// Uninitialize the game kernel
pub fn uninit_game_kernel() -> Result<(), SetupError> {
    let mut kernel = GLOBAL_GAME_KERNEL.lock().unwrap();

    if let Some(ref mut k) = *kernel {
        k.uninitialize();
    }

    Ok(())
}

/// Initialize all contexts
pub fn init_contexts() -> Result<(), SetupError> {
    // In a real implementation, this would:
    // - Initialize radar context
    // - Initialize screen context
    // - Initialize game-specific contexts

    Ok(())
}

/// Uninitialize all contexts
pub fn uninit_contexts() -> Result<(), SetupError> {
    // In a real implementation, this would:
    // - Destroy radar context
    // - Destroy other contexts

    Ok(())
}

/// Check if the game kernel is initialized
pub fn is_kernel_initialized() -> bool {
    let kernel = GLOBAL_GAME_KERNEL.lock().unwrap();
    kernel.as_ref().map(|k| k.initialized).unwrap_or(false)
}

/// Get access to the game kernel
pub fn get_kernel<F, R>(f: F) -> R
where
    F: FnOnce(&GameKernel) -> R,
    R: Default,
{
    let kernel = GLOBAL_GAME_KERNEL.lock().unwrap();
    kernel.as_ref().map(f).unwrap_or_default()
}

/// Get mutable access to the game kernel
pub fn get_kernel_mut<F, R>(f: F) -> R
where
    F: FnOnce(&mut GameKernel) -> R,
    R: Default,
{
    let mut kernel = GLOBAL_GAME_KERNEL.lock().unwrap();
    kernel.as_mut().map(f).unwrap_or_default()
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SetupError {
    AlreadyInitialized,
    NotInitialized,
    ContextInitFailed,
    InvalidState,
}

impl std::fmt::Display for SetupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SetupError::AlreadyInitialized => write!(f, "Already initialized"),
            SetupError::NotInitialized => write!(f, "Not initialized"),
            SetupError::ContextInitFailed => write!(f, "Context initialization failed"),
            SetupError::InvalidState => write!(f, "Invalid state"),
        }
    }
}

impl std::error::Error for SetupError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_game_kernel_initialize() {
        let mut kernel = GameKernel::new();

        assert!(!kernel.initialized);
        kernel.initialize().unwrap();
        assert!(kernel.initialized);
        assert!(kernel.game_state.is_some());
        assert!(kernel.game_clock.is_some());
    }

    #[test]
    fn test_game_kernel_uninitialize() {
        let mut kernel = GameKernel::new();
        kernel.initialize().unwrap();

        kernel.uninitialize();
        assert!(!kernel.initialized);
        assert!(kernel.game_state.is_none());
        assert!(kernel.game_clock.is_none());
    }

    #[test]
    fn test_init_game_kernel() {
        // Ensure clean state
        uninit_game_kernel().ok();

        assert!(!is_kernel_initialized());

        init_game_kernel().unwrap();
        assert!(is_kernel_initialized());

        // Second init should fail
        assert_eq!(init_game_kernel(), Err(SetupError::AlreadyInitialized));

        uninit_game_kernel().ok();
    }

    #[test]
    fn test_uninit_game_kernel() {
        // Ensure clean state
        uninit_game_kernel().ok();

        init_game_kernel().unwrap();
        assert!(is_kernel_initialized());

        uninit_game_kernel().unwrap();
        // Should be not initialized but kernel still exists
        assert!(!is_kernel_initialized());
    }

    #[test]
    fn test_get_kernel() {
        // Ensure clean state
        uninit_game_kernel().ok();

        init_game_kernel().unwrap();

        let initialized = get_kernel(|k| k.initialized);
        assert!(initialized);

        let has_state = get_kernel(|k| k.game_state.is_some());
        assert!(has_state);

        let not_initialized = get_kernel(|_| false);
        assert!(!not_initialized);

        uninit_game_kernel().ok();
    }

    #[test]
    fn test_get_kernel_not_initialized() {
        // Ensure clean state
        uninit_game_kernel().ok();

        let result = get_kernel(|k| k.initialized);
        assert!(!result);
    }

    #[test]
    fn test_get_kernel_mut() {
        init_game_kernel().unwrap();

        get_kernel_mut(|k| {
            k.game_state.as_mut().map(|s| s.reset());
        });

        let state_count = get_kernel(|k| {
            k.game_state
                .as_ref()
                .map(|s| s.as_bytes().iter().filter(|&&b| b != 0).count())
        });
        assert_eq!(state_count, Some(0));
    }

    #[test]
    fn test_default() {
        let kernel: GameKernel = Default::default();
        assert!(!kernel.initialized);
    }

    #[test]
    fn test_setup_error_display() {
        let err = SetupError::AlreadyInitialized;
        assert!(format!("{}", err).contains("initialized"));
    }
}
