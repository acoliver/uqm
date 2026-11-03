//! Live planet, asteroid, rubble, damage and floating-crew callbacks.

use crate::ships::native::abi::*;

fn damage_object(mass: u8, hp: &mut u16, life: &mut u16, flags: &mut u16, damage: i16) {
    if mass > 100 {
        return;
    }
    if (damage as u8 as u16) < *hp {
        *hp -= damage as u8 as u16;
    } else {
        *hp = 0;
        *life = 0;
        *flags |= 1 << 6;
    }
}

fn crew_step(
    current: (i16, i16),
    next_x: &mut i16,
    next_y: &mut i16,
    target: (i16, i16),
    dimensions: (i32, i32),
) {
    let dx = crate::math::wrap_delta(target.0.wrapping_sub(current.0), dimensions.0);
    *next_x = next_x.wrapping_add(dx.signum() * 4);
    let dy = crate::math::wrap_delta(target.1.wrapping_sub(current.1), dimensions.1);
    *next_y = next_y.wrapping_add(dy.signum() * 4);
}

const fn crew_color(high: bool) -> Color {
    if high {
        Color {
            r: (10 << 3) | (10 >> 2),
            g: (30 << 3) | (30 >> 2),
            b: (10 << 3) | (10 >> 2),
            a: 255,
        }
    } else {
        Color {
            r: 0,
            g: (20 << 3) | (20 >> 2),
            b: 0,
            a: 255,
        }
    }
}

#[cfg(not(test))]
fn width(pixels: i32) -> i32 {
    (pixels - 64) << (ONE_SHIFT + MAX_REDUCTION)
}
#[cfg(not(test))]
fn height(pixels: i32) -> i32 {
    pixels << (ONE_SHIFT + MAX_REDUCTION)
}
#[cfg(not(test))]
fn aligned_random(random: u32, extent: i32) -> i16 {
    ((i32::from(random as u16) % extent) as i16) & !((1 << ONE_SHIFT) - 1)
}

#[cfg(not(test))]
pub(crate) mod engine {
    use super::*;
    use crate::collections::observation::image;
    use crate::engine::battle::views::SharedFrames;
    use crate::engine::battle::{Cx, ElementId, Parent};
    use crate::ships::native::{angle_to_facing, cosine, normalize_facing, sine};

    pub(crate) fn gravity_postprocess(cx: &mut Cx<'_>, e: ElementId) {
        cx.gravity(e);
    }

    pub fn spawn_planet(cx: &mut Cx<'_>) {
        let Some(e) = cx.alloc_element() else {
            return;
        };
        cx.element_mut(e).player = NEUTRAL_PLAYER_NUM as i16;
        cx.element_mut(e).hits = 200;
        cx.element_mut(e).flags = APPEARING as u16;
        cx.element_mut(e).life = (NORMAL_LIFE + 1) as u16;
        cx.primitive_style_mut(e).kind = gfx_object_STAMP_PRIM as u8;
        let frames = cx.planet_frames();
        cx.set_frames(e, false, frames.clone());
        cx.set_frame(e, false, cx.frames_first(&frames).to_owned());
        cx.element_mut(e).collide = Some(CollisionHook::Native(collision));
        cx.element_mut(e).post = Some(ElementHook::Native(gravity_postprocess));
        // SAFETY: the legacy velocity record admits the zero state.
        cx.element_mut(e).velocity = crate::battle::velocity::VelocityDesc::default();
        let (sw, sh) = cx.screen_dimensions();
        loop {
            cx.element_mut(e).current.location.x = aligned_random(cx.random(), width(sw));
            cx.element_mut(e).current.location.y = aligned_random(cx.random(), height(sh));
            if !cx.gravity(e) && !cx.matter_conflict(e) {
                break;
            }
        }
        cx.element_mut(e).mass = cx.element(e).hits as u8;
        cx.put(e);
    }

