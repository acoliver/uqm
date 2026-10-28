//! Native Rust port of the cyborg battle intelligence in `cyborg.c`.
//!
//! The arithmetic helpers are kept independent of engine state so their C
//! integer-width behavior can be tested without linking the game archive.

use crate::ships::native::{
    abi, angle_to_facing, facing_to_angle, normalize_angle, normalize_facing,
};

#[inline]
fn c_size(v: i32) -> i16 {
    v as i16
}

#[inline]
fn break_opposite_facing_tie(delta: u16, random: impl FnOnce() -> u32) -> u16 {
    let jitter = angle_to_facing((abi::OCTANT >> 1) as i32);
    normalize_facing(i32::from(delta) + if random() & 1 != 0 { jitter } else { -jitter })
}

#[inline]
fn velocity_to_world(v: i64) -> i16 {
    (v >> abi::VELOCITY_SHIFT) as i16
}

/// Return the first turn at which the two swept rectangles can overlap.
///
/// This is the integer-only broad phase from `PlotIntercept`; the engine
/// narrow-phase collision callback is applied by the live implementation.
fn intercept_window(
    p0: (i16, i16),
    v0: (i16, i16),
    p1: (i16, i16),
    v1: (i16, i16),
    max_turns: u16,
    extents: [(i16, i16); 2],
) -> Option<(i16, i16)> {
    let (mut dx0, mut dy0) = v0;
    let (mut dx1, mut dy1) = v1;
    dx0 = velocity_to_world(i64::from(dx0) * i64::from(max_turns));
    dy0 = velocity_to_world(i64::from(dy0) * i64::from(max_turns));
    dx1 = velocity_to_world(i64::from(dx1) * i64::from(max_turns));
    dy1 = velocity_to_world(i64::from(dy1) * i64::from(max_turns));

    let x0 = i32::from(p0.0);
    let y0 = i32::from(p0.1);
    let x1 = i32::from(p1.0);
    let y1 = i32::from(p1.1);
    let mut t0y = i32::from(extents[0].1);
    let mut t1y = i32::from(extents[1].1);
    let diff_y = i32::from(c_size(y1 - y0));
    t0y = i32::from(c_size(diff_y - t0y + 1));
    t1y = i32::from(c_size(diff_y + t1y - 1));
    let mut rel_y = i32::from(c_size(i32::from(dy0) - i32::from(dy1)));

    if !axis_possible(t0y, t1y, rel_y) {
        return None;
    }
    let mut t0x = i32::from(extents[0].0);
    let mut t1x = i32::from(extents[1].0);
    let diff_x = i32::from(c_size(x1 - x0));
    t0x = i32::from(c_size(diff_x - t0x + 1));
    t1x = i32::from(c_size(diff_x + t1x - 1));
    let mut rel_x = i32::from(c_size(i32::from(dx0) - i32::from(dx1)));
    if !axis_possible(t0x, t1x, rel_x) {
        return None;
    }
    if rel_x == 0 && rel_y == 0 {
        return Some((0, 0));
    }

    if t1y < 0 {
        (t0y, t1y) = (i32::from(c_size(-t1y)), i32::from(c_size(-t0y)));
    } else if t0y <= 0 {
        if rel_y < 0 {
            t1y = i32::from(c_size(-t0y));
        }
        t0y = 0;
    }
    if rel_y < 0 {
        rel_y = i32::from(c_size(-rel_y));
    }
    if rel_y < t1y {
        t1y = rel_y;
    }
    if t1x < 0 {
        (t0x, t1x) = (i32::from(c_size(-t1x)), i32::from(c_size(-t0x)));
    } else if t0x <= 0 {
        if rel_x < 0 {
            t1x = i32::from(c_size(-t0x));
        }
        t0x = 0;
    }
    if rel_x < 0 {
        rel_x = i32::from(c_size(-rel_x));
    }
    if rel_x < t1x {
        t1x = rel_x;
    }

    let (begin, end, fract): (i64, i64, i64) = if rel_x == 0 {
        (i64::from(t0y), i64::from(t1y), i64::from(rel_y))
    } else if rel_y == 0 {
        (i64::from(t0x), i64::from(t1x), i64::from(rel_x))
    } else {
        let tx0 = i64::from(t0x) * i64::from(rel_y);
        let ty0 = i64::from(t0y) * i64::from(rel_x);
        let tx1 = i64::from(t1x) * i64::from(rel_y);
        let ty1 = i64::from(t1y) * i64::from(rel_x);
        (
            tx0.max(ty0),
            tx1.min(ty1),
            i64::from(rel_x) * i64::from(rel_y),
        )
    };
    let first = if begin * i64::from(max_turns) < fract {
        0
    } else {
        ((begin * i64::from(max_turns)) / fract) as i32
    } as i16;
    let last = if end >= fract {
        i32::from(max_turns) - 1
    } else {
        ((end * i64::from(max_turns)) / fract) as i32
    } as i16;
    if first <= last {
        Some((first, last))
    } else {
        None
    }
}

#[inline]
fn axis_possible(t0: i32, t1: i32, velocity: i32) -> bool {
    (t0 <= 0 && t1 >= 0) || (t0 > 0 && velocity >= t0) || (t1 < 0 && velocity <= t1)
}

#[cfg(not(test))]
#[allow(non_snake_case)]
pub(crate) mod engine {
    use super::*;
    use crate::battle::process_native::dispatch::battle_input;
    use crate::battle::process_native::invoke_race::{invoke_intelligence, invoke_weapon};
    use crate::collections::observation::image;
    use crate::ships::native::abi::*;
    const CLOSE_RANGE_WEAPON: u16 = (50 << ONE_SHIFT) as u16;
    const LONG_RANGE_WEAPON: u16 = (1000 << ONE_SHIFT) as u16;
    const FAST_SHIP: u16 = 150;

    use crate::battle::world_elements::engine::crew_preprocess;

    unsafe fn validate_element_handle(handle: HELEMENT) {
        if !handle.is_null() {
            debug_assert!(
                !disp_q.pq_tab.is_null()
                    && handle as usize >= disp_q.pq_tab as usize
                    && (handle as usize)
                        < disp_q.pq_tab as usize
                            + usize::from(disp_q.object_size) * usize::from(disp_q.num_objects)
            );
        }
    }

    unsafe fn LockElement(handle: HELEMENT, element: *mut *mut ELEMENT) {
        validate_element_handle(handle);
        *element = handle.cast();
    }

    unsafe fn UnlockElement(handle: HELEMENT) {
        validate_element_handle(handle);
    }

    unsafe fn GetHeadElement() -> HELEMENT {
        disp_q.head
    }

    unsafe fn GetSuccElement(element: *mut ELEMENT) -> HELEMENT {
        (*element).succ
    }

    unsafe fn CollisionPossible(element: *mut ELEMENT, ship: *mut ELEMENT) -> bool {
        (*element).state_flags & (NONSOLID | DISAPPEARING) as u16 == 0
            && ((*ship).state_flags & (*element).state_flags) & COLLISION as u16 == 0
            && (((*ship).state_flags & (*element).state_flags) & IGNORE_SIMILAR as u16 == 0
                || (*ship).pParent != (*element).pParent)
            && ((*ship).__bindgen_anon_3.mass_points != 0
                || (*element).__bindgen_anon_3.mass_points != 0)
    }

