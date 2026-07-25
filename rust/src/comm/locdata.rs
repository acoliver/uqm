// LOCDATA direct FFI access — reads C LOCDATA struct fields without C bridge functions
// @plan PLAN-20260314-COMM.P03
// @requirement EC-REQ-003, DS-REQ-004, SC-REQ-003

use std::ffi::{c_char, c_void};
use std::sync::Mutex;

use super::types::{AnimationDescData, CommData, TextAlign, TextValign, MAX_ANIMATIONS};

#[cfg(not(test))]
mod comm_data_extern {
    extern "C" {
        #[link_name = "CommData"]
        pub static mut COMM_DATA: super::CLocData;
    }
}

#[cfg(not(test))]
pub use comm_data_extern::COMM_DATA;

// ===========================================================================
// C-compatible repr(C) types — mirror the C struct layout exactly
// ===========================================================================

/// C `Color` struct: { BYTE r, g, b, a } — 4 bytes.
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct CColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

/// C `POINT` struct: { COORD x, y } where COORD = SIZE = SWORD = i16.
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct CPoint {
    pub x: i16,
    pub y: i16,
}

/// C `RECT` struct (units.h / comm.h).
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct CRect {
    pub corner: CPoint,
    pub width: i16,
    pub height: i16,
}

/// C `SIS_STATE` struct (sis.h) — only the fields we need.
/// CommanderName/ShipName/PlanetName are `UNICODE[SIS_NAME_SIZE]` = `char[16]`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct CSisState {
    pub star_name: [u8; 16],
    pub ship_name: [u8; 16],
    pub commander_name: [u8; 16],
    pub planet_name: [u8; 16],
}

/// Minimal C `GLOBDATA` — only the fields we access from Rust.
/// SIS_state is at offset 0 in the C struct, Game_state follows.
/// This is a partial mirror; we only need SIS_state and Game_state.GameState.
#[repr(C)]
pub struct CGlobData {
    pub sis_state: CSisState,
    // Game_state follows — we access GameState via the getGameState FFI
    // using the pointer to the GameState array within Game_state.
    // The exact layout after SIS_state is complex; we only need
    // the SIS_state fields directly.
}

/// C `ANIMATION_DESC` struct (commanim.h).
/// Fields in declaration order, matching the C layout for direct FFI reads.
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct CAnimationDesc {
    pub start_index: u16,
    pub num_frames: u8,
    pub anim_flags: u8,
    pub base_frame_rate: u16,
    pub random_frame_rate: u16,
    pub base_restart_rate: u16,
    pub random_restart_rate: u16,
    pub block_mask: u32,
}

/// This is a direct `#[repr(C)]` mirror of the C struct, allowing Rust to
/// read all fields via pointer cast without per-field C bridge functions.
#[repr(C)]
#[derive(Default)]
pub struct CLocData {
    // Function pointers (3 × 8 bytes on 64-bit)
    pub init_encounter_func: *const c_void,
    pub post_encounter_func: *const c_void,
    pub uninit_encounter_func: *const c_void,

    // Resource IDs (const char* pointers)
    pub alien_frame_res: *const c_char,
    pub alien_font_res: *const c_char,

    // Colors (4 bytes each)
    pub alien_text_fcolor: CColor,
    pub alien_text_bcolor: CColor,

    // Text baseline point (4 bytes)
    pub alien_text_baseline: CPoint,

    // Text layout scalars
    pub alien_text_width: u16,  // COUNT
    pub alien_text_align: u32,  // TEXT_ALIGN enum (int-sized)
    pub alien_text_valign: u32, // TEXT_VALIGN enum (int-sized)

    // More resource IDs
    pub alien_colormap_res: *const c_char,
    pub alien_song_res: *const c_char,
    pub alien_alt_song_res: *const c_char,

    // Song flags
    pub alien_song_flags: u32, // LDAS_FLAGS = DWORD

    // Conversation phrases resource
    pub conversation_phrases_res: *const c_char,

    // Animation array
    pub num_animations: u16, // COUNT
    pub alien_ambient_array: [CAnimationDesc; MAX_ANIMATIONS],

    // Transition / talk animation descriptors
    pub alien_transition_desc: CAnimationDesc,
    pub alien_talk_desc: CAnimationDesc,

    // Number speech (borrowed pointer)
    pub alien_number_speech: *const c_void,

