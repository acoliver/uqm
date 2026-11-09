//! Interplanetary flight and collision handling from `solarsys.c`.

use super::drawing::{draw_outer_system, draw_system_transition, find_radius, point_within_rect};
use super::*;
use crate::comm::locdata::CPoint;
use crate::engine::world::generation::Body;
use crate::engine::{Kernel as KernelOwner, WorldData as WorldFields};
use crate::planets::solar_system::{SolarSystem, SystemLocation};
use uqm_engine::input_port::frames as input;

const MIN_MOON_RADIUS: i32 = 35;
const MAX_MOONS: i32 = 4;
const MOON_DELTA: i32 = 20;
const MAX_ZOOM_RADIUS: c::SIZE = 128 << 6;
const INNER_ENTRY_DISTANCE: i32 = MIN_MOON_RADIUS + (MAX_MOONS - 1) * MOON_DELTA + MOON_DELTA / 4;
const IP_SHIP_THRUST_INCREMENT: i32 = 8;
const SHIP_AT_MAX_SPEED: c::UWORD = 1 << 7;
const END_INTERPLANETARY: c::UWORD = 1 << 11;
const MAX_TIME_VALUE: c::TIME_VALUE = (1 << 8) + 1;
const THRUST_WAIT: c::BYTE = 1;
const HALF_CIRCLE: c::COUNT = 32;
const QUADRANT: c::COUNT = 16;
const SIN_SHIFT: u32 = 14;

use crate::graphics::pixmap_abi::DecFrameIndex;

use crate::graphics::pixmap_abi::GetFrameIndex;

use crate::graphics::pixmap_abi::SetAbsFrameIndex;

use crate::graphics::context_abi::SetContext;

use crate::graphics::gfx_common_abi::SetTransitionSource;

use crate::battle::process_native::engine::redraw_queue;
use crate::engine::battle::Cx;

use crate::graphics::gfx_common_abi::ScreenTransition;

use crate::graphics::frame_abi::DrawStamp;

use crate::time::clock_abi::live::GameClockTick;

use crate::graphics::gfx_common_abi::{BatchGraphics, UnbatchGraphics};

#[inline]
fn glob() -> crate::engine::Projection<crate::engine::WorldData> {
    WorldFields::current()
}

#[inline]
fn normalize_facing(facing: c::COUNT) -> c::COUNT {
    facing & 15
}

#[inline]
fn facing_to_angle(facing: c::COUNT) -> c::COUNT {
    facing.wrapping_shl(6 - 4)
}

#[inline]
fn sine(angle: c::COUNT, magnitude: i32) -> c::SIZE {
    ((i64::from(crate::math::sinetab[usize::from(angle & 63)]) * i64::from(magnitude)) >> SIN_SHIFT)
        as c::SIZE
}

#[inline]
fn cosine(angle: c::COUNT, magnitude: i32) -> c::SIZE {
    sine(angle.wrapping_add(QUADRANT), magnitude)
}

#[inline]
fn make_word(low: c::BYTE, high: c::BYTE) -> c::COUNT {
    (c::COUNT::from(high) << 8) | c::COUNT::from(low)
}

#[inline]
fn speed_squared(dx: c::SIZE, dy: c::SIZE) -> c::DWORD {
    ((i64::from(dx) * i64::from(dx)) as c::DWORD)
        .wrapping_add((i64::from(dy) * i64::from(dy)) as c::DWORD)
}

#[inline]
fn c_point(point: CPoint) -> c::POINT {
    c::POINT {
        x: point.x,
        y: point.y,
    }
}

#[inline]
fn game_point(point: c::POINT) -> CPoint {
    CPoint {
        x: point.x,
        y: point.y,
    }
}

pub(super) unsafe fn get_collision_frame(
    solar: &SolarSystem,
    body: Body,
    wait_planet: c::COUNT,
) -> c::FRAME {
    if solar.wait_intersect != c::COUNT::MAX && solar.wait_intersect != wait_planet {
        DecFrameIndex(
            KernelOwner::current()
                .read(|owner| owner.stars_in_space)
                .cast(),
        )
        .cast()
    } else {
        solar.sys.descriptor(body).image.frame
    }
}

