//! Interplanetary group display, pursuit, collision and flagship movement.

const MAX_ZOOM: i16 = 128 << 6;
const MIN_ZOOM: i16 = MAX_ZOOM >> 3;

fn adjust_zoom(zoom: i16, dx: &mut i16, dy: &mut i16) {
    let shift = if zoom == MIN_ZOOM {
        2
    } else if zoom < MAX_ZOOM {
        1
    } else {
        0
    };
    *dx >>= shift;
    *dy >>= shift;
}

#[cfg(not(test))]
fn sine(angle: u16, magnitude: i32) -> i16 {
    ((crate::math::sinetab[(angle & 63) as usize] as i64 * magnitude as i64)
        >> crate::navigation::abi::SIN_SHIFT) as i16
}
#[cfg(not(test))]
fn cosine(angle: u16, magnitude: i32) -> i16 {
    sine(angle.wrapping_add(16), magnitude)
}

#[cfg(not(test))]
#[allow(non_snake_case)]
pub(crate) mod live {
    use super::*;
    use crate::navigation::abi::*;
    use crate::navigation::globals::GlobData;
    use crate::navigation::{get, set};
    use std::{mem, ptr};

    const IP_SPEED: [i16; 24] = [
        80, 54, 48, 80, 80, 70, 96, 80, 56, 60, 42, 60, 80, 40, 50, 54, 120, 36, 60, 80, 72, 60,
        60, 180,
    ];

    unsafe fn primitive(element: *mut ELEMENT) -> *mut PRIMITIVE {
        ptr::addr_of_mut!(DisplayArray)
            .cast::<PRIMITIVE>()
            .add((*element).PrimIndex as usize)
    }
    unsafe fn radius(location: u8) -> i16 {
        if location == 0 {
            (*pSolarSysState).SunDesc[0].radius
        } else {
            MAX_ZOOM
        }
    }
    unsafe fn flagship_location() -> u8 {
        if !playerInInnerSystem() {
            0
        } else {
            (1 + planetIndex(pSolarSysState, (*pSolarSysState).pOrbitalDesc)) as u8
        }
    }
    unsafe fn world(point: POINT) -> POINT {
        let width = (ScreenWidth - 64) << 5;
        let height = ScreenHeight << 5;
        POINT {
            x: (((point.x as i32) << 2) + (width >> 1) - (width >> 4)) as i16,
            y: (((point.y as i32) << 2) + (height >> 1) - (height >> 4)) as i16,
        }
    }
    unsafe fn intersect_start(element: *mut ELEMENT) {
        (*element).IntersectControl.IntersectStamp.origin = POINT {
            x: (*element).current.location.x >> 2,
            y: (*element).current.location.y >> 2,
        };
    }
    unsafe fn intersect_end(element: *mut ELEMENT) {
        (*element).IntersectControl.EndPoint = POINT {
            x: (*element).next.location.x >> 2,
            y: (*element).next.location.y >> 2,
        };
    }
    unsafe fn zero_velocity(element: *mut ELEMENT) {
        (*element).velocity = mem::zeroed();
    }

    /// # Safety
    /// Exclusive game-thread access to the initialized interplanetary group queue.
    #[no_mangle]
    pub unsafe extern "C" fn NotifyOthers(race: u16, target: u8) {
        let mut handle = GlobData.Game_state.ip_group_q.head;
        while !handle.is_null() {
            let group = handle.cast::<IP_GROUP>();
            handle = (*group).succ;
            if (*group).race_id as u16 != race {
                continue;
            }
            if target == 0 {
                (*group).task &= !(IGNORE_FLAGSHIP as u8);
                (*group).orbit_pos = (*group).dest_loc;
                (*group).dest_loc = 0;
            } else if target == u8::MAX {
                (*group).task |= IGNORE_FLAGSHIP as u8;
                if (*group).dest_loc == 0 {
                    (*group).dest_loc = (*group).orbit_pos;
                    (*group).orbit_pos = (TFB_Random() & 15) as u8;
                }
                if (*group).task & REFORM_GROUP as u8 == 0 {
                    (*group).group_counter =
                        if (*group).task & !(IGNORE_FLAGSHIP as u8) != EXPLORE as u8 {
                            0
                        } else {
                            (TFB_Random() as u16 % MAX_REVOLUTIONS as u16) << FACING_SHIFT
                        };
                }
            } else {
                (*group).task |= IGNORE_FLAGSHIP as u8;
                (*group).dest_loc = target;
            }
        }
    }

