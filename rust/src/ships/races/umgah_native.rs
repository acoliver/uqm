//! Umgah callbacks and descriptor translated from `umgah.c`.

#[cfg(not(test))]
pub(crate) mod engine {
    use crate::ships::native::{
        abi::*, angle_to_facing, cosine, facing_to_angle, normalize_angle, sine,
    };

    use crate::engine::battle::views::Missile;
    use crate::engine::battle::{
        Concerns, Cx, ElementId, Race, RaceState, RaceTemplate, ShipId, WeaponOrigin, WeaponSpawn,
    };
    pub static UMGAH: Race = Race {
        descriptor,
        preprocess: Some(umgah_preprocess),
        postprocess: Some(umgah_postprocess),
        init_weapon: Some(initialize_cone),
        intelligence: Some(umgah_intelligence),
        uninit: Some(uninit_umgah),
    };
    const MAX_CREW: u16 = 10;
    const MAX_ENERGY: u8 = 30;
    const MAX_THRUST: u16 = 18;
    const THRUST_INCREMENT: u16 = 6;
    const JUMP_DIST: i32 = 40 << ONE_SHIFT;
    const CLOSE_RANGE_WEAPON: u16 = 50 << ONE_SHIFT;
    const LONG_RANGE_WEAPON: u32 = 1000 << ONE_SHIFT;

    fn descriptor(_world: &mut crate::engine::battle::World) -> crate::engine::battle::RaceDesc {
        let mut d = RaceTemplate::default();
        d.info.ship_flags = (FIRES_FORE | IMMEDIATE_WEAPON) as u16;
        d.info.ship_cost = 7;
        d.info.crew_level = MAX_CREW;
        d.info.max_crew = MAX_CREW;
        d.info.energy_level = MAX_ENERGY;
        d.info.max_energy = MAX_ENERGY;
        d.resources.strings = Some(c"ship.umgah.text");
        d.resources.icons = Some(c"ship.umgah.icons");
        d.resources.melee_icon = Some(c"ship.umgah.meleeicons");
        d.fleet_strength = 833 / SPHERE_RADIUS_INCREMENT as u16 * 2;
        d.fleet_location = POINT { x: 1798, y: 6000 };
        d.characteristics = CHARACTERISTIC_STUFF {
            max_thrust: MAX_THRUST,
            thrust_increment: THRUST_INCREMENT,
            energy_regeneration: MAX_ENERGY,
            weapon_energy_cost: 0,
            special_energy_cost: 1,
            energy_wait: 150,
            turn_wait: 4,
            thrust_wait: 3,
            weapon_wait: 0,
            special_wait: 2,
            ship_mass: 1,
        };
        d.resources.ship = [
            Some(c"ship.umgah.graphics.drone.large"),
            Some(c"ship.umgah.graphics.drone.medium"),
            Some(c"ship.umgah.graphics.drone.small"),
        ];
        d.resources.weapon = [Some(c"ship.umgah.graphics.spritz"), None, None];
        d.resources.special = [
            Some(c"ship.umgah.graphics.cone.large"),
            Some(c"ship.umgah.graphics.cone.medium"),
            Some(c"ship.umgah.graphics.cone.small"),
        ];
        d.resources.captain = Some(c"ship.umgah.graphics.captain");
        d.resources.victory = Some(c"ship.umgah.ditty");
        d.resources.sounds = Some(c"ship.umgah.sounds");
        d.intelligence.weapon_range = (LONG_RANGE_WEAPON << 2) as u16;
        d.state = RaceState::Umgah {
            previous_facing: None,
        };
        d.descriptor(&UMGAH)
    }
    pub(crate) fn cone_preprocess(cx: &mut Cx<'_>, e: ElementId) {
        let ss = cx.owner(e);
        let frame = cx.relative_frame(
            cx.ship_frame(ss, 2),
            angle_to_facing(FULL_CIRCLE as i32) as i16,
        );
        cx.set_ship_frame(ss, 2, 0, frame);
        cx.element_mut(e).flags |= APPEARING as u16;
    }