    unsafe fn elementsOfSamePlayer(a: *mut ELEMENT, b: *mut ELEMENT) -> bool {
        (*a).playerNr == (*b).playerNr
    }

    unsafe fn IsVelocityZero(velocity: *mut VELOCITY_DESC) -> bool {
        let v = &*velocity;
        v.vector.width == 0
            && v.vector.height == 0
            && v.incr.width == 0
            && v.incr.height == 0
            && v.fract.width == 0
            && v.fract.height == 0
    }

    unsafe fn GetVelocityTravelAngle(velocity: *mut VELOCITY_DESC) -> COUNT {
        (*velocity).travel_angle
    }

    unsafe fn object_cloaked(element: *mut ELEMENT) -> bool {
        let primitive = &*GlobData
            .Game_state
            .DisplayArray
            .add((*element).PrimIndex as usize);
        u32::from(primitive.Type) >= gfx_object_NUM_PRIMS
            || (u32::from(primitive.Type) == gfx_object_STAMPFILL_PRIM
                && primitive.color.r == 0
                && primitive.color.g == 0
                && primitive.color.b == 0
                && primitive.color.a == 255)
    }

    unsafe fn is_crew_preprocess(element: *mut ELEMENT) -> bool {
        (*element)
            .preprocess_func
            .is_some_and(|f| f.address() == crew_preprocess as *const () as usize)
    }

    unsafe fn InitIntersectStartPoint(element: *mut ELEMENT) {
        (*element).IntersectControl.IntersectStamp.origin.x =
            (*element).current.location.x >> ONE_SHIFT;
        (*element).IntersectControl.IntersectStamp.origin.y =
            (*element).current.location.y >> ONE_SHIFT;
    }

    unsafe fn InitIntersectEndPoint(element: *mut ELEMENT) {
        (*element).IntersectControl.EndPoint.x = (*element).next.location.x >> ONE_SHIFT;
        (*element).IntersectControl.EndPoint.y = (*element).next.location.y >> ONE_SHIFT;
    }

    unsafe fn InitIntersectFrame(element: *mut ELEMENT) {
        image!(frame; (*element).IntersectControl.IntersectStamp.frame, SetEquFrameIndex(*(*element).next.image.farray, (*element).next.image.frame));
    }

    unsafe fn GetElementStarShip(element: *mut ELEMENT, ship: *mut *mut STARSHIP) {
        *ship = (*element).pParent.cast();
    }
    /// # Safety
    /// Pointers must reference live engine records and their initialized dependent
    /// records. The caller must serialize access to battle and graphics state.
    #[no_mangle]
    pub unsafe extern "C" fn PlotIntercept(
        a: *mut ELEMENT,
        b: *mut ELEMENT,
        mut turns: COUNT,
        margin: COUNT,
    ) -> COUNT {
        if ((*a).state_flags | (*b).state_flags) & FINITE_LIFE as u16 != 0 {
            if (*a).state_flags & FINITE_LIFE as u16 == 0 {
                turns = turns.min((*b).__bindgen_anon_1.life_span);
            } else if (*b).state_flags & FINITE_LIFE as u16 == 0 {
                turns = turns.min((*a).__bindgen_anon_1.life_span);
            } else {
                turns = turns
                    .min((*a).__bindgen_anon_1.life_span)
                    .min((*b).__bindgen_anon_1.life_span);
            }
        }
        let mut avx = 0;
        let mut avy = 0;
        let mut bvx = 0;
        let mut bvy = 0;
        GetCurrentVelocityComponents(&mut (*a).velocity, &mut avx, &mut avy);
        GetCurrentVelocityComponents(&mut (*b).velocity, &mut bvx, &mut bvy);
        let mut ar = RECT {
            corner: POINT { x: 0, y: 0 },
            extent: EXTENT {
                width: 0,
                height: 0,
            },
        };
        let mut br = ar;
        let (ha, hb, pos_a, pos_b) = if margin != 0 {
            let m = i32::from(margin);
            (
                (1, 1),
                ((m << 1) as i16, (m << 1) as i16),
                ((*a).current.location.x, (*a).current.location.y),
                (
                    (*b).current.location.x.wrapping_sub(m as i16),
                    (*b).current.location.y.wrapping_sub(m as i16),
                ),
            )
        } else {
            GetFrameRect((*a).IntersectControl.IntersectStamp.frame, &mut ar);
            GetFrameRect((*b).IntersectControl.IntersectStamp.frame, &mut br);
            let ax = (ar.corner.x as i32).wrapping_shl(ONE_SHIFT) as i16;
            let ay = (ar.corner.y as i32).wrapping_shl(ONE_SHIFT) as i16;
            let bx = (br.corner.x as i32).wrapping_shl(ONE_SHIFT) as i16;
            let by = (br.corner.y as i32).wrapping_shl(ONE_SHIFT) as i16;
            (
                (
                    (i32::from(ar.extent.width) << ONE_SHIFT) as i16,
                    (i32::from(ar.extent.height) << ONE_SHIFT) as i16,
                ),
                (
                    (i32::from(br.extent.width) << ONE_SHIFT) as i16,
                    (i32::from(br.extent.height) << ONE_SHIFT) as i16,
                ),
                (
                    (*a).current.location.x.wrapping_add(ax),
                    (*a).current.location.y.wrapping_add(ay),
                ),
                (
                    (*b).current.location.x.wrapping_add(bx),
                    (*b).current.location.y.wrapping_add(by),
                ),
            )
        };
        let Some((first, last)) =
            intercept_window(pos_a, (avx, avy), pos_b, (bvx, bvy), turns, [ha, hb])
        else {
            return 0;
        };
        if margin != 0 {
            return (first as u16).wrapping_add(1);
        }

        let mut ca = (*a).IntersectControl;
        let mut cb = (*b).IntersectControl;
        let mut pa = (*a).current.location;
        let mut pb = (*b).current.location;
        let mut va = (*a).velocity;
        let mut vb = (*b).velocity;
        let mut x = 0;
        let mut y = 0;
        if first != 0 {
            GetNextVelocityComponents(&mut va, &mut x, &mut y, first as u16);
            pa.x = pa.x.wrapping_add(x);
            pa.y = pa.y.wrapping_add(y);
            ca.EndPoint.x = pa.x >> ONE_SHIFT;
            ca.EndPoint.y = pa.y >> ONE_SHIFT;
            GetNextVelocityComponents(&mut vb, &mut x, &mut y, first as u16);
            pb.x = pb.x.wrapping_add(x);
            pb.y = pb.y.wrapping_add(y);
            cb.EndPoint.x = pb.x >> ONE_SHIFT;
            cb.EndPoint.y = pb.y >> ONE_SHIFT;
        }
        let mut turn = first;
        loop {
            turn = turn.wrapping_add(1);
            GetNextVelocityComponents(&mut va, &mut x, &mut y, 1);
            pa.x = pa.x.wrapping_add(x);
            pa.y = pa.y.wrapping_add(y);
            GetNextVelocityComponents(&mut vb, &mut x, &mut y, 1);
            pb.x = pb.x.wrapping_add(x);
            pb.y = pb.y.wrapping_add(y);
            ca.IntersectStamp.origin = ca.EndPoint;
            ca.EndPoint.x = pa.x >> ONE_SHIFT;
            ca.EndPoint.y = pa.y >> ONE_SHIFT;
            cb.IntersectStamp.origin = cb.EndPoint;
            cb.EndPoint.x = pb.x >> ONE_SHIFT;
            cb.EndPoint.y = pb.y >> ONE_SHIFT;
            let hit = DrawablesIntersect(&mut ca, &mut cb, MAX_TIME_VALUE as u16);
            if hit != 0 {
                if hit == 1
                    && turn == 1
                    && ((*a).state_flags | (*b).state_flags) & APPEARING as u16 != 0
                {
                    ca.EndPoint.x = pa.x >> ONE_SHIFT;
                    ca.EndPoint.y = pa.y >> ONE_SHIFT;
                    cb.EndPoint.x = pb.x >> ONE_SHIFT;
                    cb.EndPoint.y = pb.y >> ONE_SHIFT;
                } else {
                    return turn as u16;
                }
            }
            if turn >= last {
                break;
            }
        }
        0
    }

