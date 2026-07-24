//! Core types for the communication system
use std::ffi::c_void;
use std::fmt;

/// Number of animation slots in an alien communication sequence (matches C MAX_ANIMATIONS).
pub const MAX_ANIMATIONS: usize = 20;

/// Animation flags (mirrors C ANIMATION_DESC AnimFlags bits).
#[allow(
    non_snake_case,
    reason = "C ABI compatibility is fixed during the Rust migration; tracked by PLAN-20260723-RUNTIME-AUTOMATION.P00"
)]
pub mod AnimFlags {
    pub const RANDOM_ANIM: u32 = 1 << 0;
    pub const CIRCULAR_ANIM: u32 = 1 << 1;
    pub const YOYO_ANIM: u32 = 1 << 2;
    /// Set while the talking animation is active or should be active.
    pub const WAIT_TALKING: u32 = 1 << 3;
    /// Set in AlienTalkDesc to suppress the talking animation.
    pub const PAUSE_TALKING: u32 = 1 << 4;
    /// In AlienTransitionDesc: transition to talking state.
    pub const TALK_INTRO: u32 = 1 << 5;
    /// In AlienTransitionDesc/AlienTalkDesc: end of talking animation.
    pub const TALK_DONE: u32 = 1 << 6;
    pub const ANIM_DISABLED: u32 = 1 << 7;
}

/// Text alignment (mirrors C TEXT_ALIGN enum).
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextAlign {
    #[default]
    Left = 0,
    Center = 1,
    Right = 2,
}

impl From<u32> for TextAlign {
    fn from(v: u32) -> Self {
        match v {
            1 => TextAlign::Center,
            2 => TextAlign::Right,
            _ => TextAlign::Left,
        }
    }
}

/// Text vertical alignment (mirrors C TEXT_VALIGN enum).
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextValign {
    #[default]
    Top = 0,
    Middle = 1,
    Bottom = 2,
}

impl From<u32> for TextValign {
    fn from(v: u32) -> Self {
        match v {
            1 => TextValign::Middle,
            2 => TextValign::Bottom,
            _ => TextValign::Top,
        }
    }
}

/// Alien song flags (LDAS_FLAGS).
#[allow(
    non_snake_case,
    reason = "C ABI compatibility is fixed during the Rust migration; tracked by PLAN-20260723-RUNTIME-AUTOMATION.P00"
)]
pub mod AlienSongFlags {
    pub const NONE: u32 = 0;
    pub const USE_ALTERNATE: u32 = 1 << 0;
}

/// Animation descriptor matching C `ANIMATION_DESC`.
///
/// Field widths use the smallest Rust integer that fits the C type:
/// - `StartIndex` / `BaseFrameRate` / `RandomFrameRate` / `BaseRestartRate` /
///   `RandomRestartRate` are `COUNT` (u16)
/// - `NumFrames` / `AnimFlags` are `BYTE` (u8)
/// - `BlockMask` is `DWORD` (u32)
///
/// The struct is `repr(C)` so it can be used directly in FFI if needed.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct AnimationDescData {
    /// Index of the first image or color-map entry.
    pub start_index: u16,
    /// Number of frames in the animation.
    pub num_frames: u8,
    /// Animation type flags (RANDOM_ANIM, CIRCULAR_ANIM, YOYO_ANIM, …).
    pub anim_flags: u8,
    /// Minimum interframe delay (game ticks).
    pub base_frame_rate: u16,
    /// Maximum additional interframe delay.
    pub random_frame_rate: u16,
    /// Minimum delay before restarting the animation.
    pub base_restart_rate: u16,
    /// Maximum additional restart delay.
    pub random_restart_rate: u16,
    /// Bit-mask of animation indices that cannot run simultaneously.
    pub block_mask: u32,
}

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
    /// Invalid state for operation
    InvalidState(String),
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
            CommError::InvalidState(s) => write!(f, "Invalid state: {}", s),
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

