// LOCDATA direct FFI access — reads C LOCDATA struct fields without C bridge functions
// @plan PLAN-20260314-COMM.P03
// @requirement EC-REQ-003, DS-REQ-004, SC-REQ-003

use std::ffi::{c_char, c_void};
use std::sync::Mutex;

use crate::battle::velocity::VelocityDesc;
use crate::c_bindings::{assert_c_layout, global_state};

use super::types::{AnimationDescData, CommData, TextAlign, TextValign, MAX_ANIMATIONS};

pub static mut COMM_DATA: CLocData = unsafe { std::mem::zeroed() };

// ===========================================================================
// C-compatible repr(C) types — mirror the C struct layout exactly
// ===========================================================================

/// C `Color` struct: { BYTE r, g, b, a } — 4 bytes.
pub use crate::graphics::native_abi::Color as CColor;

/// C `POINT` struct: { COORD x, y } where COORD = SIZE = SWORD = i16.
pub use crate::graphics::native_abi::point as CPoint;

/// C `STAMP` struct: a screen origin and an opaque `FRAME` pointer.
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct CStamp {
    pub origin: CPoint,
    pub frame: *mut c_void,
}

/// C `RECT` struct (units.h / comm.h).
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct CRect {
    pub corner: CPoint,
    pub width: i16,
    pub height: i16,
}

const _: () = {
    use crate::graphics::native_abi::{EXTENT, RECT};
    assert!(std::mem::size_of::<CRect>() == std::mem::size_of::<RECT>());
    assert!(std::mem::align_of::<CRect>() == std::mem::align_of::<RECT>());
    assert!(std::mem::offset_of!(CRect, corner) == std::mem::offset_of!(RECT, corner));
    assert!(
        std::mem::offset_of!(CRect, width)
            == std::mem::offset_of!(RECT, extent) + std::mem::offset_of!(EXTENT, width)
    );
    assert!(
        std::mem::offset_of!(CRect, height)
            == std::mem::offset_of!(RECT, extent) + std::mem::offset_of!(EXTENT, height)
    );
};

pub use crate::engine::battle::schema::GAME_STATE as CGameState;
pub use crate::ships::native::abi::SIS_STATE as CSisState;

mod layout {
    use super::*;

