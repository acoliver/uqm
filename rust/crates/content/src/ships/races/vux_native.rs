//! VUX behavior over the battle context's ids and per-ship descriptor.
pub mod engine {
    fn build_descriptor(
        _world: &mut crate::engine::battle::World,
    ) -> crate::engine::battle::RaceDesc {
        crate::engine::battle::vux_template::descriptor(&VUX)
    }
    use crate::engine::battle::ai_constants::{
        ENEMY_SHIP_INDEX, ENEMY_WEAPON_INDEX, IGNORE_SIMILAR, LEFT, MOVEMENT_STATE_AVOID,
        MOVEMENT_STATE_ENTICE, MOVEMENT_STATE_PURSUE, RIGHT, SPECIAL,
    };
    use crate::engine::battle::flags::{
        APPEARING, CHANGING, COLLISION, CREW_OBJECT, DISAPPEARING, FINITE_LIFE, PLAYER_SHIP,
    };
    use crate::engine::battle::CollisionHook;
    use crate::engine::battle::{Concerns, Cx, ElementId, Race, ShipId, WeaponOrigin, WeaponSpawn};
    use crate::engine::world::activity::activity_kinds::IN_ENCOUNTER;
    use crate::graphics::geometry::Color;
    use crate::graphics::geometry::Point;
    use crate::math::battle_types::{HALF_CIRCLE, ONE_SHIFT};
    use crate::ships::native::{angle_to_facing, cosine, facing_to_angle, normalize_facing, sine};
    const MODIFY_IMAGE: u32 = 1;

    pub static VUX: Race = Race {
        descriptor: build_descriptor,
        preprocess: Some(vux_preprocess),
        postprocess: Some(vux_postprocess),
        init_weapon: Some(initialize_horrific_laser),
        intelligence: Some(vux_intelligence),
        uninit: None,
    };

    pub fn limpet_preprocess(cx: &mut Cx<'_>, e: ElementId) {
        let original = normalize_facing(angle_to_facing(i32::from(
            cx.element(e).velocity.travel_angle,
        )));
        let mut face = original;
        let delta = cx.track_ship(e, &mut face);
        if delta > 0 {
            face = (i32::from(original) + i32::from(delta)) as u16;
            cx.element_mut(e).velocity.set_vector(25, face);
        }
        let frame = cx.advance_frame(cx.element(e).next.image.frame.view());
        cx.set_frame(e, true, frame);
        cx.element_mut(e).flags |= CHANGING as u16;
    }

    pub fn limpet_collision(
        cx: &mut Cx<'_>,
        a: ElementId,
        _pa: &mut Point,
        b: ElementId,
        _pb: &mut Point,
    ) {
        if cx.element(b).flags & PLAYER_SHIP as u16 != 0 {
            let other = cx.owner(b);
            let mut d = cx.race_stats_mut(other);
            d.characteristics.turn_wait = d.characteristics.turn_wait.wrapping_add(1);
            if d.characteristics.turn_wait == 0 {
                d.characteristics.turn_wait = d.characteristics.turn_wait.wrapping_sub(1);
            }
            d.characteristics.thrust_wait = d.characteristics.thrust_wait.wrapping_add(1);
            if d.characteristics.thrust_wait == 0 {
                d.characteristics.thrust_wait = d.characteristics.thrust_wait.wrapping_sub(1);
            }
            if d.characteristics.thrust_increment <= 1 << ONE_SHIFT {
                d.characteristics.max_thrust = d.characteristics.thrust_increment.wrapping_shl(1);
            } else {
                let n = d.characteristics.max_thrust / d.characteristics.thrust_increment;
                d.characteristics.thrust_increment -= 1;
                d.characteristics.max_thrust = d.characteristics.thrust_increment.wrapping_mul(n);
            }
            d.intelligence.maneuverability = 0;
            drop(d);
            let ss = cx.owner(a);
            cx.process_ship_sound(ss, 2, b);
            let index = cx.random() as u16;
            let frame = cx.absolute_frame(cx.ship_frame(ss, 1), index);
            cx.silhouette_frame(b, frame, MODIFY_IMAGE as u8);
        }
        cx.element_mut(a).set_hit_points(0);
        cx.element_mut(a).set_life_span(0);
        cx.element_mut(a).flags |= (COLLISION | DISAPPEARING) as u16;
    }

    pub fn spawn_limpets(cx: &mut Cx<'_>, e: ElementId) {
        let ss = cx.owner(e);
        let mut block = crate::engine::battle::views::Missile {
            frames: cx.ship_frame_set(ss, 2),
            // The original expression adds HALF_CIRCLE directly to the facing.
            face: (i32::from(cx.ship_state(ss).facing) + HALF_CIRCLE as i32) as u16,
            index: 0,
            sender: cx.element(e).player,
            flags: IGNORE_SIMILAR as u16,
            pixoffs: 8,
            speed: 25,
            hit_points: 1,
            damage: 0,
            life: 80,
            preprocess: Some(limpet_preprocess),
            blast_offs: 0,
            cx: cx.element(e).next.location.x,
            cy: cx.element(e).next.location.y,
        };
        if let Some(limpet) = cx.missile(&mut block) {
            cx.element_mut(limpet).collide = Some(CollisionHook::Native(limpet_collision));
            cx.set_parent(limpet, Some(ss));
            cx.put(limpet);
        }
    }

