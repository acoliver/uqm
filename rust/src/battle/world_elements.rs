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
    use crate::ships::native::{
        angle_to_facing, cosine, element, normalize_facing, put_element, sine, starship,
    };
    use std::mem;

    extern "C" {
        static mut planet: [*mut crate::graphics::native_abi::frame_desc; 3];
    }

    unsafe fn primitive(e: *mut ELEMENT) -> *mut PRIMITIVE {
        (&raw mut DisplayArray)
            .cast::<PRIMITIVE>()
            .add((*e).PrimIndex as usize)
    }
    pub(crate) fn gravity_postprocess(cx: &mut Cx<'_>, e: ElementId) {
        cx.gravity(e);
    }

    /// # Safety
    /// The element pool and planet image must be initialized on the game thread.
    #[no_mangle]
    pub unsafe extern "C" fn spawn_planet() {
        let handle = AllocElement();
        if handle.is_null() {
            return;
        }
        let e = element(handle);
        (*e).playerNr = NEUTRAL_PLAYER_NUM as i16;
        (*e).__bindgen_anon_2.hit_points = 200;
        (*e).state_flags = APPEARING as u16;
        (*e).__bindgen_anon_1.life_span = (NORMAL_LIFE + 1) as u16;
        (*primitive(e)).Type = gfx_object_STAMP_PRIM as u8;
        image!(set; (*e).current.image.farray, (&raw mut planet).cast::<FRAME>());
        image!(frame; (*e).current.image.frame, *(*e).current.image.farray);
        (*e).collision_func = Some(CollisionHook::Native(collision));
        (*e).postprocess_func = Some(ElementHook::Native(gravity_postprocess));
        (*e).velocity = mem::zeroed();
        loop {
            (*e).current.location.x = aligned_random(TFB_Random(), width(ScreenWidth));
            (*e).current.location.y = aligned_random(TFB_Random(), height(ScreenHeight));
            if CalculateGravity(e) == 0 && TimeSpaceMatterConflict(e) == 0 {
                break;
            }
        }
        (*e).__bindgen_anon_3.mass_points = (*e).__bindgen_anon_2.hit_points as u8;
        put_element(handle);
    }

    pub(crate) fn spawn_rubble(cx: &mut Cx<'_>, source: ElementId) {
        let Some(e) = cx.alloc_element() else {
            return;
        };
        cx.put(e);
        cx.b.elements[e].playerNr = cx.b.elements[source].playerNr;
        cx.b.elements[e].state_flags = (APPEARING | FINITE_LIFE | NONSOLID) as u16;
        cx.b.elements[e].set_life_span(5);
        cx.b.elements[e].set_turn_wait(0);
        cx.b.elements[e].set_thrust_wait(0);
        cx.primitive(e).Type = gfx_object_STAMP_PRIM as u8;
        let array = cx.shared_frames(SharedFrames::Asteroid);
        image!(set; cx.b.elements[e].current.image.farray, array);
        let frame = cx.abs_frame(
            cx.shared_frame(SharedFrames::Asteroid),
            angle_to_facing(FULL_CIRCLE as i32) as u16,
        );
        image!(frame; cx.b.elements[e].current.image.frame, frame);
        cx.b.elements[e].current.location = cx.b.elements[source].current.location;
        cx.b.elements[e].preprocess_func = Some(ElementHook::Native(animation_preprocess));
        cx.b.elements[e].death_func = Some(ElementHook::Native(spawn_asteroid));
    }

    pub(crate) fn asteroid_preprocess(cx: &mut Cx<'_>, e: ElementId) {
        let wait = cx.b.elements[e].turn_wait();
        if wait > 0 {
            cx.b.elements[e].set_turn_wait(wait - 1);
        } else {
            let index = cx.frame_index(cx.b.elements[e].current.image.frame);
            let index = if cx.b.elements[e].thrust_wait() & 128 != 0 {
                index.wrapping_sub(1)
            } else {
                index.wrapping_add(1)
            };
            let frame = cx.abs_frame(
                cx.b.elements[e].current.image.frame,
                normalize_facing(index as i32),
            );
            image!(frame; cx.b.elements[e].next.image.frame, frame);
            cx.b.elements[e].state_flags |= CHANGING as u16;
            let wait = cx.b.elements[e].thrust_wait() & 127;
            cx.b.elements[e].set_turn_wait(wait);
        }
    }

    pub fn spawn_asteroid(cx: &mut Cx<'_>, source: ElementId) {
        create_asteroid(cx, Some(source));
    }

    pub fn create_asteroid(cx: &mut Cx<'_>, source: Option<ElementId>) {
        let Some(e) = cx.alloc_element() else {
            if let Some(source) = source {
                cx.b.elements[source].state_flags &= !(DISAPPEARING as u16);
                cx.primitive(source).Type = gfx_object_NUM_PRIMS as u8;
                cx.b.elements[source].set_life_span(1);
            }
            return;
        };
        cx.b.elements[e].playerNr = NEUTRAL_PLAYER_NUM as i16;
        cx.b.elements[e].set_hit_points(1);
        cx.b.elements[e].set_mass_points(3);
        cx.b.elements[e].state_flags = APPEARING as u16;
        cx.b.elements[e].set_life_span(NORMAL_LIFE as u16);
        cx.primitive(e).Type = gfx_object_STAMP_PRIM as u8;
        let value = cx.random() as u16;
        let (w, h) = cx.screen_dimensions();
        if value & 1 != 0 {
            cx.b.elements[e].current.location.x = if value & 2 == 0 { 0 } else { width(w) as i16 };
            cx.b.elements[e].current.location.y = aligned_random(cx.random(), height(h));
        } else {
            cx.b.elements[e].current.location.x = aligned_random(cx.random(), width(w));
            cx.b.elements[e].current.location.y = if value & 2 == 0 { 0 } else { height(h) as i16 };
        }
        let magnitude = (((cx.random() as i16 & 7) + 4) as i32) << ONE_SHIFT;
        let facing = cx.random() as u16;
        cx.b.elements[e]
            .velocity
            .set_vector(i32::from(magnitude as i16), facing);
        let array = cx.shared_frames(SharedFrames::Asteroid);
        image!(set; cx.b.elements[e].current.image.farray, array);
        let facing = cx.random() as u16 & ((1 << FACING_SHIFT) - 1) as u16;
        let frame = cx.abs_frame(cx.shared_frame(SharedFrames::Asteroid), facing);
        image!(frame; cx.b.elements[e].current.image.frame, frame);
        let wait = cx.random() as u8 & 3;
        cx.b.elements[e].set_turn_wait(wait);
        let thrust_wait = wait | (cx.random() as u8 & 128);
        cx.b.elements[e].set_thrust_wait(thrust_wait);
        cx.b.elements[e].preprocess_func = Some(ElementHook::Native(asteroid_preprocess));
        cx.b.elements[e].death_func = Some(ElementHook::Native(spawn_rubble));
        cx.b.elements[e].collision_func = Some(CollisionHook::Native(collision));
        cx.put(e);
    }

    /// # Safety
    /// Element and its owning STARSHIP, for player ships, must be live.
    #[no_mangle]
    pub unsafe extern "C" fn do_damage(e: *mut ELEMENT, damage: i16) {
        crate::app::bridge::enter(|_cx| {
            if (*e).state_flags & PLAYER_SHIP as u16 != 0 {
                if DeltaCrew(e, damage.wrapping_neg()) == 0 {
                    (*e).__bindgen_anon_1.life_span = 0;
                    (*e).state_flags |= NONSOLID as u16;
                }
            } else {
                damage_object(
                    (*e).__bindgen_anon_3.mass_points,
                    &mut (*e).__bindgen_anon_2.hit_points,
                    &mut (*e).__bindgen_anon_1.life_span,
                    &mut (*e).state_flags,
                    damage,
                );
            }
        });
    }

    impl Cx<'_> {
        pub fn damage(&mut self, e: ElementId, damage: i16) {
            if self.b.elements[e].state_flags & PLAYER_SHIP as u16 != 0 {
                if !self.delta_crew(e, damage.wrapping_neg()) {
                    self.b.elements[e].set_life_span(0);
                    self.b.elements[e].state_flags |= NONSOLID as u16;
                }
            } else {
                let mut hp = self.b.elements[e].hit_points();
                let mut life = self.b.elements[e].life_span();
                let mut flags = self.b.elements[e].state_flags;
                damage_object(
                    self.b.elements[e].mass_points(),
                    &mut hp,
                    &mut life,
                    &mut flags,
                    damage,
                );
                self.b.elements[e].set_hit_points(hp);
                self.b.elements[e].set_life_span(life);
                self.b.elements[e].state_flags = flags;
            }
        }
    }

    pub fn crew_preprocess(cx: &mut Cx<'_>, e: ElementId) {
        let old = cx.primitive(e).color;
        let low = crew_color(false);
        let high = old.r == low.r && old.g == low.g && old.b == low.b && old.a == low.a;
        cx.primitive(e).color = crew_color(high);
        cx.b.elements[e].state_flags |= CHANGING as u16;
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
            let current = cx.b.elements[e].current.location;
            let target = cx.b.elements[ship].current.location;
            let (w, h) = cx.screen_dimensions();
            let next = &mut cx.b.elements[e].next.location;
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
        if cx.b.elements[hit].state_flags & PLAYER_SHIP as u16 != 0
            && cx.b.elements[hit].life_span() >= NORMAL_LIFE as u16
            && cx.b.elements[crew].hit_points() > 0
        {
            let ship = cx.owner(hit);
            if cx.desc(ship).ship_info.ship_flags & CREW_IMMUNE as u16 == 0 {
                cx.game_sound(0, hit);
                cx.delta_crew(hit, 1);
            }
        }
        cx.b.elements[crew].set_hit_points(0);
        cx.b.elements[crew].set_life_span(0);
        cx.b.elements[crew].state_flags |= (COLLISION | DISAPPEARING | NONSOLID) as u16;
    }

    /// # Safety
    /// Ship/target elements, descriptors and intersect frames must be live.
    #[no_mangle]
    pub unsafe extern "C" fn AbandonShip(ship: *mut ELEMENT, target: *mut ELEMENT, mut loss: u16) {
        let mut owner = starship(ship);
        if (*(*owner).RaceDescPtr).ship_info.ship_flags & CREW_IMMUNE as u16 != 0 {
            return;
        }
        let mut intersect = (*ship).IntersectControl;
        let mut rect: RECT = mem::zeroed();
        GetFrameRect(intersect.IntersectStamp.frame, &mut rect);
        let direction = (*ship).velocity.travel_angle;
        let (dx, dy) = if direction == FULL_CIRCLE as u16 {
            (0, 0)
        } else {
            let direction = direction.wrapping_add(HALF_CIRCLE as u16);
            (cosine(direction as i32, 100), sine(direction as i32, 100))
        };
        while loss != 0 {
            loss -= 1;
            let handle = AllocElement();
            if handle.is_null() {
                break;
            }
            DeltaCrew(ship, -1);
            put_element(handle);
            let crew = element(handle);
            (*crew).playerNr = NEUTRAL_PLAYER_NUM as i16;
            (*crew).__bindgen_anon_2.hit_points = 1;
            (*crew).state_flags = (APPEARING | FINITE_LIFE | CREW_OBJECT) as u16;
            (*crew).__bindgen_anon_1.life_span = 300;
            (*primitive(crew)).Type = gfx_object_POINT_PRIM as u8;
            (*primitive(crew)).color = crew_color(false);
            image!(frame; (*crew).current.image.frame, DecFrameIndex(stars_in_space));
            image!(set; (*crew).current.image.farray, &raw mut stars_in_space);
            (*crew).preprocess_func = Some(ElementHook::Native(crew_preprocess));
            (*crew).collision_func = Some(CollisionHook::Native(crew_collision));
            (*crew).pParent = owner.cast();
            // The original loop retains this reassignment for subsequent crew.
            owner = starship(target);
            (*crew).hTarget = (*owner).hShip;
            intersect.IntersectStamp.origin = (*ship).IntersectControl.EndPoint;
            let w = (i32::from(TFB_Random() as u16) % i32::from(rect.extent.width)) as i16;
            let h = (i32::from(TFB_Random() as u16) % i32::from(rect.extent.height)) as i16;
            let mut crew_intersect: INTERSECT_CONTROL = mem::zeroed();
            crew_intersect.EndPoint = intersect.EndPoint;
            image!(frame; crew_intersect.IntersectStamp.frame, DecFrameIndex(stars_in_space));
            if dx == 0 && dy == 0 {
                crew_intersect.EndPoint.x = crew_intersect
                    .EndPoint
                    .x
                    .wrapping_add(w - (rect.extent.width >> 1));
                crew_intersect.EndPoint.y = crew_intersect
                    .EndPoint
                    .y
                    .wrapping_add(h - (rect.extent.height >> 1));
                crew_intersect.IntersectStamp.origin = (*target).IntersectControl.EndPoint;
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
            DrawablesIntersect(&mut crew_intersect, &mut intersect, MAX_TIME_VALUE as u16);
            (*crew).current.location.x = (i32::from(crew_intersect.EndPoint.x) << ONE_SHIFT) as i16;
            (*crew).current.location.y = (i32::from(crew_intersect.EndPoint.y) << ONE_SHIFT) as i16;
        }
    }
}

