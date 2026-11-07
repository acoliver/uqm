//! Live engine ABI for race ports. Unlike `types::RaceDesc`, these records can
//! be returned directly to the engine's `init_<race>` callers.

pub(crate) fn copy_display_name(target: &mut [std::ffi::c_char], source: &[u8]) {
    target.fill(0);
    let length = source
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(source.len());
    let count = length.min(target.len().saturating_sub(1));
    for (target, source) in target.iter_mut().zip(source).take(count) {
        *target = *source as std::ffi::c_char;
    }
}

#[allow(
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals,
    dead_code,
    clippy::all
)]
pub mod abi {
    #[cfg(not(test))]
    pub use crate::ships::DescriptorLoader;

    include!("../abi/ship_abi.rs");

    #[cfg(not(test))]
    pub use super::engine::{
        abs_frame_index as SetAbsFrameIndex, abs_string_index as SetAbsStringTableIndex,
        capture_drawable as CaptureDrawable, dec_frame_index as DecFrameIndex,
        destroy_drawable as DestroyDrawable, destroy_music as DestroyMusic,
        destroy_sound as DestroySound, destroy_string_table as DestroyStringTable,
        draw_filled_rectangle as DrawFilledRectangle, draw_filled_stamp as DrawFilledStamp,
        draw_sis_title as DrawSISTitle, draw_stamp as DrawStamp, draw_text as font_DrawText,
        drawables_intersect as DrawablesIntersect, equ_frame_index as SetEquFrameIndex,
        frame_hot as GetFrameHot, frame_rect as GetFrameRect, in_hq_space as in_hqspace,
        in_hyper_space, inc_frame_index as IncFrameIndex, music_playing as PLRPlaying,
        play_music as PlayMusic, release_drawable as ReleaseDrawable,
        release_string_table as ReleaseStringTable, screen_transition as ScreenTransition,
        set_context as SetContext, set_context_font as SetContextFont,
        set_flash_rect as SetFlashRect, set_mipmap as TFB_DrawScreen_SetMipmap,
        set_transition_source as SetTransitionSource, string_address as GetStringAddress,
    };

    // Bindgen does not translate these cast-expression macros from controls.h
    // and intel.h. Keep their C operand widths at the shared ABI boundary.
    pub const BATTLE_LEFT: BATTLE_INPUT_STATE = 1 << 0;
    pub const BATTLE_RIGHT: BATTLE_INPUT_STATE = 1 << 1;
    pub const BATTLE_THRUST: BATTLE_INPUT_STATE = 1 << 2;
    pub const BATTLE_WEAPON: BATTLE_INPUT_STATE = 1 << 3;
    pub const BATTLE_SPECIAL: BATTLE_INPUT_STATE = 1 << 4;
    pub const STANDARD_RATING: BYTE = 1 << 4;
    pub const GOOD_RATING: BYTE = 1 << 5;
    pub const AWESOME_RATING: BYTE = 1 << 6;
    #[cfg(not(test))]
    pub use super::engine::sleep_thread as SleepThread;
    #[cfg(not(test))]
    pub use super::engine::{
        draw_batch as DrawBatch, draw_rectangle as DrawRectangle, frame_count as GetFrameCount,
        frame_index as GetFrameIndex, rel_frame_index as SetRelFrameIndex,
    };
    #[cfg(not(test))]
    pub use crate::battle::encount_native::fleet_is_infinite;
    #[cfg(not(test))]
    pub use crate::battle::galaxy::move_galaxy;
    #[cfg(not(test))]
    pub use crate::math::seed_random as TFB_SeedRandom;

    #[cfg(not(test))]
    pub use crate::battle::weapon_native::{
        initialize_laser, initialize_missile, modify_silhouette, track_ship, weapon_collision,
    };
    #[cfg(not(test))]
    pub use crate::battle::world_elements::{abandon_ship, engine::ElementDamage};
}

#[cfg(not(test))]
mod engine {
    use super::abi;

    // Keep opaque-pointer declarations identical to existing engine imports.

    use crate::graphics::frame_abi::DrawStamp;

    use crate::graphics::frame_abi::DrawFilledStamp;

    use crate::graphics::frame_abi::DrawFilledRectangle;

    use crate::graphics::font_abi::font_DrawText;

    use crate::graphics::font_abi::SetContextFont;

    use crate::ui::sis::SetFlashRect;

    use crate::ui::sis::DrawSISTitle;