pub(super) unsafe fn check_intersect(solar: &mut SolarSystem) -> Option<Body> {
    let game = glob().read(|owner| owner.game);
    let mut ship: c::INTERSECT_CONTROL = std::mem::zeroed();
    let mut planet: c::INTERSECT_CONTROL = std::mem::zeroed();
    ship.IntersectStamp.origin = c_point(game.ship_stamp.origin);
    ship.EndPoint = ship.IntersectStamp.origin;
    ship.IntersectStamp.frame = game.ship_stamp.frame.cast();
    let mut candidates = Vec::with_capacity(17);
    match solar.location() {
        SystemLocation::Outer => {
            for index in 0..solar.sys.sun.num_planets {
                candidates.push((
                    Body::Planet(index),
                    make_word(index.wrapping_add(1), 0),
                    false,
                ));
            }
        }
        SystemLocation::Inner(index) => {
            candidates.push((
                Body::Planet(index),
                make_word(index.wrapping_add(1), 1),
                true,
            ));
            for moon in 0..solar.sys.planets[usize::from(index)].num_planets {
                candidates.push((
                    Body::Moon {
                        planet: index,
                        moon,
                    },
                    make_word(index.wrapping_add(1), moon.wrapping_add(2)),
                    false,
                ));
            }
        }
    }
    let mut new_wait = 0;
    for (body, wait, center) in candidates {
        planet.IntersectStamp.origin = if center {
            let (width, height) = sis_dimensions();
            c::POINT {
                x: (width >> 1) as _,
                y: (height >> 1) as _,
            }
        } else {
            solar.sys.descriptor(body).image.origin
        };
        planet.EndPoint = planet.IntersectStamp.origin;
        planet.IntersectStamp.frame = get_collision_frame(solar, body, wait);
        if crate::graphics::geometry_ffi::DrawablesIntersect(
            ptr::addr_of_mut!(ship).cast(),
            ptr::addr_of_mut!(planet).cast(),
            MAX_TIME_VALUE,
        ) != 0
        {
            new_wait = wait;
            if solar.wait_intersect != c::COUNT::MAX && solar.wait_intersect != new_wait {
                solar.wait_intersect = new_wait;
                return Some(body);
            }
            if !center && solar.wait_intersect == c::COUNT::MAX {
                break;
            }
        }
    }
    if solar.wait_intersect != c::COUNT::MAX || new_wait == 0 {
        solar.wait_intersect = new_wait;
    }
    None
}

pub(super) unsafe fn flagship_inertial_thrust(
    velocity: &mut crate::battle::velocity::VelocityDesc,
    current_angle: c::COUNT,
    max_speed: c::COUNT,
) -> c::UWORD {
    let (velocity_x, velocity_y) = (velocity).get_current_components();
    let (mut current_dx, mut current_dy) = (velocity_x as i16, velocity_y as i16);
    let travel_angle = velocity.travel_angle;
    if travel_angle == current_angle
        && current_dx == cosine(current_angle, i32::from(max_speed))
        && current_dy == sine(current_angle, i32::from(max_speed))
    {
        return SHIP_AT_MAX_SPEED;
    }

    let dx = (i32::from(current_dx) + i32::from(cosine(current_angle, IP_SHIP_THRUST_INCREMENT)))
        as c::SIZE;
    let dy = (i32::from(current_dy) + i32::from(sine(current_angle, IP_SHIP_THRUST_INCREMENT)))
        as c::SIZE;
    let mut desired_speed = speed_squared(dx, dy);
    let max_speed_squared = c::DWORD::from(max_speed) * c::DWORD::from(max_speed);
    if desired_speed <= max_speed_squared {
        (velocity).set_components(i32::from(dx), i32::from(dy));
    } else if travel_angle == current_angle {
        (velocity).set_components(
            i32::from(cosine(current_angle, i32::from(max_speed))),
            i32::from(sine(current_angle, i32::from(max_speed))),
        );
        return SHIP_AT_MAX_SPEED;
    } else {
        let mut candidate = *velocity;
        (candidate).delta_components(
            i32::from(
                (i32::from(cosine(current_angle, IP_SHIP_THRUST_INCREMENT >> 1))
                    - i32::from(cosine(travel_angle, IP_SHIP_THRUST_INCREMENT)))
                    as c::SIZE,
            ),
            i32::from(
                (i32::from(sine(current_angle, IP_SHIP_THRUST_INCREMENT >> 1))
                    - i32::from(sine(travel_angle, IP_SHIP_THRUST_INCREMENT)))
                    as c::SIZE,
            ),
        );
        {
            let (velocity_x, velocity_y) = (candidate).get_current_components();
            (current_dx, current_dy) = (velocity_x as i16, velocity_y as i16);
        };
        desired_speed = speed_squared(current_dx, current_dy);
        if desired_speed > max_speed_squared {
            (velocity).set_components(
                i32::from(cosine(current_angle, i32::from(max_speed))),
                i32::from(sine(current_angle, i32::from(max_speed))),
            );
            return SHIP_AT_MAX_SPEED;
        }
        *velocity = candidate;
    }
    0
}