    /// # Safety
    /// Pointers must reference live engine records and their initialized dependent
    /// records. The caller must serialize access to battle and graphics state.
    #[no_mangle]
    pub unsafe extern "C" fn TurnShip(ship: *mut ELEMENT, angle: COUNT) -> BOOLEAN {
        let mut starship = std::ptr::null_mut();
        GetElementStarShip(ship, &mut starship);
        let facing = (*starship).ShipFacing;
        let mut delta = normalize_facing(angle_to_facing(i32::from(angle)) - i32::from(facing));
        if delta == 0 {
            return 0;
        }
        if delta == angle_to_facing(HALF_CIRCLE as i32) as u16 {
            delta = break_opposite_facing_tie(delta, TFB_Random);
        }
        let mut f = i32::from(facing);
        if delta < angle_to_facing(HALF_CIRCLE as i32) as u16 {
            (*starship).ship_input_state |= RIGHT as u8;
            f += 1;
            image!(frame; (*ship).next.image.frame, IncFrameIndex((*ship).current.image.frame));
        } else {
            (*starship).ship_input_state |= LEFT as u8;
            f -= 1;
            image!(frame; (*ship).next.image.frame, DecFrameIndex((*ship).current.image.frame));
        }
        (*starship).ShipFacing = normalize_facing(f);
        1
    }

    /// # Safety
    /// Pointers must reference live engine records and their initialized dependent
    /// records. The caller must serialize access to battle and graphics state.
    #[no_mangle]
    pub unsafe extern "C" fn ThrustShip(ship: *mut ELEMENT, angle: COUNT) -> BOOLEAN {
        let mut starship = std::ptr::null_mut();
        GetElementStarShip(ship, &mut starship);
        let mut should = (*starship).ship_input_state & THRUST as u8 != 0;
        if !should {
            let travel = GetVelocityTravelAngle(&mut (*ship).velocity);
            let at_cap = (*starship).cur_status_flags
                & (SHIP_AT_MAX_SPEED | SHIP_BEYOND_MAX_SPEED) as u16
                != 0;
            if normalize_facing(
                angle_to_facing(i32::from(angle)) - angle_to_facing(i32::from(travel)),
            ) == 0
                && at_cap
                && (*starship).cur_status_flags & SHIP_IN_GRAVITY_WELL as u16 == 0
            {
                should = false;
            } else {
                let d = normalize_facing(
                    angle_to_facing(i32::from(angle)) - i32::from((*starship).ShipFacing)
                        + angle_to_facing(QUADRANT as i32),
                );
                should = d == angle_to_facing(QUADRANT as i32) as u16
                    || ((*starship).cur_status_flags & SHIP_BEYOND_MAX_SPEED as u16 != 0
                        && d <= angle_to_facing(HALF_CIRCLE as i32) as u16);
            }
        }
        if should {
            inertial_thrust(ship);
            (*starship).ship_input_state |= THRUST as u8;
        }
        should as BOOLEAN
    }

    /// # Safety
    /// Pointers must reference live engine records and their initialized dependent
    /// records. The caller must serialize access to battle and graphics state.
    #[no_mangle]
    pub unsafe extern "C" fn ship_weapons(
        ship: *mut ELEMENT,
        other: *mut ELEMENT,
        mut margin: COUNT,
    ) -> BOOLEAN {
        if object_cloaked(other) {
            margin = margin.wrapping_add((40 << ONE_SHIFT) as u16);
        }
        let mut moved = *ship;
        let mut dx = 0;
        let mut dy = 0;
        GetNextVelocityComponents(&mut moved.velocity, &mut dx, &mut dy, 1);
        moved.next.location.x = moved.current.location.x.wrapping_add(dx);
        moved.next.location.y = moved.current.location.y.wrapping_add(dy);
        moved.current.location = moved.next.location;
        let mut owner = std::ptr::null_mut();
        GetElementStarShip(&mut moved, &mut owner);
        let mut weapons = [std::ptr::null_mut(); 6];
        let count = invoke_weapon(owner as usize, &mut moved, weapons.as_mut_ptr());
        let mut remaining = count;
        let mut index = 0usize;
        while remaining != 0 {
            let handle = weapons[index];
            index += 1;
            remaining -= 1;
            if handle.is_null() {
                continue;
            }
            let mut weapon = std::ptr::null_mut();
            LockElement(handle, &mut weapon);
            if (*weapon).state_flags & APPEARING as u16 != 0 {
                image!(copy; (*weapon).next, (*weapon).current);
                InitIntersectStartPoint(weapon);
                InitIntersectEndPoint(weapon);
                InitIntersectFrame(weapon);
            }
            let hit =
                PlotIntercept(weapon, other, (*weapon).__bindgen_anon_1.life_span, margin) != 0;
            UnlockElement(handle);
            if hit {
                FreeElement(handle);
                while remaining != 0 {
                    let unused = weapons[index];
                    index += 1;
                    remaining -= 1;
                    if !unused.is_null() {
                        FreeElement(unused);
                    }
                }
                return 1;
            }
            FreeElement(handle);
        }
        0
    }

