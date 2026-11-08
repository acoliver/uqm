//! Planet oval ABI and topography adapter over the existing context renderer.

use crate::c_bindings::{assert_c_layout, global_state as c};
use crate::comm::locdata::{CPoint, CRect};
use crate::starbase::ids::RadarLayout;

use super::oval::OvalGfx;

assert_c_layout!(CRect, c::RECT { corner: corner });
const _: () = {
    assert!(std::mem::offset_of!(CRect, width) == std::mem::offset_of!(c::RECT, extent));
    assert!(
        std::mem::offset_of!(CRect, height)
            == std::mem::offset_of!(c::RECT, extent) + std::mem::offset_of!(c::EXTENT, height)
    );
};
const _: unsafe extern "C" fn(*mut c::RECT, c::BYTE) = c::DrawOval;
const _: unsafe extern "C" fn(*mut c::RECT) = c::DrawFilledOval;
const _: unsafe extern "C" fn(*mut c::RECT, c::BYTE) = draw_oval;
const _: unsafe extern "C" fn(*mut c::RECT) = draw_filled_oval;

fn input(rect: c::RECT) -> CRect {
    CRect {
        corner: CPoint {
            x: rect.corner.x,
            y: rect.corner.y,
        },
        width: rect.extent.width,
        height: rect.extent.height,
    }
}

/// `planets.h`: outline with the original unsigned-byte off-step cadence.
///
/// # Safety
/// Game thread only, with a readable RECT and an active graphics context and
/// foreground frame for the entire call. The renderer must not switch context
/// or reenter game menus. No input borrow survives a graphics call.
#[export_name = "DrawOval"]
pub unsafe extern "C" fn draw_oval(rect: *mut c::RECT, num_off_pixels: c::BYTE) {
    let rect = input(rect.read());
    let mut gfx = CffiOvalGfx::current(c::ScreenWidth, c::ScreenHeight);
    super::oval::draw_oval(&rect, num_off_pixels, &mut gfx);
}

/// `planets.h`: filled ellipse, retaining repeated centre rows.
///
/// # Safety
/// The same game-thread, input and graphics lifetime contract as [`draw_oval`].
#[export_name = "DrawFilledOval"]
pub unsafe extern "C" fn draw_filled_oval(rect: *mut c::RECT) {
    let rect = input(rect.read());
    let mut gfx = CffiOvalGfx::current(c::ScreenWidth, c::ScreenHeight);
    super::oval::draw_filled_oval(&rect, &mut gfx);
}

/// A short-lived borrow of the current graphics context on the game thread.
struct CffiOvalGfx {
    viewport: CRect,
    color: c::Color,
}

impl CffiOvalGfx {
    /// # Safety
    /// The current C context and foreground frame must remain active for this
    /// adapter's lifetime. Calls must stay on the game thread, with no context
    /// changes or reentrant menus between them.
    unsafe fn current(screen_width: i32, screen_height: i32) -> Self {
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

use super::topography::{
    render_topography, TopographyError, TopographyGfx, TopographyPalette, TopographyPoint,
    NUM_BATCH_POINTS,
};

use crate::graphics::cmap_abi::GetColorMapAddress;
use crate::graphics::context_abi::{ContextFrame, ContextLease};
use crate::graphics::native_abi::Color;
use crate::graphics::native_batch::NativeBatch;
use crate::strings::string_table::GetStringLengthBin;

struct CffiTopographyGfx {
    context: ContextLease,
    batch: NativeBatch,
}

impl CffiTopographyGfx {
    fn new(context: &ContextLease) -> Self {
        Self {
            context: context.clone(),
            batch: NativeBatch::new(Vec::new()),
        }
    }
}

impl TopographyGfx for CffiTopographyGfx {
    type Frame = ContextFrame;

    fn set_frame(&mut self, frame: Self::Frame) -> Self::Frame {
        let old = self.context.target();
        self.context.set_frame(frame);
        old
    }

    fn set_origin(&mut self, origin: CPoint) -> CPoint {
        let old = self.context.target().replace_hotspot(point(origin));
        CPoint { x: old.x, y: old.y }
    }

    fn clip(&mut self) -> CRect {
        input(self.context.clip_rect())
    }

    fn set_clip(&mut self, clip: Option<CRect>) {
        if let Some(clip) = clip {
            self.context.set_clip(rect(&clip));
        } else {
            self.context.clear_clip();
        }
    }

    fn batch(&mut self, points: &[TopographyPoint; NUM_BATCH_POINTS], first: usize) {
        self.batch
            .replace_point_chain(topography_points(points), first);
        self.context.draw_preserving_scale(&mut self.batch);
    }
}

pub fn topography_batch(points: &[TopographyPoint; NUM_BATCH_POINTS], first: usize) -> NativeBatch {
    NativeBatch::point_chain(topography_points(points), first)
}

fn topography_points(
    points: &[TopographyPoint; NUM_BATCH_POINTS],
) -> impl ExactSizeIterator<Item = (crate::engine::graphics::native_abi::POINT, Color)> + '_ {
    points.iter().map(|source| {
        let color = source.color;
        (
            point(source.point),
            Color {
                r: color.r,
                g: color.g,
                b: color.b,
                a: color.a,
            },
        )
    })
}

/// Render the current solar-system resources into a native frame.
///
/// # Safety
/// Game thread, active context, live destination and system/orbital descriptor.
/// Resource handles and XlatPtr must refer to their live external resource
/// storage for the call, without mutable aliases. No ownership is transferred.
pub unsafe fn render_system_topography(
    context: &ContextLease,
    system: *const c::SOLARSYS_STATE,
    destination: *mut c::FRAME_DESC,
    depth: &[i8],
    width: i32,
    height: i32,
) -> Result<(), TopographyError> {
    let mut gfx = CffiTopographyGfx::new(context);
    let destination = ContextFrame::retain(std::ptr::NonNull::new(destination.cast()));
    if (*system).XlatRef.is_null() {
        return render_topography(&mut gfx, destination, depth, width, height, None);
    }
    let index =
        (*(*system).pOrbitalDesc).data_index & !crate::planets::generate::consts::PLANET_SHIELDED;
    let planet = crate::planets::tables::PLANET_ARRAY
        .0
        .get(usize::from(index))
        .ok_or(TopographyError::PlanetIndex)?;
    // XLAT_DESC starts with three signed SIZE levels, followed by 256 bytes.
    let prefix = 3 * std::mem::size_of::<i16>();
    if usize::from(GetStringLengthBin((*system).XlatRef.cast())) < prefix + 256 {
        return Err(TopographyError::TranslationLength);
    }
    let translation = &*(*system)
        .XlatPtr
        .cast::<u8>()
        .add(prefix)
        .cast::<[u8; 256]>();
    let colors = std::slice::from_raw_parts(
        GetColorMapAddress((*system).OrbitalCMap.cast()).cast::<u8>(),
        usize::from(GetStringLengthBin((*system).OrbitalCMap.cast())),
    );
    render_topography(
        &mut gfx,
        destination,
        depth,
        width,
        height,
        Some(TopographyPalette {
            translation,
            colors,
            planet_type: planet.type_,
            base_elevation: planet.base_elevation,
        }),
    )
}
