//! Homing over the explicit ordered element pool.

#[cfg(not(test))]
use super::flags::*;
#[cfg(not(test))]
use super::{Cx, ElementId, Subject};
use crate::math::battle_types::HALF_CIRCLE;
#[cfg(not(test))]
use crate::math::battle_types::{MAX_REDUCTION, ONE_SHIFT};
#[cfg(not(test))]
use crate::math::wrap_delta;
use crate::math::{angle_to_facing, normalize_facing};

fn turn_toward(facing: u16, delta: i16, random: impl FnOnce() -> u32) -> u16 {
    if delta <= 0 {
        return facing;
    }
    let half = angle_to_facing(HALF_CIRCLE as i32) as i16;
    let step = if delta == half {
        (((random() as u8 & 1) as i32) << 1) - 1
    } else if delta < half {
        1
    } else {
        -1
    };
    normalize_facing(i32::from(facing) + step)
}

#[cfg(not(test))]
pub fn track_ship(cx: &mut Cx<'_>, id: ElementId, facing: &mut u16) -> i16 {
    let mut subject = cx.weapon_origin(id);
    let result = track_subject(cx, &mut subject, facing);
    cx.set_target(id, subject.target);
    result
}

#[cfg(not(test))]
pub fn track_subject(cx: &mut Cx<'_>, tracker: &mut Subject, facing: &mut u16) -> i16 {
    let mut best_distance: i16 = 0;
    let mut best_facing: i16 = -1;
    let explicit = tracker.target.is_some();
    let mut handle = if explicit {
        tracker.target.take()
    } else {
        cx.elements().head()
    };
    while let Some(target) = handle {
        let next = if explicit {
            None
        } else {
            cx.elements().succ(target)
        };
        let state = cx.element(target);
        if explicit
            || (state.flags & PLAYER_SHIP as u16 != 0
                && state.player != tracker.player
                && (!cx.object_cloaked(target)
                    || tracker.flags & (PLAYER_SHIP | APPEARING) as u16
                        == (PLAYER_SHIP | APPEARING) as u16))
        {
            let ship = cx.owner(target);
            if state.life != 0 && cx.race_stats(ship).ship_info.crew_level != 0 {
                let (t, s) = if tracker.flags & PRE_PROCESS as u16 != 0 {
                    (state.next.location, tracker.next.location)
                } else {
                    (state.current.location, tracker.current.location)
                };
                let (width, height) = cx.screen_dimensions();
                let mut dx = wrap_delta(
                    t.x.wrapping_sub(s.x),
                    (width - 64) << (ONE_SHIFT + MAX_REDUCTION),
                );
                let mut dy =
                    wrap_delta(t.y.wrapping_sub(s.y), height << (ONE_SHIFT + MAX_REDUCTION));
                let delta = normalize_facing(
                    angle_to_facing(crate::math::arctan(dx, dy) as i32) - i32::from(*facing),
                ) as i16;
                if dx < 0 {
                    dx = dx.wrapping_neg();
                }
                if dy < 0 {
                    dy = dy.wrapping_neg();
                }
                dx = dx.wrapping_add(dy);
                if best_distance == 0 || dx < best_distance {
                    best_distance = dx;
                    best_facing = delta;
                    tracker.target = Some(target);
                }
            }
        }
        handle = next;
    }
    *facing = turn_toward(*facing, best_facing, || cx.random());
    best_facing
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn homing_randomness_is_only_consumed_for_opposite_heading() {
        assert_eq!(turn_toward(15, 1, || panic!()), 0);
        assert_eq!(turn_toward(0, 15, || panic!()), 15);
        assert_eq!(turn_toward(3, 0, || panic!()), 3);
        assert_eq!(turn_toward(3, -1, || panic!()), 3);
        assert_eq!(turn_toward(0, 8, || 0), 15);
        assert_eq!(turn_toward(0, 8, || 1), 1);
    }
}