    pub(crate) unsafe extern "C" fn group_preprocess(element: *mut ELEMENT) {
        (*element).state_flags &= !((DISAPPEARING | NONSOLID) as u16);
        (*element).__bindgen_anon_1.life_span =
            (*element).__bindgen_anon_1.life_span.wrapping_add(1);
        let group = (*element).pParent.cast::<IP_GROUP>();
        let mut location = (*group).sys_loc;
        (*primitive(element)).Object.Point = (*group).loc;
        let zoom = radius(location);
        let mut destination = locationToDisplay((*group).loc, zoom);
        (*element).current.location = world(destination);
        intersect_start(element);
        let flagship = flagship_location();
        let mut task = (*group).task;
        if task & REFORM_GROUP as u8 != 0 {
            (*group).group_counter = (*group).group_counter.wrapping_sub(1);
            if (*group).group_counter == 0 {
                task &= !(REFORM_GROUP as u8);
                (*group).task = task;
                (*group).group_counter = if task & !(IGNORE_FLAGSHIP as u8) != EXPLORE as u8 {
                    0
                } else {
                    (TFB_Random() as u16 % MAX_REVOLUTIONS as u16) << FACING_SHIFT
                };
            }
        }
        if task & !((IGNORE_FLAGSHIP | REFORM_GROUP) as u8) == FLEE as u8
            && task & IGNORE_FLAGSHIP as u8 != 0
        {
            (*element).state_flags |= NONSOLID as u16;
        }
        let mut target = (*group).dest_loc;
        if task & ((IGNORE_FLAGSHIP | REFORM_GROUP) as u8) == 0 {
            if target == 0 && task != FLEE as u8 {
                target = flagship;
                if (*element).__bindgen_anon_5.thrust_wait > 5 {
                    (*element).__bindgen_anon_5.thrust_wait = 0;
                    zero_velocity(element);
                }
            } else if location == flagship {
                let mut detect = 1200i64;
                if location != 0 {
                    detect *= MAX_ZOOM as i64 / MIN_ZOOM as i64;
                    if (*group).race_id as u32 == URQUAN_DRONE_SHIP {
                        detect <<= 1;
                    }
                }
                let dx = GlobData
                    .Game_state
                    .ip_location
                    .x
                    .wrapping_sub((*group).loc.x) as i64;
                let dy = GlobData
                    .Game_state
                    .ip_location
                    .y
                    .wrapping_sub((*group).loc.y) as i64;
                if dx * dx + dy * dy < detect * detect {
                    (*element).__bindgen_anon_5.thrust_wait = 0;
                    zero_velocity(element);
                    NotifyOthers((*group).race_id as u16, 0);
                    task = (*group).task;
                    target = (*group).dest_loc;
                    if target == 0 {
                        target = flagship;
                    }
                }
            }
        }
        let (mut vx, mut vy) = (0, 0);
        GetCurrentVelocityComponents(&mut (*element).velocity, &mut vx, &mut vy);
        task &= !(IGNORE_FLAGSHIP as u8);
        if task <= ON_STATION as u8 {
            if task == FLEE as u8 {
                destination = POINT {
                    x: (*group).loc.x.wrapping_mul(2),
                    y: (*group).loc.y.wrapping_mul(2),
                };
            } else if ((task != ON_STATION as u8 || (*group).dest_loc == 0) && location == target)
                || (task == ON_STATION as u8 && (*group).dest_loc != 0 && location == 0)
            {
                if (*group).dest_loc == 0 {
                    destination = GlobData.Game_state.ip_location;
                } else {
                    let (distance, origin) = if task != ON_STATION as u8 {
                        (ORBIT_RADIUS as i32, POINT { x: 0, y: 0 })
                    } else {
                        (
                            STATION_RADIUS as i32,
                            planetOuterLocation(target as u16 - 1),
                        )
                    };
                    let mut angle = ((*group).orbit_pos as u16 + 1) << 2;
                    destination = POINT {
                        x: origin.x.wrapping_add(cosine(angle, distance)),
                        y: origin.y.wrapping_add(sine(angle, distance)),
                    };
                    if (*group).loc.x == destination.x && (*group).loc.y == destination.y {
                        (*group).orbit_pos = (((angle + 2) >> 2) & 15) as u8;
                        angle += 4;
                        destination = POINT {
                            x: origin.x.wrapping_add(cosine(angle, distance)),
                            y: origin.y.wrapping_add(sine(angle, distance)),
                        };
                        (*element).__bindgen_anon_5.thrust_wait = u8::MAX;
                        if (*group).group_counter != 0 {
                            (*group).group_counter -= 1;
                        } else if task == EXPLORE as u8 {
                            let next = ((TFB_Random() as u16
                                % (*pSolarSysState).SunDesc[0].NumPlanets as u16)
                                + 1) as u8;
                            if next != target {
                                (*element).__bindgen_anon_5.thrust_wait = 0;
                                target = next;
                                (*group).dest_loc = next;
                            }
                        }
                    }
                }
            } else if location == 0 {
                destination = if (*group).dest_loc == 0 {
                    (*pSolarSysState).SunDesc[0].location
                } else {
                    planetOuterLocation(target as u16 - 1)
                };
            } else {
                if task == ON_STATION as u8 {
                    target = 0;
                }
                destination = POINT {
                    x: (*group).loc.x.wrapping_mul(2),
                    y: (*group).loc.y.wrapping_mul(2),
                };
            }
            let delta_x = destination.x.wrapping_sub((*group).loc.x);
            let delta_y = destination.y.wrapping_sub((*group).loc.y);
            let angle = ARCTAN(delta_x, delta_y);
            let wait = (*element).__bindgen_anon_5.thrust_wait;
            if wait != 0 && wait != u8::MAX {
                (*element).__bindgen_anon_5.thrust_wait -= 1;
            } else if (vx == 0 && vy == 0) || angle != (*element).velocity.travel_angle {
                let speed = if wait != 0 && (*group).dest_loc != 0 {
                    if task == ON_STATION as u8 {
                        30
                    } else {
                        60
                    }
                } else {
                    (*element).__bindgen_anon_5.thrust_wait = 5;
                    IP_SPEED[(*group).race_id as usize]
                };
                vx = cosine(angle, speed as i32);
                vy = sine(angle, speed as i32);
                SetVelocityComponents(&mut (*element).velocity, vx, vy);
            }
            let (mut dx, mut dy) = (vx, vy);
            let mut transition = false;
            let mut partial = false;
            let mut escape = false;
            if location == target {
                if target == 0 {
                    if task == FLEE as u8 {
                        escape = true;
                    }
                } else if target == (*group).dest_loc {
                    partial = true;
                }
            } else if location == 0 {
                adjust_zoom(zoom, &mut dx, &mut dy);
                if task == ON_STATION as u8 && (*group).dest_loc != 0 {
                    partial = true;
                } else {
                    transition = reaches(dx, dy, delta_x, delta_y);
                }
            } else {
                escape = true;
            }
            if partial && reaches(dx, dy, delta_x, delta_y) {
                (*group).loc = destination;
                vx = 0;
                vy = 0;
                zero_velocity(element);
            }
            if escape {
                let display = locationToDisplay((*group).loc, zoom);
                transition = display.x < 0
                    || display.x as i32 >= ScreenWidth - 64 - 14
                    || display.y < 0
                    || display.y as i32 >= ScreenHeight - 13;
            }
            if transition {
                (*element).state_flags |= NONSOLID as u16;
                vx = 0;
                vy = 0;
                zero_velocity(element);
                if location != 0 {
                    (*group).loc = planetOuterLocation(location as u16 - 1);
                    location = 0;
                    (*group).sys_loc = 0;
                } else if target == 0 {
                    (*element).__bindgen_anon_1.life_span = 0;
                    (*element).state_flags |= (DISAPPEARING | NONSOLID) as u16;
                    (*group).in_system = 0;
                    return;
                } else {
                    if target == (*group).dest_loc {
                        (*group).orbit_pos = (((angle + HALF_CIRCLE as u16 + 2) >> 2) & 15) as u8;
                        (*group).group_counter =
                            (TFB_Random() as u16 % MAX_REVOLUTIONS as u16) << FACING_SHIFT;
                    }
                    let width = ScreenWidth - 64 - 14;
                    let height = ScreenHeight - 13;
                    let entry = POINT {
                        x: ((width >> 1) - cosine(angle, width * 9 / 16) as i32) as i16,
                        y: ((height >> 1) - sine(angle, height * 9 / 16) as i32) as i16,
                    };
                    (*group).loc = displayToLocation(entry, MAX_ZOOM);
                    location = target;
                    (*group).sys_loc = target;
                }
            }
        }
        let zoom = radius(location);
        adjust_zoom(zoom, &mut vx, &mut vy);
        (*group).loc.x = (*group).loc.x.wrapping_add(vx);
        (*group).loc.y = (*group).loc.y.wrapping_add(vy);
        (*element).next.location = world(locationToDisplay((*group).loc, zoom));
        if location != flagship
            || (task & REFORM_GROUP as u8 != 0 && (*group).group_counter & 1 != 0)
        {
            (*primitive(element)).Type = gfx_object_NUM_PRIMS as u8;
            (*element).state_flags |= NONSOLID as u16;
        } else {
            (*primitive(element)).Type = gfx_object_STAMP_PRIM as u8;
            if task & REFORM_GROUP as u8 != 0 {
                (*element).state_flags |= NONSOLID as u16;
            }
        }
        (*element).state_flags |= CHANGING as u16;
    }