/// Communication data for an alien encounter — the Rust equivalent of C `LOCDATA`.
///
/// Resource ID fields (`*_res`) are the string-key handles used by the C resource
/// system (opaque from Rust's perspective, stored as raw pointers).  Loaded-handle
/// fields (`alien_frame`, `alien_font`, …) are set after the resource is resolved
/// and hold whatever pointer or index the C graphics/audio layer returns.
///
/// `alien_number_speech` is a borrowed pointer into C-owned memory and must **not**
/// be freed by Rust code.
#[derive(Debug)]
pub struct CommData {
    // -----------------------------------------------------------------------
    // Lifecycle callbacks (addresses of C function pointers, or None)
    // -----------------------------------------------------------------------
    /// Called when entering communications (`init_encounter_func`).
    pub init_encounter_func: Option<usize>,
    /// Called when leaving communications normally (`post_encounter_func`).
    pub post_encounter_func: Option<usize>,
    /// Called for cleanup after the encounter ends (`uninit_encounter_func`).
    pub uninit_encounter_func: Option<usize>,

    // -----------------------------------------------------------------------
    // Resource IDs (C `RESOURCE` = `const char *`, stored as raw pointers)
    // -----------------------------------------------------------------------
    /// Resource key for the alien sprite sheet (`AlienFrameRes`).
    pub alien_frame_res: *const std::ffi::c_char,
    /// Resource key for the alien dialogue font (`AlienFontRes`).
    pub alien_font_res: *const std::ffi::c_char,
    /// Resource key for the alien color-map (`AlienColorMapRes`).
    pub alien_colormap_res: *const std::ffi::c_char,
    /// Resource key for the primary alien music track (`AlienSongRes`).
    pub alien_song_res: *const std::ffi::c_char,
    /// Resource key for the alternate alien music track (`AlienAltSongRes`).
    pub alien_alt_song_res: *const std::ffi::c_char,
    /// Resource key for the conversation phrase string table (`ConversationPhrasesRes`).
    pub conversation_phrases_res: *const std::ffi::c_char,

    // -----------------------------------------------------------------------
    // Text layout
    // -----------------------------------------------------------------------
    /// Foreground text color packed as 0xRRGGBBAA (`AlienTextFColor`).
    pub alien_text_fcolor: u32,
    /// Background text color packed as 0xRRGGBBAA (`AlienTextBColor`).
    pub alien_text_bcolor: u32,
    /// Text baseline X coordinate in screen pixels (`AlienTextBaseline.x`).
    pub alien_text_baseline_x: i16,
    /// Text baseline Y coordinate in screen pixels (`AlienTextBaseline.y`).
    pub alien_text_baseline_y: i16,
    /// Maximum text column width in pixels (`AlienTextWidth`).
    pub alien_text_width: u16,
    /// Horizontal text alignment (`AlienTextAlign`).
    pub alien_text_align: TextAlign,
    /// Vertical text alignment (`AlienTextValign`).
    pub alien_text_valign: TextValign,

    // -----------------------------------------------------------------------
    // Animation descriptors
    // -----------------------------------------------------------------------
    /// Number of active entries in `alien_ambient_array` (`NumAnimations`).
    pub num_animations: u32,
    /// Ambient / background animation slots (`AlienAmbientArray`).
    pub alien_ambient_array: [AnimationDescData; MAX_ANIMATIONS],
    /// Transition animation between silent and talking states (`AlienTransitionDesc`).
    pub alien_transition_desc: AnimationDescData,
    /// Talking animation descriptor (`AlienTalkDesc`).
    pub alien_talk_desc: AnimationDescData,

    // -----------------------------------------------------------------------
    // Song / audio flags
    // -----------------------------------------------------------------------
    /// Flags controlling which song variant to play (`AlienSongFlags` / `LDAS_FLAGS`).
    pub alien_song_flags: u32,

    // -----------------------------------------------------------------------
    // Loaded handles (set after resource resolution, opaque to Rust)
    // -----------------------------------------------------------------------
    /// Loaded alien sprite-sheet frame handle (`AlienFrame`).
    pub alien_frame: *mut c_void,
    /// Loaded alien font handle (`AlienFont`).
    pub alien_font: *mut c_void,
    /// Loaded alien color-map handle (`AlienColorMap`).
    pub alien_color_map: *mut c_void,
    /// Loaded alien music handle (`AlienSong`).
    pub alien_song: *mut c_void,
    /// Loaded conversation phrase string table (`ConversationPhrases`).
    pub conversation_phrases: *mut c_void,

    // -----------------------------------------------------------------------
    // Number-speech generator (borrowed from C; never freed by Rust)
    // -----------------------------------------------------------------------
    /// Pointer to the C `NUMBER_SPEECH_DESC` used for numeric speech synthesis.
    /// This is borrowed from alien-specific C data and must **not** be freed here.
    pub alien_number_speech: *const c_void,
}

