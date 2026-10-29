//! Live yehat callbacks translated from ships/yehat/yehat.c.
#[cfg(not(test))]
pub(crate) mod engine {
    use crate::ships::native::{abi::*, cosine, facing_to_angle, sine};
    fn descriptor(_world: &mut crate::engine::battle::World) -> crate::engine::battle::RaceDesc {
        let mut d = RaceTemplate::default();
        d.info.ship_flags = (FIRES_FORE as i32 | SHIELD_DEFENSE as i32) as _;
        d.info.ship_cost = 23;
        d.info.crew_level = 20;
        d.info.max_crew = 20;
        d.info.energy_level = 10;
        d.info.max_energy = 10;
        d.resources.strings = Some(c"ship.yehat.text");
        d.resources.icons = Some(c"ship.yehat.icons");
        d.resources.melee_icon = Some(c"ship.yehat.meleeicons");
        d.fleet_strength = (750 / SPHERE_RADIUS_INCREMENT as i32 * 2) as u16;
        d.fleet_location = POINT { x: 4970, y: 40 };
        d.characteristics.max_thrust = 30;
        d.characteristics.thrust_increment = 6;
        d.characteristics.energy_regeneration = 2;
        d.characteristics.weapon_energy_cost = 1;
        d.characteristics.special_energy_cost = 3;
        d.characteristics.energy_wait = 6;
        d.characteristics.turn_wait = 2;
        d.characteristics.thrust_wait = 2;
        d.characteristics.weapon_wait = 0;
        d.characteristics.special_wait = 2;
        d.characteristics.ship_mass = 3;
        d.resources.ship = [
            Some(c"ship.yehat.graphics.terminator.large"),
            Some(c"ship.yehat.graphics.terminator.medium"),
            Some(c"ship.yehat.graphics.terminator.small"),
        ];
        d.resources.weapon = [
            Some(c"ship.yehat.graphics.missile.large"),
            Some(c"ship.yehat.graphics.missile.medium"),
            Some(c"ship.yehat.graphics.missile.small"),
        ];
        d.resources.special = [
            Some(c"ship.yehat.graphics.shield.large"),
            Some(c"ship.yehat.graphics.shield.medium"),
            Some(c"ship.yehat.graphics.shield.small"),
        ];
        d.resources.captain = Some(c"ship.yehat.graphics.captain");
        d.resources.victory = Some(c"ship.yehat.ditty");
        d.resources.sounds = Some(c"ship.yehat.sounds");
        d.intelligence.weapon_range = (((20) << ONE_SHIFT as i32) * (10) / 3) as u16;
        d.descriptor(&YEHAT)
    }
    use crate::engine::battle::views::Missile;
    use crate::engine::battle::{
        Concerns, Cx, Race, RaceTemplate, ShipId, WeaponOrigin, WeaponSpawn,
    };
    pub static YEHAT: Race = Race {
        descriptor,
        preprocess: Some(yehat_preprocess),
        postprocess: Some(yehat_postprocess),
        init_weapon: Some(initialize_standard_missiles),
        intelligence: Some(yehat_intelligence),
        uninit: None,
    };
    pub(crate) fn initialize_standard_missiles(
        cx: &mut Cx<'_>,
        ss: ShipId,
        origin: WeaponOrigin,
    ) -> WeaponSpawn {
        let mut out = Vec::new();
        let mut block = Missile {
            farray: cx.ship_frame_set(ss, 1),
            face: cx.ship_state(ss).facing,
            index: cx.ship_state(ss).facing,
            sender: origin.player,
            flags: IGNORE_SIMILAR as u16,
            pixoffs: 16,
            speed: 20 << ONE_SHIFT,
            hit_points: 1,
            damage: 1,
            life: 10,
            preprocess: None,
            blast_offs: 1,
            cx: 0,
            cy: 0,
        };
        let x = sine(facing_to_angle(i32::from(block.face)), 8 << ONE_SHIFT).wrapping_neg();
        let y = cosine(facing_to_angle(i32::from(block.face)), 8 << ONE_SHIFT);
        block.cx = origin.next.location.x.wrapping_add(x);
        block.cy = origin.next.location.y.wrapping_add(y);
        out.push(cx.missile(&mut block));
        block.cx = origin.next.location.x.wrapping_sub(x);
        block.cy = origin.next.location.y.wrapping_sub(y);
        out.push(cx.missile(&mut block));
        out
    }
    pub(crate) fn yehat_intelligence(cx: &mut Cx<'_>, ss: ShipId, concerns: &mut Concerns) {
        let mut shield: SIZE = -1;
        let weapon = &mut concerns.objects[ENEMY_WEAPON_INDEX as usize];
        if let Some(object) = weapon
            .object
            .filter(|_| weapon.movement == MOVEMENT_STATE_ENTICE)
        {
            shield = 0;
            let threat = cx.element(object);
            if threat.flags & (FINITE_LIFE | CREW_OBJECT) as u16 == 0 {
                weapon.movement = MOVEMENT_STATE_PURSUE;
            } else if threat.mass != 0 || threat.flags & CREW_OBJECT as u16 != 0 {
                if threat.flags & FINITE_LIFE as u16 == 0 {
                    weapon.which_turn = weapon.which_turn.wrapping_shl(1);
                } else {
                    weapon.which_turn >>= 1;
                    if weapon.which_turn == 0 {
                        weapon.which_turn = 1;
                    }
                    if threat.mass != 0 {
                        weapon.object = None;
                    } else {
                        weapon.movement = MOVEMENT_STATE_PURSUE;
                    }
                }
                shield = 1;
            }
        }
        let object = weapon.object;
        if cx.ship_state(ss).special_counter == 0 {
            cx.ship_state_mut(ss).input &= !(SPECIAL as u8);
            if shield != 0 {
                if u32::from(concerns.subject.life) <= NORMAL_LIFE + 1
                    && (shield > 0 || weapon.object.is_some())
                    && weapon.which_turn <= 2
                    && (shield > 0
                        || cx
                            .element(object.unwrap_or_else(|| panic!("AI concern")))
                            .flags
                            & PLAYER_SHIP as u16
                            != 0
                        || cx.plot_intercept(
                            &cx.weapon_origin(object.unwrap_or_else(|| panic!("AI concern"))),
                            &concerns.subject,
                            2,
                            0,
                        ) != 0)
                    && cx.random() & 3 != 0
                {
                    cx.ship_state_mut(ss).input |= SPECIAL as u8;
                }
                if weapon
                    .object
                    .is_some_and(|object| cx.element(object).flags & CREW_OBJECT as u16 == 0)
                {
                    weapon.object = None;
                }
            }
        }
        let enemy = &mut concerns.objects[ENEMY_SHIP_INDEX as usize];
        if let Some(object) = enemy.object {
            let other = cx.owner(object);
            if cx.race_stats(other).ship_info.ship_flags & IMMEDIATE_WEAPON as u16 == 0 {
                enemy.movement = MOVEMENT_STATE_PURSUE;
            }
        }
        cx.ship_intelligence(ss, concerns);
    }
    pub(crate) fn yehat_postprocess(cx: &mut Cx<'_>, ss: ShipId) {
        let e = cx.ship_element(ss);
        if cx.element(e).flags & NONSOLID as u16 == 0 {
            if cx.ship_state(ss).special_counter > 0 {
                if u32::from(cx.element(e).life) == NORMAL_LIFE {
                    cx.ship_state_mut(ss).special_counter = 0;
                } else {
                    cx.process_ship_sound(ss, 1, e);
                    cx.delta_energy(e, -3);
                }
            }
        }
    }
    pub(crate) fn yehat_preprocess(cx: &mut Cx<'_>, ss: ShipId) {
        let e = cx.ship_element(ss);
        if cx.element(e).flags & APPEARING as u16 == 0 {
            let expired = if u32::from(cx.element(e).life) > NORMAL_LIFE {
                cx.element_mut(e).life -= 1;
                u32::from(cx.element(e).life) == NORMAL_LIFE
            } else {
                false
            };
            if expired
                || (u32::from(cx.element(e).life) == NORMAL_LIFE
                    && cx.element(e).next.image.frames == cx.ship_frame_set(ss, 2))
            {
                cx.set_frames(e, true, cx.ship_frame_set(ss, 0));
                cx.set_frame(
                    e,
                    true,
                    cx.equivalent_frame(cx.ship_frame(ss, 0), cx.element(e).next.image.frame),
                );
                cx.element_mut(e).flags |= CHANGING as u16;
            }
            if cx.ship_state(ss).status & SPECIAL as u16 != 0
                && cx.ship_state(ss).special_counter == 0
            {
                if cx.race_stats(ss).ship_info.energy_level < 3 {
                    cx.delta_energy(e, -3);
                } else {
                    cx.element_mut(e).life = (10 + NORMAL_LIFE) as COUNT;
                    cx.set_frames(e, true, cx.ship_frame_set(ss, 2));
                    cx.set_frame(
                        e,
                        true,
                        cx.equivalent_frame(cx.ship_frame(ss, 2), cx.element(e).next.image.frame),
                    );
                    cx.element_mut(e).flags |= CHANGING as u16;
                    cx.ship_state_mut(ss).special_counter =
                        cx.race_stats(ss).characteristics.special_wait;
                }
            }
        }
    }
}