    fn reaches(dx: i16, dy: i16, target_x: i16, target_y: i16) -> bool {
        (dx.wrapping_mul(dx) as u16 as i64 + dy.wrapping_mul(dy) as u16 as i64)
            >= target_x as i64 * target_x as i64 + target_y as i64 * target_y as i64
    }

    unsafe fn collide_flagship(first: *mut ELEMENT, second: *mut ELEMENT) {
        if (*second).state_flags & COLLISION as u16 == 0 {
            (*first).state_flags |= (COLLISION | NONSOLID) as u16;
        } else {
            (*second).state_flags &= !(COLLISION as u16);
            GlobData.Game_state.CurrentActivity |= START_ENCOUNTER as u16;
        }
    }
    pub(crate) unsafe extern "C" fn flagship_collision(
        first: *mut ELEMENT,
        _p0: *mut POINT,
        second: *mut ELEMENT,
        _p1: *mut POINT,
    ) {
        if GlobData.Game_state.CurrentActivity & START_ENCOUNTER as u16 != 0 {
            return;
        }
        collide_flagship(first, second);
    }
    pub(crate) unsafe extern "C" fn group_collision(
        first: *mut ELEMENT,
        _p0: *mut POINT,
        second: *mut ELEMENT,
        _p1: *mut POINT,
    ) {
        if GlobData.Game_state.CurrentActivity & START_ENCOUNTER as u16 != 0 {
            return;
        }
        let group = (*first).pParent.cast::<IP_GROUP>();
        if !(*second).pParent.is_null() {
            if (*first).state_flags & COLLISION as u16 != 0
                || ((*second).current.location.x == (*second).next.location.x
                    && (*second).current.location.y == (*second).next.location.y)
            {
                (*first).state_flags &= !(COLLISION as u16);
            } else {
                (*second).state_flags |= COLLISION as u16;
                (*group).loc = (*primitive(first)).Object.Point;
                (*first).next.location = (*first).current.location;
                intersect_end(first);
            }
        } else {
            EncounterGroup = (*group).group_id;
            (*group).task |= REFORM_GROUP as u8;
            (*group).group_counter = 100;
            NotifyOthers((*group).race_id as u16, u8::MAX);
            collide_flagship(first, second);
        }
    }
    unsafe fn spawn_group(group: *mut IP_GROUP) {
        let handle = AllocElement();
        if handle.is_null() {
            return;
        }
        let element = handle.cast::<ELEMENT>();
        (*element).__bindgen_anon_3.mass_points = 1;
        (*element).__bindgen_anon_2.hit_points = 1;
        (*element).state_flags = (CHANGING | FINITE_LIFE | IGNORE_VELOCITY) as u16;
        (*primitive(element)).Type = gfx_object_STAMP_PRIM as u8;
        let icon = (*group).melee_icon;
        crate::collections::observation::image!(single; (*element).current.image.farray, ptr::addr_of_mut!((*group).melee_icon), icon);
        crate::collections::observation::image!(frame; (*element).current.image.frame, SetAbsFrameIndex(icon, 1));
        (*element).death_func = Some(ElementHook::Legacy(group_preprocess));
        (*element).collision_func = Some(CollisionHook::Legacy(group_collision));
        (*element).current.location =
            world(locationToDisplay((*group).loc, radius((*group).sys_loc)));
        (*element).pParent = group.cast();
        SetUpElement(element);
        crate::collections::observation::image!(frame; (*element).IntersectControl.IntersectStamp.frame, DecFrameIndex(stars_in_space));
        PutQueue(ptr::addr_of_mut!(disp_q), handle);
    }
    pub(crate) unsafe extern "C" fn flagship_preprocess(element: *mut ELEMENT) {
        (*element).__bindgen_anon_5.thrust_wait =
            (*element).__bindgen_anon_5.thrust_wait.wrapping_sub(1);
        if (*element).__bindgen_anon_5.thrust_wait == 0 {
            let next = (*element).succ;
            if !next.is_null() && next != disp_q.tail {
                RemoveElement(next);
                PutQueue(ptr::addr_of_mut!(disp_q), next);
            }
            (*element).__bindgen_anon_5.thrust_wait = 42;
        }
        let (mut vx, mut vy) = (0, 0);
        GetCurrentVelocityComponents(
            ptr::addr_of_mut!(GlobData.Game_state.velocity),
            &mut vx,
            &mut vy,
        );
        let flagship = flagship_location();
        let zoom = radius(flagship);
        adjust_zoom(zoom, &mut vx, &mut vy);
        (*element).current.location =
            world(locationToDisplay(GlobData.Game_state.ip_location, zoom));
        intersect_start(element);
        GlobData.Game_state.ip_location.x = GlobData.Game_state.ip_location.x.wrapping_add(vx);
        GlobData.Game_state.ip_location.y = GlobData.Game_state.ip_location.y.wrapping_add(vy);
        let point = locationToDisplay(GlobData.Game_state.ip_location, zoom);
        (*element).next.location = world(point);
        GlobData.Game_state.ShipStamp.origin = point;
        crate::collections::observation::image!(frame; (*element).next.image.frame, GlobData.Game_state.ShipStamp.frame);
        if (*element).__bindgen_anon_4.sys_loc == flagship {
            (*element).state_flags &= !(NONSOLID as u16);
        } else {
            (*element).state_flags |= NONSOLID as u16;
            (*element).__bindgen_anon_4.sys_loc = flagship;
        }
        let escape = get("ESCAPE_COUNTER");
        if escape != 0 && GlobData.Game_state.CurrentActivity & START_ENCOUNTER as u16 == 0 {
            (*element).state_flags |= NONSOLID as u16;
            set("ESCAPE_COUNTER", escape - 1);
        }
        (*element).state_flags |= CHANGING as u16;
    }
    unsafe fn spawn_flagship() {
        let handle = AllocElement();
        if handle.is_null() {
            return;
        }
        let element = handle.cast::<ELEMENT>();
        (*element).__bindgen_anon_2.hit_points = 1;
        (*element).__bindgen_anon_3.mass_points = 1;
        (*element).__bindgen_anon_4.sys_loc = flagship_location();
        (*element).state_flags = (APPEARING | IGNORE_VELOCITY) as u16;
        if get("ESCAPE_COUNTER") != 0 {
            (*element).state_flags |= NONSOLID as u16;
        }
        (*element).__bindgen_anon_1.life_span = NORMAL_LIFE as u16;
        (*element).__bindgen_anon_5.thrust_wait = 42;
        (*primitive(element)).Type = gfx_object_STAMP_PRIM as u8;
        let frame = GlobData.Game_state.ShipStamp.frame;
        crate::collections::observation::image!(single; (*element).current.image.farray, ptr::addr_of_mut!(GlobData.Game_state.ShipStamp.frame), frame);
        crate::collections::observation::image!(frame; (*element).current.image.frame, frame);
        (*element).preprocess_func = Some(ElementHook::Legacy(flagship_preprocess));
        (*element).collision_func = Some(CollisionHook::Legacy(flagship_collision));
        (*element).current.location = world(GlobData.Game_state.ShipStamp.origin);
        PutQueue(ptr::addr_of_mut!(disp_q), handle);
    }

    /// # Safety
    /// The live solar system and display/group queues must be initialized on the game thread.
    #[no_mangle]
    pub unsafe extern "C" fn DoMissions() {
        spawn_flagship();
        if EncounterRace >= 0 {
            NotifyOthers(EncounterRace as u16, 0);
            EncounterRace = -1;
        }
        let mut handle = GlobData.Game_state.ip_group_q.head;
        while !handle.is_null() {
            let group = handle.cast::<IP_GROUP>();
            handle = (*group).succ;
            if (*group).in_system != 0 {
                spawn_group(group);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zoom_adjustment_keeps_signed_right_shift_rounding() {
        for (zoom, expected) in [
            (MIN_ZOOM, (15, -16)),
            (MIN_ZOOM * 2, (30, -31)),
            (MAX_ZOOM, (60, -61)),
        ] {
            let (mut dx, mut dy) = (60, -61);
            adjust_zoom(zoom, &mut dx, &mut dy);
            assert_eq!((dx, dy), expected);
        }
    }
}
