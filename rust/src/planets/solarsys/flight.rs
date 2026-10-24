//! Interplanetary flight and collision handling from `solarsys.c`.

use super::drawing::{draw_outer_system, draw_system_transition, find_radius, point_within_rect};
use super::*;
use crate::battle::velocity::{
    DeltaVelocityComponents, GetCurrentVelocityComponents, SetVelocityComponents,
};
use crate::c_bindings::controller_input as input;
use crate::comm::locdata::{CGlobData, CPoint};

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

use crate::battle::process_native::engine::RedrawQueue;

use crate::graphics::gfx_common_abi::ScreenTransition;

use crate::graphics::frame_abi::DrawStamp;

use crate::time::clock_abi::live::GameClockTick;

use crate::ui::sis::DrawAutoPilotMessage;

extern "C" {
    static mut GlobData: CGlobData;
    static mut CurrentInputState: input::CONTROLLER_INPUT_STATE;
    static mut SpaceContext: *mut crate::graphics::native_abi::context_desc;
    static mut stars_in_space: *mut c_void;
}
use crate::graphics::gfx_common_abi::{BatchGraphics, UnbatchGraphics};

#[inline]
unsafe fn system() -> *mut c::SOLARSYS_STATE {
    crate::c_bindings::solar_system::current()
}

#[inline]
unsafe fn glob() -> *mut CGlobData {
    ptr::addr_of_mut!(GlobData)
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
    planet: *mut c::PLANET_DESC,
    wait_planet: c::COUNT,
) -> c::FRAME {
    let state = system();
    if (*state).WaitIntersect != c::COUNT::MAX && (*state).WaitIntersect != wait_planet {
        DecFrameIndex((stars_in_space).cast()).cast()
    } else {
        (*planet).image.frame
    }
}

pub(super) unsafe fn check_intersect() -> *mut c::PLANET_DESC {
    let state = system();
    let game = ptr::addr_of!((*glob()).game_state);
    let mut current = (*(*state).pBaseDesc).pPrevDesc;
    let mut planet_offset = current
        .offset_from((*state).PlanetDesc.as_ptr())
        .wrapping_add(1) as c::BYTE;
    let mut moon_offset: c::BYTE = 1;
    let mut ship: c::INTERSECT_CONTROL = std::mem::zeroed();
    let mut planet: c::INTERSECT_CONTROL = std::mem::zeroed();

    ship.IntersectStamp.origin = c_point((*game).ship_stamp.origin);
    ship.EndPoint = ship.IntersectStamp.origin;
    ship.IntersectStamp.frame = (*game).ship_stamp.frame.cast();

    let (width, height) = sis_dimensions();
    planet.IntersectStamp.origin.x = (width >> 1) as _;
    planet.IntersectStamp.origin.y = (height >> 1) as _;
    planet.EndPoint = planet.IntersectStamp.origin;
    planet.IntersectStamp.frame =
        get_collision_frame(current, make_word(planet_offset, moon_offset));

    let mut new_wait_planet: c::COUNT = 0;
    if current != (*state).SunDesc.as_mut_ptr()
        && crate::graphics::geometry_ffi::DrawablesIntersect(
            ptr::addr_of_mut!(ship).cast(),
            ptr::addr_of_mut!(planet).cast(),
            MAX_TIME_VALUE,
        ) != 0
    {
        new_wait_planet = make_word(planet_offset, moon_offset);
        if (*state).WaitIntersect != c::COUNT::MAX && (*state).WaitIntersect != new_wait_planet {
            (*state).WaitIntersect = new_wait_planet;
            return current;
        }
    }

    let mut remaining = c::COUNT::from((*current).NumPlanets);
    current = (*state).pBaseDesc;
    while remaining != 0 {
        planet.IntersectStamp.origin = (*current).image.origin;
        planet.EndPoint = planet.IntersectStamp.origin;
        if playerInInnerSystem() {
            planet_offset = (*current)
                .pPrevDesc
                .offset_from((*state).PlanetDesc.as_ptr()) as c::BYTE;
            moon_offset = current
                .offset_from((*state).MoonDesc.as_ptr())
                .wrapping_add(2) as c::BYTE;
        } else {
            planet_offset = current.offset_from((*state).PlanetDesc.as_ptr()) as c::BYTE;
            moon_offset = 0;
        }
        planet_offset = planet_offset.wrapping_add(1);
        planet.IntersectStamp.frame =
            get_collision_frame(current, make_word(planet_offset, moon_offset));

        if crate::graphics::geometry_ffi::DrawablesIntersect(
            ptr::addr_of_mut!(ship).cast(),
            ptr::addr_of_mut!(planet).cast(),
            MAX_TIME_VALUE,
        ) != 0
        {
            new_wait_planet = make_word(planet_offset, moon_offset);
            if (*state).WaitIntersect == c::COUNT::MAX {
                break;
            } else if (*state).WaitIntersect != new_wait_planet {
                (*state).WaitIntersect = new_wait_planet;
                return current;
            }
        }
        remaining -= 1;
        current = current.add(1);
    }

    if (*state).WaitIntersect != c::COUNT::MAX || new_wait_planet == 0 {
        (*state).WaitIntersect = new_wait_planet;
    }
    ptr::null_mut()
}