pub(super) unsafe fn process_ship_controls(
    game: &mut crate::engine::battle::schema::GAME_STATE,
    solar: &mut SolarSystem,
    input: &crate::engine::input::InputSession,
) {
    let player = input.player_controls()[0] as usize;
    let keys = input.current();
    let mut dy: c::SIZE = if keys.key[player][input::KEY_UP as usize] != 0 {
        -1
    } else {
        0
    };
    let mut dx: c::SIZE = 0;
    if keys.key[player][input::KEY_LEFT as usize] != 0 {
        dx -= 1;
    }
    if keys.key[player][input::KEY_RIGHT as usize] != 0 {
        dx += 1;
    }

    if dx != 0 || dy < 0 {
        game.autopilot.x = -1;
        game.autopilot.y = -1;
    } else if game.autopilot.x != -1 && game.autopilot.y != -1 {
        dy = -1;
    } else {
        dy = 0;
    }

    let mut index = GetFrameIndex((game.ship_stamp.frame).cast());
    if solar.turn_counter != 0 {
        solar.turn_counter -= 1;
    } else if dx != 0 {
        if dx < 0 {
            index = normalize_facing(index.wrapping_sub(1));
        } else {
            index = normalize_facing(index.wrapping_add(1));
        }
        game.ship_stamp.frame = (SetAbsFrameIndex((game.ship_stamp.frame).cast(), index)).cast();
        solar.turn_counter = solar.turn_wait;
    }
    if solar.thrust_counter != 0 {
        solar.thrust_counter -= 1;
    } else if dy < 0 {
        flagship_inertial_thrust(
            &mut game.velocity,
            facing_to_angle(index),
            solar.max_ship_speed.into(),
        );
        solar.thrust_counter = THRUST_WAIT;
    }
}

pub(super) unsafe fn enter_inner_system(
    cx: &mut Cx<'_>,
    io: &mut crate::engine::observe::Io<'_>,
    solar: &mut SolarSystem,
    planet: u8,
) {
    let game = &mut cx.world.data.game;
    let angle =
        facing_to_angle(GetFrameIndex(game.ship_stamp.frame.cast())).wrapping_add(HALF_CIRCLE);
    let (width, height) = sis_dimensions();
    game.ship_stamp.origin.x = ((width >> 1) + i32::from(cosine(angle, INNER_ENTRY_DISTANCE))) as _;
    game.ship_stamp.origin.y = ((height >> 1) + i32::from(sine(angle, INNER_ENTRY_DISTANCE))) as _;
    if game.ship_stamp.origin.y < 0 {
        game.ship_stamp.origin.y = 1;
    } else if i32::from(game.ship_stamp.origin.y) >= height {
        game.ship_stamp.origin.y = (height - 2) as _;
    }
    game.ip_location = game_point(displayToLocation(
        c_point(game.ship_stamp.origin),
        MAX_ZOOM_RADIUS,
    ));
    solar.sys.sun.location = displayToLocation(
        solar.sys.planets[usize::from(planet)].image.origin,
        solar.sys.sun.radius,
    );
    game.velocity.zero();
    super::lifecycle::generate_moons(cx, io, solar, planet);
    solar.enter_inner(planet);
}

pub(super) unsafe fn leave_inner_system(solar: &mut SolarSystem) {
    let SystemLocation::Inner(planet) = solar.location() else {
        panic!("leaving an outer system");
    };
    solar.leave_inner();
    let outer_planet_wait = make_word(planet.wrapping_add(1), 0);
    let lease = glob();
    let game = ptr::addr_of_mut!((**lease).game);
    (*game).ip_location = game_point(solar.sys.sun.location);
    (*game).ship_stamp.origin = game_point(locationToDisplay(
        c_point((*game).ip_location),
        solar.sys.sun.radius,
    ));
    (*game).velocity.zero();
    drop(lease);
    solar.wait_intersect = outer_planet_wait;
    check_intersect(solar);
    if solar.wait_intersect != outer_planet_wait {
        solar.wait_intersect = c::COUNT::MAX;
    }
}

