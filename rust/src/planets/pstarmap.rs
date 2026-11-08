//! Owned star-map input, drawing and search for one synchronous visit.
mod animation;
mod cursor;
mod screen;
pub(crate) mod search;
mod state;

#[cfg(test)]
use crate::engine::observe_port::starmap::signed_div_with_error;
use crate::engine::observe_port::starmap::Projection;
use crate::engine::time::counter_abi::GetTimeCounter;
use crate::engine::{
    battle::{link_queue::LinkId, Cx},
    input::{Frame, InputSession},
    observe::{CheckpointView, Injected, InputView, Io, StarMapView},
    world::queues::{Fleet, FleetInfo},
    Kernel, StoryKey, World,
};
use crate::graphics::{
    context_abi::{ContextFrame, ContextLease},
    native_abi::{Color, EXTENT, POINT, RECT},
    native_font::FontLease,
};
use crate::savegame::runtime::star::{CStarDesc, NUM_SOLAR_SYSTEMS};
use screen::{enlarge, repair_rectangle, Display};
use state::MapState;
use std::{
    ffi::{c_long, CStr, CString},
    ptr::NonNull,
};

const ONE_SECOND: u32 = 840;
const MAX_ZOOM_SHIFT: i32 = 4;
const ARILOU_SPACE: POINT = POINT { x: 438, y: 6372 };
const QUASI_SPACE: POINT = POINT { x: 5000, y: 5000 };
const ARILOU_HOME: POINT = POINT { x: 6134, y: 5900 };

fn which_space(world: &World) -> u8 {
    world.story_state(StoryKey::ARILOU_SPACE_SIDE) as u8
}
fn portal_open(world: &World) -> bool {
    world.story_state(StoryKey::ARILOU_SPACE) != 0
}
fn current_location(world: &World, dimensions: (i32, i32)) -> POINT {
    if crate::navigation::globals::in_hqspace(world) == 0 {
        world
            .selected_star()
            .unwrap_or_else(|| panic!("map requires a selected star"))
            .star_pt
    } else {
        let geometry = crate::navigation::hyper_units::Geometry::new(dimensions);
        POINT {
            x: geometry.universe_x(world.data.flagship.log_x),
            y: geometry.universe_y(world.data.flagship.log_y),
        }
    }
}

