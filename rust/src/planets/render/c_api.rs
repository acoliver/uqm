//! Oval and topography drawing on caller-retained contexts.

use crate::comm::locdata::{CPoint, CRect};
use crate::starbase::ids::RadarLayout;

use super::oval::OvalGfx;

fn input(rect: crate::graphics::native_abi::RECT) -> CRect {
    CRect {
        corner: CPoint {
            x: rect.corner.x,
            y: rect.corner.y,
        },
        width: rect.extent.width,
        height: rect.extent.height,
    }
}

/// Oval drawing on a caller-retained native context.
pub struct NativeOvalGfx {
    context: ContextLease,
    viewport: CRect,
    color: Color,
}

impl NativeOvalGfx {
    pub fn new(context: &ContextLease, screen_width: i32, screen_height: i32) -> Self {
        let layout = RadarLayout::for_screen(screen_width, screen_height);
        Self {
            context: context.clone(),
            viewport: CRect {
                width: layout.sis_screen_width as i16,
                height: layout.sis_screen_height as i16,
                ..CRect::default()
            },
            color: context.foreground(),
        }
    }

    fn batch(&self, shapes: impl IntoIterator<Item = NativeShape>) {
        let commands = shapes
            .into_iter()
            .map(|shape| DrawCommand {
                color: self.color,
                shape,
            })
            .collect();
        self.context
            .draw_preserving_scale(&mut NativeBatch::forward_chain(commands));
    }
}

fn point(p: CPoint) -> crate::graphics::native_abi::POINT {
    crate::graphics::native_abi::POINT { x: p.x, y: p.y }
}

fn rect(r: &CRect) -> crate::graphics::native_abi::RECT {
    crate::graphics::native_abi::RECT {
        corner: point(r.corner),
        extent: crate::graphics::native_abi::EXTENT {
            width: r.width,
            height: r.height,
        },
    }
}

impl OvalGfx for NativeOvalGfx {
    fn viewport(&self) -> CRect {
        self.viewport
    }

    fn line_visible(&mut self, viewport: &CRect, first: CPoint, second: CPoint) -> bool {
        self.context
            .line_visible(rect(viewport), point(first), point(second))
    }

    fn draw_line(&mut self, first: CPoint, second: CPoint) {
        self.context
            .draw_line(self.color, point(first), point(second));
    }

    fn points(&mut self, points: &[CPoint]) {
        self.context
            .draw_preserving_scale(&mut NativeBatch::point_chain(
                points.iter().map(|p| (point(*p), self.color)),
                0,
            ));
    }

    fn filled_rows(&mut self, rows: &[CRect; 2]) {
        self.batch(rows.iter().map(|row| NativeShape::Rectangle {
            rect: rect(row),
            filled: true,
        }));
    }
}

use super::topography::{
    render_topography, TopographyError, TopographyGfx, TopographyPalette, TopographyPoint,
    NUM_BATCH_POINTS,
};

use crate::graphics::context_abi::{ContextFrame, ContextLease};
use crate::graphics::native_abi::Color;
use crate::graphics::native_batch::{DrawCommand, NativeBatch, NativeShape};

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

/// Render a retained palette into the supplied native frame.
pub fn render_native_topography(
    context: &ContextLease,
    destination: ContextFrame,
    depth: &[i8],
    width: i32,
    height: i32,
    palette: Option<TopographyPalette<'_>>,
) -> Result<(), TopographyError> {
    render_topography(
        &mut CffiTopographyGfx::new(context),
        destination,
        depth,
        width,
        height,
        palette,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::graphics::{
        context_abi::LoadedContext,
        drawable_abi::{LoadedDrawable, ReadFramePixelColors},
    };

    #[test]
    #[serial_test::serial]
    fn oval_retains_its_supplied_context_and_frame_after_owner_release() {
        let mut game = crate::engine::Game::new();
        let display = crate::campaign_display_test::CampaignDisplay::install(&mut game).unwrap();
        {
            let owner = LoadedContext::create(c"retained oval fixture");
            let drawable = LoadedDrawable::create(2, 32, 32, 1).unwrap();
            let frame = drawable.lease().frame(0).unwrap();
            let context = owner.lease();
            context.set_frame(drawable.lease().context_frame(0));
            let color = Color {
                r: 71,
                g: 131,
                b: 197,
                a: 255,
            };
            context.set_foreground(color);
            let mut renderer = NativeOvalGfx::new(&context, 320, 240);
            // SAFETY: the serialized fixture owns native drawing state.
            let old_scale = unsafe { crate::graphics::gfx_common_abi::set_graphic_scale(171) };
            drop(owner);
            drop(drawable);
            drop(context);
            let unrelated = LoadedContext::create(c"unrelated oval context");
            let unrelated_context = unrelated.lease();
            let unrelated_frame = LoadedDrawable::create(2, 32, 32, 1).unwrap();
            unrelated_context.set_frame(unrelated_frame.lease().context_frame(0));
            unrelated_context.set_foreground(Color {
                r: 255,
                g: 0,
                b: 0,
                a: 255,
            });
            let read_unrelated = || {
                let mut pixels = vec![
                    Color {
                        r: 0,
                        g: 0,
                        b: 0,
                        a: 0
                    };
                    32 * 32
                ];
                // SAFETY: the independent owner retains this RAM frame and complete pixel buffer.
                assert_ne!(
                    unsafe {
                        ReadFramePixelColors(
                            unrelated_frame.lease().frame(0).unwrap(),
                            pixels.as_mut_ptr(),
                            32,
                            32,
                        )
                    },
                    0
                );
                pixels
                    .into_iter()
                    .map(|pixel| (pixel.r, pixel.g, pixel.b, pixel.a))
                    .collect::<Vec<_>>()
            };
            let unrelated_before = read_unrelated();
            unrelated_context.select();
            let rectangle = CRect {
                corner: CPoint { x: 3, y: 5 },
                width: 15,
                height: 9,
            };
            assert!(!renderer.line_visible(
                &renderer.viewport(),
                CPoint { x: -8, y: -8 },
                CPoint { x: -4, y: -4 }
            ));
            super::super::oval::draw_oval(&rectangle, 1, &mut renderer);
            super::super::oval::draw_filled_oval(&rectangle, &mut renderer);
            super::super::oval::draw_oval(
                &CRect {
                    width: 1,
                    ..rectangle
                },
                0,
                &mut renderer,
            );
            renderer.context.flush();
            let mut pixels = vec![
                Color {
                    r: 0,
                    g: 0,
                    b: 0,
                    a: 0
                };
                32 * 32
            ];
            // SAFETY: the renderer retains the initialized RAM frame and the buffer covers it.
            assert_ne!(
                unsafe { ReadFramePixelColors(frame, pixels.as_mut_ptr(), 32, 32) },
                0
            );
            assert!(pixels
                .iter()
                .any(|p| (p.r, p.g, p.b, p.a) == (71, 131, 197, 255)));
            assert_eq!(read_unrelated(), unrelated_before);
            assert_eq!(unrelated_context.foreground().r, 255);
            let foreground = renderer.context.foreground();
            assert_eq!(
                (foreground.r, foreground.g, foreground.b, foreground.a),
                (71, 131, 197, 255)
            );
            // SAFETY: the serialized fixture owns native drawing state.
            unsafe {
                assert_eq!(crate::graphics::gfx_common_abi::get_graphic_scale(), 171);
                crate::graphics::gfx_common_abi::set_graphic_scale(old_scale);
            }
            ContextLease::require(game.kernel.status_context).select();
        }
        display.finish(game);
    }
}
