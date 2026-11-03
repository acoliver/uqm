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
pub(crate) mod engine {
    use super::*;
    use crate::collections::observation::image;
    use crate::engine::battle::{Cx, ElementId, Subject};
    use crate::ships::native::{cosine, sine, wrap_delta};
    use std::{mem, ptr};

    pub(crate) fn collision_cb(
        cx: &mut crate::engine::battle::Cx<'_>,
        w: crate::engine::battle::ElementId,
        wp: &mut POINT,
        h: crate::engine::battle::ElementId,
        hp: &mut POINT,
    ) {
        cx.weapon_collision(w, wp, h, hp);
    }

    pub fn initialize_laser(cx: &mut Cx<'_>, b: &LASER_BLOCK) -> Option<ElementId> {
        let e = cx.alloc_element()?;
        cx.element_mut(e).player = b.sender;
        cx.element_mut(e).hits = 1;
        cx.element_mut(e).mass = 1;
        cx.element_mut(e).flags = (APPEARING | FINITE_LIFE) as u16 | b.flags;
        cx.element_mut(e).life = 1;
        cx.element_mut(e).collide = Some(CollisionHook::Native(collision_cb));
        cx.element_mut(e).thrust = 1;
        let (position, velocity) = projectile::laser((b.cx, b.cy), b.face, b.pixoffs, (b.ex, b.ey));
        cx.element_mut(e).current.location = POINT {
            x: position.0,
            y: position.1,
        };
        cx.primitive_style_mut(e).kind = gfx_object_LINE_PRIM as u8;
        cx.primitive_style_mut(e).color = b.color;
        cx.set_star_image(e);
        cx.element_mut(e)
            .velocity
            .set_components(i32::from(velocity.0), i32::from(velocity.1));
        Some(e)
    }

    pub fn initialize_missile(
        cx: &mut Cx<'_>,
        b: &crate::engine::battle::views::Missile,
    ) -> Option<ElementId> {
        let e = cx.alloc_element()?;
        cx.element_mut(e).hits = b.hit_points as u8 as u16;
        cx.element_mut(e).mass = b.damage as u8;
        cx.element_mut(e).player = b.sender;
        cx.element_mut(e).flags = (APPEARING | FINITE_LIFE) as u16 | b.flags;
        cx.element_mut(e).life = b.life;
        cx.primitive_style_mut(e).kind = gfx_object_STAMP_PRIM as u8;
        cx.set_frames(e, false, b.frames.clone());
        let frame = cx.absolute_frame(cx.frames_first(&b.frames), b.index);
        cx.set_frame(e, false, frame);
        cx.element_mut(e).pre = b.preprocess.map(ElementHook::Native);
        cx.element_mut(e).collide = Some(CollisionHook::Native(collision_cb));
        cx.element_mut(e).thrust = b.blast_offs as u8;
        let (position, velocity) = projectile::missile((b.cx, b.cy), b.face, b.pixoffs, b.speed);
        cx.element_mut(e).current.location = POINT {
            x: position.0,
            y: position.1,
        };
        cx.element_mut(e)
            .velocity
            .set_components(i32::from(velocity.0), i32::from(velocity.1));
        Some(e)
    }

