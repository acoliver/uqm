//! Status labels rendered from the supplied campaign and display owners.
use super::{
    rgb15, StatMsgMode_SMM_CREDITS as CREDITS, StatMsgMode_SMM_RES_UNITS as RES_UNITS,
    MONTHS_STRING_BASE, STATUS_MESSAGE_HEIGHT, STATUS_MESSAGE_WIDTH, STATUS_STRING_BASE,
};
use crate::{
    engine::{Kernel, StoryKey, World},
    graphics::{
        context_abi::{ContextFrame, ContextLease},
        native_abi::{EXTENT, POINT, RECT, TEXT_ALIGN_ALIGN_CENTER},
    },
};
use std::{
    borrow::Cow,
    ffi::{CStr, CString},
    ptr::NonNull,
};

pub fn format_date(month: &CStr, day: u8, year: u16) -> CString {
    let mut bytes = month.to_bytes().to_vec();
    bytes.extend_from_slice(format!(" {day:02}·{year:04}").as_bytes());
    CString::new(bytes).unwrap_or_else(|error| panic!("invalid date label: {error}"))
}

pub fn status_message_text(kernel: &Kernel, world: &World) -> CString {
    let mut bytes = Vec::new();
    match kernel.status_message_mode {
        CREDITS => {
            let credits = u16::from(world.story_state(StoryKey::MELNORME_CREDIT0) as u8)
                | (u16::from(world.story_state(StoryKey::MELNORME_CREDIT1) as u8) << 8);
            bytes.extend_from_slice(format!("{credits} ").as_bytes());
            bytes.extend_from_slice(kernel.game_text(STATUS_STRING_BASE as usize).to_bytes());
        }
        RES_UNITS => {
            if (world.story_state(StoryKey::CHMMR_BOMB_STATE) as u8) < 2 {
                bytes.extend_from_slice(world.data.flagship.res_units.to_string().as_bytes());
            } else if kernel.configuration.preferences().menu == 2 {
                bytes.extend_from_slice(
                    kernel
                        .game_text((STATUS_STRING_BASE + 2) as usize)
                        .to_bytes(),
                );
            } else {
                bytes.extend_from_slice("∞".as_bytes());
            }
            bytes.push(b' ');
            bytes.extend_from_slice(
                kernel
                    .game_text((STATUS_STRING_BASE + 1) as usize)
                    .to_bytes(),
            );
        }
        _ => {
            let (month, day, year) = world.data.calendar_date();
            bytes.extend_from_slice(
                format_date(
                    kernel.game_text((MONTHS_STRING_BASE + u32::from(month) - 1) as usize),
                    day,
                    year,
                )
                .as_bytes(),
            );
        }
    }
    bytes.truncate(127);
    bytes.push(0);
    CString::from_vec_with_nul(bytes)
        .unwrap_or_else(|error| panic!("invalid status label: {error}"))
}

pub fn draw_status_message(kernel: &Kernel, world: &World, text: Option<&CStr>) {
    let status = ContextLease::require(kernel.status_context);
    let context = ContextLease::require(kernel.off_screen_context);
    let screen = ContextFrame::retain(NonNull::new(kernel.screen));
    let font = kernel
        .tiny_font_lease()
        .unwrap_or_else(|| panic!("status message requires the tiny font"));
    let text = text.map_or_else(
        || Cow::Owned(status_message_text(kernel, world)),
        Cow::Borrowed,
    );
    status.with_selected(|| {
        let origin = status.clip_rect().corner;
        context.with_selected(|| {
            context.set_frame(screen);
            context.set_clip(RECT {
                corner: POINT {
                    x: origin.x.wrapping_add(2),
                    y: origin.y.wrapping_add(130),
                },
                extent: EXTENT {
                    width: STATUS_MESSAGE_WIDTH,
                    height: STATUS_MESSAGE_HEIGHT,
                },
            });
            context.with_batch(|| {
                context.set_background(rgb15(0, 8, 0));
                context.clear();
                context.draw_text(
                    &font,
                    &text,
                    POINT {
                        x: STATUS_MESSAGE_WIDTH >> 1,
                        y: STATUS_MESSAGE_HEIGHT - 1,
                    },
                    TEXT_ALIGN_ALIGN_CENTER,
                    rgb15(0, 16, 0),
                );
            });
            context.clear_clip();
        });
    });
}
