//! Live laser/missile, collision, silhouette and homing behavior from weapon.c.

use crate::ships::native::{abi::*, angle_to_facing, normalize_facing};

mod projectile;

fn blast_index(angle: u16) -> u16 {
    let facing = normalize_facing(angle_to_facing(i32::from(angle) + HALF_CIRCLE as i32));
    ((facing >> 2) << 1) + u16::from(facing & 3 != 0)
}

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
#[allow(non_snake_case)]
pub(crate) mod engine {
    use super::*;
    use crate::battle::sounds::{GameSounds, ProcessSound};
    use crate::battle::world_elements::do_damage;
    use crate::collections::observation::image;
    use crate::ships::native::{
        cosine, element, frame_count, object_cloaked, put_element, sine, starship, wrap_delta,
    };
    use std::{mem, ptr};

    extern "C" {
        static mut blast: [*mut crate::graphics::native_abi::frame_desc; 3];
    }

    unsafe fn primitive(e: *mut ELEMENT) -> *mut PRIMITIVE {
        (&raw mut DisplayArray)
            .cast::<PRIMITIVE>()
            .add((*e).PrimIndex as usize)
    }

    pub(crate) fn collision_cb(
        cx: &mut crate::engine::battle::Cx<'_>,
        w: crate::engine::battle::ElementId,
        wp: &mut POINT,
        h: crate::engine::battle::ElementId,
        hp: &mut POINT,
    ) {
        cx.weapon_collision(w, wp, h, hp);
    }

    /// # Safety
    /// Block must be live and the engine element pool initialized on the game thread.
    pub unsafe extern "C" fn initialize_laser(block: *mut LASER_BLOCK) -> HELEMENT {
        let handle = AllocElement();
        if !handle.is_null() {
            let e = element(handle);
            let b = &*block;
            (*e).playerNr = b.sender;
            (*e).__bindgen_anon_2.hit_points = 1;
            (*e).__bindgen_anon_3.mass_points = 1;
            (*e).state_flags = (APPEARING | FINITE_LIFE) as u16 | b.flags;
            (*e).__bindgen_anon_1.life_span = 1;
            (*e).collision_func = Some(CollisionHook::Native(collision_cb));
            (*e).__bindgen_anon_5.blast_offset = 1;
            let (position, velocity) =
                projectile::laser((b.cx, b.cy), b.face, b.pixoffs, (b.ex, b.ey));
            (*e).current.location.x = position.0;
            (*e).current.location.y = position.1;
            (*primitive(e)).Type = gfx_object_LINE_PRIM as u8;
            (*primitive(e)).color = b.color;
            image!(frame; (*e).current.image.frame, DecFrameIndex(stars_in_space));
            image!(set; (*e).current.image.farray, &raw mut stars_in_space);
            SetVelocityComponents(&raw mut (*e).velocity, velocity.0, velocity.1);
        }
        handle
    }

    /// # Safety
    /// Block and frame array must be live; the element pool must be initialized.
    pub unsafe extern "C" fn initialize_missile(block: *mut MISSILE_BLOCK) -> HELEMENT {
        let handle = AllocElement();
        if !handle.is_null() {
            let e = element(handle);
            let b = &*block;
            (*e).__bindgen_anon_2.hit_points = b.hit_points as u8 as u16;
            (*e).__bindgen_anon_3.mass_points = b.damage as u8;
            (*e).playerNr = b.sender;
            (*e).state_flags = (APPEARING | FINITE_LIFE) as u16 | b.flags;
            (*e).__bindgen_anon_1.life_span = b.life;
            (*primitive(e)).Type = gfx_object_STAMP_PRIM as u8;
            image!(set; (*e).current.image.farray, b.farray);
            image!(frame; (*e).current.image.frame, SetAbsFrameIndex(*b.farray, b.index));
            (*e).preprocess_func = b.preprocess_func;
            (*e).collision_func = Some(CollisionHook::Native(collision_cb));
            (*e).__bindgen_anon_5.blast_offset = b.blast_offs as u8;
            let (position, velocity) =
                projectile::missile((b.cx, b.cy), b.face, b.pixoffs, b.speed);
            (*e).current.location.x = position.0;
            (*e).current.location.y = position.1;
            SetVelocityComponents(&raw mut (*e).velocity, velocity.0, velocity.1);
        }
        handle
    }

