//! Thraddash callbacks and descriptor translated from `thradd.c`.

#[cfg(not(test))]
pub(crate) mod engine {
    use crate::engine::battle::views::Missile;
    use crate::engine::battle::{
        Concerns, Cx, ElementId, Race, RaceDesc, RaceTemplate, ShipId, WeaponOrigin, WeaponSpawn,
        World,
    };

    use crate::ships::native::{
        abi::*, angle_to_facing, facing_to_angle, normalize_angle, normalize_facing,
    };

    const MAX_CREW: u16 = 8;
    const MAX_ENERGY: u8 = 24;
    const MAX_THRUST: u16 = 28;
    const THRUST_INCREMENT: u16 = 7;
    const SPECIAL_THRUST_INCREMENT: u16 = 12;
    const SPECIAL_MAX_THRUST: u16 = 72;
    const LONG_RANGE_WEAPON: i32 = (30 << ONE_SHIFT) * 15 / 2;
    const MISSILE_SPEED: i16 = 30 << ONE_SHIFT;
    const MISSILE_LIFE: u16 = 15;
    const NAPALM_LIFE: u16 = 48;
    const NAPALM_DECAY_RATE: u8 = 5;
    const NUM_NAPALM_FADES: u16 = 6;

    pub static THRADDASH: Race = Race {
        descriptor,
        preprocess: Some(thraddash_preprocess),
        postprocess: None,
        init_weapon: Some(initialize_horn),
        intelligence: Some(thraddash_intelligence),
        uninit: None,
    };

    fn descriptor(_world: &mut World) -> RaceDesc {
        let mut d = RaceTemplate::default();
        d.info.ship_flags = FIRES_FORE as u16;
        d.info.ship_cost = 10;
        d.info.crew_level = MAX_CREW;
        d.info.max_crew = MAX_CREW;
        d.info.energy_level = MAX_ENERGY;
        d.info.max_energy = MAX_ENERGY;
        d.resources.strings = Some(c"ship.thraddash.text");
        d.resources.icons = Some(c"ship.thraddash.icons");
        d.resources.melee_icon = Some(c"ship.thraddash.meleeicons");
        d.fleet_strength = 833 / SPHERE_RADIUS_INCREMENT as u16 * 2;
        d.fleet_location = POINT { x: 2535, y: 8358 };
        d.characteristics = CHARACTERISTIC_STUFF {
            max_thrust: MAX_THRUST,
            thrust_increment: THRUST_INCREMENT,
            energy_regeneration: 1,
            weapon_energy_cost: 2,
            special_energy_cost: 1,
            energy_wait: 6,
            turn_wait: 1,
            thrust_wait: 0,
            weapon_wait: 12,
            special_wait: 0,
            ship_mass: 7,
        };
        d.resources.ship = [
            Some(c"ship.thraddash.graphics.torch.large"),
            Some(c"ship.thraddash.graphics.torch.medium"),
            Some(c"ship.thraddash.graphics.torch.small"),
        ];
        d.resources.weapon = [
            Some(c"ship.thraddash.graphics.horn.large"),
            Some(c"ship.thraddash.graphics.horn.medium"),
            Some(c"ship.thraddash.graphics.horn.small"),
        ];
        d.resources.special = [
            Some(c"ship.thraddash.graphics.napalm.large"),
            Some(c"ship.thraddash.graphics.napalm.medium"),
            Some(c"ship.thraddash.graphics.napalm.small"),
        ];
        d.resources.captain = Some(c"ship.thraddash.graphics.captain");
        d.resources.victory = Some(c"ship.thraddash.ditty");
        d.resources.sounds = Some(c"ship.thraddash.sounds");
        d.intelligence.weapon_range = LONG_RANGE_WEAPON as u16;
        d.descriptor(&THRADDASH)
    }