#[cfg(not(test))]
pub use engine::{
    create_asteroid, crew_collision, crew_preprocess, do_damage, spawn_asteroid, spawn_planet,
    AbandonShip,
};

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem;

    #[test]
    fn damage_preserves_gravity_objects_and_byte_truncation() {
        unsafe {
            let mut e: ELEMENT = mem::zeroed();
            e.__bindgen_anon_2.hit_points = 200;
            e.__bindgen_anon_3.mass_points = 100;
            e.__bindgen_anon_1.life_span = 7;
            damage_object(
                e.__bindgen_anon_3.mass_points,
                &mut e.__bindgen_anon_2.hit_points,
                &mut e.__bindgen_anon_1.life_span,
                &mut e.state_flags,
                257,
            );
            assert_eq!(e.__bindgen_anon_2.hit_points, 199);
            e.__bindgen_anon_3.mass_points = 101;
            damage_object(
                e.__bindgen_anon_3.mass_points,
                &mut e.__bindgen_anon_2.hit_points,
                &mut e.__bindgen_anon_1.life_span,
                &mut e.state_flags,
                255,
            );
            assert_eq!(e.__bindgen_anon_2.hit_points, 199);
            assert_eq!(e.__bindgen_anon_1.life_span, 7);
            e.__bindgen_anon_3.mass_points = 3;
            damage_object(
                e.__bindgen_anon_3.mass_points,
                &mut e.__bindgen_anon_2.hit_points,
                &mut e.__bindgen_anon_1.life_span,
                &mut e.state_flags,
                199,
            );
            assert_eq!(e.__bindgen_anon_2.hit_points, 0);
            assert_eq!(e.__bindgen_anon_1.life_span, 0);
            assert_eq!(e.state_flags, NONSOLID as u16);
        }
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