    pub fn weapon_collision(
        cx: &mut Cx<'_>,
        w: ElementId,
        wp: &mut POINT,
        hit: ElementId,
        _hp: &mut POINT,
    ) -> Option<ElementId> {
        if cx.element(w).flags & COLLISION as u16 != 0 {
            return None;
        }
        let damage = i16::from(cx.element(w).mass);
        if damage != 0
            && (cx.element(hit).flags & FINITE_LIFE as u16 != 0
                || cx.element(hit).life == NORMAL_LIFE as u16)
        {
            cx.damage(hit, damage);
            if cx.element(hit).hits != 0 {
                cx.element_mut(w).flags |= COLLISION as u16;
            }
        }
        if cx.element(hit).flags & FINITE_LIFE as u16 == 0
            || (cx.element(hit).flags & COLLISION as u16 == 0
                && cx.element(w).hits <= u16::from(cx.element(hit).mass))
        {
            if damage != 0 {
                cx.game_sound((2 + (damage >> 1)).min(5) as u16, hit);
            }
            if cx.primitive_style(w).kind != gfx_object_LINE_PRIM as u8 {
                cx.element_mut(w).flags |= DISAPPEARING as u16;
            }
            cx.element_mut(w).hits = 0;
            cx.element_mut(w).life = 0;
            cx.element_mut(w).flags |= (COLLISION | NONSOLID) as u16;
            let b = cx.alloc_element()?;
            cx.put(b);
            cx.element_mut(b).player = cx.element(w).player;
            cx.element_mut(b).flags = (APPEARING | FINITE_LIFE | NONSOLID) as u16;
            cx.primitive_style_mut(b).kind = gfx_object_STAMP_PRIM as u8;
            cx.element_mut(b).current.location = POINT {
                x: (i32::from(wp.x) << ONE_SHIFT) as i16,
                y: (i32::from(wp.y) << ONE_SHIFT) as i16,
            };
            let angle = cx.element(w).velocity.travel_angle;
            let offset = i16::from(cx.element(w).thrust);
            if offset > 0 {
                cx.element_mut(b).current.location.x = cx
                    .element(b)
                    .current
                    .location
                    .x
                    .wrapping_add(cosine(i32::from(angle), i32::from(offset) << ONE_SHIFT));
                cx.element_mut(b).current.location.y = cx
                    .element(b)
                    .current
                    .location
                    .y
                    .wrapping_add(sine(i32::from(angle), i32::from(offset) << ONE_SHIFT));
            }
            let frames = cx.frame_count(cx.element(w).next.image.frame.view());
            let facings = angle_to_facing(FULL_CIRCLE as i32) as u16;
            if frames <= facings {
                cx.element_mut(b).life = 2;
                cx.set_blast_frames(b);
                let frame = cx.absolute_frame(
                    cx.frames_first(&cx.element(b).current.image.frames),
                    blast_index(angle),
                );
                cx.set_frame(b, false, frame);
            } else {
                cx.element_mut(b).life = frames - facings;
                cx.element_mut(b).turn = 0;
                cx.element_mut(b).thrust = 0;
                cx.element_mut(b).pre = Some(ElementHook::Native(animation_preprocess));
                cx.copy_next_frame_set(b, w);
                let frame = cx.absolute_frame(
                    cx.frames_first(&cx.element(b).current.image.frames),
                    facings,
                );
                cx.set_frame(b, false, frame);
            }
            return Some(b);
        }
        None
    }

    /// # Safety
    /// Element must own a live ship, and stamp/status resources must be loaded.
    pub unsafe fn modify_silhouette(
        cx: &mut Cx<'_>,
        e: ElementId,
        stamp: &mut STAMP,
        flags: u8,
    ) -> FRAME {
        let mut object: INTERSECT_CONTROL = mem::zeroed();
        object.IntersectStamp = *stamp;
        let mut object_rect: RECT = mem::zeroed();
        GetFrameRect(object.IntersectStamp.frame, &mut object_rect);
        let ship = cx.owner(e);
        if flags & MODIFY_IMAGE as u8 != 0 {
            let mut ship_intersect: INTERSECT_CONTROL = mem::zeroed();
            image!(frame; ship_intersect.IntersectStamp.frame, SetAbsFrameIndex(cx.desc(ship).ship_info.icons, 1));
            if ship_intersect.IntersectStamp.frame.is_null() {
                return ptr::null_mut();
            }
            let mut rect: RECT = mem::zeroed();
            GetFrameRect(ship_intersect.IntersectStamp.frame, &mut rect);
            loop {
                object.IntersectStamp.origin.x = (i32::from(cx.random() as u16)
                    % (i32::from(rect.extent.width) - i32::from(object_rect.extent.width))
                    + ((i32::from(object_rect.extent.width) - i32::from(rect.extent.width)) >> 1))
                    as i16;
                object.IntersectStamp.origin.y = (i32::from(cx.random() as u16)
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
            *(&raw const Kernel::current().status_offsets)
                .cast::<i16>()
                .offset(cx.element(e).player as isize),
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
            crate::ui::shipstat::init_ship_status(
                cx.ship_info_pointer(ship),
                Some(ship),
                &mut object_rect,
            );
        } else {
            let old = SetContext(StatusContext);
            DrawStamp(&mut object.IntersectStamp);
            SetContext(old);
        }
        ptr::null_mut()
    }

    pub fn track_ship(cx: &mut Cx<'_>, id: ElementId, facing: &mut u16) -> i16 {
        let mut subject = cx.weapon_origin(id);
        let result = track_subject(cx, &mut subject, facing);
        cx.set_target(id, subject.target);
        result
    }

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