    use crate::graphics::gfx_common_abi::SetTransitionSource;

    use crate::graphics::frame_abi::DrawRectangle;

    use crate::sound::heart_ffi::DestroySound;

    /// # Safety
    /// `table` must be null or a live table no longer in use.
    pub unsafe fn destroy_string_table(table: abi::STRING_TABLE) -> abi::BOOLEAN {
        crate::strings::string_table::DestroyStringTable(table.cast()) as abi::BOOLEAN
    }

    /// # Safety
    /// `entry` must be null or a live string-table entry.
    pub unsafe fn release_string_table(entry: abi::STRING) -> abi::STRING_TABLE {
        crate::strings::string_table::ReleaseStringTable(entry.cast()).cast()
    }

    /// # Safety
    /// `entry` must be null or a live string-table entry.
    pub unsafe fn abs_string_index(entry: abi::STRING, index: abi::COUNT) -> abi::STRING {
        crate::strings::string_table::SetAbsStringTableIndex(entry.cast(), index).cast()
    }

    /// # Safety
    /// `entry` must be null or a live string-table entry; returned storage must not outlive its table.
    pub unsafe fn string_address(entry: abi::STRING) -> *mut std::ffi::c_char {
        crate::strings::string_table::GetStringAddress(entry.cast()).cast()
    }

    /// # Safety
    /// `context` must be null or a live context owned by the game thread.
    pub unsafe fn set_context(context: abi::CONTEXT) -> abi::CONTEXT {
        crate::battle::c_bridge::SetContext(context.cast()).cast()
    }

    /// # Safety
    /// Both controls and their frames must be live and owned by the game thread.
    pub unsafe fn drawables_intersect(
        a: *mut abi::INTERSECT_CONTROL,
        b: *mut abi::INTERSECT_CONTROL,
        max_time: abi::TIME_VALUE,
    ) -> abi::TIME_VALUE {
        crate::graphics::geometry_ffi::DrawablesIntersect(a.cast(), b.cast(), max_time)
    }

    /// # Safety
    /// `drawable` must be null or a live drawable no longer in use.
    pub unsafe fn destroy_drawable(drawable: abi::DRAWABLE) -> abi::BOOLEAN {
        crate::battle::c_bridge::DestroyDrawable(drawable.cast())
    }

    /// # Safety
    /// `frame` must be live and `rect` must be writable on the game thread.
    pub unsafe fn frame_rect(frame: abi::FRAME, rect: *mut abi::RECT) {
        crate::battle::c_bridge::GetFrameRect(frame.cast(), rect.cast());
    }

    /// # Safety
    /// `frame` must be a live drawable frame.
    pub unsafe fn frame_hot(frame: abi::FRAME) -> abi::HOT_SPOT {
        let point = crate::battle::c_bridge::GetFrameHot(frame.cast());
        abi::HOT_SPOT {
            x: point.x,
            y: point.y,
        }
    }

    /// # Safety
    /// `music` must be null or a live music handle no longer playing.
    pub unsafe fn destroy_music(music: abi::MUSIC_REF) {
        crate::battle::c_bridge::DestroyMusic(music.cast());
    }

    /// # Safety
    /// `music` must be a live music handle or the engine wildcard handle.
    pub unsafe fn music_playing(music: abi::MUSIC_REF) -> abi::BOOLEAN {
        crate::battle::c_bridge::PLRPlaying(music.cast()) as abi::BOOLEAN
    }

    /// # Safety
    /// `music` must be live and audio state must be owned by the game thread.
    pub unsafe fn play_music(music: abi::MUSIC_REF, continuous: abi::BOOLEAN, priority: abi::BYTE) {
        crate::battle::c_bridge::PlayMusic(music.cast(), continuous, priority);
    }

    /// # Safety
    /// `rect` must be null or live; the game thread must own drawing state.
    pub unsafe fn screen_transition(transition: i32, rect: *const abi::RECT) {
        crate::battle::c_bridge::ScreenTransition(transition, rect.cast());
    }

    /// # Safety
    /// Images must be live and owned by the graphics thread.
    pub unsafe fn set_mipmap(
        image: *mut abi::TFB_Image,
        mipmap: *mut abi::TFB_Image,
        hot_x: i32,
        hot_y: i32,
    ) {
        crate::battle::c_bridge::TFB_DrawScreen_SetMipmap(
            image.cast(),
            mipmap.cast(),
            hot_x,
            hot_y,
        );
    }