    pub(crate) fn cone_collision(
        cx: &mut Cx<'_>,
        a: ElementId,
        pa: &mut POINT,
        b: ElementId,
        pb: &mut POINT,
    ) {
        if let Some(h) = cx.weapon_collision(a, pa, b, pb) {
            cx.unlink(h);
            cx.dispose(h);
            cx.element_mut(a).flags &= !(DISAPPEARING as u16);
        }
    }

    pub(crate) fn initialize_cone(
        cx: &mut Cx<'_>,
        ss: ShipId,
        origin: WeaponOrigin,
    ) -> WeaponSpawn {
        let facing = cx.ship_state(ss).facing;
        let mut b = Missile {
            cx: origin.next.location.x,
            cy: origin.next.location.y,
            farray: cx.ship_frame_set(ss, 2),
            face: facing,
            sender: origin.player,
            flags: IGNORE_SIMILAR as u16,
            pixoffs: 0,
            speed: 0,
            hit_points: 100,
            damage: 1,
            life: 1,
            preprocess: Some(cone_preprocess),
            blast_offs: 0,
            index: 0,
        };
        let RaceState::Umgah { previous_facing } = cx.race_stats(ss).state else {
            panic!("Umgah ship state");
        };
        if previous_facing != Some(facing) {
            cx.race_stats_mut(ss).state = RaceState::Umgah {
                previous_facing: Some(facing),
            };
            cx.set_ship_frame(ss, 2, 0, cx.absolute_frame(cx.ship_frame(ss, 2), facing));
        }
        b.index = cx.frame_number(cx.ship_frame(ss, 2));
        let out = cx.missile(&mut b);
        if let Some(cone) = out {
            cx.element_mut(cone).collide = Some(CollisionHook::Native(cone_collision));
            cx.element_mut(cone).flags &= !(APPEARING as u16);
            cx.copy_visual_to_next(cone, cx.element(cone).current);
            cx.init_intersect(cone);
            cx.set_intersect_frame(cone, cx.ship_frame(ss, 2));
        }
        vec![out]
    }

    pub(crate) fn umgah_preprocess(cx: &mut Cx<'_>, ss: ShipId) {
        let e = cx.ship_element(ss);
        if cx.element(e).flags & APPEARING as u16 != 0 {
            cx.race_stats_mut(ss).state = RaceState::Umgah {
                previous_facing: None,
            };
        } else if cx.element(e).thrust == 0
            && cx.ship_state(ss).status & SPECIAL as u16 != 0
            && cx.delta_energy(e, -1)
        {
            cx.process_ship_sound(ss, 1, e);
            let facing = facing_to_angle(i32::from(cx.ship_state(ss).facing)) + HALF_CIRCLE as i32;
            let speed = JUMP_DIST << VELOCITY_SHIFT;
            cx.element_mut(e).velocity.delta_components(
                i32::from((cosine(facing, speed)) as SIZE),
                i32::from((sine(facing, speed)) as SIZE),
            );
            cx.ship_state_mut(ss).status &= !((SHIP_AT_MAX_SPEED | SHIP_BEYOND_MAX_SPEED) as u16);
            cx.ship_state_mut(ss).special_counter = 2;
        }
    }

    pub(crate) fn umgah_postprocess(cx: &mut Cx<'_>, ss: ShipId) {
        let e = cx.ship_element(ss);
        if cx.ship_state(ss).special_counter > 0 {
            cx.ship_state_mut(ss).special_counter = 0;
            cx.element_mut(e).velocity.zero();
        }
    }

    pub(crate) fn uninit_umgah(
        _world: &mut crate::engine::battle::World,
        stats: &mut crate::engine::battle::views::RaceStats,
    ) {
        stats.state = RaceState::Umgah {
            previous_facing: None,
        };
    }

