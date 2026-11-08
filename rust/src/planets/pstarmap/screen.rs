use super::*;

use crate::comm::locdata::{CPoint, CRect};
use crate::graphics::native_batch::{DrawCommand, NativeBatch, NativeShape};
use crate::planets::render::oval::{self, OvalGfx};

pub(super) struct Display {
    pub context: ContextLease,
    pub frame: ContextFrame,
    font: FontLease,
    dimensions: (i32, i32),
}

impl Display {
    pub fn new(kernel: &Kernel, dimensions: (i32, i32)) -> Self {
        Self {
            context: ContextLease::require(kernel.space_context),
            frame: ContextFrame::retain(NonNull::new(kernel.misc_data_frame)).indexed_frame(48),
            font: kernel
                .tiny_font_lease()
                .unwrap_or_else(|| panic!("map requires the tiny font")),
            dimensions,
        }
    }

    fn sphere_text(&self, rect: RECT) -> POINT {
        POINT {
            x: rect.corner.x.wrapping_add(rect.extent.width >> 1),
            y: rect
                .corner
                .y
                .wrapping_add(rect.extent.height >> 1)
                .wrapping_sub(1),
        }
    }

    pub fn sphere_rect(
        &self,
        world: &World,
        fleet: &FleetInfo,
        projection: Projection,
    ) -> (RECT, RECT) {
        let diameter = c_long::from(i32::from(fleet.known_strength) * 11);
        let mut width = projection.x(diameter).wrapping_sub(projection.x(0));
        if width < 0 {
            width = width.wrapping_neg();
        } else if width == 0 {
            width = 1;
        }
        let mut height = projection.y(diameter).wrapping_sub(projection.y(0));
        if height < 0 {
            height = height.wrapping_neg();
        } else if height == 0 {
            height = 1;
        }
        let rect = RECT {
            corner: POINT {
                x: projection
                    .x(c_long::from(fleet.known_loc.x))
                    .wrapping_sub(width >> 1),
                y: projection
                    .y(c_long::from(fleet.known_loc.y))
                    .wrapping_sub(height >> 1),
            },
            extent: EXTENT { width, height },
        };
        let resources = world.fleet_resources(fleet.species_id);
        let (text, length) = resources.label(1);
        let mut repair = self
            .font
            .measure(text, self.sphere_text(rect), 1, length)
            .rect;
        let width = projection.width;
        let height = projection.height;
        if repair.corner.x <= 0 {
            repair.corner.x = 1;
        } else if i32::from(repair.corner.x) + i32::from(repair.extent.width) >= width {
            repair.corner.x = (width - i32::from(repair.extent.width) - 1) as i16;
        }
        if repair.corner.y <= 0 {
            repair.corner.y = 1;
        } else if i32::from(repair.corner.y) + i32::from(repair.extent.height) >= height {
            repair.corner.y = (height - i32::from(repair.extent.height) - 1) as i16;
        }
        repair = box_union(repair, rect);
        enlarge(&mut repair);
        (rect, repair)
    }

    fn sphere_label(&self, world: &World, fleet: &FleetInfo, rect: RECT, color: Color) {
        let resources = world.fleet_resources(fleet.species_id);
        let (text, length) = resources.label(1);
        let mut baseline = self.sphere_text(rect);
        let measurement = self.font.measure(text, baseline, 1, length);
        let rect = measurement.rect;
        let width = self.dimensions.0 - 78;
        let height = self.dimensions.1 - 13;
        if rect.corner.x <= 0 {
            baseline.x = baseline.x.wrapping_sub(rect.corner.x).wrapping_add(1);
        } else if i32::from(rect.corner.x) + i32::from(rect.extent.width) >= width {
            baseline.x = (i32::from(baseline.x)
                - (i32::from(rect.corner.x) + i32::from(rect.extent.width) - width + 1))
                as i16;
        }
        if rect.corner.y <= 0 {
            baseline.y = baseline.y.wrapping_sub(rect.corner.y).wrapping_add(1);
        } else if i32::from(rect.corner.y) + i32::from(rect.extent.height) >= height {
            baseline.y = (i32::from(baseline.y)
                - (i32::from(rect.corner.y) + i32::from(rect.extent.height) - height + 1))
                as i16;
        }
        self.context.draw_text_bounded(
            &self.font,
            text,
            baseline,
            1,
            color,
            measurement.characters,
        );
    }

    fn batch(&self, shapes: impl Iterator<Item = NativeShape>) {
        let color = self.context.foreground();
        let commands = shapes.map(|shape| DrawCommand { color, shape }).collect();
        self.context
            .draw_primitives(&mut NativeBatch::new(commands), 0);
    }
}