    pub(crate) fn spawn_rubble(cx: &mut Cx<'_>, source: ElementId) {
        let Some(e) = cx.alloc_element() else {
            return;
        };
        cx.put(e);
        cx.element_mut(e).player = cx.element(source).player;
        cx.element_mut(e).flags = (APPEARING | FINITE_LIFE | NONSOLID) as u16;
        cx.element_mut(e).set_life_span(5);
        cx.element_mut(e).set_turn_wait(0);
        cx.element_mut(e).set_thrust_wait(0);
        cx.primitive_style_mut(e).kind = gfx_object_STAMP_PRIM as u8;
        let array = cx.shared_frame_set(SharedFrames::Asteroid);
        cx.set_frames(e, false, array);
        let frame = cx.absolute_frame(
            cx.shared_frame_selection(SharedFrames::Asteroid).view(),
            angle_to_facing(FULL_CIRCLE as i32) as u16,
        );
        cx.set_frame(e, false, frame);
        cx.element_mut(e).current.location = cx.element(source).current.location;
        cx.element_mut(e).pre = Some(ElementHook::Native(animation_preprocess));
        cx.element_mut(e).death = Some(ElementHook::Native(spawn_asteroid));
    }

    pub(crate) fn asteroid_preprocess(cx: &mut Cx<'_>, e: ElementId) {
        let wait = cx.element(e).turn_wait();
        if wait > 0 {
            cx.element_mut(e).set_turn_wait(wait - 1);
        } else {
            let index = cx.frame_number(cx.element(e).current.image.frame.view());
            let index = if cx.element(e).thrust_wait() & 128 != 0 {
                index.wrapping_sub(1)
            } else {
                index.wrapping_add(1)
            };
            let frame = cx.absolute_frame(
                cx.element(e).current.image.frame.view(),
                normalize_facing(index as i32),
            );
            cx.set_frame(e, true, frame);
            cx.element_mut(e).flags |= CHANGING as u16;
            let wait = cx.element(e).thrust_wait() & 127;
            cx.element_mut(e).set_turn_wait(wait);
        }
    }

    pub fn spawn_asteroid(cx: &mut Cx<'_>, source: ElementId) {
        create_asteroid(cx, Some(source));
    }

    pub fn create_asteroid(cx: &mut Cx<'_>, source: Option<ElementId>) {
        let Some(e) = cx.alloc_element() else {
            if let Some(source) = source {
                cx.element_mut(source).flags &= !(DISAPPEARING as u16);
                cx.primitive_style_mut(source).kind = gfx_object_NUM_PRIMS as u8;
                cx.element_mut(source).set_life_span(1);
            }
            return;
        };
        cx.element_mut(e).player = NEUTRAL_PLAYER_NUM as i16;
        cx.element_mut(e).set_hit_points(1);
        cx.element_mut(e).set_mass_points(3);
        cx.element_mut(e).flags = APPEARING as u16;
        cx.element_mut(e).set_life_span(NORMAL_LIFE as u16);
        cx.primitive_style_mut(e).kind = gfx_object_STAMP_PRIM as u8;
        let value = cx.random() as u16;
        let (w, h) = cx.screen_dimensions();
        if value & 1 != 0 {
            cx.element_mut(e).current.location.x = if value & 2 == 0 { 0 } else { width(w) as i16 };
            cx.element_mut(e).current.location.y = aligned_random(cx.random(), height(h));
        } else {
            cx.element_mut(e).current.location.x = aligned_random(cx.random(), width(w));
            cx.element_mut(e).current.location.y =
                if value & 2 == 0 { 0 } else { height(h) as i16 };
        }
        let magnitude = (((cx.random() as i16 & 7) + 4) as i32) << ONE_SHIFT;
        let facing = cx.random() as u16;
        cx.element_mut(e)
            .velocity
            .set_vector(i32::from(magnitude as i16), facing);
        let array = cx.shared_frame_set(SharedFrames::Asteroid);
        cx.set_frames(e, false, array);
        let facing = cx.random() as u16 & ((1 << FACING_SHIFT) - 1) as u16;
        let frame = cx.absolute_frame(
            cx.shared_frame_selection(SharedFrames::Asteroid).view(),
            facing,
        );
        cx.set_frame(e, false, frame);
        let wait = cx.random() as u8 & 3;
        cx.element_mut(e).set_turn_wait(wait);
        let thrust_wait = wait | (cx.random() as u8 & 128);
        cx.element_mut(e).set_thrust_wait(thrust_wait);
        cx.element_mut(e).pre = Some(ElementHook::Native(asteroid_preprocess));
        cx.element_mut(e).death = Some(ElementHook::Native(spawn_rubble));
        cx.element_mut(e).collide = Some(CollisionHook::Native(collision));
        cx.put(e);
    }