    /// # Safety
    /// Activity state must be initialized and accessed only on the game thread.
    pub fn in_hq_space(world: &crate::engine::World) -> abi::BOOLEAN {
        crate::battle::c_bridge::in_hqspace(world) as abi::BOOLEAN
    }

    /// # Safety
    /// The argument and current graphics context must be live on the game thread.
    pub unsafe fn draw_stamp(stamp: *mut abi::STAMP) {
        DrawStamp(stamp.cast());
    }

    /// # Safety
    /// The argument and current graphics context must be live on the game thread.
    pub unsafe fn draw_filled_stamp(stamp: *mut abi::STAMP) {
        DrawFilledStamp(stamp.cast());
    }

    /// # Safety
    /// The argument and current graphics context must be live on the game thread.
    pub unsafe fn draw_filled_rectangle(rect: *mut abi::RECT) {
        DrawFilledRectangle(rect.cast());
    }

    /// # Safety
    /// The argument and current graphics context must be live on the game thread.
    pub unsafe fn draw_text(text: *mut abi::TEXT) {
        font_DrawText(text.cast());
    }

    /// # Safety
    /// `font` must be null or a live engine font on the game thread.
    pub unsafe fn set_context_font(font: abi::FONT) -> abi::FONT {
        SetContextFont(font.cast()).cast()
    }

    /// # Safety
    /// `sound` must be null or a live sound bank no longer in use.
    pub unsafe fn destroy_sound(sound: abi::SOUND_REF) {
        DestroySound(sound.cast());
    }

    /// # Safety
    /// Activity state must be initialized and accessed only on the game thread.
    pub fn in_hyper_space(world: &crate::engine::World) -> abi::BOOLEAN {
        crate::navigation::globals::in_hyper_space(world) as abi::BOOLEAN
    }

    /// # Safety
    /// `rect` must be live, null, or an engine flash sentinel on the game thread.
    pub unsafe fn set_flash_rect(rect: *const abi::RECT) {
        SetFlashRect(rect.cast());
    }

    /// # Safety
    /// `title` must be null or a NUL-terminated string valid through the draw call.
    pub unsafe fn draw_sis_title(title: *mut std::ffi::c_char) {
        DrawSISTitle(title);
    }

    /// # Safety
    /// `rect` must be null or a live rectangle on the game thread.
    pub unsafe fn set_transition_source(rect: *const abi::RECT) {
        SetTransitionSource(rect.cast_mut().cast());
    }

    /// # Safety
    /// Pointers must be null or live frames or drawables owned by the game thread.
    pub unsafe fn capture_drawable(drawable: abi::DRAWABLE) -> abi::FRAME {
        crate::graphics::pixmap_abi::CaptureDrawable(drawable.cast()).cast()
    }

    /// # Safety
    /// Pointers must be null or live frames or drawables owned by the game thread.
    pub unsafe fn release_drawable(frame: abi::FRAME) -> abi::DRAWABLE {
        crate::graphics::pixmap_abi::ReleaseDrawable(frame.cast()).cast()
    }

    /// # Safety
    /// Pointers must be null or live frames or drawables owned by the game thread.
    pub unsafe fn abs_frame_index(frame: abi::FRAME, index: abi::COUNT) -> abi::FRAME {
        crate::graphics::pixmap_abi::SetAbsFrameIndex(frame.cast(), index).cast()
    }

    /// # Safety
    /// Pointers must be null or live frames or drawables owned by the game thread.
    pub unsafe fn equ_frame_index(dst: abi::FRAME, src: abi::FRAME) -> abi::FRAME {
        crate::graphics::pixmap_abi::SetEquFrameIndex(dst.cast(), src.cast()).cast()
    }

    /// # Safety
    /// Pointers must be null or live frames or drawables owned by the game thread.
    pub unsafe fn inc_frame_index(frame: abi::FRAME) -> abi::FRAME {
        crate::graphics::pixmap_abi::IncFrameIndex(frame.cast()).cast()
    }

    /// # Safety
    /// Pointers must be null or live frames or drawables owned by the game thread.
    pub unsafe fn dec_frame_index(frame: abi::FRAME) -> abi::FRAME {
        crate::graphics::pixmap_abi::DecFrameIndex(frame.cast()).cast()
    }

