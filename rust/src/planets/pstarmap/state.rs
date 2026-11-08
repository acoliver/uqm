use super::*;

pub(super) struct MapState {
    pub cursor: POINT,
    pub projection: Projection,
}

pub(super) struct CursorMove {
    pub old: POINT,
    pub next: POINT,
    pub recentered: bool,
}

impl MapState {
    pub fn new((width, height): (i32, i32), cursor: POINT) -> Self {
        Self {
            cursor,
            projection: Projection {
                origin: POINT { x: 4999, y: 4999 },
                zoom: 0,
                width: width - 78,
                height: height - 13,
            },
        }
    }

    pub fn screen_cursor(&self) -> POINT {
        self.screen(self.cursor)
    }

    pub fn screen(&self, point: POINT) -> POINT {
        POINT {
            x: self.projection.x(c_long::from(point.x)),
            y: self.projection.y(c_long::from(point.y)),
        }
    }

    pub fn zoom(&mut self, direction: i16) -> bool {
        let projection = &mut self.projection;
        if direction > 0 && projection.zoom < MAX_ZOOM_SHIFT {
            projection.zoom += 1;
            projection.origin = self.cursor;
        } else if direction < 0 && projection.zoom > 0 {
            projection.origin = if projection.zoom > 1 {
                self.cursor
            } else {
                POINT { x: 4999, y: 4999 }
            };
            projection.zoom -= 1;
        } else {
            return false;
        }
        true
    }

    pub fn move_cursor(&mut self, mut sx: i32, mut sy: i32, absolute: Option<POINT>) -> CursorMove {
        let old = self.screen_cursor();
        let mut screen = if let Some(point) = absolute {
            sx = 0;
            sy = 0;
            self.cursor = point;
            self.screen(point)
        } else {
            POINT {
                x: (i32::from(old.x) + sx) as i16,
                y: (i32::from(old.y) + sy) as i16,
            }
        };
        if sx != 0 {
            self.cursor.x = (i32::from(self.projection.inverse_x(screen.x)) - sx) as i16;
            while self.screen_cursor().x == old.x {
                self.cursor.x = (i32::from(self.cursor.x) + sx) as i16;
            }
            self.cursor.x = self.cursor.x.clamp(0, 9999);
            screen.x = self.screen_cursor().x;
        }
        if sy != 0 {
            self.cursor.y = (i32::from(self.projection.inverse_y(screen.y)) + sy) as i16;
            while self.screen_cursor().y == old.y {
                self.cursor.y = (i32::from(self.cursor.y) - sy) as i16;
            }
            self.cursor.y = self.cursor.y.clamp(0, 9999);
            screen.y = self.screen_cursor().y;
        }
        let recentered = screen.x < 0
            || screen.y < 0
            || i32::from(screen.x) >= self.projection.width
            || i32::from(screen.y) >= self.projection.height;
        if recentered {
            self.projection.origin = self.cursor;
        }
        CursorMove {
            old,
            next: screen,
            recentered,
        }
    }

    pub fn snap_cursor(&mut self, world: &World) -> Option<crate::engine::StarId> {
        let screen = self.screen_cursor();
        let mut last = None;
        let mut best: Option<crate::engine::StarId> = None;
        while let Some(id) =
            super::super::starmap::find_star_id(world, world.catalog(), last, self.cursor, 75, 75)
        {
            last = Some(id);
            let star = &world.catalog()[id.0];
            if self.screen(star.star_pt) == screen
                && best.is_none_or(|best| {
                    star.star_type & 0x18 >= world.catalog()[best.0].star_type & 0x18
                })
            {
                best = Some(id);
            }
        }
        self.cursor = best.map_or_else(
            || POINT {
                x: self.projection.inverse_x(screen.x).clamp(0, 9999),
                y: self.projection.inverse_y(screen.y).clamp(0, 9999),
            },
            |id| world.catalog()[id.0].star_pt,
        );
        best
    }
}
