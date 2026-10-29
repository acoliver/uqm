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
pub(crate) mod live {
    use super::*;
    use crate::navigation::abi::*;
    use crate::navigation::{get, set};

    const IP_SPEED: [i16; 24] = [
        80, 54, 48, 80, 80, 70, 96, 80, 56, 60, 42, 60, 80, 40, 50, 54, 120, 36, 60, 80, 72, 60,
        60, 180,
    ];

    fn radius(cx: &crate::engine::battle::Cx<'_>, location: u8) -> i16 {
        if location == 0 {
            cx.solar().SunDesc[0].radius
        } else {
            MAX_ZOOM
        }
    }
    fn flagship_location(cx: &crate::engine::battle::Cx<'_>) -> u8 {
        let solar = cx.solar();
        if solar.pBaseDesc == solar.PlanetDesc.as_ptr().cast_mut() {
            return 0;
        }
        assert_eq!(solar.pBaseDesc, solar.MoonDesc.as_ptr().cast_mut());
        let orbital =
            cx.project(std::ptr::NonNull::new(solar.pOrbitalDesc).expect("orbital descriptor"));
        let planet = if orbital.pPrevDesc == solar.SunDesc.as_ptr().cast_mut() {
            solar.pOrbitalDesc
        } else {
            orbital.pPrevDesc
        };
        let offset = (planet as usize)
            .checked_sub(solar.PlanetDesc.as_ptr() as usize)
            .expect("planet belongs to system");
        assert!(offset.is_multiple_of(std::mem::size_of::<PLANET_DESC>()));
        (1 + offset / std::mem::size_of::<PLANET_DESC>()) as u8
    }
    fn world(cx: &crate::engine::battle::Cx<'_>, point: POINT) -> POINT {
        let (screen_width, screen_height) = cx.screen_dimensions();
        let width = (screen_width - 64) << 5;
        let height = screen_height << 5;
        POINT {
            x: (((point.x as i32) << 2) + (width >> 1) - (width >> 4)) as i16,
            y: (((point.y as i32) << 2) + (height >> 1) - (height >> 4)) as i16,
        }
    }
    /// # Safety
    /// Exclusive game-thread access to the initialized interplanetary group queue.
    pub unsafe extern "C" fn notify_others_abi(race: u16, target: u8) {
        crate::app::bridge::enter(|cx| notify_others(cx, race, target));
    }

    fn notify_others(cx: &mut crate::engine::battle::Cx<'_>, race: u16, target: u8) {
        let mut handle = cx.navigation().Game_state.ip_group_q.head;
        while !handle.is_null() {
            let pointer = std::ptr::NonNull::new(handle.cast::<IP_GROUP>()).expect("linked group");
            let mut group = *cx.project(pointer);
            handle = group.succ;
            if group.race_id as u16 != race {
                continue;
            }
            if target == 0 {
                group.task &= !(IGNORE_FLAGSHIP as u8);
                group.orbit_pos = group.dest_loc;
                group.dest_loc = 0;
            } else if target == u8::MAX {
                group.task |= IGNORE_FLAGSHIP as u8;
                if group.dest_loc == 0 {
                    group.dest_loc = group.orbit_pos;
                    group.orbit_pos = (cx.random() & 15) as u8;
                }
                if group.task & REFORM_GROUP as u8 == 0 {
                    group.group_counter = if group.task & !(IGNORE_FLAGSHIP as u8) != EXPLORE as u8
                    {
                        0
                    } else {
                        (cx.random() as u16 % MAX_REVOLUTIONS as u16) << FACING_SHIFT
                    };
                }
            } else {
                group.task |= IGNORE_FLAGSHIP as u8;
                group.dest_loc = target;
            }
            *cx.project_mut(pointer) = group;
        }
    }

