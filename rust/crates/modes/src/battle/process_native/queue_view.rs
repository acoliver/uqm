//! View calculations for the observed element queue, independent of hook invocation.

use super::*;
use uqm_engine::graphics::geometry::Point;

pub(super) fn render_scale(mode: i32, zoom: i16) -> i32 {
    if mode == SCALE_STEP {
        0
    } else {
        i32::from(calc_zoom_stuff(zoom).1)
    }
}

pub(super) fn draw_frame(counter: &mut u16) -> bool {
    let skip = (*counter >> 8) as u8;
    if skip == u8::MAX {
        return false;
    }
    if skip != 0 {
        *counter = counter.wrapping_sub(1);
        if *counter & 0x00ff != 0 {
            return false;
        }
    }
    *counter = counter.wrapping_add(u16::from(skip));
    true
}

pub(super) struct CameraBounds {
    minimum: i16,
    maximum: i16,
    pub origin: Point,
    pub ships_alive: u16,
}

pub(super) struct ViewConfig {
    pub limit: i16,
    pub zoom: i16,
    pub step: bool,
    pub activity: u8,
    pub dimensions: (i32, i32),
}

impl CameraBounds {
    pub(super) fn new(step: bool, dimensions: (i32, i32)) -> Self {
        let minimum = if step {
            (MAX_VIS_REDUCTION + 1) as i16
        } else {
            (MAX_ZOOM_OUT + (1 << ZOOM_SHIFT)) as i16
        };
        Self {
            minimum,
            maximum: minimum,
            origin: Point {
                x: (dimensions.0 >> 1) as i16,
                y: (dimensions.1 >> 1) as i16,
            },
            ships_alive: 0,
        }
    }

    pub(super) fn include(
        &mut self,
        location: Point,
        player: i16,
        active_sides: u16,
        view: &ViewConfig,
    ) {
        self.ships_alive = self.ships_alive.wrapping_add(1);
        if self.maximum > view.limit && self.minimum > view.limit {
            self.origin.x = display_align(i32::from(location.x));
            self.origin.y = display_align(i32::from(location.y));
        }
        let mut dx = wrap_delta(
            (i32::from(display_align(i32::from(location.x))) - i32::from(self.origin.x)) as i16,
            view.dimensions.0,
        );
        let mut dy = wrap_delta(
            (i32::from(display_align(i32::from(location.y))) - i32::from(self.origin.y)) as i16,
            view.dimensions.1,
        );
        if active_sides <= 2 || player == 0 {
            self.origin.x = display_align(i32::from(self.origin.x) + i32::from(dx >> 1));
            self.origin.y = display_align(i32::from(self.origin.y) + i32::from(dy >> 1));
            dx = dx.wrapping_abs();
            dy = dy.wrapping_abs();
            self.maximum = view.reduction(dx, dy);
        } else if self.maximum > view.limit && self.minimum <= view.limit {
            self.origin.x = display_align(i32::from(self.origin.x) + i32::from(dx >> 1));
            self.origin.y = display_align(i32::from(self.origin.y) + i32::from(dy >> 1));
            dx = dx.wrapping_abs();
            dy = dy.wrapping_abs();
            self.minimum = view.reduction(dx, dy);
        } else {
            dx = dx.wrapping_abs();
            dy = dy.wrapping_abs();
            let reduction =
                view.reduction((i32::from(dx) << 1) as i16, (i32::from(dy) << 1) as i16);
            if self.minimum > view.limit || reduction < self.minimum {
                self.minimum = reduction;
            }
        }
    }

    pub(super) fn finish(&mut self, limit: i16, zoom: i16, step: bool) -> i16 {
        if (self.minimum > limit || self.minimum <= self.maximum)
            && {
                self.minimum = self.maximum;
                self.minimum > limit
            }
            && {
                self.minimum = zoom;
                self.minimum > limit
            }
        {
            self.minimum = if step { 0 } else { 1 << ZOOM_SHIFT };
        }
        self.minimum
    }
}

impl ViewConfig {
    fn reduction(&self, dx: i16, dy: i16) -> i16 {
        calc_reduction(
            dx,
            dy,
            self.zoom,
            self.step,
            self.activity,
            self.dimensions.0,
            self.dimensions.1,
        ) as i16
    }
}

pub(super) struct Projection {
    pub dimensions: (i32, i32),
    pub origin: Point,
    pub reduction: i16,
    pub step: bool,
}

impl Projection {
    pub(super) fn scroll(&self, location: Point, delta: (i16, i16)) -> Point {
        Point {
            x: wrap_value(
                i32::from(location.x) + i32::from(delta.0),
                self.dimensions.0,
            ),
            y: wrap_value(
                i32::from(location.y) + i32::from(delta.1),
                self.dimensions.1,
            ),
        }
    }

    pub(super) fn display(&self, location: Point) -> Point {
        Point {
            x: calc_display_coord(location.x, self.origin.x, self.reduction, self.step),
            y: calc_display_coord(location.y, self.origin.y, self.reduction, self.step),
        }
    }

    pub(super) fn line(
        &self,
        current: Point,
        next: Point,
        delta: (i16, i16),
    ) -> (Point, Point, Point) {
        let dx = next.x.wrapping_sub(current.x);
        let dy = next.y.wrapping_sub(current.y);
        let mut next = self.scroll(current, delta);
        let first = self.display(next);
        next.x = next.x.wrapping_add(dx);
        next.y = next.y.wrapping_add(dy);
        (next, first, self.display(next))
    }
}

#[cfg(test)]
mod tests;