    pub(crate) fn umgah_intelligence(cx: &mut Cx<'_>, ss: ShipId, concerns: &mut Concerns) {
        let weapon = ENEMY_WEAPON_INDEX as usize;
        if concerns.objects[weapon].object.is_some()
            && concerns.objects[weapon].movement == MOVEMENT_STATE_ENTICE
        {
            let object = concerns.objects[weapon]
                .object
                .unwrap_or_else(|| panic!("weapon concern"));
            if concerns.objects[weapon].which_turn > 3
                || cx.ship_state(ss).old_status & SPECIAL as u16 != 0
            {
                concerns.objects[weapon].object = None;
            } else if cx.element(object).flags & FINITE_LIFE as u16 != 0
                && cx.element(object).flags & CREW_OBJECT as u16 == 0
            {
                concerns.objects[weapon].movement = MOVEMENT_STATE_AVOID;
            } else {
                concerns.objects[weapon].movement = MOVEMENT_STATE_PURSUE;
            }
        }
        let enemy = ENEMY_SHIP_INDEX as usize;
        let gravity = GRAVITY_MASS_INDEX as usize;
        if cx.ship_state(ss).special_counter != 0
            || concerns.objects[gravity].object.is_some()
            || concerns.objects[enemy].object.is_none()
        {
            cx.race_stats_mut(ss).intelligence.weapon_range = CLOSE_RANGE_WEAPON;
            cx.ship_intelligence(ss, concerns);
            if concerns.objects[enemy].which_turn < 16 {
                cx.ship_state_mut(ss).input |= WEAPON as u8;
            }
            cx.ship_state_mut(ss).input &= !(SPECIAL as u8);
        } else {
            let turn = concerns.objects[enemy].which_turn.min(255) as u8;
            let enough = ((JUMP_DIST * i32::from(cx.race_stats(ss).ship_info.energy_level)) >> 6)
                > i32::from(turn);
            let enemy_id = concerns.objects[enemy]
                .object
                .unwrap_or_else(|| panic!("enemy ship"));
            let location = cx.element(enemy_id).next.location;
            let dx = location.x.wrapping_sub(concerns.subject.next.location.x);
            let dy = location.y.wrapping_sub(concerns.subject.next.location.y);
            let behind = normalize_angle(
                crate::math::arctan(dx, dy) as i32
                    - (facing_to_angle(i32::from(cx.ship_state(ss).facing)) + HALF_CIRCLE as i32)
                    + OCTANT as i32
                    + (OCTANT >> 2) as i32,
            ) <= ((OCTANT as i32 + (OCTANT >> 2) as i32) << 1);
            let enemy_ss = cx.owner(enemy_id);
            cx.race_stats_mut(ss).intelligence.weapon_range = if enough
                && (cx.ship_state(ss).old_status & SPECIAL as u16 != 0
                    || behind
                    || (turn > 6
                        && cx.race_stats(enemy_ss).intelligence.maneuverability
                            <= SLOW_SHIP as u16)
                    || (16..=24).contains(&turn))
            {
                (LONG_RANGE_WEAPON << 3) as u16
            } else {
                CLOSE_RANGE_WEAPON
            };
            cx.ship_intelligence(ss, concerns);
            if cx.race_stats(ss).intelligence.weapon_range == CLOSE_RANGE_WEAPON {
                cx.ship_state_mut(ss).input &= !(SPECIAL as u8);
            } else {
                cx.ship_state_mut(ss).input &= !(THRUST as u8);
                let lined = concerns.subject.turn == 0
                    && cx.ship_state(ss).old_status & (LEFT | RIGHT) as u16 == 0;
                if (cx.ship_state(ss).old_status & SPECIAL as u16 != 0
                    && turn <= cx.race_stats(ss).characteristics.special_wait)
                    || (cx.ship_state(ss).old_status & SPECIAL as u16 == 0
                        && behind
                        && (lined || turn < 16))
                {
                    cx.ship_state_mut(ss).input |= SPECIAL as u8;
                    cx.race_stats_mut(ss).characteristics.special_wait = turn;
                    if turn <= 8 && lined {
                        if cx.random() & 1 != 0 {
                            cx.ship_state_mut(ss).input |= LEFT as u8;
                        } else {
                            cx.ship_state_mut(ss).input |= RIGHT as u8;
                        }
                    }
                } else if cx.ship_state(ss).old_status & SPECIAL as u16 != 0 {
                    cx.ship_state_mut(ss).input &= !((SPECIAL | LEFT | RIGHT) as u8);
                    cx.ship_state_mut(ss).input |= THRUST as u8;
                }
            }
            if turn < 16 && !behind {
                cx.ship_state_mut(ss).input |= WEAPON as u8;
            }
        }
        if cx.ship_state(ss).input & SPECIAL as u8 == 0 {
            cx.race_stats_mut(ss).characteristics.special_wait = 255;
        }
    }
}