    pub(crate) fn group_preprocess(
        cx: &mut crate::engine::battle::Cx<'_>,
        element: crate::engine::battle::ElementId,
    ) {
        begin_group_step(cx, element);
        let mut location = cx.ip_group_mut(element).sys_loc;
        let mut stamp = cx.primitive_stamp(element);
        stamp.origin = cx.ip_group(element).loc;
        cx.set_primitive_stamp(element, stamp);
        let zoom = radius(cx, location);
        let mut destination = locationToDisplay(cx.ip_group_mut(element).loc, zoom);
        cx.element_mut(element).current.location = world(cx, destination);
        cx.init_intersect_start(element);
        let flagship = flagship_location(cx);
        let mut task = reform_group(cx, element);
        if task & !((IGNORE_FLAGSHIP | REFORM_GROUP) as u8) == FLEE as u8
            && task & IGNORE_FLAGSHIP as u8 != 0
        {
            cx.element_mut(element).flags |= NONSOLID as u16;
        }
        let mut target = cx.ip_group_mut(element).dest_loc;
        if task & ((IGNORE_FLAGSHIP | REFORM_GROUP) as u8) == 0 {
            if target == 0 && task != FLEE as u8 {
                target = flagship;
                if cx.element_mut(element).thrust > 5 {
                    cx.element_mut(element).thrust = 0;
                    cx.element_mut(element).velocity.zero();
                }
            } else if location == flagship {
                let mut detect = 1200i64;
                if location != 0 {
                    detect *= MAX_ZOOM as i64 / MIN_ZOOM as i64;
                    if cx.ip_group_mut(element).race_id as u32 == URQUAN_DRONE_SHIP {
                        detect <<= 1;
                    }
                }
                let dx = cx
                    .navigation_mut()
                    .Game_state
                    .ip_location
                    .x
                    .wrapping_sub(cx.ip_group_mut(element).loc.x) as i64;
                let dy = cx
                    .navigation_mut()
                    .Game_state
                    .ip_location
                    .y
                    .wrapping_sub(cx.ip_group_mut(element).loc.y) as i64;
                if dx * dx + dy * dy < detect * detect {
                    cx.element_mut(element).thrust = 0;
                    cx.element_mut(element).velocity.zero();
                    let race = cx.ip_group(element).race_id as u16;
                    notify_others(cx, race, 0);
                    task = cx.ip_group_mut(element).task;
                    target = cx.ip_group_mut(element).dest_loc;
                    if target == 0 {
                        target = flagship;
                    }
                }
            }
        }
        let (current_x, current_y) = cx.element_mut(element).velocity.get_current_components();
        let (mut vx, mut vy) = (current_x as i16, current_y as i16);
        task &= !(IGNORE_FLAGSHIP as u8);
        if task <= ON_STATION as u8 {
            if task == FLEE as u8 {
                destination = POINT {
                    x: cx.ip_group_mut(element).loc.x.wrapping_mul(2),
                    y: cx.ip_group_mut(element).loc.y.wrapping_mul(2),
                };
            } else if ((task != ON_STATION as u8 || cx.ip_group_mut(element).dest_loc == 0)
                && location == target)
                || (task == ON_STATION as u8
                    && cx.ip_group_mut(element).dest_loc != 0
                    && location == 0)
            {
                if cx.ip_group_mut(element).dest_loc == 0 {
                    destination = cx.navigation().Game_state.ip_location;
                } else {
                    let (distance, origin) = if task != ON_STATION as u8 {
                        (ORBIT_RADIUS as i32, POINT { x: 0, y: 0 })
                    } else {
                        (
                            STATION_RADIUS as i32,
                            cx.planet_outer_location(target as u16 - 1),
                        )
                    };
                    let mut angle = (cx.ip_group_mut(element).orbit_pos as u16 + 1) << 2;
                    destination = POINT {
                        x: origin.x.wrapping_add(cosine(angle, distance)),
                        y: origin.y.wrapping_add(sine(angle, distance)),
                    };
                    if cx.ip_group_mut(element).loc.x == destination.x
                        && cx.ip_group_mut(element).loc.y == destination.y
                    {
                        cx.ip_group_mut(element).orbit_pos = (((angle + 2) >> 2) & 15) as u8;
                        angle += 4;
                        destination = POINT {
                            x: origin.x.wrapping_add(cosine(angle, distance)),
                            y: origin.y.wrapping_add(sine(angle, distance)),
                        };
                        cx.element_mut(element).thrust = u8::MAX;
                        if cx.ip_group_mut(element).group_counter != 0 {
                            cx.ip_group_mut(element).group_counter -= 1;
                        } else if task == EXPLORE as u8 {
                            let next = ((cx.random() as u16
                                % cx.solar().SunDesc[0].NumPlanets as u16)
                                + 1) as u8;
                            if next != target {
                                cx.element_mut(element).thrust = 0;
                                target = next;
                                cx.ip_group_mut(element).dest_loc = next;
                            }
                        }
                    }
                }
            } else if location == 0 {
                destination = if cx.ip_group_mut(element).dest_loc == 0 {
                    cx.solar().SunDesc[0].location
                } else {
                    cx.planet_outer_location(target as u16 - 1)
                };
            } else {
                if task == ON_STATION as u8 {
                    target = 0;
                }
                destination = POINT {
                    x: cx.ip_group_mut(element).loc.x.wrapping_mul(2),
                    y: cx.ip_group_mut(element).loc.y.wrapping_mul(2),
                };
            }
            let delta_x = destination.x.wrapping_sub(cx.ip_group_mut(element).loc.x);
            let delta_y = destination.y.wrapping_sub(cx.ip_group_mut(element).loc.y);
            let angle = ARCTAN(delta_x, delta_y);
            let wait = cx.element_mut(element).thrust;
            if wait != 0 && wait != u8::MAX {
                cx.element_mut(element).thrust -= 1;
            } else if (vx == 0 && vy == 0) || angle != cx.element_mut(element).velocity.travel_angle
            {
                let speed = if wait != 0 && cx.ip_group_mut(element).dest_loc != 0 {
                    if task == ON_STATION as u8 {
                        30
                    } else {
                        60
                    }
                } else {
                    cx.element_mut(element).thrust = 5;
                    IP_SPEED[cx.ip_group_mut(element).race_id as usize]
                };
                vx = cosine(angle, speed as i32);
                vy = sine(angle, speed as i32);
                cx.element_mut(element)
                    .velocity
                    .set_components(i32::from(vx), i32::from(vy));
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
                } else if target == cx.ip_group_mut(element).dest_loc {
                    partial = true;
                }
            } else if location == 0 {
                adjust_zoom(zoom, &mut dx, &mut dy);
                if task == ON_STATION as u8 && cx.ip_group_mut(element).dest_loc != 0 {
                    partial = true;
                } else {
                    transition = reaches(dx, dy, delta_x, delta_y);
                }
            } else {
                escape = true;
            }
            if partial && reaches(dx, dy, delta_x, delta_y) {
                cx.ip_group_mut(element).loc = destination;
                vx = 0;
                vy = 0;
                cx.element_mut(element).velocity.zero();
            }
            if escape {
                let display = locationToDisplay(cx.ip_group_mut(element).loc, zoom);
                transition = display.x < 0
                    || display.x as i32 >= cx.screen_dimensions().0 - 64 - 14
                    || display.y < 0
                    || display.y as i32 >= cx.screen_dimensions().1 - 13;
            }
            if transition {
                cx.element_mut(element).flags |= NONSOLID as u16;
                vx = 0;
                vy = 0;
                cx.element_mut(element).velocity.zero();
                if location != 0 {
                    cx.ip_group_mut(element).loc = cx.planet_outer_location(location as u16 - 1);
                    location = 0;
                    cx.ip_group_mut(element).sys_loc = 0;
                } else if target == 0 {
                    cx.element_mut(element).life = 0;
                    cx.element_mut(element).flags |= (DISAPPEARING | NONSOLID) as u16;
                    cx.ip_group_mut(element).in_system = 0;
                    return;
                } else {
                    if target == cx.ip_group_mut(element).dest_loc {
                        cx.ip_group_mut(element).orbit_pos =
                            (((angle + HALF_CIRCLE as u16 + 2) >> 2) & 15) as u8;
                        cx.ip_group_mut(element).group_counter =
                            (cx.random() as u16 % MAX_REVOLUTIONS as u16) << FACING_SHIFT;
                    }
                    let width = cx.screen_dimensions().0 - 64 - 14;
                    let height = cx.screen_dimensions().1 - 13;
                    let entry = POINT {
                        x: ((width >> 1) - cosine(angle, width * 9 / 16) as i32) as i16,
                        y: ((height >> 1) - sine(angle, height * 9 / 16) as i32) as i16,
                    };
                    cx.ip_group_mut(element).loc = displayToLocation(entry, MAX_ZOOM);
                    location = target;
                    cx.ip_group_mut(element).sys_loc = target;
                }
            }
        }
        let zoom = radius(cx, location);
        adjust_zoom(zoom, &mut vx, &mut vy);
        cx.ip_group_mut(element).loc.x = cx.ip_group_mut(element).loc.x.wrapping_add(vx);
        cx.ip_group_mut(element).loc.y = cx.ip_group_mut(element).loc.y.wrapping_add(vy);
        cx.element_mut(element).next.location =
            world(cx, locationToDisplay(cx.ip_group(element).loc, zoom));
        if location != flagship
            || (task & REFORM_GROUP as u8 != 0 && cx.ip_group_mut(element).group_counter & 1 != 0)
        {
            cx.primitive_style_mut(element).kind = gfx_object_NUM_PRIMS as u8;
            cx.element_mut(element).flags |= NONSOLID as u16;
        } else {
            cx.primitive_style_mut(element).kind = gfx_object_STAMP_PRIM as u8;
            if task & REFORM_GROUP as u8 != 0 {
                cx.element_mut(element).flags |= NONSOLID as u16;
            }
        }
        cx.element_mut(element).flags |= CHANGING as u16;
    }

    fn begin_group_step(
        cx: &mut crate::engine::battle::Cx<'_>,
        element: crate::engine::battle::ElementId,
    ) {
        cx.element_mut(element).flags &= !((DISAPPEARING | NONSOLID) as u16);
        let life = cx.element(element).life.wrapping_add(1);
        cx.element_mut(element).life = life;
    }

    fn reform_group(
        cx: &mut crate::engine::battle::Cx<'_>,
        element: crate::engine::battle::ElementId,
    ) -> u8 {
        let mut task = cx.ip_group_mut(element).task;
        if task & REFORM_GROUP as u8 != 0 {
            cx.ip_group_mut(element).group_counter =
                cx.ip_group_mut(element).group_counter.wrapping_sub(1);
            if cx.ip_group_mut(element).group_counter == 0 {
                task &= !(REFORM_GROUP as u8);
                cx.ip_group_mut(element).task = task;
                cx.ip_group_mut(element).group_counter =
                    if task & !(IGNORE_FLAGSHIP as u8) != EXPLORE as u8 {
                        0
                    } else {
                        (cx.random() as u16 % MAX_REVOLUTIONS as u16) << FACING_SHIFT
                    };
            }
        }
        task
    }

    fn reaches(dx: i16, dy: i16, target_x: i16, target_y: i16) -> bool {
        (dx.wrapping_mul(dx) as u16 as i64 + dy.wrapping_mul(dy) as u16 as i64)
            >= target_x as i64 * target_x as i64 + target_y as i64 * target_y as i64
    }

    fn collide_flagship(
        cx: &mut crate::engine::battle::Cx<'_>,
        first: crate::engine::battle::ElementId,
        second: crate::engine::battle::ElementId,
    ) {
        if cx.element(second).flags & COLLISION as u16 == 0 {
            cx.element_mut(first).flags |= (COLLISION | NONSOLID) as u16;
        } else {
            cx.element_mut(second).flags &= !(COLLISION as u16);
            cx.navigation_mut().Game_state.CurrentActivity |= START_ENCOUNTER as u16;
        }
    }
    pub(crate) fn flagship_collision(
        cx: &mut crate::engine::battle::Cx<'_>,
        first: crate::engine::battle::ElementId,
        _: &mut POINT,
        second: crate::engine::battle::ElementId,
        _: &mut POINT,
    ) {
        if cx.world.activity() & START_ENCOUNTER as u16 == 0 {
            collide_flagship(cx, first, second);
        }
    }
    pub(crate) fn group_collision(
        cx: &mut crate::engine::battle::Cx<'_>,
        first: crate::engine::battle::ElementId,
        _: &mut POINT,
        second: crate::engine::battle::ElementId,
        _: &mut POINT,
    ) {
        if cx.world.activity() & START_ENCOUNTER as u16 != 0 {
            return;
        }
        if cx.parent(second) != crate::engine::battle::Parent::None {
            let state = cx.element(second);
            if cx.element(first).flags & COLLISION as u16 != 0
                || (state.current.location.x == state.next.location.x
                    && state.current.location.y == state.next.location.y)
            {
                cx.element_mut(first).flags &= !(COLLISION as u16);
            } else {
                cx.element_mut(second).flags |= COLLISION as u16;
                let location = cx.primitive_stamp(first).origin;
                cx.ip_group_mut(first).loc = location;
                let current = cx.element(first).current.location;
                cx.element_mut(first).next.location = current;
                cx.init_intersect_end(first);
            }
        } else {
            let group_id = cx.ip_group(first).group_id;
            *cx.project_mut(std::ptr::NonNull::new(&raw mut EncounterGroup).unwrap()) = group_id;
            cx.ip_group_mut(first).task |= REFORM_GROUP as u8;
            cx.ip_group_mut(first).group_counter = 100;
            let race = cx.ip_group(first).race_id as u16;
            notify_others(cx, race, u8::MAX);
            collide_flagship(cx, first, second);
        }
    }
    fn spawn_group(cx: &mut crate::engine::battle::Cx<'_>, group: std::ptr::NonNull<IP_GROUP>) {
        use crate::engine::battle::views::{Frame, Frames};
        let Some(element) = cx.alloc_element() else {
            return;
        };
        cx.element_mut(element).mass = 1;
        cx.element_mut(element).hits = 1;
        cx.element_mut(element).flags = (CHANGING | FINITE_LIFE | IGNORE_VELOCITY) as u16;
        cx.primitive_style_mut(element).kind = gfx_object_STAMP_PRIM as u8;
        let icon = cx.project(group).melee_icon;
        let frames = std::ptr::NonNull::from(&cx.project(group).melee_icon);
        cx.set_single_frames(
            element,
            Frames::from_native(frames),
            Frame::from_native(icon),
        );
        let frame = cx.absolute_frame(Frame::from_native(icon), 1);
        cx.set_frame(element, false, frame);
        cx.element_mut(element).death = Some(ElementHook::Native(group_preprocess));
        cx.element_mut(element).collide = Some(CollisionHook::Native(group_collision));
        let record = *cx.project(group);
        cx.element_mut(element).current.location = world(
            cx,
            locationToDisplay(record.loc, radius(cx, record.sys_loc)),
        );
        let queue = &cx.navigation().Game_state.ip_group_q;
        let offset = (group.as_ptr() as usize)
            .checked_sub(queue.pq_tab as usize)
            .expect("group belongs to pool");
        let stride = usize::from(queue.object_size);
        assert!(offset.is_multiple_of(stride));
        cx.set_ip_group_parent(
            element,
            (offset / stride).try_into().expect("IP-group slot"),
        );
        cx.setup_element(element);
        let stars = Frame::from_native(
            *cx.project(std::ptr::NonNull::new(&raw mut stars_in_space).unwrap()),
        );
        let frame = cx.retreat_frame(stars);
        cx.set_intersect_frame(element, frame);
        cx.put(element);
    }
    pub(crate) fn flagship_preprocess(
        cx: &mut crate::engine::battle::Cx<'_>,
        element: crate::engine::battle::ElementId,
    ) {
        let wait = cx.element(element).thrust.wrapping_sub(1);
        cx.element_mut(element).thrust = wait;
        if cx.element_mut(element).thrust == 0 {
            if let Some(next) = cx.elements().succ(element) {
                if Some(next) != cx.elements().tail() {
                    cx.unlink(next);
                    cx.put(next);
                }
            }
            cx.element_mut(element).thrust = 42;
        }
        let (vx, vy) = cx.navigation().Game_state.velocity.get_current_components();
        let (mut vx, mut vy) = (vx as i16, vy as i16);
        let flagship = flagship_location(cx);
        let zoom = radius(cx, flagship);
        adjust_zoom(zoom, &mut vx, &mut vy);
        cx.element_mut(element).current.location = world(
            cx,
            locationToDisplay(cx.navigation().Game_state.ip_location, zoom),
        );
        cx.init_intersect_start(element);
        cx.navigation_mut().Game_state.ip_location.x = cx
            .navigation_mut()
            .Game_state
            .ip_location
            .x
            .wrapping_add(vx);
        cx.navigation_mut().Game_state.ip_location.y = cx
            .navigation_mut()
            .Game_state
            .ip_location
            .y
            .wrapping_add(vy);
        let point = locationToDisplay(cx.navigation_mut().Game_state.ip_location, zoom);
        cx.element_mut(element).next.location = world(cx, point);
        cx.navigation_mut().Game_state.ShipStamp.origin = point;
        let frame = crate::engine::battle::views::Frame::from_native(
            cx.navigation().Game_state.ShipStamp.frame,
        );
        cx.set_frame(element, true, frame);
        if cx.element_mut(element).turn == flagship {
            cx.element_mut(element).flags &= !(NONSOLID as u16);
        } else {
            cx.element_mut(element).flags |= NONSOLID as u16;
            cx.element_mut(element).turn = flagship;
        }
        let escape = get("ESCAPE_COUNTER");
        if escape != 0
            && cx.navigation_mut().Game_state.CurrentActivity & START_ENCOUNTER as u16 == 0
        {
            cx.element_mut(element).flags |= NONSOLID as u16;
            set("ESCAPE_COUNTER", escape - 1);
        }
        cx.element_mut(element).flags |= CHANGING as u16;
    }
    fn spawn_flagship(cx: &mut crate::engine::battle::Cx<'_>) {
        use crate::engine::battle::views::{Frame, Frames};
        let Some(element) = cx.alloc_element() else {
            return;
        };
        cx.element_mut(element).hits = 1;
        cx.element_mut(element).mass = 1;
        let location = flagship_location(cx);
        cx.element_mut(element).turn = location;
        cx.element_mut(element).flags = (APPEARING | IGNORE_VELOCITY) as u16;
        if get("ESCAPE_COUNTER") != 0 {
            cx.element_mut(element).flags |= NONSOLID as u16;
        }
        cx.element_mut(element).life = NORMAL_LIFE as u16;
        cx.element_mut(element).thrust = 42;
        cx.primitive_style_mut(element).kind = gfx_object_STAMP_PRIM as u8;
        let frame = cx.navigation().Game_state.ShipStamp.frame;
        let frames = std::ptr::NonNull::from(&cx.navigation().Game_state.ShipStamp.frame);
        cx.set_single_frames(
            element,
            Frames::from_native(frames),
            Frame::from_native(frame),
        );
        cx.set_frame(element, false, Frame::from_native(frame));
        cx.element_mut(element).pre = Some(ElementHook::Native(flagship_preprocess));
        cx.element_mut(element).collide = Some(CollisionHook::Native(flagship_collision));
        cx.element_mut(element).current.location =
            world(cx, cx.navigation().Game_state.ShipStamp.origin);
        cx.put(element);
    }

    /// # Safety
    /// The live solar system and display/group queues must be initialized on the game thread.
    pub unsafe extern "C" fn do_missions() {
        crate::app::bridge::enter(|cx| {
            spawn_flagship(cx);
            if EncounterRace >= 0 {
                notify_others(cx, EncounterRace as u16, 0);
                EncounterRace = -1;
            }
            let mut handle = cx.navigation().Game_state.ip_group_q.head;
            while let Some(group) = std::ptr::NonNull::new(handle.cast::<IP_GROUP>()) {
                handle = cx.project(group).succ;
                if cx.project(group).in_system != 0 {
                    spawn_group(cx, group);
                }
            }
        });
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
