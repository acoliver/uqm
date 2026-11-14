use crate::comm::Comm;
// Mode-owned conversation instances and retained presentation ABI values.
// @plan PLAN-20260314-COMM.P03
// @requirement EC-REQ-003, DS-REQ-004, SC-REQ-003

use std::ffi::{c_char, c_void};

use crate::battle::velocity::VelocityDesc;
use crate::c_bindings::assert_c_layout;

#[cfg(test)]
use super::races::descriptor::AnimationDescriptor;
use super::races::descriptor::{CommDescriptor, MAX_ANIMATIONS};
use super::types::{CommData, TextAlign, TextValign};

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

    assert_c_layout!(CPoint, crate::graphics::native_abi::POINT { x: x, y: y });
    assert_c_layout!(
        CStamp,
        crate::graphics::native_abi::STAMP {
            origin: origin,
            frame: frame
        }
    );
    assert_c_layout!(
        VelocityDesc,
        crate::ships::native::abi::VELOCITY_DESC {
            travel_angle: travel_angle,
            vector: vector,
            fract: fract,
            error: error,
            incr: incr,
        }
    );
}

/// Loaded resources and callbacks owned by one conversation mode.
#[derive(Default)]
pub struct SessionData {
    pub definition: CommDescriptor,
    pub init_encounter_func: Option<fn(&mut Comm, &mut crate::engine::battle::Cx<'_>)>,
    pub post_encounter_func: Option<fn(&mut Comm, &mut crate::engine::battle::Cx<'_>)>,
    pub uninit_encounter_func: Option<fn(&mut Comm, &mut crate::engine::battle::Cx<'_>) -> u16>,
    pub alien_frame: *mut c_void,
    pub alien_font: *mut c_void,
    pub alien_colormap: *mut c_void,
    pub alien_song: *mut c_void,
    pub conversation_phrases: *mut c_void,
}

impl SessionData {
    pub fn new(definition: CommDescriptor) -> Self {
        Self {
            definition,
            ..Self::default()
        }
    }
}

pub(crate) fn resource_key(key: Option<&'static std::ffi::CStr>) -> *const c_char {
    key.map_or(std::ptr::null(), std::ffi::CStr::as_ptr)
}

fn color_to_u32(c: CColor) -> u32 {
    ((c.r as u32) << 24) | ((c.g as u32) << 16) | ((c.b as u32) << 8) | (c.a as u32)
}

/// Copy a dialogue descriptor into the owned communication state.
pub fn session_view(ld: &SessionData) -> CommData {
    let mut ambient = [Default::default(); MAX_ANIMATIONS];
    let count = usize::from(ld.definition.num_animations).min(MAX_ANIMATIONS);
    ambient[..count].copy_from_slice(&ld.definition.alien_ambient_array[..count]);
    let definition = &ld.definition;
    CommData {
        init_encounter_func: ld.init_encounter_func,
        post_encounter_func: ld.post_encounter_func,
        uninit_encounter_func: ld.uninit_encounter_func,
        alien_frame_res: resource_key(definition.alien_frame_res),
        alien_font_res: resource_key(definition.alien_font_res),
        alien_colormap_res: resource_key(definition.alien_colormap_res),
        alien_song_res: resource_key(definition.alien_song_res),
        alien_alt_song_res: resource_key(definition.alien_alt_song_res),
        conversation_phrases_res: resource_key(definition.conversation_phrases_res),
        alien_text_fcolor: color_to_u32(definition.alien_text_fcolor),
        alien_text_bcolor: color_to_u32(definition.alien_text_bcolor),
        alien_text_baseline_x: definition.alien_text_baseline.x,
        alien_text_baseline_y: definition.alien_text_baseline.y,
        alien_text_width: definition.alien_text_width,
        alien_text_align: TextAlign::from(definition.alien_text_align),
        alien_text_valign: TextValign::from(definition.alien_text_valign),
        alien_song_flags: definition.alien_song_flags,
        alien_transition_desc: definition.alien_transition_desc,
        alien_talk_desc: definition.alien_talk_desc,
        alien_number_speech: definition.alien_number_speech,
        alien_frame: ld.alien_frame,
        alien_font: ld.alien_font,
        alien_color_map: ld.alien_colormap,
        alien_song: ld.alien_song,
        conversation_phrases: ld.conversation_phrases,
        num_animations: u32::from(definition.num_animations),
        alien_ambient_array: ambient,
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
        fn init(_comm: &mut Comm, _cx: &mut crate::engine::battle::Cx<'_>) {
            assert_eq!(CALLS.fetch_add(1, Ordering::SeqCst), 0);
        }
        fn post(_comm: &mut Comm, _cx: &mut crate::engine::battle::Cx<'_>) {
            assert_eq!(CALLS.fetch_add(1, Ordering::SeqCst), 1);
        }
        fn uninit(_comm: &mut Comm, _cx: &mut crate::engine::battle::Cx<'_>) -> u16 {
            assert_eq!(CALLS.fetch_add(1, Ordering::SeqCst), 2);
            17
        }
        let descriptor = SessionData {
            init_encounter_func: Some(init),
            post_encounter_func: Some(post),
            uninit_encounter_func: Some(uninit),
            ..SessionData::default()
        };
        let data = session_view(&descriptor);
        crate::comm::with_test_context(|cx| {
            data.init_encounter_func.unwrap()(&mut Comm::new(), cx)
        });
        crate::comm::with_test_context(|cx| {
            data.post_encounter_func.unwrap()(&mut Comm::new(), cx)
        });
        assert_eq!(
            crate::comm::with_test_context(|cx| data.uninit_encounter_func.unwrap()(
                &mut Comm::new(),
                cx
            )),
            17
        );
        assert_eq!(CALLS.load(Ordering::SeqCst), 3);
        let empty = session_view(&SessionData::default());
        assert!(empty.init_encounter_func.is_none());
        assert!(empty.post_encounter_func.is_none());
        assert!(empty.uninit_encounter_func.is_none());
    }

    #[test]
    fn loaded_state_keeps_content_independent_of_its_resource_handles() {
        let mut first = SessionData::new(crate::comm::races::orz::descriptor::descriptor());
        let second = SessionData::new(first.definition.clone());
        first.alien_frame = std::ptr::dangling_mut();
        first.definition.alien_ambient_array[12].block_mask = 0;
        assert_eq!(first.definition.alien_ambient_array[12].block_mask, 0);
        assert!(second.alien_frame.is_null());
        assert!(second.alien_font.is_null());
        assert!(second.alien_colormap.is_null());
        assert!(second.alien_song.is_null());
        assert!(second.conversation_phrases.is_null());
        assert_eq!(second.definition.alien_ambient_array[12].block_mask, 1024);
        assert_eq!(
            session_view(&second).alien_ambient_array[11].start_index,
            93
        );
        assert_eq!(session_view(&second).alien_ambient_array[12].block_mask, 0);
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
        // AnimationDescriptor has 4×u16 + 2×u8 + pad + u32 — verify it's
        // at least the sum of field sizes (exact may include padding)
        let size = std::mem::size_of::<AnimationDescriptor>();
        // 4×u16(8) + 2×u8(2) + u32(4) = 14 minimum, padded to 16 likely
        assert!(size >= 14, "AnimationDescriptor too small: {}", size);
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