    // Loaded handles (opaque pointers)
    pub alien_frame: *mut c_void,          // FRAME
    pub alien_font: *mut c_void,           // FONT
    pub alien_colormap: *mut c_void,       // COLORMAP = STRING
    pub alien_song: *mut c_void,           // MUSIC_REF
    pub conversation_phrases: *mut c_void, // STRING
}
fn color_to_u32(c: CColor) -> u32 {
    ((c.r as u32) << 24) | ((c.g as u32) << 16) | ((c.b as u32) << 8) | (c.a as u32)
}

/// Convert a C `CAnimationDesc` to the Rust-friendly `AnimationDescData`.
#[inline]
fn anim_desc_to_rust(src: &CAnimationDesc) -> AnimationDescData {
    AnimationDescData {
        start_index: src.start_index,
        num_frames: src.num_frames,
        anim_flags: src.anim_flags,
        base_frame_rate: src.base_frame_rate,
        random_frame_rate: src.random_frame_rate,
        base_restart_rate: src.base_restart_rate,
        random_restart_rate: src.random_restart_rate,
        block_mask: src.block_mask,
    }
}

// ===========================================================================
// CommData singleton
// ===========================================================================

/// Global CommData singleton. Populated by `sync_comm_data_from_locdata()`
/// when C's InitCommunication copies LOCDATA to CommData.
/// Rust code reads from this instead of going through C accessors.
static GLOBAL_COMM_DATA: Mutex<Option<CommData>> = Mutex::new(None);

/// Store a CommData into the global singleton.
pub fn set_comm_data(data: CommData) {
    let mut guard = GLOBAL_COMM_DATA.lock().expect("GLOBAL_COMM_DATA poisoned");
    *guard = Some(data);
}

/// Clear the global CommData singleton (e.g., when ending an encounter).
pub fn clear_comm_data() {
    let mut guard = GLOBAL_COMM_DATA.lock().expect("GLOBAL_COMM_DATA poisoned");
    *guard = None;
}

/// Get a clone of the global CommData, if populated.
pub fn get_comm_data() -> Option<CommData> {
    let guard = GLOBAL_COMM_DATA.lock().expect("GLOBAL_COMM_DATA poisoned");
    guard.clone()
}

/// Check whether the global CommData has been populated.
pub fn has_comm_data() -> bool {
    let guard = GLOBAL_COMM_DATA.lock().expect("GLOBAL_COMM_DATA poisoned");
    guard.is_some()
}

// ---------------------------------------------------------------------------
// Direct C FFI: init_race (still a C global, not a bridge function)
// ---------------------------------------------------------------------------

extern "C" {
    /// C's `init_race()` — dispatches to the correct race's `init_*_comm()`
    /// and returns a static `LOCDATA*`.  This calls our Rust
    /// `rust_init_race_dispatch()` first, falling back to the C switch.
    #[allow(
        clashing_extern_declarations,
        reason = "C ABI compatibility is fixed during the Rust migration; tracked by PLAN-20260723-RUNTIME-AUTOMATION.P00"
    )]
    fn c_init_race(comm_id: u32) -> *const CLocData;
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Dispatch to the C-owned `init_race` switch and return the raw LOCDATA pointer.
///
/// Returns null if the conversation ID is unrecognized.
///
/// # Safety
/// Must be called from the game thread. The returned pointer is valid for
/// the lifetime of the encounter.
pub unsafe fn init_race(comm_id: i32) -> *const c_void {
    unsafe { c_init_race(comm_id as u32) as *const c_void }
}