pub(super) unsafe fn flagship_inertial_thrust(current_angle: c::COUNT) -> c::UWORD {
    let max_speed = (*system()).max_ship_speed;
    let velocity = ptr::addr_of_mut!((*glob()).game_state.velocity);
    let mut current_dx: c::SIZE = 0;
    let mut current_dy: c::SIZE = 0;
    GetCurrentVelocityComponents(velocity, &mut current_dx, &mut current_dy);
    let travel_angle = (*velocity).travel_angle;
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
        SetVelocityComponents(velocity, dx, dy);
    } else if travel_angle == current_angle {
        SetVelocityComponents(
            velocity,
            cosine(current_angle, i32::from(max_speed)),
            sine(current_angle, i32::from(max_speed)),
        );
        return SHIP_AT_MAX_SPEED;
    } else {
        let mut candidate = *velocity;
        DeltaVelocityComponents(
            &mut candidate,
            (i32::from(cosine(current_angle, IP_SHIP_THRUST_INCREMENT >> 1))
                - i32::from(cosine(travel_angle, IP_SHIP_THRUST_INCREMENT))) as c::SIZE,
            (i32::from(sine(current_angle, IP_SHIP_THRUST_INCREMENT >> 1))
                - i32::from(sine(travel_angle, IP_SHIP_THRUST_INCREMENT))) as c::SIZE,
        );
        GetCurrentVelocityComponents(&mut candidate, &mut current_dx, &mut current_dy);
        desired_speed = speed_squared(current_dx, current_dy);
        if desired_speed > max_speed_squared {
            SetVelocityComponents(
                velocity,
                cosine(current_angle, i32::from(max_speed)),
                sine(current_angle, i32::from(max_speed)),
            );
            return SHIP_AT_MAX_SPEED;
        }
        *velocity = candidate;
    }
    0
}

