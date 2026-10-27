//! View calculations for the observed element queue, independent of hook invocation.

use super::*;

pub(super) struct CameraBounds {
    minimum: SIZE,
    maximum: SIZE,
    pub origin: POINT,
    pub ships_alive: COUNT,
}

pub(super) struct ViewConfig {
    pub limit: SIZE,
    pub zoom: SIZE,
    pub step: bool,
    pub activity: BYTE,
    pub dimensions: (i32, i32),
}

impl CameraBounds {
    pub(super) fn new(step: bool, dimensions: (i32, i32)) -> Self {
        let minimum = if step {
            (MAX_VIS_REDUCTION + 1) as SIZE
        } else {
            (MAX_ZOOM_OUT + (1 << ZOOM_SHIFT)) as SIZE
        };
        Self {
            minimum,
            maximum: minimum,
            origin: POINT {
                x: (dimensions.0 >> 1) as COORD,
                y: (dimensions.1 >> 1) as COORD,
            },
            ships_alive: 0,
        }
    }

    pub(super) fn include(
        &mut self,
        location: POINT,
        player: SIZE,
        active_sides: COUNT,
        view: &ViewConfig,
    ) {
        self.ships_alive = self.ships_alive.wrapping_add(1);
        if self.maximum > view.limit && self.minimum > view.limit {
            self.origin.x = display_align(i32::from(location.x));
            self.origin.y = display_align(i32::from(location.y));
        }
        let mut dx = wrap_delta(
            (i32::from(display_align(i32::from(location.x))) - i32::from(self.origin.x)) as SIZE,
            view.dimensions.0,
        );
        let mut dy = wrap_delta(
            (i32::from(display_align(i32::from(location.y))) - i32::from(self.origin.y)) as SIZE,
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
                view.reduction((i32::from(dx) << 1) as SIZE, (i32::from(dy) << 1) as SIZE);
            if self.minimum > view.limit || reduction < self.minimum {
                self.minimum = reduction;
            }
        }
    }

    pub(super) fn finish(&mut self, limit: SIZE, zoom: SIZE, step: bool) -> SIZE {
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
    fn reduction(&self, dx: SIZE, dy: SIZE) -> SIZE {
        calc_reduction(
            dx,
            dy,
            self.zoom,
            self.step,
            self.activity,
            self.dimensions.0,
            self.dimensions.1,
        ) as SIZE
    }
}

pub(super) struct Projection {
    pub dimensions: (i32, i32),
    pub origin: POINT,
    pub reduction: SIZE,
    pub step: bool,
}

impl Projection {
    pub(super) fn scroll(&self, location: POINT, delta: (SIZE, SIZE)) -> POINT {
        POINT {
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

    pub(super) fn display(&self, location: POINT) -> POINT {
        POINT {
            x: calc_display_coord(location.x, self.origin.x, self.reduction, self.step),
            y: calc_display_coord(location.y, self.origin.y, self.reduction, self.step),
        }
    }

    pub(super) fn line(
        &self,
        current: POINT,
        next: POINT,
        delta: (SIZE, SIZE),
    ) -> (POINT, POINT, POINT) {
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