/// Read all fields from a C `LOCDATA*` into a Rust-owned `CommData`.
///
/// Uses direct `#[repr(C)]` struct access — no per-field C bridge functions.
///
/// # Safety
/// `locdata_ptr` must be a valid, non-null pointer to a C `LOCDATA` struct.
pub unsafe fn read_locdata_from_c(locdata_ptr: *const c_void) -> CommData {
    let ld: &CLocData = unsafe { &*(locdata_ptr as *const CLocData) };

    let mut data = CommData::default();

    // Lifecycle callbacks — store as raw usize addresses
    if !ld.init_encounter_func.is_null() {
        data.init_encounter_func = Some(ld.init_encounter_func as usize);
    }
    if !ld.post_encounter_func.is_null() {
        data.post_encounter_func = Some(ld.post_encounter_func as usize);
    }
    if !ld.uninit_encounter_func.is_null() {
        data.uninit_encounter_func = Some(ld.uninit_encounter_func as usize);
    }

    // Resource IDs
    data.alien_frame_res = ld.alien_frame_res;
    data.alien_font_res = ld.alien_font_res;
    data.alien_colormap_res = ld.alien_colormap_res;
    data.alien_song_res = ld.alien_song_res;
    data.alien_alt_song_res = ld.alien_alt_song_res;
    data.conversation_phrases_res = ld.conversation_phrases_res;

    // Text layout
    data.alien_text_fcolor = color_to_u32(ld.alien_text_fcolor);
    data.alien_text_bcolor = color_to_u32(ld.alien_text_bcolor);
    data.alien_text_baseline_x = ld.alien_text_baseline.x;
    data.alien_text_baseline_y = ld.alien_text_baseline.y;
    data.alien_text_width = ld.alien_text_width;
    data.alien_text_align = TextAlign::from(ld.alien_text_align);
    data.alien_text_valign = TextValign::from(ld.alien_text_valign);

    // Song flags
    data.alien_song_flags = ld.alien_song_flags;

    // Animation descriptors
    data.num_animations = ld.num_animations as u32;
    let n = std::cmp::min(data.num_animations as usize, MAX_ANIMATIONS);
    for i in 0..n {
        data.alien_ambient_array[i] = anim_desc_to_rust(&ld.alien_ambient_array[i]);
    }
    data.alien_transition_desc = anim_desc_to_rust(&ld.alien_transition_desc);
    data.alien_talk_desc = anim_desc_to_rust(&ld.alien_talk_desc);

    // Number speech (borrowed)
    data.alien_number_speech = ld.alien_number_speech;

    // Loaded handles
    data.alien_frame = ld.alien_frame;
    data.alien_font = ld.alien_font;
    data.alien_color_map = ld.alien_colormap;
    data.alien_song = ld.alien_song;
    data.conversation_phrases = ld.conversation_phrases;

    data
}

/// Sync C's LOCDATA into Rust's global CommData singleton.
///
/// Called from C's `InitCommunication` after `CommData = *LocDataPtr`.
/// This makes Rust's CommData the authoritative copy for Rust-side reads.
///
/// # Safety
/// `locdata_ptr` must be a valid, non-null pointer to a C `LOCDATA` struct,
/// or null to clear the singleton.
#[no_mangle]
pub unsafe extern "C" fn rust_sync_comm_data(locdata_ptr: *const c_void) {
    if locdata_ptr.is_null() {
        clear_comm_data();
        return;
    }
    let data = unsafe { read_locdata_from_c(locdata_ptr) };
    set_comm_data(data);
}

/// Clear Rust's global CommData singleton.
///
/// Called from C when an encounter ends and CommData is no longer valid.
///
/// # Safety
///
/// This is an FFI function called from C. It accesses a global Mutex; the
/// caller must ensure no other thread is simultaneously accessing the
/// CommData singleton.
#[no_mangle]
pub unsafe extern "C" fn rust_clear_comm_data() {
    clear_comm_data();
}

// ===========================================================================
// Tests — unit-testable without C linkage
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_c_locdata_struct_layout() {
        // CLocData must be repr(C) and have a reasonable size.
        // The exact size depends on platform pointer width and padding,
        // but it must be non-zero and at least 3 pointers (24 bytes on 64-bit).
        let size = std::mem::size_of::<CLocData>();
        assert!(size > 0, "CLocData has zero size");
        assert!(
            size >= 24,
            "CLocData too small for 3 function pointers: {}",
            size
        );
    }

    #[test]
    fn test_animation_desc_data_repr_c_size() {
        // AnimationDescData has 4×u16 + 2×u8 + pad + u32 — verify it's
        // at least the sum of field sizes (exact may include padding)
        let size = std::mem::size_of::<AnimationDescData>();
        // 4×u16(8) + 2×u8(2) + u32(4) = 14 minimum, padded to 16 likely
        assert!(size >= 14, "AnimationDescData too small: {}", size);
    }

    #[test]
    fn test_text_align_from_u32() {
        assert_eq!(TextAlign::from(0), TextAlign::Left);
        assert_eq!(TextAlign::from(1), TextAlign::Center);
        assert_eq!(TextAlign::from(2), TextAlign::Right);
        assert_eq!(TextAlign::from(99), TextAlign::Left);
    }

    #[test]
    fn test_text_valign_from_u32() {
        assert_eq!(TextValign::from(0), TextValign::Top);
        assert_eq!(TextValign::from(1), TextValign::Middle);
        assert_eq!(TextValign::from(2), TextValign::Bottom);
        assert_eq!(TextValign::from(99), TextValign::Top);
    }
}