pub(super) fn enter_orbital(cx: &mut Cx<'_>, solar: &mut SolarSystem, body: Body) {
    cx.world.data.game.velocity.zero();
    solar.enter_orbit(body);
}

pub(super) unsafe fn check_ship_location(
    game: &crate::engine::battle::schema::GAME_STATE,
    solar: &mut SolarSystem,
    new_radius: *mut c::SIZE,
) -> (c::BOOLEAN, Option<u8>) {
    let radius = solar.sys.sun.radius;
    *new_radius = solar.sys.sun.radius;

    let (width, height) = sis_dimensions();
    if game.ship_stamp.origin.x < 0
        || i32::from(game.ship_stamp.origin.x) >= width
        || game.ship_stamp.origin.y < 0
        || i32::from(game.ship_stamp.origin.y) >= height
    {
        if !matches!(solar.location(), SystemLocation::Inner(_)) {
            if radius == MAX_ZOOM_RADIUS {
                crate::engine::World::current()
                    .write(|world| world.set_activity(world.activity() | END_INTERPLANETARY));
                return (0, None);
            }
            *new_radius =
                find_radius(i32::from(MAX_ZOOM_RADIUS) << 1, c_point(game.ip_location)) as c::SIZE;
        } else {
            leave_inner_system(solar);
        }
        return (1, None);
    }

    if !matches!(solar.location(), SystemLocation::Inner(_))
        && point_within_rect(SCALE_RECT, c_point(game.ship_stamp.origin))
    {
        *new_radius = find_radius(i32::from(radius), c_point(game.ip_location)) as c::SIZE;
        return (1, None);
    }

    if game.autopilot.x == -1 && game.autopilot.y == -1 {
        if let Some(body) = check_intersect(solar) {
            if matches!(solar.location(), SystemLocation::Inner(_)) {
                (**glob()).game.velocity.zero();
                solar.enter_orbit(body);
                return (0, None);
            } else {
                return (1, Some(body.planet() as u8));
            }
        }
    }
    (0, None)
}

pub(super) unsafe fn scale_system(cx: &mut Cx<'_>, solar: &mut SolarSystem, new_radius: c::SIZE) {
    let rect = cx.with_legacy(|| {
        let lease = glob();
        let game = ptr::addr_of_mut!((**lease).game);
        solar.sys.sun.radius = new_radius;
        (*game).ship_stamp.origin = game_point(locationToDisplay(
            c_point((*game).ip_location),
            solar.sys.sun.radius,
        ));

        drop(lease);
        // SMOOTH_SYSTEM_ZOOM is disabled in the C source; use its crossfade path.
        let mut rect: c::RECT = std::mem::zeroed();
        c::GetContextClipRect(&mut rect);
        SetTransitionSource((&raw mut rect).cast());
        BatchGraphics();
        rect
    });
    draw_outer_system(cx, solar);
    solar.refresh_frame(cx);
    redraw_queue(cx, false);
    cx.with_legacy(|| {
        ScreenTransition(3, (&raw const rect).cast());
        UnbatchGraphics();
    });
}

pub(super) unsafe fn restore_system_view() {
    let mut stamp = c::STAMP {
        origin: c::POINT { x: 0, y: 0 },
        frame: SOLAR_SYS_FRAME,
    };
    DrawStamp((&raw mut stamp).cast());
}

pub(super) unsafe fn ip_frame(
    cx: &mut Cx<'_>,
    io: &mut crate::engine::observe::Io<'_>,
    solar: &mut SolarSystem,
) {
    let runtime = cx.kernel.input_runtime();
    let input = runtime.input();
    cx.with_legacy(|| {
        SetContext(KernelOwner::current().read(|owner| owner.space_context));
        GameClockTick();
    });
    process_ship_controls(&mut cx.world.data.game, solar, input);
    let game = cx.world.data.game;
    let mut new_radius: c::SIZE = 0;
    let (location_change, inner_planet) =
        cx.with_legacy(|| check_ship_location(&game, solar, &mut new_radius));
    if let Some(planet) = inner_planet {
        enter_inner_system(cx, io, solar, planet);
    }
    if location_change != 0 {
        if cx.with_legacy(|| matches!(solar.location(), SystemLocation::Inner(_))) {
            draw_system_transition(cx, solar, true);
        } else if solar.sys.sun.radius == new_radius {
            draw_system_transition(cx, solar, false);
        } else {
            scale_system(cx, solar, new_radius);
        }
    } else {
        solar.refresh_frame(cx);
        cx.with_legacy(|| {
            BatchGraphics();
            restore_system_view();
        });
        redraw_queue(cx, false);
        crate::ui::sis::draw_autopilot_message(
            cx.kernel,
            cx.world,
            cx.screen_dimensions().0,
            uqm_engine::time::counter_abi::GetTimeCounter,
        );
        cx.with_legacy(|| UnbatchGraphics());
    }
}

