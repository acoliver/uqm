//! Byte-bounded labels used by the save-name editor.
use std::ffi::{CStr, CString};

pub(super) struct SaveLabel {
    pub text: CString,
    pub cursor_prefix: usize,
}

pub(super) fn save_label(date: &CStr, name: &CStr) -> SaveLabel {
    let mut bytes = date.to_bytes().iter().take(79).copied().collect::<Vec<_>>();
    bytes.extend(b": ".iter().take(79 - bytes.len()));
    let cursor_prefix = bytes.len() - 1;
    bytes.extend(name.to_bytes());
    bytes.truncate(255);
    SaveLabel {
        text: CString::new(bytes).unwrap_or_else(|error| panic!("invalid save label: {error}")),
        cursor_prefix,
    }
}

use super::layout::{self, BLACK_COLOR, CAPTAIN_NAME_TEXT_COLOR};
use super::picker::{MAX_SAVED_GAMES, SAVES_PER_PAGE};
use crate::ui::rgb15;
use crate::{
    engine::{Kernel, World},
    navigation::abi::*,
};
const DDSHS_EDIT: u16 = 1;
const DDSHS_BLOCKCUR: u16 = 2;
pub(super) fn draw_name_string(
    kernel: &Kernel,
    world: &World,
    captain: bool,
    text: &std::ffi::CStr,
    cursor: usize,
    state: u16,
) -> bool {
    use crate::graphics::{context_abi::ContextLease, native_font::FontLease};
    let context = ContextLease::require(kernel.status_context);
    context.select();
    if state & DDSHS_EDIT == 0 {
        if captain {
            crate::ui::sis::gauges::draw_captains_name(kernel, world);
        } else {
            crate::ui::sis::gauges::draw_flagship_name(kernel, world, 0, true);
        }
        return true;
    }
    let font = if captain {
        kernel
            .tiny_font_lease()
            .unwrap_or_else(|| panic!("captain naming font"))
    } else {
        FontLease::retain(
            std::ptr::NonNull::new(kernel.star_con_font)
                .unwrap_or_else(|| panic!("ship naming font")),
        )
    };
    let (rect, background, foreground) = layout::name_style(captain);
    let baseline = POINT {
        x: rect.corner.x + (rect.extent.width >> 1) - i16::from(captain),
        y: rect.corner.y + rect.extent.height - 1,
    };
    let measured = font.measure(text, baseline, TEXT_ALIGN_ALIGN_CENTER, COUNT::MAX);
    if i32::from(measured.rect.extent.width) + 2 >= i32::from(rect.extent.width) {
        return false;
    }
    assert!(
        cursor <= usize::from(measured.characters),
        "name cursor exceeds text"
    );
    kernel.sis_highlight.pre_update_flash_rect();
    context.fill_rectangle(background, rect);
    let mut caret = measured.rect;
    caret.corner.x = measured.deltas[..cursor]
        .iter()
        .fold(caret.corner.x, |x, &delta| x.wrapping_add(i16::from(delta)));
    let count = usize::from(measured.characters);
    if cursor < count {
        caret.corner.x -= 1;
    }
    caret.extent.width = if state & DDSHS_BLOCKCUR != 0 && cursor < count {
        i16::from(measured.deltas[cursor]) + if cursor + 1 == count { 2 } else { 1 }
    } else {
        1
    };
    caret.corner.y = rect.corner.y;
    caret.extent.height = rect.extent.height;
    context.fill_rectangle(BLACK_COLOR, caret);
    context.draw_text(&font, text, baseline, TEXT_ALIGN_ALIGN_CENTER, foreground);
    kernel.sis_highlight.post_update_flash_rect();
    true
}
pub(super) fn draw_save_name_string(
    kernel: &Kernel,
    world: &World,
    text: &std::ffi::CStr,
    cursor: usize,
    state: u16,
    game_index: u16,
) -> bool {
    use crate::graphics::context_abi::ContextLease;
    let context = ContextLease::require(kernel.space_context);
    let (month, day, year) = world.data.calendar_date();
    let date = crate::ui::sis::format_date(
        kernel.game_text((MONTHS_STRING_BASE + u32::from(month) - 1) as usize),
        day,
        year,
    );
    let label = save_label(&date, text);
    let mut rect = RECT {
        corner: POINT {
            x: 8,
            y: (160 + ((game_index % SAVES_PER_PAGE) * 13)) as i16,
        },
        extent: EXTENT {
            width: 15 + if MAX_SAVED_GAMES > 99 { 5 } else { 0 },
            height: 11,
        },
    };
    let background = rgb15(27, 0, 27);
    context.draw_rectangle(background, rect);
    rect.extent.width = (204 - SAFE_X) as i16;
    rect.corner.x = (30 + SAFE_X) as i16;
    context.draw_rectangle(background, rect);
    let baseline = POINT {
        x: rect.corner.x + 3,
        y: rect.corner.y + 8,
    };
    let font = kernel
        .tiny_font_lease()
        .unwrap_or_else(|| panic!("save naming font"));
    if state & DDSHS_EDIT == 0 {
        context.fill_rectangle(BLACK_COLOR, rect);
        context.draw_text(
            &font,
            &label.text,
            baseline,
            TEXT_ALIGN_ALIGN_LEFT,
            CAPTAIN_NAME_TEXT_COLOR,
        );
        return true;
    }
    let measured =
        font.measure_bounded::<256>(&label.text, baseline, TEXT_ALIGN_ALIGN_LEFT, COUNT::MAX);
    if i32::from(measured.rect.extent.width) + 2 >= i32::from(rect.extent.width) {
        return false;
    }
    let cursor = cursor + label.cursor_prefix;
    kernel.sis_highlight.pre_update_flash_rect();
    context.fill_rectangle(background, rect);
    let mut caret = measured.rect;
    caret.corner.x = measured.deltas[..cursor]
        .iter()
        .fold(caret.corner.x, |x, &delta| x.wrapping_add(i16::from(delta)));
    let count = usize::from(measured.characters);
    if cursor < count {
        caret.corner.x -= 1;
    }
    caret.extent.width = if state & DDSHS_BLOCKCUR != 0 && cursor < count {
        i16::from(measured.deltas[cursor]) + if cursor + 1 == count { 2 } else { 1 }
    } else {
        1
    };
    caret.corner.y = rect.corner.y;
    caret.extent.height = rect.extent.height;
    context.fill_rectangle(BLACK_COLOR, caret);
    context.draw_text(
        &font,
        &label.text,
        baseline,
        TEXT_ALIGN_ALIGN_LEFT,
        rgb15(0, 0, 20),
    );
    kernel.sis_highlight.post_update_flash_rect();
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[serial_test::serial]
    fn owned_name_renderers_measure_and_draw_without_published_campaign_owners() {
        if !crate::native_test_process::own_graphics_process("front_end::gameopt::naming::tests::owned_name_renderers_measure_and_draw_without_published_campaign_owners") { return; }
        use crate::graphics::context_abi::{ContextFrame, ContextLease};
        use crate::{campaign_display_test::CampaignDisplay, engine::Game};
        let mut game = Game::new();
        let display = CampaignDisplay::install(&mut game)
            .unwrap_or_else(|error| panic!("naming display: {error}"));
        game.kernel.space_context = game.kernel.status_context;
        game.kernel.star_con_font = game.kernel.tiny_font;
        game.kernel
            .configuration
            .update_preferences(|preferences| preferences.fonts = 1);
        game.world.data.game.game_clock.month_index = 1;
        game.world.data.game.game_clock.day_index = 7;
        game.world.data.game.game_clock.year_index = 2155;
        game.world.data.flagship.commander_name[..2].copy_from_slice(b"A\0");
        game.world.data.flagship.ship_name[..2].copy_from_slice(b"A\0");
        let context = ContextLease::require(game.kernel.status_context);
        context.set_frame(ContextFrame::retain(std::ptr::NonNull::new(
            game.kernel.screen,
        )));
        for captain in [true, false] {
            assert!(draw_name_string(
                &game.kernel,
                &game.world,
                captain,
                c"A",
                0,
                0
            ));
            for (text, cursor) in [(c"", 0), (c"AA", 0), (c"AA", 1), (c"AA", 2)] {
                for state in [DDSHS_EDIT, DDSHS_EDIT | DDSHS_BLOCKCUR] {
                    assert!(draw_name_string(
                        &game.kernel,
                        &game.world,
                        captain,
                        text,
                        cursor,
                        state
                    ));
                }
            }
        }
        let wide = CString::new("A".repeat(400)).unwrap_or_else(|error| panic!("{error}"));
        assert!(!draw_name_string(
            &game.kernel,
            &game.world,
            true,
            &wide,
            0,
            DDSHS_EDIT
        ));
        for (text, cursor) in [(c"", 0), (c"AA", 0), (c"AA", 1), (c"AA", 2)] {
            for state in [0, DDSHS_EDIT, DDSHS_EDIT | DDSHS_BLOCKCUR] {
                assert!(draw_save_name_string(
                    &game.kernel,
                    &game.world,
                    text,
                    cursor,
                    state,
                    7
                ));
            }
        }
        assert!(!draw_save_name_string(
            &game.kernel,
            &game.world,
            &wide,
            0,
            DDSHS_EDIT,
            7
        ));
        draw_name_string(&game.kernel, &game.world, true, c"A", 0, DDSHS_EDIT);
        context.flush();
        // SAFETY: this serial fixture retains its screen until all queued drawing is flushed.
        let pixel = unsafe {
            crate::graphics::drawable_abi::GetFramePixel(game.kernel.screen, POINT { x: 3, y: 10 })
        };
        assert_eq!((pixel.r, pixel.g, pixel.b), (82, 82, 255));
        drop(context);
        display.finish(game);
    }

    #[test]
    fn save_labels_preserve_the_date_prefix_and_byte_buffer_boundaries() {
        let label = save_label(c"November 07·2155", c"Earth");
        assert_eq!(label.text.to_bytes(), "November 07·2155: Earth".as_bytes());
        assert_eq!(label.cursor_prefix, "November 07·2155: ".len() - 1);
        let date = CString::new(format!("{} 07·2155", "é".repeat(40)))
            .unwrap_or_else(|error| panic!("{error}"));
        let bounded = save_label(&date, c"B");
        assert_eq!(bounded.cursor_prefix, 78);
        assert_eq!(bounded.text.to_bytes().len(), 80);
        assert_eq!(&bounded.text.to_bytes()[78..], &[0xc3, b'B']);
        let name = CString::new("A".repeat(300)).unwrap_or_else(|error| panic!("{error}"));
        assert_eq!(
            save_label(c"November 07·2155", &name).text.to_bytes().len(),
            255
        );
    }
}