    pub(crate) fn flame_napalm_preprocess(cx: &mut Cx<'_>, e: ElementId) {
        cx.element_mut(e).velocity = crate::engine::battle::record::VELOCITY_DESC::default();
        if cx.element(e).flags & NONSOLID as u16 != 0 {
            cx.element_mut(e).flags &= !(NONSOLID as u16);
            cx.element_mut(e).flags |= APPEARING as u16;
            cx.primitive_style_mut(e).kind = gfx_object_STAMP_PRIM as GRAPHICS_PRIM;
            cx.init_intersect(e);
            let frame = cx.equivalent_frame(
                cx.element_frame_set_first(e, true),
                cx.element(e).next.image.frame,
            );
            cx.set_intersect_frame(e, frame);
        } else if cx.element(e).turn > 0 {
            cx.element_mut(e).turn -= 1;
        } else {
            if cx.element(e).life <= NUM_NAPALM_FADES * (NAPALM_DECAY_RATE as u16 + 1)
                || cx.frame_number(cx.element(e).current.image.frame) != NUM_NAPALM_FADES
            {
                cx.set_frame(e, true, cx.retreat_frame(cx.element(e).current.image.frame));
            } else if cx.element(e).life > NUM_NAPALM_FADES * (NAPALM_DECAY_RATE as u16 + 1) {
                cx.set_frame(
                    e,
                    true,
                    cx.absolute_frame(
                        cx.element(e).current.image.frame,
                        cx.frame_count(cx.element(e).current.image.frame)
                            .wrapping_sub(1),
                    ),
                );
            }
            cx.element_mut(e).turn = NAPALM_DECAY_RATE;
            cx.element_mut(e).flags |= CHANGING as u16;
        }
    }

    pub(crate) fn initialize_horn(
        cx: &mut Cx<'_>,
        ss: ShipId,
        origin: WeaponOrigin,
    ) -> WeaponSpawn {
        let mut b = Missile {
            cx: origin.next.location.x,
            cy: origin.next.location.y,
            farray: cx.ship_frame_set(ss, 1),
            face: cx.ship_state(ss).facing,
            index: cx.ship_state(ss).facing,
            sender: origin.player,
            flags: IGNORE_SIMILAR as u16,
            pixoffs: 9,
            speed: MISSILE_SPEED,
            hit_points: 2,
            damage: 1,
            life: MISSILE_LIFE,
            preprocess: None,
            blast_offs: 3,
        };
        vec![cx.missile(&mut b)]
    }

    pub(crate) fn thraddash_preprocess(cx: &mut Cx<'_>, ss: ShipId) {
        let e = cx.ship_element(ss);
        if cx.ship_state(ss).status & SPECIAL as u16 == 0 {
            if cx.ship_state(ss).old_status & SPECIAL as u16 != 0
                && cx.ship_state(ss).status & SHIP_AT_MAX_SPEED as u16 != 0
            {
                cx.ship_state_mut(ss).status |= SHIP_BEYOND_MAX_SPEED as u16;
            }
            return;
        }
        if !cx.delta_energy(e, -1) {
            return;
        }
        if cx.ship_state(ss).old_status & SPECIAL as u16 == 0 {
            cx.ship_state_mut(ss).status &= !((SHIP_AT_MAX_SPEED | SHIP_BEYOND_MAX_SPEED) as u16);
        }
        if cx.element(e).thrust == 0 {
            cx.element_mut(e).thrust += 1;
        }
        let inc = cx.race_stats(ss).characteristics.thrust_increment;
        let max = cx.race_stats(ss).characteristics.max_thrust;
        cx.race_stats_mut(ss).characteristics.thrust_increment = SPECIAL_THRUST_INCREMENT;
        cx.race_stats_mut(ss).characteristics.max_thrust = SPECIAL_MAX_THRUST;
        let status = cx.apply_inertial_thrust(e);
        cx.ship_state_mut(ss).status &=
            !((SHIP_AT_MAX_SPEED | SHIP_BEYOND_MAX_SPEED | SHIP_IN_GRAVITY_WELL) as u16);
        cx.ship_state_mut(ss).status |= status;
        cx.race_stats_mut(ss).characteristics.thrust_increment = inc;
        cx.race_stats_mut(ss).characteristics.max_thrust = max;
        let mut b = Missile {
            cx: cx.element(e).next.location.x,
            cy: cx.element(e).next.location.y,
            farray: cx.ship_frame_set(ss, 2),
            face: 0,
            index: cx.frame_count(cx.ship_frame(ss, 2)).wrapping_sub(1),
            sender: cx.element(e).player,
            flags: IGNORE_SIMILAR as u16,
            pixoffs: 0,
            speed: 0,
            hit_points: 1,
            damage: 2,
            life: NAPALM_LIFE,
            preprocess: Some(flame_napalm_preprocess),
            blast_offs: 0,
        };
        if let Some(trail) = cx.missile(&mut b) {
            cx.set_parent(trail, Some(ss));
            cx.set_target(trail, None);
            cx.element_mut(trail).turn = NAPALM_DECAY_RATE;
            cx.element_mut(trail).flags |= NONSOLID as u16;
            cx.primitive_style_mut(trail).kind = gfx_object_NUM_PRIMS as GRAPHICS_PRIM;
            cx.copy_visual_to_next(trail, cx.element(trail).current);
            cx.element_mut(trail).flags |= PRE_PROCESS as u16;
            cx.insert_before(trail, cx.elements().head());
            cx.process_ship_sound(ss, 1, e);
        }
    }