    impl Cx<'_> {
        pub fn damage(&mut self, e: ElementId, damage: i16) {
            if self.element(e).flags & PLAYER_SHIP as u16 != 0 {
                if !self.delta_crew(e, damage.wrapping_neg()) {
                    self.element_mut(e).set_life_span(0);
                    self.element_mut(e).flags |= NONSOLID as u16;
                }
            } else {
                let mut hp = self.element(e).hit_points();
                let mut life = self.element(e).life_span();
                let mut flags = self.element(e).flags;
                damage_object(
                    self.element(e).mass_points(),
                    &mut hp,
                    &mut life,
                    &mut flags,
                    damage,
                );
                self.element_mut(e).set_hit_points(hp);
                self.element_mut(e).set_life_span(life);
                self.element_mut(e).flags = flags;
            }
        }
    }

    pub fn crew_preprocess(cx: &mut Cx<'_>, e: ElementId) {
        let old = cx.primitive_style_mut(e).color;
        let low = crew_color(false);
        let high = old.r == low.r && old.g == low.g && old.b == low.b && old.a == low.a;
        cx.primitive_style_mut(e).color = crew_color(high);
        cx.element_mut(e).flags |= CHANGING as u16;
        let target = cx.target(e);
        if target.is_none() {
            if let Parent::Ship(ship) = cx.parent(e) {
                if cx.desc(ship).ship_info.crew_level != 0 {
                    let target = cx.ship_element(ship);
                    cx.set_target(e, Some(target));
                } else {
                    cx.track_ship(e, &mut 0);
                }
            } else {
                cx.track_ship(e, &mut 0);
            }
        }
        if let Some(ship) = target {
            let current = cx.element(e).current.location;
            let target = cx.element(ship).current.location;
            let (w, h) = cx.screen_dimensions();
            let next = &mut cx.element_mut(e).next.location;
            crew_step(
                (current.x, current.y),
                &mut next.x,
                &mut next.y,
                (target.x, target.y),
                (width(w), height(h)),
            );
        }
    }

    pub fn crew_collision(
        cx: &mut Cx<'_>,
        crew: ElementId,
        _cp: &mut POINT,
        hit: ElementId,
        _hp: &mut POINT,
    ) {
        if cx.element(hit).flags & PLAYER_SHIP as u16 != 0
            && cx.element(hit).life_span() >= NORMAL_LIFE as u16
            && cx.element(crew).hit_points() > 0
        {
            let ship = cx.owner(hit);
            if cx.desc(ship).ship_info.ship_flags & CREW_IMMUNE as u16 == 0 {
                cx.game_sound(0, hit);
                cx.delta_crew(hit, 1);
            }
        }
        cx.element_mut(crew).set_hit_points(0);
        cx.element_mut(crew).set_life_span(0);
        cx.element_mut(crew).flags |= (COLLISION | DISAPPEARING | NONSOLID) as u16;
    }

    pub fn abandon_ship(cx: &mut Cx<'_>, ship: ElementId, target: ElementId, mut loss: u16) {
        let mut owner = cx.owner(ship);
        if cx.race_stats(owner).ship_info.ship_flags & CREW_IMMUNE as u16 != 0 {
            return;
        }
        let source = cx.weapon_origin(ship);
        let mut intersect = source.intersection();
        let rect = cx.frame_rect(cx.subject_intersect_frame(&source));
        let direction = cx.element(ship).velocity.travel_angle;
        let (dx, dy) = if direction == FULL_CIRCLE as u16 {
            (0, 0)
        } else {
            let direction = direction.wrapping_add(HALF_CIRCLE as u16);
            (cosine(direction as i32, 100), sine(direction as i32, 100))
        };
        while loss != 0 {
            loss -= 1;
            let Some(crew) = cx.alloc_element() else {
                break;
            };
            cx.delta_crew(ship, -1);
            cx.put(crew);
            cx.element_mut(crew).player = NEUTRAL_PLAYER_NUM as i16;
            cx.element_mut(crew).hits = 1;
            cx.element_mut(crew).flags = (APPEARING | FINITE_LIFE | CREW_OBJECT) as u16;
            cx.element_mut(crew).life = 300;
            cx.primitive_style_mut(crew).kind = gfx_object_POINT_PRIM as u8;
            cx.primitive_style_mut(crew).color = crew_color(false);
            cx.set_star_image(crew);
            cx.element_mut(crew).pre = Some(ElementHook::Native(crew_preprocess));
            cx.element_mut(crew).collide = Some(CollisionHook::Native(crew_collision));
            cx.set_parent(crew, Some(owner));
            // The original loop retains this reassignment for subsequent crew.
            owner = cx.owner(target);
            cx.set_target(crew, cx.optional_ship_element(owner));
            intersect.IntersectStamp.origin = cx.weapon_origin(ship).intersection().EndPoint;
            let w = (i32::from(cx.random() as u16) % i32::from(rect.extent.width)) as i16;
            let h = (i32::from(cx.random() as u16) % i32::from(rect.extent.height)) as i16;
            let frame = cx.element(crew).current.image.frame;
            let mut crew_intersect = intersect;
            crew_intersect.last_time_val = 0;
            crew_intersect.IntersectStamp.origin = POINT { x: 0, y: 0 };
            image!(frame; crew_intersect.IntersectStamp.frame, frame.raw());
            if dx == 0 && dy == 0 {
                crew_intersect.EndPoint.x = crew_intersect
                    .EndPoint
                    .x
                    .wrapping_add(w - (rect.extent.width >> 1));
                crew_intersect.EndPoint.y = crew_intersect
                    .EndPoint
                    .y
                    .wrapping_add(h - (rect.extent.height >> 1));
                crew_intersect.IntersectStamp.origin =
                    cx.weapon_origin(target).intersection().EndPoint;
            } else {
                crew_intersect.EndPoint.x = crew_intersect.EndPoint.x.wrapping_add(if dx == 0 {
                    w - (rect.extent.width >> 1)
                } else if dx > 0 {
                    w
                } else {
                    -w
                });
                crew_intersect.EndPoint.y = crew_intersect.EndPoint.y.wrapping_add(if dy == 0 {
                    h - (rect.extent.height >> 1)
                } else if dy > 0 {
                    h
                } else {
                    -h
                });
                crew_intersect.IntersectStamp.origin.x = crew_intersect.EndPoint.x.wrapping_add(dx);
                crew_intersect.IntersectStamp.origin.y = crew_intersect.EndPoint.y.wrapping_add(dy);
            }
            cx.projected_intersect(&mut crew_intersect, &mut intersect, MAX_TIME_VALUE as u16);
            cx.element_mut(crew).current.location.x =
                (i32::from(crew_intersect.EndPoint.x) << ONE_SHIFT) as i16;
            cx.element_mut(crew).current.location.y =
                (i32::from(crew_intersect.EndPoint.y) << ONE_SHIFT) as i16;
        }
    }
}