    /// # Safety
    /// Pointers must reference live engine records and their initialized dependent
    /// records. The caller must serialize access to battle and graphics state.
    #[no_mangle]
    pub unsafe extern "C" fn ship_intelligence(
        ship: *mut ELEMENT,
        concerns: *mut EVALUATE_DESC,
        count: COUNT,
    ) {
        let mut owner = std::ptr::null_mut();
        GetElementStarShip(ship, &mut owner);
        let mut ship_moved =
            (*ship).__bindgen_anon_4.turn_wait != 0 && (*ship).__bindgen_anon_5.thrust_wait != 0;
        let mut ship_fired = true;
        if (*owner).weapon_counter == 0 {
            (*owner).ship_input_state &= !(WEAPON as u8);
            if (*(*owner).RaceDescPtr).ship_info.ship_flags & SEEKING_WEAPON as u16 == 0 {
                ship_fired = false;
            }
        }
        let margin = if (*owner).control & AWESOME_RATING != 0 {
            0
        } else if (*owner).control & GOOD_RATING != 0 {
            (20 << ONE_SHIFT) as u16
        } else {
            (40 << ONE_SHIFT) as u16
        };
        let mut i = count;
        let mut current = concerns.add(count as usize);
        while i != 0 {
            i -= 1;
            current = current.sub(1);
            let item = &mut *current;
            if item.ObjectPtr.is_null() {
                continue;
            }
            if !ship_moved
                && (i != ENEMY_WEAPON_INDEX as u16
                    || item.MoveState == MOVEMENT_STATE_PURSUE
                    || (*item.ObjectPtr).state_flags & CREW_OBJECT as u16 != 0
                    || (*(*owner).RaceDescPtr).cyborg_control.ManeuverabilityIndex
                        >= MEDIUM_SHIP as u16)
            {
                if item.which_turn == 0 {
                    item.which_turn = 1;
                }
                match item.MoveState {
                    MOVEMENT_STATE_PURSUE => Pursue(ship, item),
                    MOVEMENT_STATE_AVOID | MOVEMENT_STATE_ENTICE => Entice(ship, item),
                    _ => {}
                }
                ship_moved = true;
            }
            if !ship_fired
                && (i == ENEMY_SHIP_INDEX as u16
                    || (i == ENEMY_WEAPON_INDEX as u16 && item.MoveState != MOVEMENT_STATE_AVOID))
            {
                ship_fired = ship_weapons(ship, item.ObjectPtr, margin) != 0;
                if ship_fired {
                    (*owner).ship_input_state |= WEAPON as u8;
                }
            }
        }
    }

    /// # Safety
    /// Pointers must reference live engine records and their initialized dependent
    /// records. The caller must serialize access to battle and graphics state.
    #[no_mangle]
    pub unsafe extern "C" fn Pursue(ship: *mut ELEMENT, eval: *mut EVALUATE_DESC) {
        let other = (*eval).ObjectPtr;
        let mut ship_velocity = (*ship).velocity;
        let mut other_velocity = (*other).velocity;
        let mut sx = 0i16;
        let mut sy = 0i16;
        let mut ox = 0i16;
        let mut oy = 0i16;
        GetNextVelocityComponents(&mut ship_velocity, &mut sx, &mut sy, (*eval).which_turn);
        (*ship).next.location.x = (*ship).current.location.x.wrapping_add(sx);
        (*ship).next.location.y = (*ship).current.location.y.wrapping_add(sy);
        GetNextVelocityComponents(&mut other_velocity, &mut ox, &mut oy, (*eval).which_turn);
        let mut dx = ((*other).current.location.x as i32 + ox as i32
            - (*ship).next.location.x as i32) as i16;
        let mut dy = ((*other).current.location.y as i32 + oy as i32
            - (*ship).next.location.y as i32) as i16;
        dx =
            crate::ships::native::wrap_delta(dx, (ScreenWidth - 64) << (ONE_SHIFT + MAX_REDUCTION));
        dy = crate::ships::native::wrap_delta(dy, ScreenHeight << (ONE_SHIFT + MAX_REDUCTION));
        let mut thrust_angle = ARCTAN(dx, dy);
        let mut maneuver: u8 = 0;
        if (*ship).__bindgen_anon_4.turn_wait == 0 {
            maneuver |= (LEFT | RIGHT) as u8;
        }
        if (*ship).__bindgen_anon_5.thrust_wait == 0
            && ((*other).state_flags & PLAYER_SHIP as u16 != 0
                || elementsOfSamePlayer(other, ship)
                || is_crew_preprocess(other))
        {
            maneuver |= THRUST as u8;
        }
        let turn_angle = normalize_angle(thrust_angle as i32 + HALF_CIRCLE as i32) as u16;
        if (*other).state_flags & PLAYER_SHIP as u16 != 0
            && (*other).__bindgen_anon_3.mass_points <= MAX_SHIP_MASS as u8
        {
            let mut us = std::ptr::null_mut();
            GetElementStarShip(ship, &mut us);
            let mut enemy = std::ptr::null_mut();
            GetElementStarShip(other, &mut enemy);
            let ur = (*us).RaceDescPtr;
            let er = (*enemy).RaceDescPtr;
            if ((*ur).cyborg_control.ManeuverabilityIndex >= FAST_SHIP
                && (*ur).cyborg_control.WeaponRange > CLOSE_RANGE_WEAPON)
                || ((*eval).which_turn >= 24
                    && (((*ur).characteristics.max_thrust as u32 * 2 / 3)
                        < (*er).characteristics.max_thrust as u32
                        || (*enemy).cur_status_flags & SHIP_BEYOND_MAX_SPEED as u16 != 0))
            {
                let flags = (*er).ship_info.ship_flags;
                if (*ur).cyborg_control.ManeuverabilityIndex >= MEDIUM_SHIP as u16 {
                    let mut fire = FIRES_FORE as u16;
                    let mut facing = (*eval).facing;
                    while fire <= FIRES_LEFT as u16 {
                        if flags & fire != 0
                            && normalize_angle(turn_angle as i32 - facing as i32 + OCTANT as i32)
                                <= QUADRANT as i32
                            && (ox != 0 || oy != 0)
                            && normalize_angle(
                                GetVelocityTravelAngle(&mut other_velocity) as i32
                                    + HALF_CIRCLE as i32
                                    - facing as i32
                                    + OCTANT as i32
                                    + (OCTANT >> 1) as i32,
                            ) <= ((OCTANT + (OCTANT >> 1)) << 1) as i32
                        {
                            thrust_angle = turn_angle;
                            break;
                        }
                        fire <<= 1;
                        facing = facing.wrapping_add(QUADRANT as u16);
                    }
                }
                if thrust_angle != turn_angle
                    && (ox != 0 || oy != 0)
                    && (*eval).which_turn >= 24
                    && normalize_angle(
                        thrust_angle as i32 - GetVelocityTravelAngle(&mut other_velocity) as i32
                            + OCTANT as i32,
                    ) <= QUADRANT as i32
                    && ((normalize_angle(
                        GetVelocityTravelAngle(&mut other_velocity) as i32
                            - GetVelocityTravelAngle(&mut ship_velocity) as i32
                            + OCTANT as i32,
                    ) <= QUADRANT as i32
                        && (((*us).cur_status_flags & SHIP_AT_MAX_SPEED as u16 != 0
                            && (*us).cur_status_flags & SHIP_BEYOND_MAX_SPEED as u16 == 0)
                            || flags & DONT_CHASE as u16 != 0))
                        || normalize_angle(
                            turn_angle as i32 - facing_to_angle((*us).ShipFacing as i32)
                                + OCTANT as i32,
                        ) <= QUADRANT as i32)
                {
                    thrust_angle = turn_angle;
                }
            }
        }
        if maneuver & (LEFT | RIGHT) as u8 != 0 {
            TurnShip(ship, thrust_angle);
        }
        if maneuver & THRUST as u8 != 0 {
            ThrustShip(ship, thrust_angle);
        }
    }