struct StarMap {
    state: MapState,
    display: Display,
    universe: POINT,
    initialized: bool,
    last_label: CString,
    transition: bool,
}
impl StarMap {
    fn new(cx: &Cx<'_>) -> Self {
        let dimensions = cx.screen_dimensions();
        let universe = current_location(cx.world, dimensions);
        let destination = cx.world.data.game.autopilot;
        Self {
            state: MapState::new(
                dimensions,
                if destination == (POINT { x: -1, y: -1 }) {
                    universe
                } else {
                    destination
                },
            ),
            display: Display::new(cx.kernel, dimensions),
            universe,
            initialized: false,
            last_label: c"".into(),
            transition: true,
        }
    }
    fn dimensions(&self) -> (i32, i32) {
        (
            self.state.projection.width + 78,
            self.state.projection.height + 13,
        )
    }
    fn observe(&self, io: &mut Io<'_>) {
        io.observer
            .checkpoint(&CheckpointView::StarMap(self.initialized.then_some(
                StarMapView {
                    cursor: self.state.cursor,
                    projection: self.state.projection,
                },
            )));
    }
    fn flash(&self, kernel: &mut Kernel) {
        if let Some(color) = kernel.star_map_flash.advance(GetTimeCounter) {
            let point = self.state.screen(self.universe);
            self.display.context.with_selected(|| {
                self.display.context.draw_filled_stamp_preserving_color(
                    &self.display.frame.next_frame(),
                    point,
                    rgb15(color, color, color),
                )
            });
        }
    }
    fn draw_cursor(&self, point: POINT) {
        self.display.context.draw_stamp(&self.display.frame, point);
    }
    fn erase_cursor(&self, point: POINT) {
        let mut rect = self.display.frame.rect();
        rect.corner.x = rect.corner.x.wrapping_add(point.x);
        if rect.corner.x < 0 {
            rect.extent.width = rect.extent.width.wrapping_add(rect.corner.x);
            rect.corner.x = 0;
        } else if i32::from(rect.corner.x) + i32::from(rect.extent.width)
            >= self.state.projection.width
        {
            rect.extent.width = (self.state.projection.width - i32::from(rect.corner.x)) as i16;
        }
        rect.corner.y = rect.corner.y.wrapping_add(point.y);
        if rect.corner.y < 0 {
            rect.extent.height = rect.extent.height.wrapping_add(rect.corner.y);
            rect.corner.y = 0;
        } else if i32::from(rect.corner.y) + i32::from(rect.extent.height)
            >= self.state.projection.height
        {
            rect.extent.height = (self.state.projection.height - i32::from(rect.corner.y)) as i16;
        }
        let origin = self.display.context.clip_rect().corner;
        rect.corner.x = rect.corner.x.wrapping_add(origin.x);
        rect.corner.y = rect.corner.y.wrapping_add(origin.y);
        rect.extent.height = rect.extent.height.wrapping_add(rect.corner.y & 1);
        rect.corner.y &= !1;
        self.display.context.restore_screen_region(rect);
    }
    fn move_cursor(&mut self, world: &World, sx: i32, sy: i32, absolute: Option<POINT>) {
        let movement = self.state.move_cursor(sx, sy, absolute);
        if movement.recentered {
            self.draw_map(world, 0, None, true);
        } else {
            self.erase_cursor(movement.old);
            self.draw_cursor(movement.next);
        }
    }
    fn update_cursor_info(&mut self, kernel: &Kernel, world: &World) {
        let screen = self.state.screen_cursor();
        let selected = self.state.snap_cursor(world);
        let mut text = selected.map_or_else(
            || c"".into(),
            |id| {
                super::starmap::naming::cluster_name(
                    kernel,
                    &world.catalog()[id.0],
                    crate::navigation::abi::STAR_NUMBER_BASE as usize,
                )
            },
        );
        if portal_open(world) {
            let portal = if which_space(world) <= 1 {
                ARILOU_SPACE
            } else {
                QUASI_SPACE
            };
            if self.state.screen(portal) == screen {
                self.state.cursor = portal;
                text = kernel.game_text(132).into();
            }
        }
        crate::ui::sis::draw_hyper_coords(kernel, self.dimensions().0, self.state.cursor);
        if text != self.last_label {
            self.last_label = text;
            crate::ui::sis::draw_message(kernel, self.dimensions().0, &self.last_label, -1, -1, 0);
        }
    }
    fn fuel(&self, kernel: &Kernel, world: &World) {
        let fuel = fuel_requirement(
            current_location(world, self.dimensions()),
            self.state.cursor,
            which_space(world),
            crate::math::square_root,
        );
        let mut text = kernel.game_text(439).to_bytes().to_vec();
        text.extend_from_slice(format!(" {}.{}", fuel / 100, (fuel % 100) / 10).as_bytes());
        let text = CString::new(text).unwrap_or_else(|error| panic!("invalid fuel label: {error}"));
        crate::ui::sis::draw_status_message(kernel, world, Some(&text));
    }
    fn zoom(&mut self, cx: &mut Cx<'_>, direction: i16) {
        if self.state.zoom(direction) {
            self.draw_map(cx.world, 0, None, true);
            cx.kernel.input_runtime().sleep(cx, ONE_SECOND / 8);
        }
    }
}