fn rect_input(rect: RECT) -> CRect {
    CRect {
        corner: rect.corner,
        width: rect.extent.width,
        height: rect.extent.height,
    }
}
fn rect_output(rect: CRect) -> RECT {
    RECT {
        corner: rect.corner,
        extent: EXTENT {
            width: rect.width,
            height: rect.height,
        },
    }
}
impl OvalGfx for Display {
    fn viewport(&self) -> CRect {
        CRect {
            width: (self.dimensions.0 - 78) as i16,
            height: (self.dimensions.1 - 13) as i16,
            ..CRect::default()
        }
    }
    fn line_visible(&mut self, viewport: &CRect, first: CPoint, second: CPoint) -> bool {
        self.context
            .line_visible(rect_output(*viewport), first, second)
    }
    fn draw_line(&mut self, first: CPoint, second: CPoint) {
        self.context
            .draw_line(self.context.foreground(), first, second);
    }
    fn points(&mut self, points: &[CPoint]) {
        self.batch(points.iter().copied().map(NativeShape::Point));
    }
    fn filled_rows(&mut self, rows: &[CRect; 2]) {
        self.batch(rows.iter().copied().map(|rect| NativeShape::Rectangle {
            rect: rect_output(rect),
            filled: true,
        }));
    }
}

impl StarMap {
    pub(super) fn draw_map(&mut self, world: &World, race: u16, clip: Option<RECT>, cursor: bool) {
        let context = self.display.context.clone();
        context.select();
        if let Some(clip) = clip {
            let old = context.clip_rect();
            let mut screen = clip;
            screen.corner.x = screen.corner.x.wrapping_add(old.corner.x);
            screen.corner.y = screen.corner.y.wrapping_add(old.corner.y);
            context.with_clip(screen, || {
                context.with_origin(
                    POINT {
                        x: clip.corner.x.wrapping_neg(),
                        y: clip.corner.y.wrapping_neg(),
                    },
                    || self.draw_content(world, race, Some(clip)),
                )
            });
        } else {
            self.draw_content(world, race, None);
        }
        if race == 0 && cursor {
            context.save_screen_region(context.clip_rect());
            self.draw_cursor(self.state.screen_cursor());
        }
    }

    fn draw_content(&mut self, world: &World, race: u16, clip: Option<RECT>) {
        let context = self.display.context.clone();
        if self.transition {
            context.transition_source(None);
        }
        context.with_batch(|| {
            let space = which_space(world);
            let grid = if space <= 1 {
                rgb15(0, 0, 7)
            } else {
                rgb15(0, 11, 0)
            };
            context.set_foreground(grid);
            context.set_background(if space <= 1 {
                rgb15(0, 0, 0)
            } else {
                rgb15(0, 8, 0)
            });
            context.clear();
            self.draw_fuel(world, race, space, grid);
            self.draw_grid(grid);
            self.draw_spheres(world, race, space, clip);
            self.draw_stars(world, space);
            self.draw_autopilot(world, race);
            if self.transition {
                context.transition(Some(context.clip_rect()));
                self.transition = false;
            }
        });
    }
    fn draw_fuel(&mut self, world: &World, race: u16, space: u8, grid: Color) {
        let context = self.display.context.clone();
        let projection = self.state.projection;
        let diameter = (i64::from(world.data.flagship.fuel_on_board) << 1).min(9999 * 4) as c_long;
        if race == 0 && space < 2 && diameter != 0 {
            let location = current_location(world, self.dimensions());
            let width = projection
                .x(diameter)
                .wrapping_sub(projection.x(0))
                .wrapping_abs();
            let height = projection
                .y(diameter)
                .wrapping_sub(projection.y(0))
                .wrapping_abs();
            let center = self.state.screen(location);
            let rect = RECT {
                corner: POINT {
                    x: center.x.wrapping_sub(width >> 1),
                    y: center.y.wrapping_sub(height >> 1),
                },
                extent: EXTENT { width, height },
            };
            context.set_foreground(rgb15(3, 3, 3));
            oval::draw_filled_oval(&rect_input(rect), &mut self.display);
            context.set_foreground(grid);
        }
    }

    fn draw_grid(&mut self, grid: Color) {
        let context = &self.display.context;
        let projection = self.state.projection;
        for i in (0..=10000).rev().step_by(500) {
            let mut rect = RECT {
                corner: POINT {
                    x: projection.x(0),
                    y: projection.y(i),
                },
                extent: EXTENT {
                    width: (projection.width << projection.zoom) as i16,
                    height: 1,
                },
            };
            context.fill_rectangle(grid, rect);
            rect.corner.y = projection.y(10000);
            rect.extent.width = 1;
            rect.extent.height = (projection.height << projection.zoom) as i16;
            for j in (0..=10000).rev().step_by(500) {
                rect.corner.x = projection.x(j);
                context.fill_rectangle(grid, rect);
            }
        }
    }

