//! Flagship screen border, ported from uqm/border.c.
use super::util::box_rectangles;
use crate::engine::Kernel;
use crate::graphics::context_abi::{ContextFrame, ContextLease};
use crate::navigation::abi::*;
fn color(r: u8, g: u8, b: u8) -> Color {
    Color {
        r: (r << 3) | (r >> 2),
        g: (g << 3) | (g >> 2),
        b: (b << 3) | (b >> 2),
        a: 255,
    }
}
pub fn init_siscontexts(kernel: &Kernel, dimensions: (i32, i32)) {
    let status = ContextLease::require(kernel.status_context);
    let space = ContextLease::require(kernel.space_context);
    let screen = ContextFrame::retain(std::ptr::NonNull::new(kernel.screen));
    status.select();
    space.select();
    space.set_frame(screen);
    space.set_clip(RECT {
        corner: POINT { x: 7, y: 10 },
        extent: EXTENT {
            width: (dimensions.0 - 78) as i16,
            height: (dimensions.1 - 13) as i16,
        },
    });
}

pub fn draw_sis_frame(kernel: &Kernel, world: &mut crate::engine::World, dimensions: (i32, i32)) {
    let (width, height) = dimensions;
    let context = ContextLease::require(kernel.screen_context);
    let mut r = RECT {
        corner: POINT { x: 0, y: 0 },
        extent: EXTENT {
            width: 0,
            height: 0,
        },
    };
    let ink = color(0x0A, 0x0A, 0x0A);
    context.select();
    context.with_batch(|| {
        r.corner.x = 0;
        r.corner.y = 0;
        r.extent.width = 7 + ((width - 78) as i16) + 1;
        r.extent.height = 10 - 1;
        context.fill_rectangle(ink, r);
        r.corner.x = 0;
        r.corner.y = 0;
        r.extent.width = 7 - 1;
        r.extent.height = 10 + ((height - 13) as i16) + 1;
        context.fill_rectangle(ink, r);
        r.corner.x = 0;
        r.corner.y = r.extent.height;
        r.extent.width = 7 + ((width - 78) as i16) + 1;
        r.extent.height = (height as i16) - 10 + ((height - 13) as i16);
        context.fill_rectangle(ink, r);
        r.corner.x = 7 + ((width - 78) as i16) + 1;
        r.corner.y = 0;
        r.extent.width = (width as i16) - r.corner.x;
        r.extent.height = height as i16;
        context.fill_rectangle(ink, r);

        r.corner.x = 7 - 1;
        r.corner.y = 10 - 1;
        r.extent.width = ((width - 78) as i16) + 2;
        r.extent.height = ((height - 13) as i16) + 2;
        box_rectangles(
            r,
            1,
            color(0x10, 0x10, 0x10),
            color(0x08, 0x08, 0x08),
            Some(color(0, 0, 0)),
            |ink, rect| context.fill_rectangle(ink, rect),
        );

        r.corner.y = 0;
        r.extent.height = 10;

        r.corner.x = 7;
        r.extent.width = (width - 147) as i16;
        box_rectangles(
            r,
            1,
            color(0x00, 0x00, 0x0E),
            color(0x00, 0x01, 0x1C),
            Some(color(0x00, 0x00, 0x14)),
            |ink, rect| context.fill_rectangle(ink, rect),
        );

        r.extent.width = 57;
        r.corner.x = 7 + ((width - 78) as i16) - 57;
        box_rectangles(
            r,
            1,
            color(0x00, 0x00, 0x0E),
            color(0x00, 0x01, 0x1C),
            Some(color(0x00, 0x00, 0x14)),
            |ink, rect| context.fill_rectangle(ink, rect),
        );

        draw_edges(&context, dimensions);
        init_siscontexts(kernel, dimensions);
        super::sis::delta_sis_gauges(kernel, world, 32767, 32767, 32767);
    });
}