pub(super) unsafe fn process_ship_controls() {
    let state = system();
    let game = ptr::addr_of_mut!((*glob()).game_state);
    let input = ptr::addr_of!(CurrentInputState);
    let player = crate::c_bindings::player_control_template(0)
        .expect("player zero has a control template") as usize;
    let mut dy: c::SIZE = if (*input).key[player][input::KEY_UP as usize] != 0 {
        -1
    } else {
        0
    };
    let mut dx: c::SIZE = 0;
    if (*input).key[player][input::KEY_LEFT as usize] != 0 {
        dx -= 1;
    }
    if (*input).key[player][input::KEY_RIGHT as usize] != 0 {
        dx += 1;
    }

    if dx != 0 || dy < 0 {
        (*game).autopilot.x = -1;
        (*game).autopilot.y = -1;
    } else if (*game).autopilot.x != -1 && (*game).autopilot.y != -1 {
        dy = -1;
    } else {
        dy = 0;
    }

    let mut index = GetFrameIndex(((*game).ship_stamp.frame).cast());
    if (*state).turn_counter != 0 {
        (*state).turn_counter -= 1;
    } else if dx != 0 {
        if dx < 0 {
            index = normalize_facing(index.wrapping_sub(1));
        } else {
            index = normalize_facing(index.wrapping_add(1));
        }
        (*game).ship_stamp.frame =
            (SetAbsFrameIndex(((*game).ship_stamp.frame).cast(), index)).cast();
        (*state).turn_counter = (*state).turn_wait;
    }
    if (*state).thrust_counter != 0 {
        (*state).thrust_counter -= 1;
    } else if dy < 0 {
        flagship_inertial_thrust(facing_to_angle(index));
        (*state).thrust_counter = THRUST_WAIT;
    }
}

pub(super) unsafe fn enter_inner_system(planet: *mut c::PLANET_DESC) {
    let state = system();
    let game = ptr::addr_of_mut!((*glob()).game_state);
    let angle =
        facing_to_angle(GetFrameIndex(((*game).ship_stamp.frame).cast())).wrapping_add(HALF_CIRCLE);
    let (width, height) = sis_dimensions();
    (*game).ship_stamp.origin.x =
        ((width >> 1) + i32::from(cosine(angle, INNER_ENTRY_DISTANCE))) as _;
    (*game).ship_stamp.origin.y =
        ((height >> 1) + i32::from(sine(angle, INNER_ENTRY_DISTANCE))) as _;
    if (*game).ship_stamp.origin.y < 0 {
        (*game).ship_stamp.origin.y = 1;
    } else if i32::from((*game).ship_stamp.origin.y) >= height {
        (*game).ship_stamp.origin.y = (height - 1 - 1) as _;
    }
    (*game).ip_location = game_point(displayToLocation(
        c_point((*game).ship_stamp.origin),
        MAX_ZOOM_RADIUS,
    ));
    (*state).SunDesc[0].location = planetOuterLocation(planetIndex(state, planet));
    (*game).velocity.zero();

    super::lifecycle::generate_moons(state, planet);
    (*state).pBaseDesc = (*state).MoonDesc.as_mut_ptr();
    (*state).pOrbitalDesc = planet;
}

pub(super) unsafe fn leave_inner_system(planet: *mut c::PLANET_DESC) {
    let state = system();
    let game = ptr::addr_of_mut!((*glob()).game_state);
    (*state).pBaseDesc = (*state).PlanetDesc.as_mut_ptr();
    (*state).pOrbitalDesc = ptr::null_mut();

    let outer_planet_wait = make_word(
        planet
            .offset_from((*state).PlanetDesc.as_ptr())
            .wrapping_add(1) as c::BYTE,
        0,
    );
    (*game).ip_location = game_point((*state).SunDesc[0].location);
    XFormIPLoc(
        ptr::addr_of_mut!((*game).ip_location).cast(),
        ptr::addr_of_mut!((*game).ship_stamp.origin).cast(),
        1,
    );
    (*game).velocity.zero();

    (*state).WaitIntersect = outer_planet_wait;
    check_intersect();
    if (*state).WaitIntersect != outer_planet_wait {
        (*state).WaitIntersect = c::COUNT::MAX;
    }
}

pub(super) unsafe fn enter_orbital(planet: *mut c::PLANET_DESC) {
    (*glob()).game_state.velocity.zero();
    let state = system();
    (*state).pOrbitalDesc = planet;
    (*state).InOrbit = 1;
}

