//! Live zoqfotpik callbacks translated from ships/zoqfot/zoqfot.c.
#[cfg(not(test))]
pub(crate) mod engine {
    use crate::ships::native::{
        abi::*, angle_to_facing, cosine, facing_to_angle, normalize_facing, sine,
    };
    fn descriptor(_world: &mut crate::engine::battle::World) -> crate::engine::battle::RaceDesc {
        let mut d = RaceTemplate::default();
        d.info.ship_flags = (FIRES_FORE as i32) as _;
        d.info.ship_cost = 6;
        d.info.crew_level = 10;
        d.info.max_crew = 10;
        d.info.energy_level = 10;
        d.info.max_energy = 10;
        d.resources.strings = Some(c"ship.zoqfotpik.text");
        d.resources.icons = Some(c"ship.zoqfotpik.icons");
        d.resources.melee_icon = Some(c"ship.zoqfotpik.meleeicons");
        d.fleet_strength = (320 / SPHERE_RADIUS_INCREMENT as i32 * 2) as u16;
        d.fleet_location = POINT { x: 3761, y: 5333 };
        d.characteristics.max_thrust = 40;
        d.characteristics.thrust_increment = 10;
        d.characteristics.energy_regeneration = 1;
        d.characteristics.weapon_energy_cost = 1;
        d.characteristics.special_energy_cost = ((10) * 3 / 4) as _;
        d.characteristics.energy_wait = 4;
        d.characteristics.turn_wait = 1;
        d.characteristics.thrust_wait = 0;
        d.characteristics.weapon_wait = 0;
        d.characteristics.special_wait = 6;
        d.characteristics.ship_mass = 5;
        d.resources.ship = [
            Some(c"ship.zoqfotpik.graphics.stinger.large"),
            Some(c"ship.zoqfotpik.graphics.stinger.medium"),
            Some(c"ship.zoqfotpik.graphics.stinger.small"),
        ];
        d.resources.weapon = [
            Some(c"ship.zoqfotpik.graphics.spit.large"),
            Some(c"ship.zoqfotpik.graphics.spit.medium"),
            Some(c"ship.zoqfotpik.graphics.spit.small"),
        ];
        d.resources.special = [
            Some(c"ship.zoqfotpik.graphics.proboscis.large"),
            Some(c"ship.zoqfotpik.graphics.proboscis.medium"),
            Some(c"ship.zoqfotpik.graphics.proboscis.small"),
        ];
        d.resources.captain = Some(c"ship.zoqfotpik.graphics.captain");
        d.resources.victory = Some(c"ship.zoqfotpik.ditty");
        d.resources.sounds = Some(c"ship.zoqfotpik.sounds");
        d.intelligence.weapon_range = (((10) << ONE_SHIFT as i32) * (10)) as u16;
        d.descriptor(&ZOQFOT)
    }
    use crate::engine::battle::views::Missile;
    use crate::engine::battle::{
        Concerns, Cx, ElementId, Race, RaceTemplate, ShipId, WeaponOrigin, WeaponSpawn,
    };
    pub static ZOQFOT: Race = Race {
        descriptor,
        preprocess: None,
        postprocess: Some(zoqfotpik_postprocess),
        init_weapon: Some(initialize_spit),
        intelligence: Some(zoqfotpik_intelligence),
        uninit: None,
    };
    pub(crate) fn spit_preprocess(cx: &mut Cx<'_>, e: ElementId) {
        if cx.element(e).turn > 0 {
            cx.element_mut(e).turn -= 1;
        } else {
            cx.set_frame(e, true, cx.advance_frame(cx.element(e).next.image.frame));
            let mut angle = cx.element(e).velocity.travel_angle;
            let index = cx.frame_number(cx.element(e).next.image.frame);
            if index == 1 {
                angle = (i32::from(angle) + (i32::from(cx.random() as COUNT) % 3) - 1) as COUNT;
            }
            let speed = (((i32::from(cx.frame_count(cx.element(e).next.image.frame))
                - i32::from(index))
                << ONE_SHIFT)
                << 1)
                << VELOCITY_SHIFT;
            let speed = speed as COUNT;
            cx.element_mut(e).velocity.set_components(
                i32::from(cosine(i32::from(angle), i32::from(speed))),
                i32::from(sine(i32::from(angle), i32::from(speed))),
            );
            cx.element_mut(e).turn = 2;
            cx.element_mut(e).flags |= CHANGING as u16;
        }
    }
    pub(crate) fn initialize_spit(
        cx: &mut Cx<'_>,
        ss: ShipId,
        origin: WeaponOrigin,
    ) -> WeaponSpawn {
        let mut out = Vec::new();
        let mut block = Missile {
            cx: origin.next.location.x,
            cy: origin.next.location.y,
            farray: cx.ship_frame_set(ss, 1),
            face: cx.ship_state(ss).facing,
            index: 0,
            sender: origin.player,
            flags: IGNORE_SIMILAR as u16,
            pixoffs: 13,
            speed: ((i32::from(cx.frame_count(cx.ship_frame(ss, 1))) << ONE_SHIFT) << 1) as SIZE,
            hit_points: 1,
            damage: 1,
            life: 10,
            preprocess: Some(spit_preprocess),
            blast_offs: 0,
        };
        out.push(cx.missile(&mut block));
        out
    }
    pub(crate) fn tongue_postprocess(cx: &mut Cx<'_>, e: ElementId) {
        if cx.element(e).turn != 0 {
            spawn_tongue(cx, e);
        }
    }
    pub(crate) fn tongue_collision(
        cx: &mut Cx<'_>,
        a: ElementId,
        pa: &mut POINT,
        b: ElementId,
        pb: &mut POINT,
    ) {
        let ss = cx.owner(a);
        if cx.ship_state(ss).special_counter == cx.race_stats(ss).characteristics.special_wait {
            cx.weapon_collision(a, pa, b, pb);
        }
        cx.ship_state_mut(ss).special_counter = cx
            .ship_state(ss)
            .special_counter
            .wrapping_sub(cx.element(a).turn);
        cx.element_mut(a).turn = 0;
        cx.element_mut(a).flags |= NONSOLID as u16;
    }
    pub(crate) fn spawn_tongue(cx: &mut Cx<'_>, e: ElementId) {
        let ss = cx.owner(e);
        let mut block = Missile {
            cx: cx.element(e).next.location.x,
            cy: cx.element(e).next.location.y,
            farray: cx.ship_frame_set(ss, 2),
            face: cx.ship_state(ss).facing,
            index: cx.ship_state(ss).facing,
            sender: cx.element(e).player,
            flags: IGNORE_SIMILAR as u16,
            pixoffs: 0,
            speed: 0,
            hit_points: 1,
            damage: 12,
            life: 1,
            preprocess: None,
            blast_offs: 4,
        };
        let h = cx.missile(&mut block);
        if let Some(tongue) = h {
            cx.element_mut(tongue).post = Some(ElementHook::Native(tongue_postprocess));
            cx.element_mut(tongue).collide = Some(CollisionHook::Native(tongue_collision));
            if cx.element(e).flags & PLAYER_SHIP as u16 != 0 {
                cx.element_mut(tongue).turn = cx.ship_state(ss).special_counter;
            } else {
                cx.element_mut(tongue).turn = cx.element(e).turn.wrapping_sub(1);
                let r = cx.frame_rect(cx.element(tongue).current.image.frame);
                let x = ((i32::from(r.extent.width) >> 1) << ONE_SHIFT) as SIZE;
                let y = ((i32::from(r.extent.height) >> 1) << ONE_SHIFT) as SIZE;
                let angle = facing_to_angle(i32::from(cx.ship_state(ss).facing)) as COUNT;
                if u32::from(angle) > HALF_CIRCLE && u32::from(angle) < FULL_CIRCLE {
                    cx.element_mut(tongue).current.location.x =
                        cx.element(tongue).current.location.x.wrapping_sub(x);
                } else if angle > 0 && u32::from(angle) < HALF_CIRCLE {
                    cx.element_mut(tongue).current.location.x =
                        cx.element(tongue).current.location.x.wrapping_add(x);
                }
                if u32::from(angle) < QUADRANT || u32::from(angle) > FULL_CIRCLE - QUADRANT {
                    cx.element_mut(tongue).current.location.y =
                        cx.element(tongue).current.location.y.wrapping_sub(y);
                } else if u32::from(angle) > QUADRANT && u32::from(angle) < FULL_CIRCLE - QUADRANT {
                    cx.element_mut(tongue).current.location.y =
                        cx.element(tongue).current.location.y.wrapping_add(y);
                }
            }
            cx.set_parent(tongue, Some(ss));
            cx.put(tongue);
        }
    }
    pub(crate) fn zoqfotpik_intelligence(cx: &mut Cx<'_>, ss: ShipId, concerns: &mut Concerns) {
        let mut tongue = false;
        if cx.ship_state(ss).special_counter == 0 {
            cx.ship_state_mut(ss).input &= !(SPECIAL as u8);
            let enemy = ENEMY_SHIP_INDEX as usize;
            if concerns.objects[enemy].object.is_some() && concerns.objects[enemy].which_turn <= 4 {
                tongue = true;
                concerns.objects[enemy].movement = MOVEMENT_STATE_PURSUE;
                let dx = cx
                    .element(
                        concerns.objects[enemy]
                            .object
                            .unwrap_or_else(|| panic!("AI concern")),
                    )
                    .next
                    .location
                    .x
                    .wrapping_sub(concerns.subject.next.location.x);
                let dy = cx
                    .element(
                        concerns.objects[enemy]
                            .object
                            .unwrap_or_else(|| panic!("AI concern")),
                    )
                    .next
                    .location
                    .y
                    .wrapping_sub(concerns.subject.next.location.y);
                if cx.ship_state(ss).facing
                    == normalize_facing(angle_to_facing(i32::from(crate::math::arctan(dx, dy))))
                {
                    cx.ship_state_mut(ss).input |= SPECIAL as u8;
                }
            }
        }
        cx.ship_state_mut(ss).weapon_counter = cx.ship_state(ss).weapon_counter.wrapping_add(1);
        cx.ship_intelligence(ss, concerns);
        cx.ship_state_mut(ss).weapon_counter = cx.ship_state(ss).weapon_counter.wrapping_sub(1);
        if cx.ship_state(ss).weapon_counter == 0 {
            cx.ship_state_mut(ss).input &= !(WEAPON as u8);
            if !tongue {
                let mut count = concerns.count;
                let mut eval = count as usize;
                while count != 0 {
                    count -= 1;
                    eval -= 1;
                    if concerns.objects[eval].object.is_some()
                        && (u32::from(count) == ENEMY_SHIP_INDEX
                            || (u32::from(count) == ENEMY_WEAPON_INDEX
                                && concerns.objects[eval].movement != MOVEMENT_STATE_AVOID))
                        && cx.ship_weapons(
                            &concerns.subject,
                            &cx.weapon_origin(
                                concerns.objects[eval]
                                    .object
                                    .unwrap_or_else(|| panic!("AI concern")),
                            ),
                            20 << ONE_SHIFT,
                        ) != 0
                    {
                        cx.ship_state_mut(ss).input |= WEAPON as u8;
                        break;
                    }
                }
            }
        }
    }
    pub(crate) fn zoqfotpik_postprocess(cx: &mut Cx<'_>, ss: ShipId) {
        let e = cx.ship_element(ss);
        if cx.ship_state(ss).status & SPECIAL as u16 != 0
            && cx.ship_state(ss).special_counter == 0
            && cx.delta_energy(e, -7)
        {
            cx.process_ship_sound(ss, 1, e);
            cx.ship_state_mut(ss).special_counter = cx.race_stats(ss).characteristics.special_wait;
        }
        if cx.ship_state(ss).special_counter != 0 {
            spawn_tongue(cx, e);
        }
    }
}