#[cfg(not(test))]
pub use engine::{
    abandon_ship, create_asteroid, crew_collision, crew_preprocess, spawn_asteroid, spawn_planet,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn damage_preserves_gravity_objects_and_byte_truncation() {
        let mut e = ElementRecord {
            hits: 200,
            mass: 100,
            life: 7,
            ..ElementRecord::default()
        };
        damage_object(e.mass, &mut e.hits, &mut e.life, &mut e.state_flags, 257);
        assert_eq!(e.hits, 199);
        e.mass = 101;
        damage_object(e.mass, &mut e.hits, &mut e.life, &mut e.state_flags, 255);
        assert_eq!(e.hits, 199);
        assert_eq!(e.life, 7);
        e.mass = 3;
        damage_object(e.mass, &mut e.hits, &mut e.life, &mut e.state_flags, 199);
        assert_eq!(e.hits, 0);
        assert_eq!(e.life, 0);
        assert_eq!(e.state_flags, NONSOLID as u16);
    }

    #[test]
    fn crew_motion_preserves_signed_wrapping_and_exact_half_extent_ties() {
        use crate::battle::element::Point;
        for (from, target, expected) in [
            (Point::new(0, 0), Point::new(1, -1), Point::new(4, -4)),
            (Point::new(0, 0), Point::new(512, -512), Point::new(4, -4)),
            (Point::new(0, 0), Point::new(513, -513), Point::new(-4, 4)),
            (
                Point::new(i16::MAX, i16::MIN),
                Point::new(i16::MIN, i16::MAX),
                Point::new(-32765, 32764),
            ),
        ] {
            let mut next = from;
            crew_step(
                (from.x, from.y),
                &mut next.x,
                &mut next.y,
                (target.x, target.y),
                (1024, 1024),
            );
            assert_eq!(next, expected, "from={from:?} target={target:?}");
        }
    }

    #[test]
    fn crew_colors_expand_rgb15_channels() {
        let low = crew_color(false);
        let high = crew_color(true);
        assert_eq!((low.r, low.g, low.b, low.a), (0, 165, 0, 255));
        assert_eq!((high.r, high.g, high.b, high.a), (82, 247, 82, 255));
    }
}