    /// # Safety
    /// Elements, collision points and their image resources must be live.
    pub unsafe extern "C" fn weapon_collision(
        w: *mut ELEMENT,
        wp: *mut POINT,
        hit: *mut ELEMENT,
        _hp: *mut POINT,
    ) -> HELEMENT {
        if (*w).state_flags & COLLISION as u16 != 0 {
            return ptr::null_mut();
        }
        let damage = (*w).__bindgen_anon_3.mass_points as i16;
        if damage != 0
            && ((*hit).state_flags & FINITE_LIFE as u16 != 0
                || (*hit).__bindgen_anon_1.life_span == NORMAL_LIFE as u16)
        {
            do_damage(hit, damage);
            if (*hit).__bindgen_anon_2.hit_points != 0 {
                (*w).state_flags |= COLLISION as u16;
            }
        }
        if (*hit).state_flags & FINITE_LIFE as u16 == 0
            || ((*hit).state_flags & COLLISION as u16 == 0
                && (*w).__bindgen_anon_2.hit_points <= (*hit).__bindgen_anon_3.mass_points as u16)
        {
            if damage != 0 {
                let sound = (2 + (damage >> 1)).min(5);
                ProcessSound(SetAbsStringTableIndex(GameSounds, sound as u16), hit);
            }
            if (*primitive(w)).Type != gfx_object_LINE_PRIM as u8 {
                (*w).state_flags |= DISAPPEARING as u16;
            }
            (*w).__bindgen_anon_2.hit_points = 0;
            (*w).__bindgen_anon_1.life_span = 0;
            (*w).state_flags |= (COLLISION | NONSOLID) as u16;
            let handle = AllocElement();
            if !handle.is_null() {
                put_element(handle);
                let b = element(handle);
                (*b).playerNr = (*w).playerNr;
                (*b).state_flags = (APPEARING | FINITE_LIFE | NONSOLID) as u16;
                (*primitive(b)).Type = gfx_object_STAMP_PRIM as u8;
                (*b).current.location.x = (i32::from((*wp).x) << ONE_SHIFT) as i16;
                (*b).current.location.y = (i32::from((*wp).y) << ONE_SHIFT) as i16;
                let angle = (*w).velocity.travel_angle;
                let offset = (*w).__bindgen_anon_5.blast_offset as i16;
                if offset > 0 {
                    (*b).current.location.x = (*b)
                        .current
                        .location
                        .x
                        .wrapping_add(cosine(angle as i32, i32::from(offset) << ONE_SHIFT));
                    (*b).current.location.y = (*b)
                        .current
                        .location
                        .y
                        .wrapping_add(sine(angle as i32, i32::from(offset) << ONE_SHIFT));
                }
                let frames = frame_count((*w).next.image.frame);
                let facings = angle_to_facing(FULL_CIRCLE as i32) as u16;
                if frames <= facings {
                    (*b).__bindgen_anon_1.life_span = 2;
                    image!(set; (*b).current.image.farray, (&raw mut blast).cast::<FRAME>());
                    image!(frame; (*b).current.image.frame, SetAbsFrameIndex(*(*b).current.image.farray, blast_index(angle)));
                } else {
                    (*b).__bindgen_anon_1.life_span = frames - facings;
                    (*b).__bindgen_anon_4.turn_wait = 0;
                    (*b).__bindgen_anon_5.next_turn = 0;
                    (*b).preprocess_func = Some(ElementHook::Native(animation_preprocess));
                    image!(copy; (*b).current.image.farray, (*w).next.image.farray);
                    image!(frame; (*b).current.image.frame, SetAbsFrameIndex(*(*b).current.image.farray, facings));
                }
                return handle;
            }
        }
        ptr::null_mut()
    }

