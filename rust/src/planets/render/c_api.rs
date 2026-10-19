//! Existing C graphics boundary for the Rust oval implementation.
//! No oval symbols are exported until the runtime cutover checkpoint.

use crate::c_bindings::global_state as c;
use crate::comm::locdata::{CPoint, CRect};
use crate::starbase::ids::RadarLayout;

use super::oval::OvalGfx;

/// A short-lived borrow of the current graphics context on the game thread.
pub struct CffiOvalGfx {
    viewport: CRect,
    color: c::Color,
}

impl CffiOvalGfx {
    /// # Safety
    /// The current C context and foreground frame must remain active for this
    /// adapter's lifetime. Calls must stay on the game thread, with no context
    /// changes or reentrant menus between them.
    pub unsafe fn current(screen_width: i32, screen_height: i32) -> Self {
        let layout = RadarLayout::for_screen(screen_width, screen_height);
        Self {
            viewport: CRect {
                width: layout.sis_screen_width as i16,
                height: layout.sis_screen_height as i16,
                ..CRect::default()
            },
            color: c::GetContextForeGroundColor(),
        }
    }

    fn batch(&mut self, primitives: &mut [c::PRIMITIVE]) {
        for index in 0..primitives.len() {
            let next = if index + 1 == primitives.len() {
                u16::MAX
            } else {
                (index + 1) as u16
            };
            primitives[index].Links = u32::from(u16::MAX) | (u32::from(next) << 16);
            primitives[index].color = self.color;
        }
        unsafe { c::DrawBatch(primitives.as_mut_ptr(), 0, 0) };
    }
}

fn point(p: CPoint) -> c::POINT {
    c::POINT { x: p.x, y: p.y }
}

fn rect(r: &CRect) -> c::RECT {
    c::RECT {
        corner: point(r.corner),
        extent: c::EXTENT {
            width: r.width,
            height: r.height,
        },
    }
}

impl OvalGfx for CffiOvalGfx {
    fn viewport(&self) -> CRect {
        self.viewport
    }

    fn line_visible(&mut self, viewport: &CRect, first: CPoint, second: CPoint) -> bool {
        let mut line: c::BRESENHAM_LINE = unsafe { std::mem::zeroed() };
        line.first = point(first);
        line.second = point(second);
        unsafe { c::_clip_line(&rect(viewport), &mut line) != 0 }
    }

    fn draw_line(&mut self, first: CPoint, second: CPoint) {
        let mut line = c::LINE {
            first: point(first),
            second: point(second),
        };
        unsafe { c::DrawLine(&mut line) };
    }

    fn points(&mut self, points: &[CPoint]) {
        let mut primitives: [c::PRIMITIVE; 4] = unsafe { std::mem::zeroed() };
        for (prim, p) in primitives.iter_mut().zip(points) {
            prim.Type = c::gfx_object_POINT_PRIM as u8;
            prim.Object.Point = point(*p);
        }
        self.batch(&mut primitives[..points.len()]);
    }

    fn filled_rows(&mut self, rows: &[CRect; 2]) {
        let mut primitives: [c::PRIMITIVE; 2] = unsafe { std::mem::zeroed() };
        for (prim, row) in primitives.iter_mut().zip(rows) {
            prim.Type = c::gfx_object_RECTFILL_PRIM as u8;
            prim.Object.Rect = rect(row);
        }
        self.batch(&mut primitives);
    }
}
