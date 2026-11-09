use crate::engine::world::activity::activity_flags;
use crate::engine::Kernel;
use crate::engine::World;
use crate::graphics::context_abi::ContextLease;
use crate::starbase::ids;
use crate::ui::palette::pilot_color;
use crate::ui::rgb15;
use uqm_engine::time::counter_abi::ONE_SECOND;

use crate::graphics::native_abi::*;

use std::ffi::{CStr, CString};

pub(crate) mod gauges;
pub use gauges::{delta_sis_gauges, draw_flagship_name};
mod message_display;
pub use message_display::{draw_message, set_menu_flash};

mod message;
pub use message::default_message_text;

mod status;
pub use status::{
    draw_status_message, format_date, message_rect as status_message_rect, status_message_text,
};

mod title;
pub use title::{draw_hyper_coords, draw_title};

pub(crate) mod capacity;

#[cfg(test)]
mod pilot_tests;
#[cfg(test)]
mod services_tests;

const OPT_PC: i32 = 2;
const SIS_MESSAGE_HEIGHT: i16 = 8;

const SIS_ORG_X: SIZE = 7;
const SIS_ORG_Y: SIZE = 10;
const SIS_TITLE_BOX_WIDTH: SIZE = 57;
const SIS_TITLE_WIDTH: SIZE = 55;
const SIS_TITLE_HEIGHT: SIZE = 8;
const SIS_SPACER_BOX_WIDTH: SIZE = 12;
const STATUS_MESSAGE_WIDTH: SIZE = 60;
const STATUS_MESSAGE_HEIGHT: SIZE = 7;
const RADAR_WIDTH: SIZE = 56;
const RADAR_HEIGHT: SIZE = 53;
mod stats_display;
pub use gauges::{draw_landers, draw_storage_bays};
pub use stats_display::draw_flagship_stats;

pub fn draw_autopilot_message(
    kernel: &Kernel,
    world: &crate::engine::World,
    width: i32,
    now: impl FnMut() -> u32,
) {
    use crate::engine::kernel::PilotUpdate;
    let autopilot = world.data.game.autopilot;
    let fuel_empty = world.data.flagship.fuel_on_board == 0;
    let on = (autopilot.x != -1 && autopilot.y != -1) || fuel_empty;
    let can_draw = world.activity() & activity_flags::CHECK_ABORT == 0
        && world.data.flagship.crew_enlisted != u16::MAX;
    kernel
        .pilot_flash
        .borrow_mut()
        .update(on, can_draw, now, |update| match update {
            PilotUpdate::DefaultMessage => {
                let text = default_message_text(kernel, world);
                draw_message(kernel, width, &text, -1, -1, DSME_NONE);
            }
            PilotUpdate::Pulse(phase) => {
                let context = ContextLease::require(kernel.off_screen_context);
                context.with_selected(|| {
                    context.set_foreground(pilot_color(usize::from(phase)));
                    let text = kernel.game_text(
                        (ids::NAVIGATION_STRING_BASE + if fuel_empty { 2 } else { 3 }) as usize,
                    );
                    draw_message(kernel, width, text, -1, -1, DSME_MYCOLOR);
                });
            }
        });
}

pub fn set_flash_rect(
    highlight: &crate::engine::kernel::SisHighlight,
    screen: &ContextLease,
    clip: RECT,
    rect: Option<RECT>,
) {
    screen.with_selected(|| {
        let mut slot = highlight.context.borrow_mut();
        let current = &mut *slot;
        if let Some(mut rect) = rect.filter(|rect| rect.extent.width != 0) {
            rect.corner.x = (i32::from(rect.corner.x) + i32::from(clip.corner.x)) as SIZE;
            rect.corner.y = (i32::from(rect.corner.y) + i32::from(clip.corner.y)) as SIZE;
            if let Some(current) = current {
                current.set_rect(rect);
            } else {
                let mut highlight = screen.highlight(rect);
                highlight.set_merge_factors(3, 2, 2);
                highlight.set_speed(0, ONE_SECOND / 16, 0, ONE_SECOND / 16);
                highlight.set_frame_time(ONE_SECOND / 16);
                *current = Some(highlight.schedule());
            }
        } else {
            *current = None;
        }
    });
}

pub fn draw_campaign_message(kernel: &Kernel, world: &World, width: i32, text: Option<&CStr>) {
    let text = text.map_or_else(
        || std::borrow::Cow::Owned(default_message_text(kernel, world)),
        std::borrow::Cow::Borrowed,
    );
    draw_message(kernel, width, &text, -1, -1, 0);
}

pub fn campaign_date(kernel: &Kernel, month: u8, day: u8, year: u16) -> CString {
    format_date(
        kernel.game_text(ids::MONTHS_STRING_BASE as usize + usize::from(month) - 1),
        day,
        year,
    )
}

pub fn clear_flash(kernel: &Kernel) {
    *kernel.sis_highlight.context.borrow_mut() = None;
}

pub fn highlight_rect(kernel: &Kernel, rect: RECT) {
    if rect.extent.width == 0 {
        clear_flash(kernel);
        return;
    }
    let screen = ContextLease::require(kernel.screen_context);
    let selected = screen
        .selected_context()
        .unwrap_or_else(|| panic!("SIS highlight requires a selected context"));
    set_flash_rect(
        &kernel.sis_highlight,
        &screen,
        selected.clip_rect(),
        Some(rect),
    );
}

pub fn repair_sis_border(kernel: &Kernel, dimensions: (i32, i32)) {
    let context = ContextLease::require(kernel.screen_context);
    let mut rect = RECT {
        corner: POINT {
            x: SIS_ORG_X - 1,
            y: SIS_ORG_Y - 1,
        },
        extent: EXTENT {
            width: 1,
            height: (dimensions.1 - 13 + 2) as i16,
        },
    };
    context.draw_batch(|| {
        context.fill_rectangle(rgb15(16, 16, 16), rect);
        rect.corner.x = (i32::from(rect.corner.x) + dimensions.0 - 78 + 2 - 1) as i16;
        context.fill_rectangle(rgb15(8, 8, 8), rect);
        rect.corner.x = SIS_ORG_X - 1;
        rect.corner.y = (i32::from(rect.corner.y) + dimensions.1 - 13 + 2 - 1) as i16;
        rect.extent.width = (dimensions.0 - 78 + 2) as i16;
        rect.extent.height = 1;
        context.fill_rectangle(rgb15(8, 8, 8), rect);
    });
}

pub fn clear_sis_rect(kernel: &Kernel, world: &mut World, dimensions: (i32, i32), flags: u8) {
    let context = ContextLease::require(kernel.status_context);
    context.with_text_state(|| {
        context.set_foreground(rgb15(10, 10, 10));
        context.with_batch(|| {
            if flags & 8 != 0 {
                delta_sis_gauges(kernel, world, 32767, 32767, 32767);
            }
            if flags & 4 != 0 {
                crate::ui::menu::draw_menu_state(kernel, world, dimensions, u8::MAX, 1);
            }
        });
    });
}

pub(crate) const CLEAR_RADAR: u8 = 4;
pub(crate) const DRAW_DISPLAY: u8 = 8;
pub(crate) const UNDEFINED_DELTA: i16 = i16::MAX;
pub(crate) const DSME_NONE: u16 = 0;
pub(crate) const DSME_SETFR: u16 = 1;
pub(crate) const DSME_CLEARFR: u16 = 2;
pub(crate) const DSME_BLOCKCUR: u16 = 4;
pub(crate) const DSME_MYCOLOR: u16 = 8;