    /// # Safety
    /// Element must own a live ship, and stamp/status resources must be loaded.
    #[no_mangle]
    pub unsafe extern "C" fn ModifySilhouette(
        e: *mut ELEMENT,
        stamp: *mut STAMP,
        flags: u8,
    ) -> FRAME {
        let mut object: INTERSECT_CONTROL = mem::zeroed();
        object.IntersectStamp = *stamp;
        let mut object_rect: RECT = mem::zeroed();
        GetFrameRect(object.IntersectStamp.frame, &mut object_rect);
        let ship = starship(e);
        if flags & MODIFY_IMAGE as u8 != 0 {
            let mut ship_intersect: INTERSECT_CONTROL = mem::zeroed();
            image!(frame; ship_intersect.IntersectStamp.frame, SetAbsFrameIndex((*(*ship).RaceDescPtr).ship_info.icons, 1));
            if ship_intersect.IntersectStamp.frame.is_null() {
                return ptr::null_mut();
            }
            let mut rect: RECT = mem::zeroed();
            GetFrameRect(ship_intersect.IntersectStamp.frame, &mut rect);
            loop {
                object.IntersectStamp.origin.x = (i32::from(TFB_Random() as u16)
                    % (i32::from(rect.extent.width) - i32::from(object_rect.extent.width))
                    + ((i32::from(object_rect.extent.width) - i32::from(rect.extent.width)) >> 1))
                    as i16;
                object.IntersectStamp.origin.y = (i32::from(TFB_Random() as u16)
                    % (i32::from(rect.extent.height) - i32::from(object_rect.extent.height))
                    + ((i32::from(object_rect.extent.height) - i32::from(rect.extent.height)) >> 1))
                    as i16;
                object.EndPoint = object.IntersectStamp.origin;
                if DrawablesIntersect(&mut object, &mut ship_intersect, MAX_TIME_VALUE as u16) != 0
                {
                    break;
                }
            }
            object.IntersectStamp.origin.x = object.IntersectStamp.origin.x.wrapping_add(64 >> 1);
            object.IntersectStamp.origin.y = object.IntersectStamp.origin.y.wrapping_add(31);
        }
        object.IntersectStamp.origin.y = object.IntersectStamp.origin.y.wrapping_add(
            *(&raw const status_y_offsets)
                .cast::<i16>()
                .offset((*e).playerNr as isize),
        );
        if flags & MODIFY_SWAP as u8 != 0 {
            object_rect.corner.x = object_rect
                .corner
                .x
                .wrapping_add(object.IntersectStamp.origin.x);
            object_rect.corner.y = object_rect
                .corner
                .y
                .wrapping_add(object.IntersectStamp.origin.y);
            InitShipStatus(
                &raw mut (*(*ship).RaceDescPtr).ship_info,
                ship,
                &mut object_rect,
            );
        } else {
            let old = SetContext(StatusContext);
            DrawStamp(&mut object.IntersectStamp);
            SetContext(old);
        }
        ptr::null_mut()
    }

    /// # Safety
    /// Tracker, facing and any target/queue elements must be live engine records.
    pub unsafe extern "C" fn TrackShip(tracker: *mut ELEMENT, facing: *mut u16) -> i16 {
        let mut best_distance: i16 = 0;
        let mut best_facing: i16 = -1;
        let mut handle = (*tracker).hTarget;
        let explicit = !handle.is_null();
        if explicit {
            (*tracker).hTarget = ptr::null_mut();
        } else {
            handle = disp_q.head;
        }
        while !handle.is_null() {
            let target = element(handle);
            let next = if explicit {
                ptr::null_mut()
            } else {
                (*target).succ
            };
            if explicit
                || ((*target).state_flags & PLAYER_SHIP as u16 != 0
                    && (*target).playerNr != (*tracker).playerNr
                    && (!object_cloaked(target)
                        || (*tracker).state_flags & (PLAYER_SHIP | APPEARING) as u16
                            == (PLAYER_SHIP | APPEARING) as u16))
            {
                let ship = starship(target);
                if (*target).__bindgen_anon_1.life_span != 0
                    && (*(*ship).RaceDescPtr).ship_info.crew_level != 0
                {
                    let (t, s) = if (*tracker).state_flags & PRE_PROCESS as u16 != 0 {
                        ((*target).next.location, (*tracker).next.location)
                    } else {
                        ((*target).current.location, (*tracker).current.location)
                    };
                    let mut dx = wrap_delta(
                        t.x.wrapping_sub(s.x),
                        (ScreenWidth - 64) << (ONE_SHIFT + MAX_REDUCTION),
                    );
                    let mut dy = wrap_delta(
                        t.y.wrapping_sub(s.y),
                        ScreenHeight << (ONE_SHIFT + MAX_REDUCTION),
                    );
                    let delta = normalize_facing(
                        angle_to_facing(ARCTAN(dx, dy) as i32) - i32::from(*facing),
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
                        (*tracker).hTarget = handle;
                    }
                }
            }
            handle = next;
        }
        *facing = turn_toward(*facing, best_facing, TFB_Random);
        best_facing
    }
}

#[cfg(not(test))]
pub use engine::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blast_frames_match_eight_direction_buckets() {
        assert_eq!(
            (0..16).map(|i| blast_index(i * 4)).collect::<Vec<_>>(),
            vec![4, 5, 5, 5, 6, 7, 7, 7, 0, 1, 1, 1, 2, 3, 3, 3]
        );
    }

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
