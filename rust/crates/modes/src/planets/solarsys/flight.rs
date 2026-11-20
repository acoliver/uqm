//! Interplanetary flight and collision handling from `solarsys.c`.

use super::drawing::{draw_outer_system, draw_system_transition, find_radius, point_within_rect};
use super::*;
use crate::comm::locdata::CPoint;
use crate::planets::solar_system::{SolarSystem, SystemLocation};
use uqm_engine::input_port::frames as input;
use uqm_engine::world::generation::Body;
use uqm_engine::{Kernel, World};

const MIN_MOON_RADIUS: i32 = 35;
const MAX_MOONS: i32 = 4;
const MOON_DELTA: i32 = 20;
const MAX_ZOOM_RADIUS: i16 = 128 << 6;
const INNER_ENTRY_DISTANCE: i32 = MIN_MOON_RADIUS + (MAX_MOONS - 1) * MOON_DELTA + MOON_DELTA / 4;
const IP_SHIP_THRUST_INCREMENT: i32 = 8;
const SHIP_AT_MAX_SPEED: u16 = 1 << 7;
const END_INTERPLANETARY: u16 = 1 << 11;
const MAX_TIME_VALUE: u16 = (1 << 8) + 1;
const THRUST_WAIT: u8 = 1;
const HALF_CIRCLE: u16 = 32;
const QUADRANT: u16 = 16;
const SIN_SHIFT: u32 = 14;

use crate::battle::process_native::engine::redraw_queue;
use crate::navigation::events::game_clock_tick;
use uqm_engine::battle::Cx;
use uqm_engine::graphics::{
    context::ContextLease, drawable::ContextFrame, intersection::FrameIntersection,
};

#[inline]
fn normalize_facing(facing: u16) -> u16 {
    facing & 15
}

#[inline]
fn facing_to_angle(facing: u16) -> u16 {
    facing.wrapping_shl(6 - 4)
}

#[inline]
fn sine(angle: u16, magnitude: i32) -> i16 {
    ((i64::from(uqm_engine::math::SINE_TABLE[usize::from(angle & 63)]) * i64::from(magnitude))
        >> SIN_SHIFT) as i16
}

#[inline]
fn cosine(angle: u16, magnitude: i32) -> i16 {
    sine(angle.wrapping_add(QUADRANT), magnitude)
}

#[inline]
fn make_word(low: u8, high: u8) -> u16 {
    (u16::from(high) << 8) | u16::from(low)
}

#[inline]
fn speed_squared(dx: i16, dy: i16) -> u32 {
    ((i64::from(dx) * i64::from(dx)) as u32).wrapping_add((i64::from(dy) * i64::from(dy)) as u32)
}

#[inline]
fn c_point(point: CPoint) -> uqm_engine::graphics::geometry::Point {
    uqm_engine::graphics::geometry::Point {
        x: point.x,
        y: point.y,
    }
}

#[inline]
fn game_point(point: uqm_engine::graphics::geometry::Point) -> CPoint {
    CPoint {
        x: point.x,
        y: point.y,
    }
}

pub(super) fn get_collision_frame(
    kernel: &Kernel,
    solar: &SolarSystem,
    body: Body,
    wait_planet: u16,
) -> ContextFrame {
    if solar.wait_intersect != u16::MAX && solar.wait_intersect != wait_planet {
        let allocations = uqm_engine::graphics::drawable::retain_frames(&[kernel.stars_in_space]);
        let allocation = allocations
            .first()
            .unwrap_or_else(|| panic!("retained collision placeholder"));
        let index = allocation
            .frame_index(kernel.stars_in_space)
            .unwrap_or_else(|| panic!("collision placeholder frame index"));
        let frame = ContextFrame::retain(kernel.stars_in_space);
        let previous = if index == 0 {
            usize::from(frame.frame_count()) - 1
        } else {
            index - 1
        };
        frame.wrapped_frame(previous as u16)
    } else {
        ContextFrame::retain(solar.sys.descriptor(body).image.frame)
    }
}