pub(super) unsafe fn check_zoom_level(solar: &mut SolarSystem) -> c::BOOLEAN {
    let game = ptr::addr_of_mut!((**glob()).game);
    let inner_system = matches!(solar.location(), SystemLocation::Inner(_));
    let ship_location = if inner_system {
        solar.sys.sun.location
    } else {
        c_point((*game).ip_location)
    };
    solar.sys.sun.radius = find_radius(i32::from(MAX_ZOOM_RADIUS) << 1, ship_location) as c::SIZE;
    if !inner_system {
        (*game).ship_stamp.origin =
            game_point(locationToDisplay(ship_location, solar.sys.sun.radius));
    }
    inner_system as c::BOOLEAN
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collision_wait_word_keeps_planet_in_low_byte() {
        assert_eq!(make_word(1, 0), 0x0001);
        assert_eq!(make_word(16, 1), 0x0110);
        assert_eq!(make_word(3, 5), 0x0503);
        assert_eq!(make_word(255, 255), c::COUNT::MAX);
    }

    #[test]
    fn facing_wraps_and_angles_use_sixty_four_positions() {
        assert_eq!(normalize_facing(0_u16.wrapping_sub(1)), 15);
        assert_eq!(normalize_facing(15 + 1), 0);
        for facing in 0..16 {
            assert_eq!(facing_to_angle(facing), facing * 4);
        }
    }

    #[test]
    fn fixed_point_trigonometry_uses_the_c_orientation_and_negative_rounding() {
        assert_eq!((cosine(0, 100), sine(0, 100)), (0, -100));
        assert_eq!((cosine(16, 100), sine(16, 100)), (100, 0));
        assert_eq!((cosine(32, 100), sine(32, 100)), (0, 100));
        assert_eq!((cosine(48, 100), sine(48, 100)), (-100, 0));
        assert_eq!(sine(1, 8), -8);
        assert_eq!(sine(33, 8), 7);
        assert_eq!(sine(65, 8), sine(1, 8));
        assert_eq!(cosine(c::COUNT::MAX, 100), cosine(63, 100));
    }

    #[test]
    fn squared_speed_preserves_unsigned_dword_range() {
        assert_eq!(speed_squared(3, -4), 25);
        assert_eq!(speed_squared(-128, 0), 128 * 128);
        assert_eq!(speed_squared(i16::MIN, i16::MIN), 0x8000_0000);
        assert_eq!(MAX_ZOOM_RADIUS, 8192);
        assert_eq!(INNER_ENTRY_DISTANCE, 100);
    }

    #[test]
    fn zoom_rectangle_is_half_open_and_promotes_coordinates_before_adding() {
        let rect = c::RECT {
            corner: c::POINT { x: -10, y: -20 },
            extent: c::EXTENT {
                width: 20,
                height: 40,
            },
        };
        assert!(point_within_rect(rect, c::POINT { x: -10, y: -20 }));
        assert!(point_within_rect(rect, c::POINT { x: 9, y: 19 }));
        assert!(!point_within_rect(rect, c::POINT { x: 10, y: 0 }));
        assert!(!point_within_rect(rect, c::POINT { x: 0, y: 20 }));
        assert!(!point_within_rect(rect, c::POINT { x: -11, y: 0 }));
        let high_rect = c::RECT {
            corner: c::POINT { x: 32760, y: 0 },
            extent: c::EXTENT {
                width: 20,
                height: 1,
            },
        };
        assert!(point_within_rect(high_rect, c::POINT { x: 32767, y: 0 }));
        let empty = c::RECT {
            corner: c::POINT { x: 0, y: 0 },
            extent: c::EXTENT {
                width: 0,
                height: 0,
            },
        };
        assert!(!point_within_rect(empty, c::POINT { x: 0, y: 0 }));
    }

    #[test]
    fn point_conversion_preserves_signed_coordinates() {
        let source = CPoint {
            x: i16::MIN,
            y: i16::MAX,
        };
        let result = game_point(c_point(source));
        assert_eq!(result.x, source.x);
        assert_eq!(result.y, source.y);
    }
}