    pub fn initialize_horrific_laser(
        cx: &mut Cx<'_>,
        ss: ShipId,
        origin: WeaponOrigin,
    ) -> WeaponSpawn {
        let face = cx.ship_state(ss).facing;
        let mut block = crate::engine::battle::views::Laser {
            face,
            origin: origin.next.location,
            end: Point {
                x: cosine(facing_to_angle(i32::from(face)), 162 << ONE_SHIFT),
                y: sine(facing_to_angle(i32::from(face)), 162 << ONE_SHIFT),
            },
            sender: origin.player,
            flags: IGNORE_SIMILAR as u16,
            pixoffs: 12,
            color: Color {
                r: 82,
                g: 255,
                b: 82,
                a: 255,
            },
        };
        vec![cx.laser(&mut block)]
    }

    pub fn vux_intelligence(cx: &mut Cx<'_>, ss: ShipId, concerns: &mut Concerns) {
        let enemy = ENEMY_SHIP_INDEX as usize;
        let weapon = ENEMY_WEAPON_INDEX as usize;
        concerns.objects[enemy].movement = MOVEMENT_STATE_PURSUE;
        if let Some(threat) = concerns.objects[weapon].object {
            if concerns.objects[weapon].movement == MOVEMENT_STATE_ENTICE {
                let flags = cx.element(threat).flags;
                concerns.objects[weapon].movement =
                    if flags & FINITE_LIFE as u16 != 0 && flags & CREW_OBJECT as u16 == 0 {
                        MOVEMENT_STATE_AVOID
                    } else {
                        MOVEMENT_STATE_PURSUE
                    };
            }
        }
        cx.ship_intelligence(ss, concerns);
        if cx.ship_state(ss).special_counter == 0
            && concerns.objects[enemy].object.is_some()
            && concerns.objects[enemy].which_turn <= 12
            && cx.ship_state(ss).input & (LEFT | RIGHT) as u8 != 0
            && cx.race_stats(ss).ship_info.energy_level
                >= cx.race_stats(ss).ship_info.max_energy >> 1
        {
            cx.ship_state_mut(ss).input |= SPECIAL as u8;
        } else {
            cx.ship_state_mut(ss).input &= !(SPECIAL as u8);
        }
    }

    pub fn vux_postprocess(cx: &mut Cx<'_>, ss: ShipId) {
        let e = cx.ship_element(ss);
        if cx.ship_state(ss).status & SPECIAL as u16 != 0
            && cx.ship_state(ss).special_counter == 0
            && cx.delta_energy(e, -2)
        {
            cx.process_ship_sound(ss, 1, e);
            spawn_limpets(cx, e);
            cx.ship_state_mut(ss).special_counter = cx.race_stats(ss).characteristics.special_wait;
        }
    }

    pub fn vux_preprocess(cx: &mut Cx<'_>, ss: ShipId) {
        let e = cx.ship_element(ss);
        if cx.element(e).flags & APPEARING as u16 != 0 {
            let mut face = cx.ship_state(ss).facing;
            if cx.world.activity() as u8 != IN_ENCOUNTER as u8 && cx.track_ship(e, &mut face) >= 0 {
                let other = cx
                    .target(e)
                    .unwrap_or_else(|| panic!("{}", "track_ship found no target"));
                const ENTRY: i32 = ((150 + 12 + 46) * 2) << ONE_SHIFT;
                loop {
                    cx.element_mut(e).current.location.x =
                        (i32::from(cx.element(other).current.location.x) - (ENTRY >> 1)
                            + i32::from(cx.random() as u16) % ENTRY) as i16;
                    cx.element_mut(e).current.location.y =
                        (i32::from(cx.element(other).current.location.y) - (ENTRY >> 1)
                            + i32::from(cx.random() as u16) % ENTRY) as i16;
                    let dx = cx
                        .element(other)
                        .current
                        .location
                        .x
                        .wrapping_sub(cx.element(e).current.location.x);
                    let dy = cx
                        .element(other)
                        .current
                        .location
                        .y
                        .wrapping_sub(cx.element(e).current.location.y);
                    face =
                        normalize_facing(angle_to_facing(i32::from(crate::math::arctan(dx, dy))));
                    let frame = cx.absolute_frame(cx.element(e).current.image.frame.view(), face);
                    cx.set_frame(e, false, frame);
                    let x = i32::from(cx.element(e).current.location.x) & !((1 << ONE_SHIFT) - 1);
                    let y = i32::from(cx.element(e).current.location.y) & !((1 << ONE_SHIFT) - 1);
                    let (pixel_width, pixel_height) = cx.screen_dimensions();
                    let width = (pixel_width - 64) << (ONE_SHIFT + 3);
                    let height = pixel_height << (ONE_SHIFT + 3);
                    cx.element_mut(e).current.location.x = (if x < 0 {
                        x + width
                    } else if x >= width {
                        x - width
                    } else {
                        x
                    }) as u16 as i16;
                    cx.element_mut(e).current.location.y = (if y < 0 {
                        y + height
                    } else if y >= height {
                        y - height
                    } else {
                        y
                    }) as u16 as i16;
                    if !cx.gravity(e) && !cx.matter_conflict(e) {
                        break;
                    }
                }
                cx.set_target(e, None);
                let current = cx.element(e).current;
                cx.copy_visual_to_next(e, current);
                cx.init_intersect(e);
                let frame = cx.equivalent_frame(
                    cx.element_frame_set_first(e, true),
                    cx.element(e).next.image.frame.view(),
                );
                cx.set_intersect_frame(e, frame);
                cx.ship_state_mut(ss).facing = face;
            }
            cx.set_preprocess(ss, crate::engine::battle::Selection::Disabled);
        }
    }
}