pub(super) fn check_intersect(
    world: &World,
    kernel: &Kernel,
    solar: &mut SolarSystem,
) -> Option<Body> {
    let game = world.data.game;
    let mut ship = FrameIntersection::new(
        ContextFrame::retain(game.ship_stamp.frame),
        c_point(game.ship_stamp.origin),
    );
    let mut planet = FrameIntersection::new(
        ContextFrame::default(),
        uqm_engine::graphics::geometry::Point::default(),
    );
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
        let origin = if center {
            let (width, height) = solar.coordinates().dimensions();
            uqm_engine::graphics::geometry::Point {
                x: (width >> 1) as _,
                y: (height >> 1) as _,
            }
        } else {
            solar.sys.descriptor(body).image.origin
        };
        planet.set_frame(get_collision_frame(kernel, solar, body, wait), origin);
        if ship.intersect(&mut planet, MAX_TIME_VALUE) != 0 {
            new_wait = wait;
            if solar.wait_intersect != u16::MAX && solar.wait_intersect != new_wait {
                solar.wait_intersect = new_wait;
                return Some(body);
            }
            if !center && solar.wait_intersect == u16::MAX {
                break;
            }
        }
    }
    if solar.wait_intersect != u16::MAX || new_wait == 0 {
        solar.wait_intersect = new_wait;
    }
    None
}

