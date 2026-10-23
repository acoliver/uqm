//! Hyperspace movement and encounter spawning.

#[cfg(not(test))]
use super::abi::*;

#[cfg(not(test))]
#[derive(Default)]
struct FuelDistance {
    dx: u16,
    dy: u16,
    extra: u16,
    ticks: u8,
}

fn move_axis(position: &mut i32, delta: i16, maximum: i32) -> i16 {
    *position -= delta as i32;
    if *position < 0 {
        let clipped = delta.wrapping_add(*position as i16);
        *position = 0;
        clipped
    } else if *position > maximum {
        let clipped = delta.wrapping_add((*position - maximum) as i16);
        *position = maximum;
        clipped
    } else {
        delta
    }
}

#[cfg(not(test))]
#[allow(non_snake_case)]
pub(crate) mod live {
    use super::*;
    use crate::navigation::globals::GlobData;
    use crate::navigation::{get, set};
    use std::ptr;

    static mut DISTANCE: FuelDistance = FuelDistance {
        dx: 0,
        dy: 0,
        extra: 0,
        ticks: 1,
    };
    const PERCENT: [u16; 24] = [
        20, 0, 0, 20, 40, 0, 20, 40, 60, 40, 40, 60, 0, 30, 60, 40, 2, 30, 70, 0, 0, 70, 60, 0,
    ];
    pub(crate) unsafe fn width() -> i32 {
        (ScreenWidth - 64) << 5
    }
    pub(crate) unsafe fn height() -> i32 {
        ScreenHeight << 5
    }
    pub(crate) unsafe fn logx(value: i16) -> i32 {
        crate::navigation::hyper_units::log_x(value, ScreenWidth - 64)
    }
    pub(crate) unsafe fn logy(value: i16) -> i32 {
        crate::navigation::hyper_units::log_y(value, ScreenHeight)
    }
    pub(crate) unsafe fn universe_x(value: i32) -> i16 {
        crate::navigation::hyper_units::universe_x(value, ScreenWidth - 64)
    }
    pub(crate) unsafe fn universe_y(value: i32) -> i16 {
        crate::navigation::hyper_units::universe_y(value, ScreenHeight)
    }
    pub(crate) unsafe fn primitive(element: *mut ELEMENT) -> *mut PRIMITIVE {
        ptr::addr_of_mut!(DisplayArray)
            .cast::<PRIMITIVE>()
            .add((*element).PrimIndex as usize)
    }
    pub(crate) unsafe fn sine(angle: u16, magnitude: i32) -> i16 {
        ((crate::math::sinetab[(angle & 63) as usize] as i64 * magnitude as i64) >> SIN_SHIFT)
            as i16
    }
    pub(crate) unsafe fn cosine(angle: u16, magnitude: i32) -> i16 {
        sine(angle.wrapping_add(16), magnitude)
    }
    /// # Safety
    /// Called on the game thread when entering hyperspace.
    pub unsafe extern "C" fn rust_reset_hyper_distance() {
        DISTANCE = FuelDistance {
            dx: 0,
            dy: 0,
            extra: 0,
            ticks: 1,
        };
    }

    /// # Safety
    /// `dx` and `dy` are writable; the game world/display queue is exclusively accessible.
    #[no_mangle]
    pub unsafe extern "C" fn MoveSIS(dx: *mut i16, dy: *mut i16) {
        let mut x = move_axis(
            &mut *ptr::addr_of_mut!(GlobData.SIS_state.log_x),
            *dx,
            maximum_x(),
        );
        let mut y = move_axis(
            &mut *ptr::addr_of_mut!(GlobData.SIS_state.log_y),
            *dy,
            maximum_y(),
        );
        if x != *dx || y != *dy {
            *dx = x;
            *dy = y;
            let mut handle = disp_q.tail;
            while !handle.is_null() {
                let element = handle.cast::<ELEMENT>();
                if (*element).state_flags & PLAYER_SHIP as u16 == 0 {
                    handle = (*element).pred;
                } else {
                    (*element).next.location = POINT {
                        x: ((width() >> 1) - x as i32) as i16,
                        y: ((height() >> 1) - y as i32) as i16,
                    };
                    break;
                }
            }
        }
        if GlobData.SIS_state.FuelOnBoard != 0 && get("ARILOU_SPACE_SIDE") <= 1 {
            x = x.wrapping_abs();
            y = y.wrapping_abs();
            DISTANCE.dx = DISTANCE.dx.wrapping_add(x as u16);
            DISTANCE.dy = DISTANCE.dy.wrapping_add(y as u16);
            let adjusted_x = universe_x(16 * DISTANCE.dx as i32) as u32;
            let adjusted_y = (9999 - universe_y(16 * DISTANCE.dy as i32) as i32) as u32;
            let distance = square_root(
                adjusted_x
                    .wrapping_mul(adjusted_x)
                    .wrapping_add(adjusted_y.wrapping_mul(adjusted_y)),
            )
            .wrapping_add(DISTANCE.extra);
            let mut ticks = distance >> 4;
            if ticks > DISTANCE.ticks as u16 {
                DeltaSISGauges(0, DISTANCE.ticks as i16 - ticks as i16, 0);
                if ticks > 255 {
                    DISTANCE.dx = 0;
                    DISTANCE.dy = 0;
                    DISTANCE.extra = distance & 15;
                    ticks = 0;
                }
                DISTANCE.ticks = ticks as u8;
            }
        }
    }
    unsafe fn maximum_x() -> i32 {
        logx(10000) - (logx(1) >> 1) - 1
    }
    unsafe fn maximum_y() -> i32 {
        logy(-1) - (logy(9998) >> 1) - 1
    }