    fn draw_spheres(&mut self, world: &World, race: u16, space: u8, clip: Option<RECT>) {
        let context = self.display.context.clone();
        let projection = self.state.projection;
        if space <= 1 {
            let queue = world.avail_races_ref();
            let mut handle = queue.head();
            let mut index = 0;
            while let Some(id) = handle {
                let fleet = *queue.get(id);
                handle = queue.succ(id);
                if fleet.known_strength != 0 {
                    let (rect, repair) = self.display.sphere_rect(world, &fleet, projection);
                    if i32::from(rect.corner.x) < projection.width
                        && i32::from(rect.corner.y) < projection.height
                        && i32::from(rect.corner.x) + i32::from(rect.extent.width) > 0
                        && i32::from(rect.corner.y) + i32::from(rect.extent.height) > 0
                        && clip.is_none_or(|clip| intersects(repair, clip))
                    {
                        let mut color = RACE_COLORS[index];
                        context.set_foreground(if index + 1 == usize::from(race) {
                            rgb15(31, 31, 31)
                        } else {
                            color
                        });
                        oval::draw_oval(&rect_input(rect), 0, &mut self.display);
                        color.r = color.r.saturating_add(24);
                        color.g = color.g.saturating_add(24);
                        color.b = color.b.saturating_add(24);
                        self.display.sphere_label(world, &fleet, rect, color);
                    }
                }
                index += 1;
            }
        }
    }

    fn draw_stars(&mut self, world: &World, space: u8) {
        let context = &self.display.context;
        let start = if space <= 1 { 0 } else { NUM_SOLAR_SYSTEMS + 1 };
        for star in &world.catalog()[start..] {
            if star.star_pt.x > 9999 || star.star_pt.y > 9999 {
                break;
            }
            let index = if space <= 1 {
                u16::from((star.star_type & 0x18) >> 3) * 6 + u16::from(star.star_type >> 5)
            } else if star.star_pt == ARILOU_HOME {
                13
            } else {
                7
            };
            context.draw_stamp(
                &self.display.frame.wrapped_frame(50 + index),
                self.state.screen(star.star_pt),
            );
        }
        if portal_open(world) {
            let point = if space <= 1 {
                ARILOU_SPACE
            } else {
                QUASI_SPACE
            };
            context.draw_stamp(
                &self.display.frame.wrapped_frame(57),
                self.state.screen(point),
            );
        }
    }

    fn draw_autopilot(&mut self, world: &World, race: u16) {
        let context = &self.display.context;
        let destination = world.data.game.autopilot;
        if race == 0 && destination.x != -1 && destination.y != -1 {
            let mut point = self
                .state
                .screen(current_location(world, self.dimensions()));
            let destination = self.state.screen(destination);
            let mut walk = LineWalk::new(
                destination.x.wrapping_sub(point.x),
                destination.y.wrapping_sub(point.y),
            );
            context.set_foreground(rgb15(4, 4, 31));
            walk.delta &= !1;
            loop {
                let previous = walk.delta;
                walk.delta = walk.delta.wrapping_sub(1);
                if previous == 0 {
                    break;
                }
                if walk.delta & 1 == 0 {
                    context.draw_point(rgb15(4, 4, 31), point);
                }
                walk.step(&mut point);
            }
        }
    }
}

pub(super) fn repair_rectangle(last: RECT, next: RECT, width: i32, height: i32) -> RECT {
    let mut r = RECT {
        corner: POINT {
            x: (i32::from(next.corner.x) + i32::from(next.extent.width >> 1) - 25) as i16,
            y: (i32::from(next.corner.y) + i32::from(next.extent.height >> 1) - 9) as i16,
        },
        extent: EXTENT {
            width: 50,
            height: 9,
        },
    };
    if r.corner.x < 0 {
        r.corner.x = 0;
    } else if i32::from(r.corner.x) + 50 >= width {
        r.corner.x = (width - 50) as i16;
    }
    if r.corner.y < 0 {
        r.corner.y = 0;
    } else if i32::from(r.corner.y) + 9 >= height {
        r.corner.y = (height - 9) as i16;
    }
    r = box_union(last, r);
    r = box_union(next, r);
    if r.corner.x < 0 {
        r.extent.width = r.extent.width.wrapping_add(r.corner.x);
        r.corner.x = 0;
    }
    if i32::from(r.corner.x) + i32::from(r.extent.width) > width {
        r.extent.width = (width - i32::from(r.corner.x)) as i16;
    }
    if r.corner.y < 0 {
        r.extent.height = r.extent.height.wrapping_add(r.corner.y);
        r.corner.y = 0;
    }
    if i32::from(r.corner.y) + i32::from(r.extent.height) > height {
        r.extent.height = (height - i32::from(r.corner.y)) as i16;
    }
    r.extent.height = r.extent.height.wrapping_add(r.corner.y & 1);
    r.corner.y &= !1;
    r
}

pub(super) fn enlarge(r: &mut RECT) {
    r.extent.width = r.extent.width.wrapping_add(1);
    r.extent.height = r.extent.height.wrapping_add(1);
}
