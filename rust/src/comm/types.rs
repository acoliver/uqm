//! Core types for the communication system

use std::fmt;

/// Error type for communication operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommError {
    /// Communication not initialized
    NotInitialized,
    /// Already initialized
    AlreadyInitialized,
    /// Invalid track
    InvalidTrack,
    /// Track not playing
    TrackNotPlaying,
    /// No responses available
    NoResponses,
    /// Invalid response index
    InvalidResponse(i32),
    /// Audio error
    AudioError(String),
    /// Animation error
    AnimationError(String),
}

impl fmt::Display for CommError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CommError::NotInitialized => write!(f, "Communication not initialized"),
            CommError::AlreadyInitialized => write!(f, "Communication already initialized"),
            CommError::InvalidTrack => write!(f, "Invalid track"),
            CommError::TrackNotPlaying => write!(f, "Track not playing"),
            CommError::NoResponses => write!(f, "No responses available"),
            CommError::InvalidResponse(idx) => write!(f, "Invalid response index: {}", idx),
            CommError::AudioError(s) => write!(f, "Audio error: {}", s),
            CommError::AnimationError(s) => write!(f, "Animation error: {}", s),
        }
    }
}

impl std::error::Error for CommError {}

/// Result type for communication operations
pub type CommResult<T> = Result<T, CommError>;

/// Communication intro mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u32)]
pub enum CommIntroMode {
    /// Default intro (fade in)
    #[default]
    Default = 0,
    /// Fade in from black
    FadeIn = 1,
    /// Cross-fade from previous screen
    CrossFade = 2,
    /// Immediate (no transition)
    Immediate = 3,
}

impl From<u32> for CommIntroMode {
    fn from(value: u32) -> Self {
        match value {
            0 => CommIntroMode::Default,
            1 => CommIntroMode::FadeIn,
            2 => CommIntroMode::CrossFade,
            3 => CommIntroMode::Immediate,
            _ => CommIntroMode::Default,
        }
    }
}

/// Communication data for an alien encounter (LOCDATA equivalent)
#[derive(Debug, Default)]
pub struct CommData {
    /// Initialization callback
    pub init_encounter_func: Option<usize>,
    /// Post-encounter callback
    pub post_encounter_func: Option<usize>,
    /// Cleanup callback
    pub uninit_encounter_func: Option<usize>,

    /// Alien graphics frame handle
    pub alien_frame: u32,
    /// Alien font handle
    pub alien_font: u32,
    /// Alien color map handle
    pub alien_color_map: u32,

    /// Number of animations
    pub num_animations: u32,

    /// Ambient sound flags
    pub ambient_flags: u32,
    /// Transition time in ticks
    pub transition_time: u32,
}

impl CommData {
    /// Create new empty CommData
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the initialization callback (as function pointer address)
    pub fn set_init_func(&mut self, func: usize) {
        self.init_encounter_func = Some(func);
    }

    /// Set the post-encounter callback
    pub fn set_post_func(&mut self, func: usize) {
        self.post_encounter_func = Some(func);
    }

    /// Set the cleanup callback
    pub fn set_uninit_func(&mut self, func: usize) {
        self.uninit_encounter_func = Some(func);
    }
}

/// Ambient sound flags
pub mod AmbientFlags {
    pub const NONE: u32 = 0;
    pub const WAIT_TALKING: u32 = 1 << 0;
    pub const WAIT_TRACK: u32 = 1 << 1;
}

/// One second in game ticks (for timing calculations)
pub const ONE_SECOND: u32 = 60;

/// Communication animation rate (40 FPS)
pub const COMM_ANIM_RATE: u32 = ONE_SECOND / 40;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_comm_error_display() {
        assert_eq!(
            format!("{}", CommError::NotInitialized),
            "Communication not initialized"
        );
        assert_eq!(
            format!("{}", CommError::InvalidResponse(5)),
            "Invalid response index: 5"
        );
    }

    #[test]
    fn test_comm_intro_mode_from_u32() {
        assert_eq!(CommIntroMode::from(0), CommIntroMode::Default);
        assert_eq!(CommIntroMode::from(1), CommIntroMode::FadeIn);
        assert_eq!(CommIntroMode::from(2), CommIntroMode::CrossFade);
        assert_eq!(CommIntroMode::from(3), CommIntroMode::Immediate);
        assert_eq!(CommIntroMode::from(99), CommIntroMode::Default);
    }

    #[test]
    fn test_comm_data_new() {
        let data = CommData::new();
        assert!(data.init_encounter_func.is_none());
        assert_eq!(data.num_animations, 0);
        assert_eq!(data.ambient_flags, 0);
    }

    #[test]
    fn test_comm_data_set_callbacks() {
        let mut data = CommData::new();
        data.set_init_func(0x1000);
        data.set_post_func(0x2000);
        data.set_uninit_func(0x3000);

        assert_eq!(data.init_encounter_func, Some(0x1000));
        assert_eq!(data.post_encounter_func, Some(0x2000));
        assert_eq!(data.uninit_encounter_func, Some(0x3000));
    }

    #[test]
    fn test_ambient_flags() {
        assert_eq!(AmbientFlags::NONE, 0);
        assert_eq!(AmbientFlags::WAIT_TALKING, 1);
        assert_eq!(AmbientFlags::WAIT_TRACK, 2);
    }

    #[test]
    fn test_timing_constants() {
        assert_eq!(ONE_SECOND, 60);
        assert_eq!(COMM_ANIM_RATE, 1); // 60/40 = 1.5, truncated to 1
    }
}