impl Default for CommData {
    fn default() -> Self {
        Self {
            init_encounter_func: None,
            post_encounter_func: None,
            uninit_encounter_func: None,
            alien_frame_res: std::ptr::null(),
            alien_font_res: std::ptr::null(),
            alien_colormap_res: std::ptr::null(),
            alien_song_res: std::ptr::null(),
            alien_alt_song_res: std::ptr::null(),
            conversation_phrases_res: std::ptr::null(),
            alien_text_fcolor: 0,
            alien_text_bcolor: 0,
            alien_text_baseline_x: 0,
            alien_text_baseline_y: 0,
            alien_text_width: 0,
            alien_text_align: TextAlign::Left,
            alien_text_valign: TextValign::Top,
            num_animations: 0,
            alien_ambient_array: [AnimationDescData::default(); MAX_ANIMATIONS],
            alien_transition_desc: AnimationDescData::default(),
            alien_talk_desc: AnimationDescData::default(),
            alien_song_flags: 0,
            alien_frame: std::ptr::null_mut(),
            alien_font: std::ptr::null_mut(),
            alien_color_map: std::ptr::null_mut(),
            alien_song: std::ptr::null_mut(),
            conversation_phrases: std::ptr::null_mut(),
            alien_number_speech: std::ptr::null(),
        }
    }
}

// SAFETY: CommData contains raw pointers. All C-side pointers are either
// null-initialized or set by C code before being read; Rust never dereferences
// them. The struct is Send because it is only accessed behind a Mutex/RwLock.
unsafe impl Send for CommData {}
unsafe impl Sync for CommData {}

impl CommData {
    /// Create new empty CommData.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the initialization callback (as function pointer address).
    pub fn set_init_func(&mut self, func: usize) {
        self.init_encounter_func = Some(func);
    }

    /// Set the post-encounter callback.
    pub fn set_post_func(&mut self, func: usize) {
        self.post_encounter_func = Some(func);
    }

    /// Set the cleanup callback.
    pub fn set_uninit_func(&mut self, func: usize) {
        self.uninit_encounter_func = Some(func);
    }

    /// Clear all loaded handles (set to null), leaving resource IDs intact.
    ///
    /// Call this before freeing C-side resources so Rust cannot use stale pointers.
    pub fn clear_handles(&mut self) {
        self.alien_frame = std::ptr::null_mut();
        self.alien_font = std::ptr::null_mut();
        self.alien_color_map = std::ptr::null_mut();
        self.alien_song = std::ptr::null_mut();
        self.conversation_phrases = std::ptr::null_mut();
        self.alien_number_speech = std::ptr::null();
    }
}

/// Ambient sound flags
#[allow(
    non_snake_case,
    reason = "C ABI compatibility is fixed during the Rust migration; tracked by PLAN-20260723-RUNTIME-AUTOMATION.P00"
)]
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
        assert_eq!(data.alien_song_flags, 0);
    }

    #[test]
    fn test_comm_data_default_zeroed() {
        let data = CommData::default();
        assert!(data.alien_frame_res.is_null());
        assert!(data.alien_font_res.is_null());
        assert!(data.alien_frame.is_null());
        assert!(data.alien_number_speech.is_null());
        assert_eq!(data.alien_text_fcolor, 0);
        assert_eq!(data.alien_text_width, 0);
        assert_eq!(data.alien_text_align, TextAlign::Left);
        assert_eq!(data.alien_text_valign, TextValign::Top);
    }

    #[test]
    fn test_animation_desc_data_default() {
        let desc = AnimationDescData::default();
        assert_eq!(desc.start_index, 0);
        assert_eq!(desc.num_frames, 0);
        assert_eq!(desc.anim_flags, 0);
        assert_eq!(desc.block_mask, 0);
    }

    #[test]
    fn test_comm_data_clear_handles() {
        let mut data = CommData::new();
        data.alien_frame = 0x1234 as *mut c_void;
        data.alien_font = 0x5678 as *mut c_void;
        data.clear_handles();
        assert!(data.alien_frame.is_null());
        assert!(data.alien_font.is_null());
        assert!(data.alien_number_speech.is_null());
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
