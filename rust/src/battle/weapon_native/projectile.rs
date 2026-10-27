//! Fixed-width projectile geometry, independent of allocation and frame ownership.

const WORLD_SHIFT: u32 = 2;
const VELOCITY_SHIFT: u32 = 5;

fn origin(position: (i16, i16), facing: u16, offset: i16) -> (i16, i16) {
    let angle = i32::from(facing) << 2;
    let magnitude = i32::from(offset) << WORLD_SHIFT;
    (
        (i32::from(position.0) + i32::from(crate::math::sine(angle + 16, magnitude))) as i16,
        (i32::from(position.1) + i32::from(crate::math::sine(angle, magnitude))) as i16,
    )
}

pub(super) fn laser(
    position: (i16, i16),
    facing: u16,
    offset: i16,
    end: (i16, i16),
) -> ((i16, i16), (i16, i16)) {
    let start = origin(position, facing, offset);
    let velocity = (
        ((i32::from(position.0) + i32::from(end.0) - i32::from(start.0)) << VELOCITY_SHIFT) as i16,
        ((i32::from(position.1) + i32::from(end.1) - i32::from(start.1)) << VELOCITY_SHIFT) as i16,
    );
    (start, velocity)
}

pub(super) fn missile(
    position: (i16, i16),
    facing: u16,
    offset: i16,
    speed: i16,
) -> ((i16, i16), (i16, i16)) {
    let mut start = origin(position, facing, offset);
    let angle = i32::from(facing) << 2;
    let velocity = (
        crate::math::sine(angle + 16, i32::from(speed) << VELOCITY_SHIFT),
        crate::math::sine(angle, i32::from(speed) << VELOCITY_SHIFT),
    );
    start.0 = start.0.wrapping_sub(velocity.0 >> VELOCITY_SHIFT);
    start.1 = start.1.wrapping_sub(velocity.1 >> VELOCITY_SHIFT);
    (start, velocity)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn laser_narrows_each_position_before_deriving_velocity() {
        assert_eq!(
            laser((i16::MAX, i16::MIN), 4, 3, (1025, -1025)),
            ((-32757, i16::MIN), (32416, 32736))
        );
    }

    #[test]
    fn missile_applies_velocity_backoff_after_offset_and_size_truncation() {
        assert_eq!(missile((100, 200), 4, 3, 7), ((105, 200), (224, 0)));
        assert_eq!(
            missile((100, 200), 4 + 16384, 3, 7),
            missile((100, 200), 4, 3, 7)
        );
    }
}
