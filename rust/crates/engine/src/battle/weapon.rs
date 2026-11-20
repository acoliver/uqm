//! Projectile allocation, collision and blast records on the supplied context.
use super::flags::*;
use super::hooks::animation_preprocess;
use super::projectile;
use super::{CollisionHook, Cx, ElementHook, ElementId};
use crate::battle::STAMP_PRIMITIVE;
use crate::graphics::geometry::Point;
use crate::math::battle_types::HALF_CIRCLE;
use crate::math::battle_types::{FULL_CIRCLE, ONE_SHIFT};
use crate::math::{angle_to_facing, normalize_facing};
use crate::math::{cosine, sine};

fn blast_index(angle: u16) -> u16 {
    let facing = normalize_facing(angle_to_facing(i32::from(angle) + HALF_CIRCLE as i32));
    ((facing >> 2) << 1) + u16::from(facing & 3 != 0)
}

pub fn collision_cb(
    cx: &mut crate::battle::Cx<'_>,
    w: crate::battle::ElementId,
    wp: &mut Point,
    h: crate::battle::ElementId,
    hp: &mut Point,
) {
    cx.weapon_collision(w, wp, h, hp);
}

pub fn initialize_laser(cx: &mut Cx<'_>, b: &crate::battle::views::Laser) -> Option<ElementId> {
    let e = cx.alloc_element()?;
    cx.element_mut(e).player = b.sender;
    cx.element_mut(e).hits = 1;
    cx.element_mut(e).mass = 1;
    cx.element_mut(e).flags = (APPEARING | FINITE_LIFE) as u16 | b.flags;
    cx.element_mut(e).life = 1;
    cx.element_mut(e).collide = Some(CollisionHook::Native(collision_cb));
    cx.element_mut(e).thrust = 1;
    let (position, velocity) = projectile::laser(
        (b.origin.x, b.origin.y),
        b.face,
        b.pixoffs,
        (b.end.x, b.end.y),
    );
    cx.element_mut(e).current.location = Point {
        x: position.0,
        y: position.1,
    };
    cx.primitive_style_mut(e).kind = 3;
    cx.primitive_style_mut(e).color = b.color;
    cx.set_star_image(e);
    cx.element_mut(e)
        .velocity
        .set_components(i32::from(velocity.0), i32::from(velocity.1));
    Some(e)
}

pub fn initialize_missile(cx: &mut Cx<'_>, b: &crate::battle::views::Missile) -> Option<ElementId> {
    let e = cx.alloc_element()?;
    cx.element_mut(e).hits = b.hit_points as u8 as u16;
    cx.element_mut(e).mass = b.damage as u8;
    cx.element_mut(e).player = b.sender;
    cx.element_mut(e).flags = (APPEARING | FINITE_LIFE) as u16 | b.flags;
    cx.element_mut(e).life = b.life;
    cx.primitive_style_mut(e).kind = STAMP_PRIMITIVE;
    cx.set_frames(e, false, b.frames.clone());
    let frame = cx.absolute_frame(cx.frames_first(&b.frames), b.index);
    cx.set_frame(e, false, frame);
    cx.element_mut(e).pre = b.preprocess.map(ElementHook::Native);
    cx.element_mut(e).collide = Some(CollisionHook::Native(collision_cb));
    cx.element_mut(e).thrust = b.blast_offs as u8;
    let (position, velocity) = projectile::missile((b.cx, b.cy), b.face, b.pixoffs, b.speed);
    cx.element_mut(e).current.location = Point {
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
    wp: &mut Point,
    hit: ElementId,
    _hp: &mut Point,
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
        if cx.primitive_style(w).kind != 3 {
            cx.element_mut(w).flags |= DISAPPEARING as u16;
        }
        cx.element_mut(w).hits = 0;
        cx.element_mut(w).life = 0;
        cx.element_mut(w).flags |= (COLLISION | NONSOLID) as u16;
        let b = cx.alloc_element()?;
        cx.put(b);
        cx.element_mut(b).player = cx.element(w).player;
        cx.element_mut(b).flags = (APPEARING | FINITE_LIFE | NONSOLID) as u16;
        cx.primitive_style_mut(b).kind = STAMP_PRIMITIVE;
        cx.element_mut(b).current.location = Point {
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
}