    fn entice_boost(
        facing: COUNT,
        input: u8,
        origin: POINT,
        opponent: crate::engine::battle::views::ElementState,
        which_turn: COUNT,
    ) -> bool {
        let dx = opponent.next.location.x.wrapping_sub(origin.x);
        let dy = opponent.next.location.y.wrapping_sub(origin.y);
        let dir = crate::math::arctan(dx, dy);
        (which_turn > 24 && input & (LEFT | RIGHT) as u8 == 0)
            || (which_turn <= 16
                && normalize_angle(
                    dir as i32 - (facing_to_angle(i32::from(facing)) + HALF_CIRCLE as i32)
                        + QUADRANT as i32,
                ) <= HALF_CIRCLE as i32
                && (which_turn < 12
                    || normalize_angle(
                        dir as i32 - (opponent.velocity.travel_angle as i32 + HALF_CIRCLE as i32)
                            + OCTANT as i32
                            + 2,
                    ) <= (OCTANT as i32 + 2) * 2))
    }

    pub(crate) fn thraddash_intelligence(cx: &mut Cx<'_>, ss: ShipId, concerns: &mut Concerns) {
        let enemy = ENEMY_SHIP_INDEX as usize;
        if let Some(target) = concerns.objects[enemy].object {
            let opponent = cx.element(target);
            let (vx, vy) = opponent.velocity.get_current_components();
            let dx = vx as SIZE;
            let dy = vy as SIZE;
            let stationary = (4 << ONE_SHIFT) << VELOCITY_SHIFT;
            let speed2 = i64::from(dx) * i64::from(dx) + i64::from(dy) * i64::from(dy);
            concerns.objects[enemy].movement = if concerns.objects[enemy].which_turn > 8
                || speed2 <= i64::from(stationary) * i64::from(stationary)
            {
                MOVEMENT_STATE_PURSUE
            } else {
                MOVEMENT_STATE_ENTICE
            };
        }
        cx.ship_intelligence(ss, concerns);
        if cx.ship_state(ss).special_counter != 0 {
            return;
        }
        cx.ship_state_mut(ss).input &= !(SPECIAL as u8);
        let weapon = ENEMY_WEAPON_INDEX as usize;
        if let (Some(threat), MOVEMENT_STATE_ENTICE) = (
            concerns.objects[weapon].object,
            concerns.objects[weapon].movement,
        ) {
            if cx.ship_state(ss).input & THRUST as u8 != 0
                || (concerns.subject.turn == 0
                    && cx.ship_state(ss).input & (LEFT | RIGHT) as u8 == 0)
                || normalize_facing(
                    angle_to_facing(
                        i32::from(cx.element(threat).velocity.travel_angle)
                            + HALF_CIRCLE as i32
                            + OCTANT as i32,
                    ) - i32::from(cx.ship_state(ss).facing),
                ) > angle_to_facing(QUADRANT as i32) as u16
            {
                cx.ship_state_mut(ss).input |= SPECIAL as u8;
            }
        } else if let Some(target) = concerns.objects[enemy].object {
            let opponent = cx.element(target);
            if concerns.objects[enemy].movement == MOVEMENT_STATE_PURSUE {
                if cx.race_stats(ss).ship_info.energy_level >= 3
                    && concerns.subject.turn == 0
                    && cx.ship_state(ss).input & (LEFT | RIGHT) as u8 == 0
                    && (cx.ship_state(ss).status & SPECIAL as u16 == 0
                        || cx.ship_state(ss).status
                            & (SHIP_AT_MAX_SPEED | SHIP_BEYOND_MAX_SPEED) as u16
                            == 0)
                {
                    cx.ship_state_mut(ss).input |= SPECIAL as u8;
                }
            } else if concerns.objects[enemy].movement == MOVEMENT_STATE_ENTICE
                && entice_boost(
                    cx.ship_state(ss).facing,
                    cx.ship_state(ss).input,
                    concerns.subject.next.location,
                    opponent,
                    concerns.objects[enemy].which_turn,
                )
            {
                cx.ship_state_mut(ss).input |= SPECIAL as u8;
            }
        }
        if cx.ship_state(ss).input & SPECIAL as u8 != 0
            && cx.race_stats(ss).ship_info.energy_level >= 1
        {
            cx.ship_state_mut(ss).input &= !(THRUST as u8);
        }
    }
}