fn draw_edges(context: &ContextLease, dimensions: (i32, i32)) {
    let (width, height) = dimensions;
    let mut r = RECT {
        corner: POINT { x: 0, y: 0 },
        extent: EXTENT {
            width: 0,
            height: 0,
        },
    };
    let mut ink;
    ink = color(0, 0, 0);
    r.corner.x = ((width - 64) as i16) - 1;
    r.corner.y = 0;
    r.extent.width = 1;
    r.extent.height = height as i16;
    context.fill_rectangle(ink, r);
    r.corner.x = (width - 64) as i16;
    r.corner.y = 139;
    context.fill_rectangle(
        ink,
        RECT {
            corner: r.corner,
            extent: EXTENT {
                width: 1,
                height: 1,
            },
        },
    );
    r.corner.x = (width as i16) - 1;
    context.fill_rectangle(
        ink,
        RECT {
            corner: r.corner,
            extent: EXTENT {
                width: 1,
                height: 1,
            },
        },
    );

    ink = color(0x10, 0x10, 0x10);
    r.corner.y = 1;
    r.extent.width = 1;
    r.extent.height = 8;
    r.corner.x = 7 - 1;
    context.fill_rectangle(ink, r);
    r.corner.x = 7 + ((width - 78) as i16) - 57 - 1;
    context.fill_rectangle(ink, r);

    r.corner.x = 0;
    r.corner.y = (height as i16) - 1;
    r.extent.width = ((width - 64) as i16) - 1;
    r.extent.height = 1;
    context.fill_rectangle(ink, r);
    r.corner.x = ((width - 64) as i16) - 2;
    r.corner.y = 0;
    r.extent.width = 1;
    r.extent.height = (height as i16) - 1;
    context.fill_rectangle(ink, r);
    r.corner.x = (width as i16) - 1;
    r.corner.y = 0;
    r.extent.width = 1;
    r.extent.height = 139;
    context.fill_rectangle(ink, r);
    r.corner.x = (width - 64) as i16;
    r.corner.y = (height as i16) - 1;
    r.extent.width = (width as i16) - r.corner.x;
    r.extent.height = 1;
    context.fill_rectangle(ink, r);
    r.corner.x = (width as i16) - 1;
    r.corner.y = 140;
    r.extent.width = 1;
    r.extent.height = ((height as i16) - 1) - r.corner.y;
    context.fill_rectangle(ink, r);

    draw_dark_edges(context, dimensions);
}

fn draw_dark_edges(context: &ContextLease, dimensions: (i32, i32)) {
    let (width, height) = dimensions;
    let mut r = RECT {
        corner: POINT { x: 0, y: 0 },
        extent: EXTENT {
            width: 0,
            height: 0,
        },
    };
    let ink = color(0x08, 0x08, 0x08);
    r.corner.y = 1;
    r.extent.width = 1;
    r.extent.height = 8;
    r.corner.x = 7 + ((width - 147) as i16);
    context.fill_rectangle(ink, r);
    r.corner.x = 7 + ((width - 78) as i16);
    r.extent.height += 1;
    context.fill_rectangle(ink, r);
    r.corner.y = 0;
    r.extent.width = (((width - 64) as i16) - 2) - r.corner.x;
    r.extent.height = 1;
    context.fill_rectangle(ink, r);
    r.corner.x = 0;
    r.extent.width = 7 - r.corner.x;
    context.fill_rectangle(ink, r);
    r.corner.x = 7 + ((width - 147) as i16);
    r.extent.width = 12;
    context.fill_rectangle(ink, r);

    r.corner.x = 0;
    r.corner.y = 1;
    r.extent.width = 1;
    r.extent.height = ((height as i16) - 1) - r.corner.y;
    context.fill_rectangle(ink, r);
    r.corner.x = (width - 64) as i16;
    r.corner.y = 0;
    r.extent.width = 1;
    r.extent.height = 139;
    context.fill_rectangle(ink, r);
    r.corner.x = ((width - 64) as i16) + 1;
    r.corner.y = 139;
    r.extent.width = 64 - 2;
    r.extent.height = 1;
    context.fill_rectangle(ink, r);
    r.corner.x = (width - 64) as i16;
    r.corner.y = 140;
    r.extent.width = 1;
    r.extent.height = (height as i16) - r.corner.y;
    context.fill_rectangle(ink, r);
}

/// Draw the conversation chrome without borrowing the communication runtime.
pub(crate) fn draw_conversation_frame(
    kernel: &crate::engine::Kernel,
    world: &mut crate::engine::World,
    dimensions: (i32, i32),
    starbase_title: Option<usize>,
) {
    use super::sis::{draw_campaign_message, draw_title};
    draw_sis_frame(kernel, world, dimensions);
    if let Some(title) = starbase_title {
        draw_campaign_message(
            kernel,
            world,
            dimensions.0,
            Some(kernel.game_text(title + 1)),
        );
        draw_title(kernel, dimensions.0, kernel.game_text(title));
    } else {
        draw_campaign_message(kernel, world, dimensions.0, None);
        let name = std::ffi::CStr::from_bytes_until_nul(&world.data.flagship.planet_name)
            .unwrap_or_else(|error| panic!("invalid planet name: {error}"));
        draw_title(kernel, dimensions.0, name);
    }
}