pub(super) fn flagship_inertial_thrust(
    velocity: &mut crate::battle::velocity::VelocityDesc,
    current_angle: u16,
    max_speed: u16,
) -> u16 {
    let (velocity_x, velocity_y) = (velocity).get_current_components();
    let (mut current_dx, mut current_dy) = (velocity_x as i16, velocity_y as i16);
    let travel_angle = velocity.travel_angle;
    if travel_angle == current_angle
        && current_dx == cosine(current_angle, i32::from(max_speed))
        && current_dy == sine(current_angle, i32::from(max_speed))
    {
        return SHIP_AT_MAX_SPEED;
    }

    let dx =
        (i32::from(current_dx) + i32::from(cosine(current_angle, IP_SHIP_THRUST_INCREMENT))) as i16;
    let dy =
        (i32::from(current_dy) + i32::from(sine(current_angle, IP_SHIP_THRUST_INCREMENT))) as i16;
    let mut desired_speed = speed_squared(dx, dy);
    let max_speed_squared = u32::from(max_speed) * u32::from(max_speed);
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
                    as i16,
            ),
            i32::from(
                (i32::from(sine(current_angle, IP_SHIP_THRUST_INCREMENT >> 1))
                    - i32::from(sine(travel_angle, IP_SHIP_THRUST_INCREMENT)))
                    as i16,
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

pub(super) fn process_ship_controls(
    kernel: &Kernel,
    game: &mut uqm_engine::battle::schema::GameRecord,
    solar: &mut SolarSystem,
    input: &uqm_engine::input::InputSession,
) {
    let player = input.player_controls()[0] as usize;
    let keys = input.current();
    let mut dy: i16 = if keys.key[player][input::KEY_UP as usize] != 0 {
        -1
    } else {
        0
    };
    let mut dx: i16 = 0;
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

    let mut index = resources::ship_facing(kernel, game);
    if solar.turn_counter != 0 {
        solar.turn_counter -= 1;
    } else if dx != 0 {
        if dx < 0 {
            index = normalize_facing(index.wrapping_sub(1));
        } else {
            index = normalize_facing(index.wrapping_add(1));
        }
        game.ship_stamp.frame = resources::frame(&resources::retained(kernel).flagship, index);
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

pub(super) fn enter_inner_system(
    cx: &mut Cx<'_>,
    io: &mut uqm_engine::observe::Io<'_>,
    solar: &mut SolarSystem,
    planet: u8,
) {
    let game = &mut cx.world.data.game;
    let angle = facing_to_angle(resources::ship_facing(cx.kernel, game)).wrapping_add(HALF_CIRCLE);
    let (width, height) = solar.coordinates().dimensions();
    game.ship_stamp.origin.x = ((width >> 1) + i32::from(cosine(angle, INNER_ENTRY_DISTANCE))) as _;
    game.ship_stamp.origin.y = ((height >> 1) + i32::from(sine(angle, INNER_ENTRY_DISTANCE))) as _;
    if game.ship_stamp.origin.y < 0 {
        game.ship_stamp.origin.y = 1;
    } else if i32::from(game.ship_stamp.origin.y) >= height {
        game.ship_stamp.origin.y = (height - 2) as _;
    }
    game.ip_location = game_point(
        solar
            .coordinates()
            .to_location(c_point(game.ship_stamp.origin), MAX_ZOOM_RADIUS),
    );
    solar.sys.sun.location = solar.coordinates().to_location(
        solar.sys.planets[usize::from(planet)].image.origin,
        solar.sys.sun.radius,
    );
    game.velocity.zero();
    super::lifecycle::generate_moons(cx, io, solar, planet);
    solar.enter_inner(planet);
}

pub(super) fn leave_inner_system(world: &mut World, kernel: &Kernel, solar: &mut SolarSystem) {
    let SystemLocation::Inner(planet) = solar.location() else {
        panic!("leaving an outer system");
    };
    solar.leave_inner();
    let outer_planet_wait = make_word(planet.wrapping_add(1), 0);
    let game = &mut world.data.game;
    game.ip_location = game_point(solar.sys.sun.location);
    game.ship_stamp.origin = game_point(
        solar
            .coordinates()
            .to_display(c_point(game.ip_location), solar.sys.sun.radius),
    );
    game.velocity.zero();
    solar.wait_intersect = outer_planet_wait;
    check_intersect(world, kernel, solar);
    if solar.wait_intersect != outer_planet_wait {
        solar.wait_intersect = u16::MAX;
    }
}

pub(super) fn enter_orbital(cx: &mut Cx<'_>, solar: &mut SolarSystem, body: Body) {
    cx.world.data.game.velocity.zero();
    solar.enter_orbit(body);
}

pub(super) fn check_ship_location(
    world: &mut World,
    kernel: &Kernel,
    solar: &mut SolarSystem,
    new_radius: &mut i16,
) -> (u32, Option<u8>) {
    let game = world.data.game;
    let radius = solar.sys.sun.radius;
    *new_radius = solar.sys.sun.radius;

    let (width, height) = solar.coordinates().dimensions();
    if game.ship_stamp.origin.x < 0
        || i32::from(game.ship_stamp.origin.x) >= width
        || game.ship_stamp.origin.y < 0
        || i32::from(game.ship_stamp.origin.y) >= height
    {
        if !matches!(solar.location(), SystemLocation::Inner(_)) {
            if radius == MAX_ZOOM_RADIUS {
                world.set_activity(world.activity() | END_INTERPLANETARY);
                return (0, None);
            }
            *new_radius = find_radius(
                solar,
                i32::from(MAX_ZOOM_RADIUS) << 1,
                c_point(game.ip_location),
            ) as i16;
        } else {
            leave_inner_system(world, kernel, solar);
        }
        return (1, None);
    }

    if !matches!(solar.location(), SystemLocation::Inner(_))
        && point_within_rect(solar.scale_rect, c_point(game.ship_stamp.origin))
    {
        *new_radius = find_radius(solar, i32::from(radius), c_point(game.ip_location)) as i16;
        return (1, None);
    }

    if game.autopilot.x == -1 && game.autopilot.y == -1 {
        if let Some(body) = check_intersect(world, kernel, solar) {
            if matches!(solar.location(), SystemLocation::Inner(_)) {
                world.data.game.velocity.zero();
                solar.enter_orbit(body);
                return (0, None);
            } else {
                return (1, Some(body.planet() as u8));
            }
        }
    }
    (0, None)
}

pub(super) fn scale_system(cx: &mut Cx<'_>, solar: &mut SolarSystem, new_radius: i16) {
    solar.sys.sun.radius = new_radius;
    let game = &mut cx.world.data.game;
    game.ship_stamp.origin = game_point(
        solar
            .coordinates()
            .to_display(c_point(game.ip_location), solar.sys.sun.radius),
    );
    let context = ContextLease::require(cx.kernel.space_context);
    // SMOOTH_SYSTEM_ZOOM is disabled in the C source; use its crossfade path.
    let rect = context.clip_rect();
    context.transition_source(Some(rect));
    let batch = context.batch();
    draw_outer_system(cx, solar);
    solar.refresh_frame(cx);
    redraw_queue(cx, false);
    context.transition(Some(rect));
    drop(batch);
}

pub(super) fn restore_system_view(context: &ContextLease, solar: &SolarSystem) {
    let image = solar
        .system_image
        .as_ref()
        .unwrap_or_else(|| panic!("solar system image is not initialized"));
    context.draw_stamp(
        &image.lease().context_frame(0),
        uqm_engine::graphics::geometry::Point::default(),
    );
}

pub(super) fn ip_frame(
    cx: &mut Cx<'_>,
    io: &mut uqm_engine::observe::Io<'_>,
    solar: &mut SolarSystem,
) {
    let runtime = cx.kernel.input_runtime();
    let input = runtime.input();
    let context = ContextLease::require(cx.kernel.space_context);
    context.select();
    game_clock_tick(cx);
    process_ship_controls(cx.kernel, &mut cx.world.data.game, solar, input);
    let mut new_radius: i16 = 0;
    let (location_change, inner_planet) =
        check_ship_location(cx.world, cx.kernel, solar, &mut new_radius);
    if let Some(planet) = inner_planet {
        enter_inner_system(cx, io, solar, planet);
    }
    if location_change != 0 {
        if matches!(solar.location(), SystemLocation::Inner(_)) {
            draw_system_transition(cx, solar, true);
        } else if solar.sys.sun.radius == new_radius {
            draw_system_transition(cx, solar, false);
        } else {
            scale_system(cx, solar, new_radius);
        }
    } else {
        solar.refresh_frame(cx);
        let batch = context.batch();
        restore_system_view(&context, solar);
        redraw_queue(cx, false);
        crate::ui::sis::draw_autopilot_message(
            cx.kernel,
            cx.world,
            cx.screen_dimensions().0,
            uqm_engine::time::counter_abi::get_time_counter,
        );
        drop(batch);
    }
}

pub(super) fn check_zoom_level(
    game: &mut uqm_engine::battle::schema::GameRecord,
    solar: &mut SolarSystem,
) -> u32 {
    let inner_system = matches!(solar.location(), SystemLocation::Inner(_));
    let ship_location = if inner_system {
        solar.sys.sun.location
    } else {
        c_point(game.ip_location)
    };
    solar.sys.sun.radius =
        find_radius(solar, i32::from(MAX_ZOOM_RADIUS) << 1, ship_location) as i16;
    if !inner_system {
        game.ship_stamp.origin = game_point(
            solar
                .coordinates()
                .to_display(ship_location, solar.sys.sun.radius),
        );
    }
    inner_system as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collision_wait_word_keeps_planet_in_low_byte() {
        assert_eq!(make_word(1, 0), 0x0001);
        assert_eq!(make_word(16, 1), 0x0110);
        assert_eq!(make_word(3, 5), 0x0503);
        assert_eq!(make_word(255, 255), u16::MAX);
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
        assert_eq!(cosine(u16::MAX, 100), cosine(63, 100));
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
        let rect = uqm_engine::graphics::geometry::Rect {
            corner: uqm_engine::graphics::geometry::Point { x: -10, y: -20 },
            extent: uqm_engine::graphics::geometry::Extent {
                width: 20,
                height: 40,
            },
        };
        assert!(point_within_rect(
            rect,
            uqm_engine::graphics::geometry::Point { x: -10, y: -20 }
        ));
        assert!(point_within_rect(
            rect,
            uqm_engine::graphics::geometry::Point { x: 9, y: 19 }
        ));
        assert!(!point_within_rect(
            rect,
            uqm_engine::graphics::geometry::Point { x: 10, y: 0 }
        ));
        assert!(!point_within_rect(
            rect,
            uqm_engine::graphics::geometry::Point { x: 0, y: 20 }
        ));
        assert!(!point_within_rect(
            rect,
            uqm_engine::graphics::geometry::Point { x: -11, y: 0 }
        ));
        let high_rect = uqm_engine::graphics::geometry::Rect {
            corner: uqm_engine::graphics::geometry::Point { x: 32760, y: 0 },
            extent: uqm_engine::graphics::geometry::Extent {
                width: 20,
                height: 1,
            },
        };
        assert!(point_within_rect(
            high_rect,
            uqm_engine::graphics::geometry::Point { x: 32767, y: 0 }
        ));
        let empty = uqm_engine::graphics::geometry::Rect {
            corner: uqm_engine::graphics::geometry::Point { x: 0, y: 0 },
            extent: uqm_engine::graphics::geometry::Extent {
                width: 0,
                height: 0,
            },
        };
        assert!(!point_within_rect(
            empty,
            uqm_engine::graphics::geometry::Point { x: 0, y: 0 }
        ));
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
