//! Ship velocity and gravity collision primitives.
use super::flags::{SHIP_AT_MAX_SPEED, SHIP_BEYOND_MAX_SPEED, SHIP_IN_GRAVITY_WELL};
use crate::math::battle_types::ONE_SHIFT;
use crate::math::velocity::VELOCITY_SHIFT;
const MAX_SPEED: u16 = SHIP_AT_MAX_SPEED as u16;
const BEYOND_SPEED: u16 = SHIP_BEYOND_MAX_SPEED as u16;
const GRAVITY: u16 = SHIP_IN_GRAVITY_WELL as u16;
use super::flags::{COLLISION, FINITE_LIFE};
use super::{Cx, ElementId};
use crate::graphics::native_abi::POINT;
const GRAVITY_MASS_NATIVE: u16 = 100;
const TARGET_DAMAGED_FOR_1_PT_NATIVE: i16 = 2;
const TARGET_DAMAGED_FOR_6_PLUS_PT_NATIVE: i16 = 5;
fn velocity_squared(dx: i16, dy: i16) -> u32 {
    (i64::from(dx) * i64::from(dx) + i64::from(dy) * i64::from(dy)) as u32
}
#[derive(Debug, PartialEq)]
enum Acceleration {
    Normal,
    Beyond,
    Aligned,
    Redirect,
}
fn acceleration(
    desired: u32,
    maximum: u32,
    current: u32,
    gravity: bool,
    aligned: bool,
) -> Acceleration {
    if desired <= maximum {
        Acceleration::Normal
    } else if (gravity && desired <= (18u32 << (ONE_SHIFT + VELOCITY_SHIFT)).pow(2))
        || desired < current
    {
        Acceleration::Beyond
    } else if aligned {
        Acceleration::Aligned
    } else {
        Acceleration::Redirect
    }
}
pub fn thrust(
    velocity: &mut crate::math::velocity::VelocityDesc,
    facing: u16,
    status: u16,
    increment: u16,
    maximum: u16,
) -> u16 {
    let current_angle = (i32::from(facing) << 2) as u16 as i32;
    let travel_angle = velocity.travel_angle;
    let mut thrust = increment;
    let maximum_thrust = maximum;
    if thrust == maximum_thrust {
        velocity.set_vector(i32::from(maximum_thrust as i16), facing);
        return MAX_SPEED;
    }
    if travel_angle == current_angle as u16
        && status & (MAX_SPEED | BEYOND_SPEED) != 0
        && status & GRAVITY == 0
    {
        return status & (MAX_SPEED | BEYOND_SPEED);
    }
    thrust = (i32::from(thrust) << 5) as u16;
    let (dx, dy) = velocity.get_current_components();
    let (mut dx, mut dy) = (dx as i16, dy as i16);
    let current_speed = velocity_squared(dx, dy);
    let next_dx = dx.wrapping_add(crate::math::cosine(current_angle, i32::from(thrust)));
    let next_dy = dy.wrapping_add(crate::math::sine(current_angle, i32::from(thrust)));
    let desired = velocity_squared(next_dx, next_dy);
    let max_speed = velocity_squared((i32::from(maximum_thrust) << 5) as i16, 0);
    match acceleration(
        desired,
        max_speed,
        current_speed,
        status & GRAVITY != 0,
        travel_angle == current_angle as u16,
    ) {
        Acceleration::Normal => {
            velocity.set_components(i32::from(next_dx), i32::from(next_dy));
        }
        Acceleration::Beyond => {
            velocity.set_components(i32::from(next_dx), i32::from(next_dy));
            return MAX_SPEED | BEYOND_SPEED;
        }
        Acceleration::Aligned => {
            if current_speed <= max_speed {
                velocity.set_vector(i32::from(maximum_thrust as i16), facing);
            }
            return MAX_SPEED;
        }
        Acceleration::Redirect => {
            let mut next_velocity = *velocity;
            next_velocity.delta_components(
                i32::from(
                    crate::math::cosine(current_angle, i32::from(thrust >> 1)).wrapping_sub(
                        crate::math::cosine(i32::from(travel_angle), i32::from(thrust)),
                    ),
                ),
                i32::from(
                    crate::math::sine(current_angle, i32::from(thrust >> 1)).wrapping_sub(
                        crate::math::sine(i32::from(travel_angle), i32::from(thrust)),
                    ),
                ),
            );
            let components = next_velocity.get_current_components();
            (dx, dy) = (components.0 as i16, components.1 as i16);
            let desired = velocity_squared(dx, dy);
            if desired > max_speed {
                if desired < current_speed {
                    *velocity = next_velocity;
                }
                return MAX_SPEED | BEYOND_SPEED;
            }
            *velocity = next_velocity;
        }
    }
    0
}
pub fn inertial_thrust(cx: &mut Cx<'_>, e: ElementId) -> u16 {
    let ship = cx.owner(e);
    let state = cx.ship_state(ship);
    let stats = cx.race_stats(ship).characteristics;
    let mut velocity = cx.element(e).velocity;
    let result = thrust(
        &mut velocity,
        state.facing,
        state.status,
        stats.thrust_increment,
        stats.max_thrust,
    );
    cx.element_mut(e).velocity = velocity;
    result
}
pub fn collision(cx: &mut Cx<'_>, e0: ElementId, _p0: &mut POINT, e1: ElementId, _p1: &mut POINT) {
    if cx.element(e1).flags & FINITE_LIFE as u16 == 0 {
        cx.element_mut(e0).flags |= COLLISION as u16;
        if u16::from(cx.element(e1).mass_points()) > GRAVITY_MASS_NATIVE {
            let damage = (cx.element(e0).hit_points() >> 2).max(1);
            cx.damage(e0, damage as i16);
            let sound = (TARGET_DAMAGED_FOR_1_PT_NATIVE + (damage >> 1) as i16)
                .min(TARGET_DAMAGED_FOR_6_PLUS_PT_NATIVE);
            cx.game_sound(sound as u16, e0);
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn inertial_thrust_preserves_speed_flags_and_complete_velocity_state() {
        use crate::math::velocity::VelocityDesc;
        for facing in 0..16 {
            let mut instant = VelocityDesc::new();
            let mut expected = instant;
            expected.set_vector(24, facing);
            assert_eq!(
                super::thrust(&mut instant, facing, 0, 24, 24),
                super::SHIP_AT_MAX_SPEED as u16
            );
            assert_eq!(instant, expected);
            for flags in [
                super::SHIP_AT_MAX_SPEED as u16,
                super::SHIP_BEYOND_MAX_SPEED as u16,
                (super::SHIP_AT_MAX_SPEED | super::SHIP_BEYOND_MAX_SPEED) as u16,
            ] {
                let before = instant;
                assert_eq!(super::thrust(&mut instant, facing, flags, 2, 24), flags);
                assert_eq!(instant, before);
            }
            let mut accelerating = VelocityDesc::new();
            assert_eq!(super::thrust(&mut accelerating, facing, 0, 2, 24), 0);
            let angle = i32::from(facing) << 2;
            let mut expected = VelocityDesc::new();
            expected.set_components(
                i32::from(crate::math::cosine(angle, 64)),
                i32::from(crate::math::sine(angle, 64)),
            );
            assert_eq!(accelerating, expected);
        }
    }
    #[test]
    fn inertial_thrust_gravity_redirect_and_native_widths_keep_complete_state() {
        use crate::math::velocity::VelocityDesc;
        for (initial, facing, status, increment, maximum, components, flags) in [
            ((0, -768), 0, 1 << 8, 2, 24, (0, -832), 192),
            ((0, -768), 4, 0, 2, 24, (32, -704), 0),
            ((0, -2000), 4, 0, 2, 24, (32, -1936), 192),
            (
                (i16::MIN as i32, i16::MAX as i32),
                4,
                0,
                u16::MAX - 1,
                u16::MAX,
                (32704, 32767),
                192,
            ),
        ] {
            let mut actual = VelocityDesc::default();
            actual.set_components(initial.0, initial.1);
            let mut expected = VelocityDesc::default();
            expected.set_components(components.0, components.1);
            assert_eq!(
                thrust(&mut actual, facing, status, increment, maximum),
                flags
            );
            assert_eq!(actual, expected, "initial={initial:?}, facing={facing}");
        }
    }
    #[test]
    fn thrust_branches_prioritize_normal_then_gravity_and_deceleration() {
        assert_eq!(
            acceleration(100, 100, 90, false, true),
            Acceleration::Normal
        );
        assert_eq!(acceleration(101, 100, 90, true, true), Acceleration::Beyond);
        assert_eq!(
            acceleration(101, 100, 102, false, true),
            Acceleration::Beyond
        );
        assert_eq!(
            acceleration(101, 100, 90, false, true),
            Acceleration::Aligned
        );
        assert_eq!(
            acceleration(101, 100, 90, false, false),
            Acceleration::Redirect
        );
        let limit = (18u32 << (ONE_SHIFT + VELOCITY_SHIFT)).pow(2);
        assert_eq!(
            acceleration(limit, 100, 90, true, false),
            Acceleration::Beyond
        );
        assert_eq!(
            acceleration(limit + 1, 100, 90, true, false),
            Acceleration::Redirect
        );
    }
    #[test]
    fn velocity_squared_matches_widened_c_long_arithmetic() {
        assert_eq!(velocity_squared(3, 4), 25);
        assert_eq!(velocity_squared(i16::MIN, 0), 1_073_741_824);
        assert_eq!(velocity_squared(i16::MIN, i16::MIN), 2_147_483_648);
    }
}