    /// # Safety
    /// Pointers must reference live engine records and their initialized dependent
    /// records. The caller must serialize access to battle and graphics state.
    #[no_mangle]
    pub unsafe extern "C" fn Entice(ship: *mut ELEMENT, eval: *mut EVALUATE_DESC) {
        let other = (*eval).ObjectPtr;
        let mut ship_velocity = (*ship).velocity;
        let mut other_velocity = (*other).velocity;
        let (mut sx, mut sy, mut ox, mut oy) = (0i16, 0i16, 0i16, 0i16);
        GetNextVelocityComponents(&mut ship_velocity, &mut sx, &mut sy, (*eval).which_turn);
        (*ship).next.location.x = (*ship).current.location.x.wrapping_add(sx);
        (*ship).next.location.y = (*ship).current.location.y.wrapping_add(sy);
        GetNextVelocityComponents(&mut other_velocity, &mut ox, &mut oy, (*eval).which_turn);
        let mut dx = ((*other).current.location.x as i32 + ox as i32
            - (*ship).next.location.x as i32) as i16;
        let mut dy = ((*other).current.location.y as i32 + oy as i32
            - (*ship).next.location.y as i32) as i16;
        dx =
            crate::ships::native::wrap_delta(dx, (ScreenWidth - 64) << (ONE_SHIFT + MAX_REDUCTION));
        dy = crate::ships::native::wrap_delta(dy, ScreenHeight << (ONE_SHIFT + MAX_REDUCTION));
        let mut thrust_angle = ARCTAN(dx, dy);
        let mut maneuver = 0u8;
        if (*ship).__bindgen_anon_4.turn_wait == 0 {
            maneuver |= (LEFT | RIGHT) as u8;
        }
        if (*ship).__bindgen_anon_5.thrust_wait == 0 {
            maneuver |= THRUST as u8;
        }
        let travel = ARCTAN(sx.wrapping_sub(ox), sy.wrapping_sub(oy));
        let mut turn_angle = normalize_angle(thrust_angle as i32 + HALF_CIRCLE as i32) as u16;
        let mut owner = std::ptr::null_mut();
        GetElementStarShip(ship, &mut owner);
        let rd = (*owner).RaceDescPtr;

        if (*eval).MoveState == MOVEMENT_STATE_AVOID {
            turn_angle = normalize_angle(turn_angle as i32 - (*eval).facing as i32) as u16;
            let direction = if normalize_facing(angle_to_facing(turn_angle as i32)) != 0 {
                if turn_angle <= HALF_CIRCLE as u16 {
                    RIGHT as u16
                } else {
                    LEFT as u16
                }
            } else {
                turn_angle = normalize_angle(
                    facing_to_angle((*owner).ShipFacing as i32) - (*eval).facing as i32,
                ) as u16;
                if turn_angle & (HALF_CIRCLE as u16 - 1) == 0 {
                    if TFB_Random() & 1 != 0 {
                        RIGHT as u16
                    } else {
                        LEFT as u16
                    }
                } else if turn_angle < HALF_CIRCLE as u16 {
                    RIGHT as u16
                } else {
                    LEFT as u16
                }
            };
            thrust_angle = if direction == LEFT as u16 {
                (*eval)
                    .facing
                    .wrapping_sub(QUADRANT as u16)
                    .wrapping_add(OCTANT as u16 / 2)
            } else {
                (*eval)
                    .facing
                    .wrapping_add(QUADRANT as u16)
                    .wrapping_sub(OCTANT as u16 / 2)
            };
            thrust_angle = normalize_angle(thrust_angle as i32) as u16;
        } else if (*other).__bindgen_anon_3.mass_points as u16 > (MAX_SHIP_MASS * 10) as u16 {
            let planet_facing = normalize_facing(angle_to_facing(thrust_angle as i32));
            let cone = normalize_facing(
                i32::from(planet_facing) - i32::from((*owner).ShipFacing)
                    + angle_to_facing(QUADRANT as i32),
            );
            if (*rd).characteristics.thrust_increment != (*rd).characteristics.max_thrust {
                maneuver &= !(THRUST as u8);
            }
            if cone > angle_to_facing((QUADRANT << 1) as i32) as u16 {
                turn_angle = thrust_angle;
            } else if cone == angle_to_facing(QUADRANT as i32) as u16
                && normalize_facing(angle_to_facing(travel as i32)) != planet_facing
            {
                turn_angle = travel;
            } else if cone == 0
                || cone == angle_to_facing((QUADRANT << 1) as i32) as u16
                || (maneuver & THRUST as u8 == 0
                    && (cone < angle_to_facing(OCTANT as i32) as u16
                        || cone > angle_to_facing(((QUADRANT << 1) - OCTANT) as i32) as u16))
            {
                turn_angle = facing_to_angle((*owner).ShipFacing as i32) as u16;
                if normalize_angle(turn_angle as i32 - travel as i32 + QUADRANT as i32)
                    > HALF_CIRCLE as i32
                {
                    turn_angle = travel;
                }
                if (*ship).__bindgen_anon_5.thrust_wait == 0 {
                    maneuver |= THRUST as u8;
                }
            }
            thrust_angle = turn_angle;
        } else {
            let wrange = (*rd).cyborg_control.WeaponRange;
            let mut cone =
                normalize_angle(turn_angle as i32 - (*eval).facing as i32 + OCTANT as i32) as u16;
            if (*other).state_flags & PLAYER_SHIP as u16 != 0 {
                let mut enemy = std::ptr::null_mut();
                GetElementStarShip(other, &mut enemy);
                let flags = (*(*enemy).RaceDescPtr).ship_info.ship_flags;
                let mut fire = FIRES_FORE as u16;
                let mut facing = (*eval).facing;
                let mut go_maneuver = false;
                while fire <= FIRES_LEFT as u16 {
                    if flags & fire != 0 {
                        cone = normalize_angle(turn_angle as i32 - facing as i32 + OCTANT as i32)
                            as u16;
                        if cone <= QUADRANT as u16
                            && (ox != 0 || oy != 0)
                            && normalize_angle(
                                GetVelocityTravelAngle(&mut other_velocity) as i32
                                    + HALF_CIRCLE as i32
                                    - facing as i32
                                    + OCTANT as i32,
                            ) <= QUADRANT as i32
                        {
                            if wrange < (1000 << ONE_SHIFT) as u16 && (*eval).which_turn <= 32 {
                                thrust_angle = turn_angle;
                                go_maneuver = true;
                                break;
                            }
                            break;
                        }
                    }
                    fire <<= 1;
                    facing = facing.wrapping_add(QUADRANT as u16);
                }
                if go_maneuver {
                    if maneuver & (LEFT | RIGHT) as u8 != 0 {
                        TurnShip(ship, thrust_angle);
                    }
                    if maneuver & THRUST as u8 != 0 {
                        ThrustShip(ship, thrust_angle);
                    }
                    return;
                }
                if (*eval).which_turn <= 8
                    && (*rd).characteristics.max_thrust
                        <= (*(*enemy).RaceDescPtr).characteristics.max_thrust
                {
                    if maneuver & (LEFT | RIGHT) as u8 != 0 {
                        TurnShip(ship, thrust_angle);
                    }
                    if maneuver & THRUST as u8 != 0 {
                        ThrustShip(ship, thrust_angle);
                    }
                    return;
                }
            }
            if (*owner).cur_status_flags & (SHIP_AT_MAX_SPEED | SHIP_BEYOND_MAX_SPEED) as u16 == 0
                && PlotIntercept(ship, other, 40, CLOSE_RANGE_WEAPON << 1) != 0
            {
                if normalize_angle(
                    thrust_angle as i32 - facing_to_angle((*owner).ShipFacing as i32)
                        + OCTANT as i32,
                ) <= QUADRANT as i32
                    || cone > QUADRANT as u16
                {
                    thrust_angle = turn_angle;
                } else {
                    thrust_angle = facing_to_angle((*owner).ShipFacing as i32) as u16;
                }
            } else {
                cone = PlotIntercept(ship, other, 10, wrange.wrapping_sub(wrange >> 2));
                if cone != 0 {
                    if (*rd).characteristics.thrust_increment != (*rd).characteristics.max_thrust
                        && (*owner).cur_status_flags
                            & (SHIP_AT_MAX_SPEED | SHIP_BEYOND_MAX_SPEED) as u16
                            != 0
                        && (normalize_angle(
                            turn_angle as i32 - ARCTAN(sx, sy) as i32 + OCTANT as i32 + 2,
                        ) <= ((OCTANT + 2) << 1) as i32
                            || PlotIntercept(ship, other, 30, CLOSE_RANGE_WEAPON << 1) == 0)
                    {
                        maneuver &= !(THRUST as u8);
                    } else if cone == 1
                        || (*rd).characteristics.thrust_increment
                            != (*rd).characteristics.max_thrust
                    {
                        if maneuver & (LEFT | RIGHT) as u8 != 0 {
                            TurnShip(ship, turn_angle);
                            maneuver &= !((LEFT | RIGHT) as u8);
                        }
                        if normalize_angle(
                            thrust_angle as i32 - ARCTAN(sx, sy) as i32 + OCTANT as i32 + 2,
                        ) <= ((OCTANT + 2) << 1) as i32
                        {
                            thrust_angle = facing_to_angle((*owner).ShipFacing as i32) as u16;
                        } else {
                            thrust_angle = turn_angle;
                        }
                    }
                }
            }
        }
        if maneuver & (LEFT | RIGHT) as u8 != 0 {
            TurnShip(ship, thrust_angle);
        }
        if maneuver & THRUST as u8 != 0 {
            ThrustShip(ship, thrust_angle);
        }
    }