    /// # Safety
    /// The game thread must own initialized pacing and display state.
    pub unsafe fn sleep_thread(ticks: u32) {
        crate::battle::c_bridge::SleepThread(ticks);
    }

    /// # Safety
    /// `rect` must be live and drawing state must be owned by the game thread.
    pub unsafe fn draw_rectangle(rect: *mut abi::RECT) {
        DrawRectangle(rect.cast());
    }

    /// # Safety
    /// `primitives` and its linked list must be valid for the current drawing context.
    pub unsafe fn draw_batch(
        primitives: *mut abi::PRIMITIVE,
        links: abi::PRIM_LINKS,
        flags: abi::BATCH_FLAGS,
    ) {
        crate::c_bindings::global_state::DrawBatch(primitives.cast(), links, flags);
    }

    /// # Safety
    /// `frame` must be null or a live engine frame.
    pub unsafe fn frame_index(frame: abi::FRAME) -> u16 {
        crate::graphics::pixmap_abi::GetFrameIndex(frame.cast())
    }
    /// # Safety
    /// `frame` must be a live engine frame.
    pub unsafe fn frame_count(frame: abi::FRAME) -> u16 {
        crate::graphics::pixmap_abi::GetFrameCount(frame.cast())
    }

    /// # Safety
    /// `frame` must be null or a live engine frame.
    pub unsafe fn rel_frame_index(frame: abi::FRAME, offset: i16) -> abi::FRAME {
        crate::graphics::pixmap_abi::SetRelFrameIndex(frame.cast(), offset).cast()
    }
}

#[cfg(not(test))]
pub use engine::{frame_count, frame_index, rel_frame_index};

pub use crate::math::{angle_to_facing, facing_to_angle, normalize_angle, normalize_facing};

pub use crate::math::{cosine, sine};

pub use crate::math::wrap_delta;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_trigonometry_uses_the_immutable_table_with_size_truncation() {
        for angle in -128..128 {
            for magnitude in [i32::MIN, -65537, -1, 0, 1, 32767, 65537, i32::MAX] {
                let value = crate::math::sinetab[(angle & 63) as usize];
                let expected = ((i64::from(value) * i64::from(magnitude)) >> 14) as i16;
                assert_eq!(sine(angle, magnitude), expected);
                assert_eq!(cosine(angle - 16, magnitude), expected);
            }
        }
        assert_eq!(sine(0, 4096), -4096);
        assert_eq!(sine(32, 4096), 4096);
        assert_eq!(cosine(16, 4096), 4096);
    }

    #[test]
    fn c_angle_rounding_and_wrap_boundaries() {
        assert_eq!(angle_to_facing(1), 0);
        assert_eq!(angle_to_facing(2), 1);
        assert_eq!(angle_to_facing(-3), -1);
        assert_eq!(facing_to_angle(15), 60);
        assert_eq!(normalize_facing(-1), 15);
        assert_eq!(normalize_facing(16), 0);
        assert_eq!(normalize_angle(-1), 63);
        assert_eq!(normalize_angle(64), 0);
    }

    #[test]
    fn wrap_delta_preserves_half_extent_and_adjusts_once() {
        for (delta, expected) in [
            (-4097, 4095),
            (-4096, -4096),
            (4096, 4096),
            (4097, -4095),
            (20000, 11808),
            (i16::MIN, -24576),
        ] {
            assert_eq!(wrap_delta(delta, 8192), expected);
        }
    }
}

#[cfg(test)]
mod name_copy_tests {
    use super::copy_display_name;

    #[test]
    fn display_names_are_terminated_padded_and_truncated_by_bytes() {
        let mut target = [7; 8];
        copy_display_name(&mut target, b"Orz");
        assert_eq!(target, [b'O' as _, b'r' as _, b'z' as _, 0, 0, 0, 0, 0]);
        copy_display_name(&mut target, b"abc\0ignored");
        assert_eq!(target, [b'a' as _, b'b' as _, b'c' as _, 0, 0, 0, 0, 0]);
        let mut target = [0; 3];
        copy_display_name(&mut target, &[b'A', 0xc3, 0xa9]);
        assert_eq!(target, [b'A' as _, 0xc3_u8 as _, 0]);
    }

    #[test]
    fn empty_and_single_byte_destinations_do_not_copy_a_name() {
        copy_display_name(&mut [], b"Orz");
        let mut target = [7];
        copy_display_name(&mut target, b"Orz");
        assert_eq!(target, [0]);
    }
}