pub(super) unsafe fn check_ship_location(new_radius: *mut c::SIZE) -> c::BOOLEAN {
    let state = system();
    let game = ptr::addr_of_mut!((*glob()).game_state);
    let radius = (*state).SunDesc[0].radius;
    *new_radius = (*state).SunDesc[0].radius;

    let (width, height) = sis_dimensions();
    if (*game).ship_stamp.origin.x < 0
        || i32::from((*game).ship_stamp.origin.x) >= width
        || (*game).ship_stamp.origin.y < 0
        || i32::from((*game).ship_stamp.origin.y) >= height
    {
        if !playerInInnerSystem() {
            if radius == MAX_ZOOM_RADIUS {
                (*game).current_activity |= END_INTERPLANETARY;
                return 0;
            }
            *new_radius = find_radius(
                i32::from(MAX_ZOOM_RADIUS) << 1,
                c_point((*game).ip_location),
            ) as c::SIZE;
        } else {
            leave_inner_system((*state).pOrbitalDesc);
        }
        return 1;
    }

    if !playerInInnerSystem() && point_within_rect(SCALE_RECT, c_point((*game).ship_stamp.origin)) {
        *new_radius = find_radius(i32::from(radius), c_point((*game).ip_location)) as c::SIZE;
        return 1;
    }

    if (*game).autopilot.x == -1 && (*game).autopilot.y == -1 {
        let planet = check_intersect();
        if !planet.is_null() {
            if playerInInnerSystem() {
                enter_orbital(planet);
                return 0;
            } else {
                enter_inner_system(planet);
                return 1;
            }
        }
    }
    0
}

pub(super) unsafe fn scale_system(new_radius: c::SIZE) {
    let game = ptr::addr_of_mut!((*glob()).game_state);
    (*system()).SunDesc[0].radius = new_radius;
    XFormIPLoc(
        ptr::addr_of_mut!((*game).ip_location).cast(),
        ptr::addr_of_mut!((*game).ship_stamp.origin).cast(),
        1,
    );

    // SMOOTH_SYSTEM_ZOOM is disabled in the C source; use its crossfade path.
    let mut rect: c::RECT = std::mem::zeroed();
    c::GetContextClipRect(&mut rect);
    SetTransitionSource((&raw mut rect).cast());
    BatchGraphics();
    draw_outer_system();
    RedrawQueue(0);
    ScreenTransition(3, (&raw const rect).cast());
    UnbatchGraphics();
}

pub(super) unsafe fn restore_system_view() {
    let mut stamp = c::STAMP {
        origin: c::POINT { x: 0, y: 0 },
        frame: SOLAR_SYS_FRAME,
    };
    DrawStamp((&raw mut stamp).cast());
}

pub(super) unsafe fn ip_frame() {
    SetContext(SpaceContext);
    GameClockTick();
    process_ship_controls();

    let mut new_radius: c::SIZE = 0;
    let location_change = check_ship_location(&mut new_radius);
    if location_change != 0 {
        if playerInInnerSystem() {
            draw_system_transition(true);
        } else if (*system()).SunDesc[0].radius == new_radius {
            draw_system_transition(false);
        } else {
            scale_system(new_radius);
        }
    } else {
        BatchGraphics();
        restore_system_view();
        RedrawQueue(0);
        DrawAutoPilotMessage(0);
        UnbatchGraphics();
    }
}

pub(super) unsafe fn check_zoom_level() -> c::BOOLEAN {
    let state = system();
    let game = ptr::addr_of_mut!((*glob()).game_state);
    let inner_system = playerInInnerSystem();
    let mut ship_location = if inner_system {
        (*state).SunDesc[0].location
    } else {
        c_point((*game).ip_location)
    };
    (*state).SunDesc[0].radius =
        find_radius(i32::from(MAX_ZOOM_RADIUS) << 1, ship_location) as c::SIZE;
    if !inner_system {
        XFormIPLoc(
            &mut ship_location,
            ptr::addr_of_mut!((*game).ship_stamp.origin).cast(),
            1,
        );
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
