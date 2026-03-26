// LOCDATA FFI accessors — read C LOCDATA fields into Rust CommData
// @plan PLAN-20260314-COMM.P03
// @requirement EC-REQ-003, DS-REQ-004, SC-REQ-003

use std::ffi::c_void;

use super::types::{AnimationDescData, CommData, TextAlign, TextValign, MAX_ANIMATIONS};

// ---------------------------------------------------------------------------
// C-side accessor declarations
// ---------------------------------------------------------------------------

extern "C" {
    // Race dispatch: returns static LOCDATA* for a conversation ID
    fn c_init_race(comm_id: i32) -> *const c_void;

    // Lifecycle callbacks
    fn c_locdata_get_init_func(locdata: *const c_void) -> Option<unsafe extern "C" fn()>;
    fn c_locdata_get_post_func(locdata: *const c_void) -> Option<unsafe extern "C" fn()>;
    fn c_locdata_get_uninit_func(locdata: *const c_void) -> Option<unsafe extern "C" fn() -> u32>;

    // Resource IDs (const char*)
    fn c_locdata_get_alien_frame_res(locdata: *const c_void) -> *const std::ffi::c_char;
    fn c_locdata_get_alien_font_res(locdata: *const c_void) -> *const std::ffi::c_char;
    fn c_locdata_get_alien_colormap_res(locdata: *const c_void) -> *const std::ffi::c_char;
    fn c_locdata_get_alien_song_res(locdata: *const c_void) -> *const std::ffi::c_char;
    fn c_locdata_get_alien_alt_song_res(locdata: *const c_void) -> *const std::ffi::c_char;
    fn c_locdata_get_conversation_phrases_res(locdata: *const c_void) -> *const std::ffi::c_char;

    // Text layout
    fn c_locdata_get_text_fcolor(locdata: *const c_void) -> u32;
    fn c_locdata_get_text_bcolor(locdata: *const c_void) -> u32;
    fn c_locdata_get_text_baseline_x(locdata: *const c_void) -> i16;
    fn c_locdata_get_text_baseline_y(locdata: *const c_void) -> i16;
    fn c_locdata_get_text_width(locdata: *const c_void) -> u16;
    fn c_locdata_get_text_align(locdata: *const c_void) -> u32;
    fn c_locdata_get_text_valign(locdata: *const c_void) -> u32;

    // Song flags
    fn c_locdata_get_song_flags(locdata: *const c_void) -> u32;

    // Animation counts and descriptors
    fn c_locdata_get_num_animations(locdata: *const c_void) -> u32;
    fn c_locdata_get_ambient_anim(locdata: *const c_void, index: u32, out: *mut AnimationDescData);
    fn c_locdata_get_transition_desc(locdata: *const c_void, out: *mut AnimationDescData);
    fn c_locdata_get_talk_desc(locdata: *const c_void, out: *mut AnimationDescData);

    // Number speech (borrowed pointer, valid for encounter lifetime)
    fn c_locdata_get_number_speech(locdata: *const c_void) -> *const c_void;

    // Loaded handles
    fn c_locdata_get_alien_frame(locdata: *const c_void) -> *mut c_void;
    fn c_locdata_get_alien_font(locdata: *const c_void) -> *mut c_void;
    fn c_locdata_get_alien_colormap(locdata: *const c_void) -> *mut c_void;
    fn c_locdata_get_alien_song(locdata: *const c_void) -> *mut c_void;
    fn c_locdata_get_conversation_phrases(locdata: *const c_void) -> *mut c_void;
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
    unsafe { c_init_race(comm_id) }
}

/// Read all fields from a C `LOCDATA*` into a Rust-owned `CommData`.
///
/// # Safety
/// `locdata_ptr` must be a valid, non-null pointer to a C `LOCDATA` struct.
pub unsafe fn read_locdata_from_c(locdata_ptr: *const c_void) -> CommData {
    let mut data = CommData::default();

    // Lifecycle callbacks — store as raw usize addresses
    unsafe {
        if let Some(f) = c_locdata_get_init_func(locdata_ptr) {
            data.init_encounter_func = Some(f as usize);
        }
        if let Some(f) = c_locdata_get_post_func(locdata_ptr) {
            data.post_encounter_func = Some(f as usize);
        }
        if let Some(f) = c_locdata_get_uninit_func(locdata_ptr) {
            data.uninit_encounter_func = Some(f as usize);
        }

        // Resource IDs
        data.alien_frame_res = c_locdata_get_alien_frame_res(locdata_ptr);
        data.alien_font_res = c_locdata_get_alien_font_res(locdata_ptr);
        data.alien_colormap_res = c_locdata_get_alien_colormap_res(locdata_ptr);
        data.alien_song_res = c_locdata_get_alien_song_res(locdata_ptr);
        data.alien_alt_song_res = c_locdata_get_alien_alt_song_res(locdata_ptr);
        data.conversation_phrases_res = c_locdata_get_conversation_phrases_res(locdata_ptr);

        // Text layout
        data.alien_text_fcolor = c_locdata_get_text_fcolor(locdata_ptr);
        data.alien_text_bcolor = c_locdata_get_text_bcolor(locdata_ptr);
        data.alien_text_baseline_x = c_locdata_get_text_baseline_x(locdata_ptr);
        data.alien_text_baseline_y = c_locdata_get_text_baseline_y(locdata_ptr);
        data.alien_text_width = c_locdata_get_text_width(locdata_ptr);
        data.alien_text_align = TextAlign::from(c_locdata_get_text_align(locdata_ptr));
        data.alien_text_valign = TextValign::from(c_locdata_get_text_valign(locdata_ptr));

        // Song flags
        data.alien_song_flags = c_locdata_get_song_flags(locdata_ptr);

        // Animation descriptors
        data.num_animations = c_locdata_get_num_animations(locdata_ptr);
        let n = std::cmp::min(data.num_animations as usize, MAX_ANIMATIONS);
        for i in 0..n {
            c_locdata_get_ambient_anim(locdata_ptr, i as u32, &mut data.alien_ambient_array[i]);
        }
        c_locdata_get_transition_desc(locdata_ptr, &mut data.alien_transition_desc);
        c_locdata_get_talk_desc(locdata_ptr, &mut data.alien_talk_desc);

        // Number speech (borrowed)
        data.alien_number_speech = c_locdata_get_number_speech(locdata_ptr);

        // Loaded handles
        data.alien_frame = c_locdata_get_alien_frame(locdata_ptr);
        data.alien_font = c_locdata_get_alien_font(locdata_ptr);
        data.alien_color_map = c_locdata_get_alien_colormap(locdata_ptr);
        data.alien_song = c_locdata_get_alien_song(locdata_ptr);
        data.conversation_phrases = c_locdata_get_conversation_phrases(locdata_ptr);
    }

    data
}

// ===========================================================================
// Tests — unit-testable without C linkage
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

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