/// Run the map with its cursor, graphics leases and search owned by this stack frame.
pub fn star_map(cx: &mut Cx<'_>, io: &mut Io<'_>) -> u32 {
    let runtime = cx.kernel.input_runtime();
    let input = runtime.input();
    let mut map = StarMap::new(cx);
    if which_space(cx.world) <= 1 {
        map.update_map(cx, input);
    }
    map.draw_map(cx.world, 0, None, false);
    map.transition = false;
    map.display.context.with_batch(|| {
        map.display
            .context
            .save_screen_region(map.display.context.clip_rect());
        map.draw_cursor(map.state.screen_cursor());
    });
    input.set_menu_sounds(0, 0);
    input.configure_repeat(14, 105, 7, true, GetTimeCounter());
    let previous = map.display.context.selected_context();
    runtime.run(cx, io, false, &mut map);
    io.observer.checkpoint(&CheckpointView::StarMap(None));
    input.set_menu_sounds(15, 16);
    input.configure_repeat(
        ONE_SECOND / 12,
        ONE_SECOND / 2,
        ONE_SECOND / 12,
        false,
        GetTimeCounter(),
    );
    let dimensions = map.dimensions();
    crate::ui::sis::draw_hyper_coords(cx.kernel, dimensions.0, map.universe);
    let text = crate::ui::sis::default_message_text(cx.kernel, cx.world);
    crate::ui::sis::draw_message(cx.kernel, dimensions.0, &text, -1, -1, 0);
    crate::ui::sis::draw_status_message(cx.kernel, cx.world, None);
    if let Some(previous) = previous {
        previous.select();
    }
    if cx.world.data.game.autopilot == map.universe {
        cx.world.data.game.autopilot = POINT { x: -1, y: -1 };
    }
    u32::from(cx.world.data.game.autopilot.x != -1 && cx.world.data.game.autopilot.y != -1)
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod owned_tests;

const RACE_COLORS: [Color; 23] = [
    rgb15(0, 0, 16),
    rgb15(0, 0, 0),
    rgb15(0, 1, 31),
    rgb15(14, 0, 14),
    rgb15(0, 6, 8),
    rgb15(0, 0, 0),
    rgb15(12, 5, 0),
    rgb15(12, 5, 0),
    rgb15(0, 6, 8),
    rgb15(0, 6, 8),
    rgb15(0, 0, 16),
    rgb15(10, 0, 17),
    rgb15(0, 6, 8),
    rgb15(15, 0, 0),
    rgb15(14, 0, 14),
    rgb15(14, 0, 14),
    rgb15(12, 5, 0),
    rgb15(10, 0, 17),
    rgb15(0, 8, 0),
    rgb15(15, 0, 0),
    rgb15(0, 0, 0),
    rgb15(6, 6, 6),
    rgb15(20, 7, 31),
];

const fn rgb15(r: u8, g: u8, b: u8) -> Color {
    Color {
        r: (r << 3) | (r >> 2),
        g: (g << 3) | (g >> 2),
        b: (b << 3) | (b >> 2),
        a: 255,
    }
}

fn zero_rect() -> RECT {
    RECT {
        corner: POINT { x: 0, y: 0 },
        extent: EXTENT {
            width: 0,
            height: 0,
        },
    }
}

struct LineWalk {
    dx: i16,
    dy: i16,
    xi: i16,
    yi: i16,
    xe: i16,
    ye: i16,
    cycle: i16,
    delta: i16,
}

impl LineWalk {
    fn new(mut dx: i16, mut dy: i16) -> Self {
        let xi = if dx >= 0 {
            1
        } else {
            dx = dx.wrapping_neg();
            -1
        };
        dx = dx.wrapping_shl(1);
        let yi = if dy >= 0 {
            1
        } else {
            dy = dy.wrapping_neg();
            -1
        };
        dy = dy.wrapping_shl(1);
        let cycle = dx.max(dy);
        Self {
            dx,
            dy,
            xi,
            yi,
            xe: cycle >> 1,
            ye: cycle >> 1,
            cycle,
            delta: cycle >> 1,
        }
    }
    fn step(&mut self, p: &mut POINT) {
        self.xe = self.xe.wrapping_sub(self.dx);
        if self.xe <= 0 {
            p.x = p.x.wrapping_add(self.xi);
            self.xe = self.xe.wrapping_add(self.cycle);
        }
        self.ye = self.ye.wrapping_sub(self.dy);
        if self.ye <= 0 {
            p.y = p.y.wrapping_add(self.yi);
            self.ye = self.ye.wrapping_add(self.cycle);
        }
    }
}

fn box_union(a: RECT, b: RECT) -> RECT {
    let x = i32::from(a.corner.x).min(i32::from(b.corner.x));
    let y = i32::from(a.corner.y).min(i32::from(b.corner.y));
    RECT {
        corner: POINT {
            x: x as i16,
            y: y as i16,
        },
        extent: EXTENT {
            width: ((i32::from(a.corner.x) + i32::from(a.extent.width))
                .max(i32::from(b.corner.x) + i32::from(b.extent.width))
                - x) as i16,
            height: ((i32::from(a.corner.y) + i32::from(a.extent.height))
                .max(i32::from(b.corner.y) + i32::from(b.extent.height))
                - y) as i16,
        },
    }
}

fn intersects(a: RECT, b: RECT) -> bool {
    i32::from(a.corner.x) < i32::from(b.corner.x) + i32::from(b.extent.width)
        && i32::from(a.corner.y) < i32::from(b.corner.y) + i32::from(b.extent.height)
        && i32::from(a.corner.x) + i32::from(a.extent.width) > i32::from(b.corner.x)
        && i32::from(a.corner.y) + i32::from(a.extent.height) > i32::from(b.corner.y)
}

fn fuel_requirement(from: POINT, to: POINT, space: u8, sqrt: impl FnOnce(u32) -> u16) -> u16 {
    let x = from.x.wrapping_sub(to.x);
    let y = from.y.wrapping_sub(to.y);
    let f = (c_long::from(x) * c_long::from(x) + c_long::from(y) * c_long::from(y)) as u32;
    if f == 0 || space > 1 {
        0
    } else {
        sqrt(f).wrapping_add(100 / 20)
    }
}