    /// # Safety
    /// Game-thread access to initialized world fleet and encounter queues.
    #[no_mangle]
    pub unsafe extern "C" fn check_hyperspace_encounter() {
        let universe = POINT {
            x: universe_x(GlobData.SIS_state.log_x),
            y: universe_y(GlobData.SIS_state.log_y),
        };
        let queue = ptr::addr_of_mut!(GlobData.Game_state.encounter_q);
        let mut handle = GlobData.Game_state.avail_race_q.head;
        let mut race = 0usize;
        while !handle.is_null() && GlobData.Game_state.CurrentActivity & IN_BATTLE as u16 != 0 {
            let fleet = handle.cast::<FLEET_INFO>();
            handle = (*fleet).succ;
            let mut radius = (*fleet).actual_strength;
            if radius != 0 {
                let mut flags = 0;
                let mut percent = PERCENT[race];
                if radius != u16::MAX {
                    radius = ((radius as u32 * SPHERE_RADIUS_INCREMENT) >> 1) as u16;
                } else {
                    radius = 20000;
                    if race == SLYLANDRO_SHIP as usize {
                        flags = ONE_SHOT_ENCOUNTER as u8;
                        percent = if get("STARBASE_AVAILABLE") == 0 {
                            100
                        } else {
                            percent * get("SLYLANDRO_MULTIPLIER") as u16
                        };
                    } else if race == MELNORME_SHIP as usize
                        && (GlobData.SIS_state.FuelOnBoard == 0 || get("USED_BROADCASTER") != 0)
                        && get("MELNORME_ANGER") < 3
                    {
                        percent = if get("USED_BROADCASTER") == 0 {
                            30
                        } else {
                            100
                        };
                        flags = ONE_SHOT_ENCOUNTER as u8;
                    }
                    let mut encounter = (*queue).head;
                    while !encounter.is_null() {
                        let current = encounter.cast::<ENCOUNTER>();
                        encounter = (*current).succ;
                        if (*current).race_id as usize == race {
                            percent = 0;
                            break;
                        }
                    }
                    if percent == 100 && race == MELNORME_SHIP as usize {
                        set("BROADCASTER_RESPONSE", 1);
                    }
                }
                let dx = universe.x.wrapping_sub((*fleet).loc.x).wrapping_abs() as u16;
                let dy = universe.y.wrapping_sub((*fleet).loc.y).wrapping_abs() as u16;
                if dx < radius
                    && dy < radius
                    && dx as u32 * dx as u32 + dy as u32 * (dy as u32)
                        < radius as u32 * radius as u32
                    && (TFB_Random() as u16 % 100) < percent
                {
                    let encounter = AllocLink(queue);
                    if !encounter.is_null() {
                        let current = encounter.cast::<ENCOUNTER>();
                        ptr::write_bytes(current, 0, 1);
                        (*current).origin = (*fleet).loc;
                        (*current).radius = radius;
                        (*current).flags = flags;
                        (*current).race_id = race as u8;
                        PutQueue(queue, encounter);
                    }
                }
            }
            race += 1;
        }
        set("USED_BROADCASTER", 0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn boundary_clipping_returns_the_actual_displacement() {
        let mut position = 2;
        assert_eq!(move_axis(&mut position, 5, 100), 2);
        assert_eq!(position, 0);
        position = 98;
        assert_eq!(move_axis(&mut position, -5, 100), -2);
        assert_eq!(position, 100);
        assert_eq!(move_axis(&mut position, 7, 100), 7);
        assert_eq!(position, 93);
    }
}
