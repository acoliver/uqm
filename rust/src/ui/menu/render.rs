//! Campaign menus rendered from retained display and text owners.
use super::*;
use crate::{
    engine::World,
    graphics::{
        context_abi::{ContextFrame, ContextLease},
        native_font::FontLease,
    },
};
use std::{ffi::CString, ptr::NonNull};

const INACTIVE_COLOR: Color = Color {
    r: 82,
    g: 82,
    b: 82,
    a: 255,
};

fn draw_pc_frame(context: &ContextLease, rect: &mut RECT) {
    context.draw_rectangle(PCMENU_TOP_LEFT_BORDER_COLOR, *rect);
    rect.corner.x += 1;
    rect.corner.y += 1;
    rect.extent.height -= 1;
    rect.extent.width -= 1;
    context.draw_rectangle(PCMENU_BOTTOM_RIGHT_BORDER_COLOR, *rect);
    rect.extent.height -= 1;
    rect.extent.width -= 1;
    context.fill_rectangle(PCMENU_BACKGROUND_COLOR, *rect);
}

fn draw_pc_menu(
    kernel: &Kernel,
    world: &World,
    context: &ContextLease,
    range: (u8, u8),
    selection: (u8, bool),
    rect: &mut RECT,
) {
    const ITEM_HEIGHT: i16 = 8;
    let (begin, end) = range;
    let (selected, highlight) = selection;
    let selected_item = begin.wrapping_add(selected);
    let count = 1 + end - begin;
    rect.corner.x -= 1;
    rect.extent.width += 1;
    context.fill_rectangle(INACTIVE_COLOR, *rect);
    if i16::from(count) * ITEM_HEIGHT > rect.extent.height {
        crate::log_abi::log_message(
            log_Level_log_Error as i32,
            "Warning, no room for all menu items!",
        );
    } else {
        rect.corner.y += (rect.extent.height - i16::from(count) * ITEM_HEIGHT) / 2;
    }
    rect.extent.height = i16::from(count) * ITEM_HEIGHT + 4;
    draw_pc_frame(context, rect);
    let font = FontLease::retain(
        NonNull::new(kernel.star_con_font).unwrap_or_else(|| panic!("PC menu requires its font")),
    );
    let mut baseline = POINT {
        x: rect.corner.x + 2,
        y: rect.corner.y + ITEM_HEIGHT - 1,
    };
    rect.corner.x += 1;
    rect.extent.width -= 2;
    let mut color = PCMENU_TEXT_COLOR;
    context.with_text_state(|| {
        for item in begin..=end {
            let mut label = kernel
                .game_text((MAINMENU_STRING_BASE + u32::from(item)) as usize)
                .to_bytes()
                .to_vec();
            let limit = if item == PM_FUEL || item == PM_CREW {
                let cost = if item == PM_FUEL {
                    world.data.game.fuel_cost
                } else {
                    world.data.game.crew_cost
                };
                label.extend_from_slice(format!("({cost})").as_bytes());
                127
            } else {
                255
            };
            label.truncate(limit);
            let label =
                CString::new(label).unwrap_or_else(|error| panic!("invalid menu label: {error}"));
            color = if highlight && selected_item == item {
                rect.corner.y = baseline.y - ITEM_HEIGHT + 2;
                rect.extent.height = ITEM_HEIGHT - 1;
                context.fill_rectangle(PCMENU_SELECTION_BACKGROUND_COLOR, *rect);
                PCMENU_SELECTION_TEXT_COLOR
            } else {
                PCMENU_TEXT_COLOR
            };
            context.draw_text(&font, &label, baseline, TEXT_ALIGN_ALIGN_LEFT, color);
            baseline.y += ITEM_HEIGHT;
        }
    });
    context.set_foreground(color);
}

fn pc_selection(begin: u8, state: i16) -> (u8, u8) {
    let mut end = end_state(begin);
    let mut selected = state as u8;
    if begin == PM_SOUND_ON {
        end = begin + 5;
        selected = match begin.wrapping_add(selected) {
            PM_SOUND_ON | PM_SOUND_OFF => 0,
            PM_MUSIC_ON | PM_MUSIC_OFF => 1,
            PM_CYBORG_OFF..=PM_CYBORG_SUPER => 2,
            PM_CHANGE_CAPTAIN => 3,
            PM_CHANGE_SHIP => 4,
            PM_EXIT_SETTINGS => 5,
            _ => selected,
        };
    }
    (end, selected)
}

fn draw_cost(kernel: &Kernel, world: &World, context: &ContextLease, begin: u8, origin: POINT) {
    let value = if begin == PM_CREW {
        world.data.game.crew_cost
    } else {
        world.data.game.fuel_cost
    };
    let text = CString::new(value.to_string())
        .unwrap_or_else(|error| panic!("invalid menu cost: {error}"));
    let font = kernel
        .tiny_font_lease()
        .unwrap_or_else(|| panic!("3DO menu requires its font"));
    context.draw_text(
        &font,
        &text,
        POINT {
            x: origin.x + 54,
            y: origin.y + 51,
        },
        TEXT_ALIGN_ALIGN_RIGHT,
        THREEDOMENU_TEXT_COLOR,
    );
}

pub fn draw_menu_state(
    kernel: &Kernel,
    world: &World,
    dimensions: (i32, i32),
    mut begin: u8,
    mut state: i16,
) {
    let preferences = kernel.configuration.preferences();
    let highlight = state >= 0;
    if state < 0 {
        state = -state;
    }
    if preferences.menu == OPT_PC {
        let mut temp = state as u8;
        alternate(&mut begin, &mut temp);
        state = i16::from(temp);
    }
    if begin == PM_STARMAP {
        state -= 1;
    }
    let frame = (state <= i16::from(end_state(begin)) - i16::from(begin))
        .then_some(NonNull::new(kernel.play_frame))
        .flatten()
        .map(|raw| {
            ContextFrame::retain(Some(raw)).wrapped_frame((i16::from(begin) + state) as u16)
        });
    kernel.sis_highlight.pre_update_flash_rect();
    let context = ContextLease::require(kernel.status_context);
    context.with_selected(|| {
        let clip = context.clip_rect();
        let origin = POINT {
            x: (dimensions.0 - 60) as i16 - clip.corner.x,
            y: (dimensions.1 - 56) as i16 - clip.corner.y,
        };
        let mut rect = RECT {
            corner: POINT {
                x: origin.x - 1,
                y: origin.y - 11,
            },
            extent: EXTENT {
                width: 58,
                height: clip.extent.height,
            },
        };
        context.with_batch(|| {
            context.set_foreground(INACTIVE_COLOR);
            let pc = frame.is_some() && preferences.menu == OPT_PC;
            if pc {
                let selection = pc_selection(begin, state);
                rect.extent.height = 64;
                draw_pc_menu(
                    kernel,
                    world,
                    &context,
                    (begin, selection.0),
                    (selection.1, highlight),
                    &mut rect,
                );
            } else {
                if preferences.menu == OPT_PC {
                    rect.corner.x -= 1;
                    rect.extent.width += 1;
                    rect.extent.height = 64;
                } else {
                    rect.extent.height = 11;
                }
                context.fill_rectangle(INACTIVE_COLOR, rect);
                if let Some(frame) = frame {
                    context.draw_stamp(&frame, origin);
                    if begin == PM_CREW || begin == PM_FUEL {
                        draw_cost(kernel, world, &context, begin, origin);
                    }
                }
            }
        });
    });
    kernel.sis_highlight.post_update_flash_rect();
}