    assert_c_layout!(CPoint, global_state::POINT { x: x, y: y });
    assert_c_layout!(
        CStamp,
        global_state::STAMP {
            origin: origin,
            frame: frame
        }
    );
    assert_c_layout!(
        VelocityDesc,
        global_state::VELOCITY_DESC {
            travel_angle: travel_angle,
            vector: vector,
            fract: fract,
            error: error,
            incr: incr,
        }
    );
    assert_c_layout!(
        CAnimationDesc,
        global_state::ANIMATION_DESC {
            start_index: StartIndex,
            num_frames: NumFrames,
            anim_flags: AnimFlags,
            base_frame_rate: BaseFrameRate,
            random_frame_rate: RandomFrameRate,
            base_restart_rate: BaseRestartRate,
            random_restart_rate: RandomRestartRate,
            block_mask: BlockMask,
        }
    );
    assert_c_layout!(
        CLocData,
        global_state::LOCDATA {
            init_encounter_func: init_encounter_func,
            post_encounter_func: post_encounter_func,
            uninit_encounter_func: uninit_encounter_func,
            alien_frame_res: AlienFrameRes,
            alien_font_res: AlienFontRes,
            alien_text_fcolor: AlienTextFColor,
            alien_text_bcolor: AlienTextBColor,
            alien_text_baseline: AlienTextBaseline,
            alien_text_width: AlienTextWidth,
            alien_text_align: AlienTextAlign,
            alien_text_valign: AlienTextValign,
            alien_colormap_res: AlienColorMapRes,
            alien_song_res: AlienSongRes,
            alien_alt_song_res: AlienAltSongRes,
            alien_song_flags: AlienSongFlags,
            conversation_phrases_res: ConversationPhrasesRes,
            num_animations: NumAnimations,
            alien_ambient_array: AlienAmbientArray,
            alien_transition_desc: AlienTransitionDesc,
            alien_talk_desc: AlienTalkDesc,
            alien_number_speech: AlienNumberSpeech,
            alien_frame: AlienFrame,
            alien_font: AlienFont,
            alien_colormap: AlienColorMap,
            alien_song: AlienSong,
            conversation_phrases: ConversationPhrases,
        }
    );
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
    pub init_encounter_func: Option<fn(&mut crate::engine::battle::Cx<'_>)>,
    pub post_encounter_func: Option<fn(&mut crate::engine::battle::Cx<'_>)>,
    pub uninit_encounter_func: Option<fn(&mut crate::engine::battle::Cx<'_>) -> u16>,

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
// Public API
// ---------------------------------------------------------------------------

/// Copy a dialogue descriptor into the owned communication state.
pub fn read_locdata(ld: &CLocData) -> CommData {
    let mut alien_ambient_array = [AnimationDescData::default(); MAX_ANIMATIONS];
    let count = usize::from(ld.num_animations).min(MAX_ANIMATIONS);
    for (target, source) in alien_ambient_array[..count]
        .iter_mut()
        .zip(&ld.alien_ambient_array[..count])
    {
        *target = anim_desc_to_rust(source);
    }
    CommData {
        init_encounter_func: ld.init_encounter_func,
        post_encounter_func: ld.post_encounter_func,
        uninit_encounter_func: ld.uninit_encounter_func,
        alien_frame_res: ld.alien_frame_res,
        alien_font_res: ld.alien_font_res,
        alien_colormap_res: ld.alien_colormap_res,
        alien_song_res: ld.alien_song_res,
        alien_alt_song_res: ld.alien_alt_song_res,
        conversation_phrases_res: ld.conversation_phrases_res,
        alien_text_fcolor: color_to_u32(ld.alien_text_fcolor),
        alien_text_bcolor: color_to_u32(ld.alien_text_bcolor),
        alien_text_baseline_x: ld.alien_text_baseline.x,
        alien_text_baseline_y: ld.alien_text_baseline.y,
        alien_text_width: ld.alien_text_width,
        alien_text_align: TextAlign::from(ld.alien_text_align),
        alien_text_valign: TextValign::from(ld.alien_text_valign),
        alien_song_flags: ld.alien_song_flags,
        alien_transition_desc: anim_desc_to_rust(&ld.alien_transition_desc),
        alien_talk_desc: anim_desc_to_rust(&ld.alien_talk_desc),
        alien_number_speech: ld.alien_number_speech,
        alien_frame: ld.alien_frame,
        alien_font: ld.alien_font,
        alien_color_map: ld.alien_colormap,
        alien_song: ld.alien_song,
        conversation_phrases: ld.conversation_phrases,
        num_animations: u32::from(ld.num_animations),
        alien_ambient_array,
    }
}

// ===========================================================================
// Tests — unit-testable without C linkage
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn descriptor_conversion_preserves_typed_lifecycle_callbacks() {
        use std::sync::atomic::{AtomicU8, Ordering};
        static CALLS: AtomicU8 = AtomicU8::new(0);
        fn init(_cx: &mut crate::engine::battle::Cx<'_>) {
            assert_eq!(CALLS.fetch_add(1, Ordering::SeqCst), 0);
        }
        fn post(_cx: &mut crate::engine::battle::Cx<'_>) {
            assert_eq!(CALLS.fetch_add(1, Ordering::SeqCst), 1);
        }
        fn uninit(_cx: &mut crate::engine::battle::Cx<'_>) -> u16 {
            assert_eq!(CALLS.fetch_add(1, Ordering::SeqCst), 2);
            17
        }
        let descriptor = CLocData {
            init_encounter_func: Some(init),
            post_encounter_func: Some(post),
            uninit_encounter_func: Some(uninit),
            ..CLocData::default()
        };
        let data = read_locdata(&descriptor);
        crate::comm::with_test_context(data.init_encounter_func.unwrap());
        crate::comm::with_test_context(data.post_encounter_func.unwrap());
        assert_eq!(
            crate::comm::with_test_context(data.uninit_encounter_func.unwrap()),
            17
        );
        assert_eq!(CALLS.load(Ordering::SeqCst), 3);
        let empty = read_locdata(&CLocData::default());
        assert!(empty.init_encounter_func.is_none());
        assert!(empty.post_encounter_func.is_none());
        assert!(empty.uninit_encounter_func.is_none());
    }

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
    fn retained_save_records_keep_their_abi_layout() {
        assert_eq!(std::mem::size_of::<CSisState>(), 124);
        assert_eq!(std::mem::offset_of!(CSisState, module_slots), 22);
        assert_eq!(std::mem::offset_of!(CSisState, drive_slots), 38);
        assert_eq!(std::mem::offset_of!(CSisState, ship_name), 74);
        assert_eq!(std::mem::size_of::<CStamp>(), 16);
        assert_eq!(std::mem::size_of::<CGameState>(), 104);
        assert_eq!(std::mem::offset_of!(CGameState, game_clock), 40);
        assert_eq!(std::mem::offset_of!(CGameState, ship_stamp), 56);
        assert_eq!(std::mem::offset_of!(CGameState, battle_group_ref), 96);
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
