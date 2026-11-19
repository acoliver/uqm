//! Campaign title drawing over retained display resources.
use super::{rgb15, SIS_ORG_X, SIS_ORG_Y, SIS_TITLE_BOX_WIDTH, SIS_TITLE_HEIGHT, SIS_TITLE_WIDTH};
use crate::{
    engine::Kernel,
    graphics::{
        context_abi::{ContextFrame, ContextLease},
        native_abi::{EXTENT, POINT, RECT, TEXT_ALIGN_ALIGN_CENTER},
    },
};
use std::{
    ffi::{CStr, CString},
    ptr::NonNull,
};

pub fn draw_title(kernel: &Kernel, screen_width: i32, text: &CStr) {
    let context = ContextLease::require(kernel.off_screen_context);
    let screen = ContextFrame::retain(NonNull::new(kernel.screen));
    let font = kernel
        .tiny_font_lease()
        .unwrap_or_else(|| panic!("SIS title requires its tiny font"));
    let clip = RECT {
        corner: POINT {
            x: (i32::from(SIS_ORG_X) + screen_width - 78 - i32::from(SIS_TITLE_BOX_WIDTH) + 1)
                as i16,
            y: SIS_ORG_Y - SIS_TITLE_HEIGHT,
        },
        extent: EXTENT {
            width: SIS_TITLE_WIDTH,
            height: SIS_TITLE_HEIGHT - 1,
        },
    };
    context.with_selected(|| {
        context.set_frame(screen);
        context.set_clip(clip);
        context.set_font(&font);
        context.with_batch(|| {
            context.set_background(rgb15(0, 0, 20));
            context.clear();
            context.draw_text(
                &font,
                text,
                POINT {
                    x: SIS_TITLE_WIDTH >> 1,
                    y: SIS_TITLE_HEIGHT - 2,
                },
                TEXT_ALIGN_ALIGN_CENTER,
                rgb15(27, 0, 27),
            );
        });
        context.clear_clip();
    });
}

pub fn draw_hyper_coords(kernel: &Kernel, screen_width: i32, point: POINT) {
    draw_title(kernel, screen_width, &hyper_coord_text(point));
}

pub(super) fn hyper_coord_text(point: POINT) -> CString {
    CString::new(format!(
        "{:03}.{} : {:03}.{}",
        (i32::from(point.x) / 10) as u32,
        (i32::from(point.x) % 10) as u32,
        (i32::from(point.y) / 10) as u32,
        (i32::from(point.y) % 10) as u32,
    ))
    .unwrap_or_else(|error| panic!("coordinate label contains a NUL: {error}"))
}