    /// C retains this entry point as a no-op; AVOID dispatches to Entice.
    /// # Safety
    /// Pointers must reference live engine records and their initialized dependent
    /// records. The caller must serialize access to battle and graphics state.
    #[no_mangle]
    pub unsafe extern "C" fn Avoid(ship: *mut ELEMENT, eval: *mut EVALUATE_DESC) {
        let _ = (ship, eval);
    }

    unsafe fn init_cyborg(ship: *mut STARSHIP) {
        let race = (*ship).RaceDescPtr;
        let characteristics = (*race).characteristics;
        let product = (u32::from(characteristics.max_thrust)
            * u32::from(characteristics.thrust_increment)) as u16;
        let divisor =
            (u32::from(characteristics.turn_wait) + u32::from(characteristics.thrust_wait)) as u16;
        (*race).cyborg_control.ManeuverabilityIndex =
            product.checked_div(divisor).unwrap_or(product >> 1);
    }

    /// # Safety
    /// Pointers must reference live engine records and their initialized dependent
    /// records. The caller must serialize access to battle and graphics state.
    #[no_mangle]
    pub unsafe extern "C" fn tactical_intelligence(
        _context: *mut ComputerInputContext,
        owner: *mut STARSHIP,
    ) -> BATTLE_INPUT_STATE {
        let rd = (*owner).RaceDescPtr;
        if (*rd).cyborg_control.ManeuverabilityIndex == 0 {
            init_cyborg(owner);
        }
        let handle = (*owner).hShip;
        let mut ship = std::ptr::null_mut();
        LockElement(handle, &mut ship);
        if (*rd).ship_info.crew_level == 0
            || u32::from(DisplayArray[(*ship).PrimIndex as usize].Type) == gfx_object_NUM_PRIMS
        {
            UnlockElement(handle);
            return 0;
        }
        let mut ship_copy = *ship;
        if (*owner).control & STANDARD_RATING != 0 {
            (*owner).special_counter = (*owner).special_counter.wrapping_add(1);
        }
        UnlockElement(handle);
        let original_facing = (*owner).ShipFacing;
        let mut concerns: [EVALUATE_DESC; 10] = std::mem::zeroed();
        for concern in &mut concerns[..=FIRST_EMPTY_INDEX as usize] {
            concern.ObjectPtr = std::ptr::null_mut();
            concern.MoveState = MOVEMENT_STATE_NO_MOVEMENT;
            concern.which_turn = u16::MAX;
        }
        let ultra = (*rd).characteristics.thrust_increment == (*rd).characteristics.max_thrust
            && (*rd).cyborg_control.ManeuverabilityIndex >= MEDIUM_SHIP as u16;
        let mut moved = true;
        if ship_copy.__bindgen_anon_4.turn_wait == 0 {
            moved = false;
            (*owner).ship_input_state &= !((LEFT | RIGHT) as u8);
        }
        if ship_copy.__bindgen_anon_5.thrust_wait == 0 {
            moved = false;
            (*owner).ship_input_state &= !(THRUST as u8);
        }

        let mut h = GetHeadElement();
        while !h.is_null() {
            let mut e = std::ptr::null_mut();
            LockElement(h, &mut e);
            let next = GetSuccElement(e);
            if CollisionPossible(e, &mut ship_copy) {
                let mut dx = e
                    .as_ref()
                    .unwrap()
                    .next
                    .location
                    .x
                    .wrapping_sub(ship_copy.next.location.x);
                let mut dy = e
                    .as_ref()
                    .unwrap()
                    .next
                    .location
                    .y
                    .wrapping_sub(ship_copy.next.location.y);
                dx = crate::ships::native::wrap_delta(
                    dx,
                    (ScreenWidth - 64) << (ONE_SHIFT + MAX_REDUCTION),
                );
                dy = crate::ships::native::wrap_delta(
                    dy,
                    ScreenHeight << (ONE_SHIFT + MAX_REDUCTION),
                );
                let mass = (*e).__bindgen_anon_3.mass_points;
                if mass as u16 > (MAX_SHIP_MASS * 10) as u16 {
                    let turn = if ultra {
                        16
                    } else if (*rd).cyborg_control.ManeuverabilityIndex <= MEDIUM_SHIP as u16 {
                        48
                    } else {
                        32
                    };
                    let mut rect: RECT = std::mem::zeroed();
                    let frame =
                        SetAbsFrameIndex(ship_copy.IntersectControl.IntersectStamp.frame, 0);
                    GetFrameRect(frame, &mut rect);
                    let bounds =
                        (i32::from(rect.extent.width) + i32::from(rect.extent.height)) as u16;
                    let mut intercept_turn = 0;
                    if !moved {
                        intercept_turn = PlotIntercept(
                            e,
                            &mut ship_copy,
                            turn,
                            ((30 + bounds as i32 * 3) << ONE_SHIFT) as u16,
                        );
                    }
                    if intercept_turn != 0 {
                        if intercept_turn > 1
                            || PlotIntercept(
                                e,
                                &mut ship_copy,
                                1,
                                ((35 + bounds as i32) << ONE_SHIFT) as u16,
                            ) != 0
                            || PlotIntercept(
                                e,
                                &mut ship_copy,
                                turn << 1,
                                ((40 + bounds as i32) << ONE_SHIFT) as u16,
                            ) > 1
                        {
                            concerns[GRAVITY_MASS_INDEX as usize].ObjectPtr = e;
                            concerns[GRAVITY_MASS_INDEX as usize].which_turn = intercept_turn;
                            concerns[GRAVITY_MASS_INDEX as usize].facing =
                                ARCTAN(dx.wrapping_neg(), dy.wrapping_neg());
                            concerns[GRAVITY_MASS_INDEX as usize].MoveState = if ultra {
                                MOVEMENT_STATE_AVOID
                            } else {
                                MOVEMENT_STATE_ENTICE
                            };
                        } else if !ultra && !IsVelocityZero(&mut ship_copy.velocity) {
                            ship_copy.__bindgen_anon_5.thrust_wait =
                                ship_copy.__bindgen_anon_5.thrust_wait.wrapping_add(1);
                            if ship_copy.__bindgen_anon_4.turn_wait != 0 {
                                moved = true;
                            }
                        }
                    }
                } else if (*e).state_flags & PLAYER_SHIP as u16 != 0 {
                    let mut enemy = std::ptr::null_mut();
                    GetElementStarShip(e, &mut enemy);
                    let erd = (*enemy).RaceDescPtr;
                    if (*erd).cyborg_control.ManeuverabilityIndex == 0 {
                        init_cyborg(enemy);
                    }
                    let dist2 = (dx as i64 * dx as i64 + dy as i64 * dy as i64) as u32;
                    let mut turns = square_root(dist2) >> 6;
                    let idx = ENEMY_SHIP_INDEX as usize;
                    if turns <= concerns[idx].which_turn {
                        if turns == 0 {
                            turns = 1;
                        }
                        concerns[idx].ObjectPtr = e;
                        concerns[idx].which_turn = turns;
                        concerns[idx].facing = facing_to_angle((*enemy).ShipFacing as i32) as u16;
                        let pursue = moved
                            || mass > MAX_SHIP_MASS as u8
                            || ((*rd).cyborg_control.WeaponRange < LONG_RANGE_WEAPON
                                && ((*rd).cyborg_control.WeaponRange <= CLOSE_RANGE_WEAPON
                                    || ((*erd).cyborg_control.WeaponRange >= LONG_RANGE_WEAPON
                                        && (*erd).ship_info.ship_flags & SEEKING_WEAPON as u16
                                            != 0)
                                    || ((*rd).characteristics.max_thrust
                                        < (*erd).characteristics.max_thrust
                                        && (*rd).cyborg_control.WeaponRange
                                            < (*erd).cyborg_control.WeaponRange)));
                        concerns[idx].MoveState = if pursue {
                            MOVEMENT_STATE_PURSUE
                        } else {
                            MOVEMENT_STATE_ENTICE
                        };
                        if (*erd).ship_info.ship_flags & IMMEDIATE_WEAPON as u16 != 0
                            && ship_weapons(e, &mut ship_copy, 0) != 0
                        {
                            concerns[ENEMY_WEAPON_INDEX as usize] = concerns[idx];
                            concerns[ENEMY_WEAPON_INDEX as usize].which_turn = 1;
                            concerns[ENEMY_WEAPON_INDEX as usize].MoveState = MOVEMENT_STATE_AVOID;
                        }
                    }
                } else if (*e).pParent.is_null() {
                    if (*e).state_flags & FINITE_LIFE as u16 == 0 {
                        let turns =
                            square_root((dx as i64 * dx as i64 + dy as i64 * dy as i64) as u32)
                                >> 6;
                        let idx = FIRST_EMPTY_INDEX as usize;
                        if turns < concerns[idx].which_turn {
                            concerns[idx].ObjectPtr = e;
                            concerns[idx].which_turn = turns;
                            concerns[idx].MoveState = MOVEMENT_STATE_PURSUE;
                            concerns[idx].facing = GetVelocityTravelAngle(&mut (*e).velocity);
                        }
                    }
                } else if !elementsOfSamePlayer(e, &mut ship_copy)
                    && !is_crew_preprocess(e)
                    && concerns[ENEMY_WEAPON_INDEX as usize].which_turn > 1
                    && (*e).__bindgen_anon_1.life_span > 0
                {
                    let mut enemy = std::ptr::null_mut();
                    GetElementStarShip(e, &mut enemy);
                    let erd = (*enemy).RaceDescPtr;
                    let special_frames = (*erd).ship_data.special.as_mut_ptr();
                    let seeking = ((*erd).ship_info.ship_flags & SEEKING_WEAPON as u16 != 0
                        && (*e).next.image.farray != special_frames)
                        || ((*erd).ship_info.ship_flags & SEEKING_SPECIAL as u16 != 0
                            && (*e).next.image.farray == special_frames);
                    let mut turns = 0u16;
                    let mut movement = MOVEMENT_STATE_NO_MOVEMENT;
                    if seeking {
                        let travel = GetVelocityTravelAngle(&mut (*e).velocity);
                        let too_fast = (*e).state_flags & (FINITE_LIFE | CREW_OBJECT) as u16 == 0
                            && (*rd).characteristics.max_thrust > (8 << ONE_SHIFT) as u16;
                        let behind = normalize_angle(
                            travel as i32 - ARCTAN(dx.wrapping_neg(), dy.wrapping_neg()) as i32
                                + QUADRANT as i32,
                        ) > HALF_CIRCLE as i32;
                        if !too_fast && !behind {
                            turns =
                                square_root((dx as i64 * dx as i64 + dy as i64 * dy as i64) as u32)
                                    >> 6;
                            movement = MOVEMENT_STATE_ENTICE;
                            if ultra {
                                if turns == 0 {
                                    turns = 1;
                                } else if turns > 16 {
                                    turns = 0;
                                }
                            } else if turns == 0 {
                                turns = 1;
                            } else if turns > 16
                                || ((*rd).cyborg_control.ManeuverabilityIndex > MEDIUM_SHIP as u16
                                    && turns > 8)
                            {
                                turns = 0;
                            }
                        }
                    } else if (*owner).control & AWESOME_RATING != 0 {
                        turns = PlotIntercept(
                            e,
                            &mut ship_copy,
                            (*e).__bindgen_anon_1.life_span,
                            (40 << ONE_SHIFT) as u16,
                        );
                        movement = MOVEMENT_STATE_AVOID;
                    }
                    let idx = ENEMY_WEAPON_INDEX as usize;
                    if turns > 0
                        && (turns < concerns[idx].which_turn
                            || (turns == concerns[idx].which_turn
                                && movement == MOVEMENT_STATE_AVOID))
                    {
                        concerns[idx].ObjectPtr = e;
                        concerns[idx].which_turn = turns;
                        concerns[idx].MoveState = movement;
                        concerns[idx].facing = GetVelocityTravelAngle(&mut (*e).velocity);
                    }
                } else if (*e).state_flags & CREW_OBJECT as u16 != 0
                    && (((*e).state_flags & IGNORE_SIMILAR as u16 == 0
                        && elementsOfSamePlayer(e, &mut ship_copy))
                        || is_crew_preprocess(e))
                    && concerns[CREW_OBJECT_INDEX as usize].which_turn > 1
                {
                    let mut turns =
                        square_root((dx as i64 * dx as i64 + dy as i64 * dy as i64) as u32) >> 6;
                    if turns == 0 {
                        turns = 1;
                    }
                    let idx = CREW_OBJECT_INDEX as usize;
                    let enemy_turns = concerns[ENEMY_SHIP_INDEX as usize].which_turn;
                    if turns < concerns[idx].which_turn
                        && (enemy_turns > 32 || (enemy_turns > 8 && (*owner).hShip == (*e).hTarget))
                    {
                        concerns[idx].ObjectPtr = e;
                        concerns[idx].which_turn = turns;
                        concerns[idx].MoveState = MOVEMENT_STATE_PURSUE;
                        concerns[idx].facing = 0;
                    }
                }
            }
            UnlockElement(h);
            h = next;
        }

        invoke_intelligence(
            owner as usize,
            &mut ship_copy,
            concerns.as_mut_ptr(),
            FIRST_EMPTY_INDEX as u16,
        );
        (*owner).ShipFacing = original_facing;
        battle_input((*owner).ship_input_state)
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn collision_filter_matches_shared_flags_parent_and_mass_rules() {
            unsafe {
                let mut a: ELEMENT = std::mem::zeroed();
                let mut b: ELEMENT = std::mem::zeroed();
                assert!(!CollisionPossible(&mut a, &mut b));
                a.__bindgen_anon_3.mass_points = 1;
                assert!(CollisionPossible(&mut a, &mut b));
                for flag in [NONSOLID, DISAPPEARING] {
                    a.state_flags = flag as u16;
                    assert!(!CollisionPossible(&mut a, &mut b));
                    a.state_flags = 0;
                    b.state_flags = flag as u16;
                    assert!(CollisionPossible(&mut a, &mut b));
                }
                a.state_flags = COLLISION as u16;
                b.state_flags = COLLISION as u16;
                assert!(!CollisionPossible(&mut a, &mut b));
                b.state_flags = 0;
                assert!(CollisionPossible(&mut a, &mut b));
                a.state_flags = IGNORE_SIMILAR as u16;
                b.state_flags = IGNORE_SIMILAR as u16;
                assert!(!CollisionPossible(&mut a, &mut b));
                a.pParent = (&raw mut b).cast();
                assert!(CollisionPossible(&mut a, &mut b));
            }
        }

        #[test]
        fn zero_velocity_ignores_travel_angle_and_error_but_checks_all_six_components() {
            unsafe {
                let mut velocity: VELOCITY_DESC = std::mem::zeroed();
                velocity.travel_angle = FULL_CIRCLE as u16;
                velocity.error = EXTENT {
                    width: 31,
                    height: 31,
                };
                assert!(IsVelocityZero(&mut velocity));
                for i in 0..6 {
                    let mut moving = velocity;
                    let components = [
                        &raw mut moving.vector.width,
                        &raw mut moving.vector.height,
                        &raw mut moving.incr.width,
                        &raw mut moving.incr.height,
                        &raw mut moving.fract.width,
                        &raw mut moving.fract.height,
                    ];
                    *components[i] = -1;
                    assert!(!IsVelocityZero(&mut moving));
                }
            }
        }

        #[test]
        fn same_player_uses_player_number_including_neutral_objects() {
            unsafe {
                let mut a: ELEMENT = std::mem::zeroed();
                let mut b: ELEMENT = std::mem::zeroed();
                a.playerNr = -1;
                b.playerNr = -1;
                assert!(elementsOfSamePlayer(&mut a, &mut b));
                b.playerNr = 0;
                assert!(!elementsOfSamePlayer(&mut a, &mut b));
            }
        }
    }
}
#[cfg(not(test))]
pub use engine::{
    ship_intelligence, ship_weapons, tactical_intelligence, Avoid, Entice, PlotIntercept, Pursue,
    ThrustShip, TurnShip,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intersect_window_handles_stationary_and_relative_motion() {
        assert_eq!(
            intercept_window((0, 0), (0, 0), (0, 0), (0, 0), 10, [(1, 1), (1, 1)]),
            Some((0, 0))
        );
        assert_eq!(
            intercept_window((0, 0), (640, 0), (50, 0), (0, 0), 10, [(1, 1), (1, 1)]),
            Some((2, 2))
        );
        assert_eq!(
            intercept_window((0, 0), (0, 0), (100, 100), (0, 0), 10, [(1, 1), (1, 1)]),
            None
        );
    }

    #[test]
    fn intercept_requires_overlapping_time_windows_on_both_axes() {
        assert_eq!(
            intercept_window((0, 0), (320, 640), (100, 100), (0, 0), 20, [(1, 1), (1, 1)]),
            None
        );
        assert_eq!(
            intercept_window((0, 0), (320, 320), (100, 100), (0, 0), 20, [(1, 1), (1, 1)]),
            Some((10, 10))
        );
    }

    #[test]
    fn intercept_handles_negative_motion_and_one_stationary_axis() {
        for (position, velocity) in [(100, 320), (-100, -320)] {
            assert_eq!(
                intercept_window(
                    (0, 0),
                    (velocity, 0),
                    (position, 0),
                    (-velocity, 0),
                    10,
                    [(1, 1), (1, 1)]
                ),
                Some((5, 5))
            );
        }
        assert_eq!(
            intercept_window((0, 0), (0, 320), (0, 50), (0, 0), 10, [(1, 1), (1, 1)]),
            Some((5, 5))
        );
    }

    #[test]
    fn intercept_rejects_receding_objects_and_preserves_initial_overlap() {
        assert_eq!(
            intercept_window((0, 0), (-320, 0), (100, 0), (320, 0), 10, [(1, 1), (1, 1)]),
            None
        );
        assert_eq!(
            intercept_window((0, 0), (0, 0), (3, 3), (0, 0), 0, [(4, 4), (4, 4)]),
            Some((0, 0))
        );
    }

    #[test]
    fn intercept_narrows_turn_endpoints_to_signed_size_before_comparing() {
        assert_eq!(
            intercept_window((0, 0), (16, 0), (19000, 0), (0, 0), 40000, [(1, 1), (1, 1)]),
            Some((-27536, -27536))
        );
    }

    #[test]
    fn opposite_heading_tie_uses_c_jitter_and_consumes_one_random_value() {
        let opposite = angle_to_facing(abi::HALF_CIRCLE as i32) as u16;
        let mut calls = 0;
        assert_eq!(
            break_opposite_facing_tie(opposite, || {
                calls += 1;
                0
            }),
            7
        );
        assert_eq!(calls, 1);
        assert_eq!(break_opposite_facing_tie(opposite, || 1), 9);
    }

    #[test]
    fn conversion_widths_follow_signed_size_and_unsigned_count() {
        assert_eq!(c_size(32768), i16::MIN);

        assert_eq!(
            velocity_to_world(i64::from(i16::MAX) * 64),
            (i32::from(i16::MAX) * 2) as i16
        );
        assert_eq!(normalize_angle(-1), 63);
        assert_eq!(facing_to_angle(15), 60);
    }
}
