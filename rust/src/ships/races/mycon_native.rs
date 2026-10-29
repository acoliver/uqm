//! Mycon Podship callbacks and descriptor ported from `sc2/src/uqm/ships/mycon/mycon.c`.

#[cfg(not(test))]
pub(crate) mod engine {
    use crate::ships::native::{abi::*, angle_to_facing, normalize_facing};

    const MAX_CREW: u16 = 20;
    const MAX_ENERGY: u8 = 40;
    const ENERGY_REGENERATION: u8 = 1;
    const ENERGY_WAIT: u8 = 4;
    const MAX_THRUST: u16 = 27;
    const THRUST_INCREMENT: u16 = 9;
    const THRUST_WAIT: u8 = 6;
    const TURN_WAIT: u8 = 6;
    const SHIP_MASS: u8 = 7;
    const WEAPON_ENERGY_COST: u8 = 20;
    const WEAPON_WAIT: u8 = 5;
    const MYCON_OFFSET: i16 = 24;
    const MISSILE_OFFSET: i16 = 0;
    const NUM_PLASMAS: i32 = 11;
    const NUM_GLOBALLS: i32 = 8;
    const PLASMA_DURATION: i32 = 13;
    const MISSILE_LIFE: i32 = NUM_PLASMAS * PLASMA_DURATION;
    const MISSILE_SPEED: i16 = 8 << ONE_SHIFT;
    const MISSILE_DAMAGE: i32 = 10;
    const TRACK_WAIT: u8 = 1;
    const SPECIAL_ENERGY_COST: u8 = MAX_ENERGY;
    const REGENERATION_AMOUNT: i32 = 4;

    fn descriptor(_world: &mut crate::engine::battle::World) -> crate::engine::battle::RaceDesc {
        let mut d = RaceTemplate::default();
        d.info.ship_flags = (FIRES_FORE | SEEKING_WEAPON) as u16;
        d.info.ship_cost = 21;
        d.info.crew_level = MAX_CREW;
        d.info.max_crew = MAX_CREW;
        d.info.energy_level = MAX_ENERGY;
        d.info.max_energy = MAX_ENERGY;
        d.resources.strings = Some(c"ship.mycon.text");
        d.resources.icons = Some(c"ship.mycon.icons");
        d.resources.melee_icon = Some(c"ship.mycon.meleeicons");
        d.fleet_strength = (1070 / SPHERE_RADIUS_INCREMENT * 2) as u16;
        d.fleet_location = POINT { x: 6392, y: 2200 };
        d.characteristics = CHARACTERISTIC_STUFF {
            max_thrust: MAX_THRUST,
            thrust_increment: THRUST_INCREMENT,
            energy_regeneration: ENERGY_REGENERATION,
            weapon_energy_cost: WEAPON_ENERGY_COST,
            special_energy_cost: SPECIAL_ENERGY_COST,
            energy_wait: ENERGY_WAIT,
            turn_wait: TURN_WAIT,
            thrust_wait: THRUST_WAIT,
            weapon_wait: WEAPON_WAIT,
            special_wait: 0,
            ship_mass: SHIP_MASS,
        };
        d.resources.ship = [
            Some(c"ship.mycon.graphics.podship.large"),
            Some(c"ship.mycon.graphics.podship.medium"),
            Some(c"ship.mycon.graphics.podship.small"),
        ];
        d.resources.weapon = [
            Some(c"ship.mycon.graphics.plasma.large"),
            Some(c"ship.mycon.graphics.plasma.medium"),
            Some(c"ship.mycon.graphics.plasma.small"),
        ];
        d.resources.captain = Some(c"ship.mycon.graphics.captain");
        d.resources.victory = Some(c"ship.mycon.ditty");
        d.resources.sounds = Some(c"ship.mycon.sounds");
        d.intelligence.weapon_range = 800 << ONE_SHIFT;
        d.descriptor(&MYCON)
    }
    use crate::engine::battle::views::Missile;
    use crate::engine::battle::{
        Concerns, Cx, ElementId, Race, RaceTemplate, ShipId, WeaponOrigin, WeaponSpawn,
    };
    pub static MYCON: Race = Race {
        descriptor,
        preprocess: None,
        postprocess: Some(mycon_postprocess),
        init_weapon: Some(initialize_plasma),
        intelligence: Some(mycon_intelligence),
        uninit: None,
    };
    pub(crate) fn plasma_preprocess(cx: &mut Cx<'_>, e: ElementId) {
        if u16::from(cx.element(e).mass) > cx.element(e).hits {
            cx.element_mut(e).life = cx.element(e).hits * PLASMA_DURATION as u16;
        } else {
            cx.element_mut(e).hits =
                (((i32::from(cx.element(e).life) * MISSILE_DAMAGE + MISSILE_LIFE - 1)
                    / MISSILE_LIFE) as u8) as _;
        }
        cx.element_mut(e).mass = cx.element(e).hits as u8;
        let index = (NUM_PLASMAS
            - ((i32::from(cx.element(e).life) + PLASMA_DURATION - 1) / PLASMA_DURATION))
            as u16;
        if index != cx.frame_number(cx.element(e).next.image.frame) {
            cx.set_frame(
                e,
                true,
                cx.absolute_frame(cx.element(e).next.image.frame, index),
            );
            cx.element_mut(e).flags |= CHANGING as u16;
        }
        if cx.element(e).turn > 0 {
            cx.element_mut(e).turn -= 1;
        } else {
            let mut face = normalize_facing(angle_to_facing(i32::from(
                cx.element(e).velocity.travel_angle,
            )));
            if cx.track_ship(e, &mut face) > 0 {
                cx.element_mut(e)
                    .velocity
                    .set_vector(i32::from(MISSILE_SPEED), face);
            }
            cx.element_mut(e).turn = TRACK_WAIT;
        }
    }

    pub(crate) fn plasma_blast_preprocess(cx: &mut Cx<'_>, e: ElementId) {
        if cx.element(e).life >= u16::from(cx.element(e).thrust) {
            cx.set_frame(e, true, cx.advance_frame(cx.element(e).next.image.frame));
        } else {
            cx.set_frame(e, true, cx.retreat_frame(cx.element(e).next.image.frame));
        }
        if cx.target(e).is_some() {
            let target = cx.target(e).unwrap_or_else(|| panic!("tracked target"));
            cx.element_mut(e).next.location = cx.element(target).next.location;
        }
        cx.element_mut(e).flags |= CHANGING as u16;
    }

    pub(crate) fn plasma_collision(
        cx: &mut Cx<'_>,
        e0: ElementId,
        p0: &mut POINT,
        e1: ElementId,
        p1: &mut POINT,
    ) {
        let old_mass = cx.element(e0).mass as i16;
        let h = if cx.parent(e0) != cx.parent(e1) || cx.element(e1).flags & PLAYER_SHIP as u16 != 0
        {
            cx.weapon_collision(e0, p0, e1, p1)
        } else {
            None
        };
        if h.is_none() {
            return;
        }
        let blast = h.unwrap_or_else(|| panic!("allocated element"));
        cx.set_parent(blast, Some(cx.owner(e0)));
        if cx.element(e1).flags & PLAYER_SHIP as u16 == 0 {
            cx.set_target(blast, None);
        } else {
            cx.set_target(blast, Some(cx.ship_element(cx.owner(e1))));
        }
        cx.element_mut(blast).current.location = cx.element(e1).current.location;
        let mut animations =
            ((i32::from(old_mass) * NUM_GLOBALLS + MISSILE_DAMAGE - 1) / MISSILE_DAMAGE) as u16;
        if animations == 0 {
            animations = 1;
        }
        cx.element_mut(blast).thrust = animations as u8;
        cx.element_mut(blast).life = (animations << 1) - 1;
        cx.element_mut(blast).pre = Some(ElementHook::Native(plasma_blast_preprocess));
        cx.set_frames(blast, false, cx.element(e0).next.image.frames);
        cx.set_frame(
            blast,
            false,
            cx.absolute_frame(cx.element_frame_set_first(blast, false), NUM_PLASMAS as u16),
        );
    }

    pub(crate) fn mycon_intelligence(cx: &mut Cx<'_>, ss: ShipId, concerns: &mut Concerns) {
        let enemy_weapon = ENEMY_WEAPON_INDEX as usize;
        if concerns.objects[enemy_weapon].object.is_some()
            && concerns.objects[enemy_weapon].movement == MOVEMENT_STATE_ENTICE
        {
            if cx
                .element(
                    concerns.objects[enemy_weapon]
                        .object
                        .unwrap_or_else(|| panic!("AI concern")),
                )
                .flags
                & FINITE_LIFE as u16
                != 0
                && cx
                    .element(
                        concerns.objects[enemy_weapon]
                            .object
                            .unwrap_or_else(|| panic!("AI concern")),
                    )
                    .flags
                    & CREW_OBJECT as u16
                    == 0
            {
                concerns.objects[enemy_weapon].movement = MOVEMENT_STATE_AVOID;
            } else {
                concerns.objects[enemy_weapon].movement = MOVEMENT_STATE_PURSUE;
            }
        }
        cx.ship_intelligence(ss, concerns);
        if concerns.objects[ENEMY_WEAPON_INDEX as usize].movement == MOVEMENT_STATE_PURSUE {
            cx.ship_state_mut(ss).input &= !(THRUST as u8);
        }
        let enemy = ENEMY_SHIP_INDEX as usize;
        if cx.ship_state(ss).weapon_counter == 0
            && concerns.objects[enemy].object.is_some()
            && (concerns.objects[enemy].which_turn <= 16
                || concerns.subject.hits == cx.race_stats(ss).ship_info.max_crew)
        {
            let travel = normalize_facing(angle_to_facing(
                i32::from(concerns.subject.velocity.travel_angle) + HALF_CIRCLE as i32,
            ));
            let target = concerns.objects[enemy]
                .object
                .unwrap_or_else(|| panic!("AI concern"));
            let dx = (i32::from(cx.element(target).current.location.x)
                - i32::from(concerns.subject.current.location.x)) as SIZE;
            let dy = (i32::from(cx.element(target).current.location.y)
                - i32::from(concerns.subject.current.location.y)) as SIZE;
            let direction =
                normalize_facing(angle_to_facing(i32::from(crate::math::arctan(dx, dy))));
            if normalize_facing(
                i32::from(direction) - i32::from(cx.ship_state(ss).facing)
                    + angle_to_facing(QUADRANT as i32),
            ) <= angle_to_facing(HALF_CIRCLE as i32) as u16
                && (cx.ship_state(ss).status
                    & (SHIP_BEYOND_MAX_SPEED | SHIP_IN_GRAVITY_WELL) as u16
                    == 0
                    || normalize_facing(
                        i32::from(direction) - i32::from(travel) + angle_to_facing(OCTANT as i32),
                    ) <= angle_to_facing(QUADRANT as i32) as u16)
            {
                cx.ship_state_mut(ss).input |= WEAPON as u8;
            }
        }
        if cx.ship_state(ss).special_counter == 0 {
            cx.ship_state_mut(ss).input &= !(SPECIAL as u8);
            cx.race_stats_mut(ss).intelligence.weapon_range = 800 << ONE_SHIFT;
            if concerns.subject.hits < cx.race_stats(ss).ship_info.max_crew {
                cx.race_stats_mut(ss).intelligence.weapon_range =
                    (i32::from(MISSILE_SPEED) * MISSILE_LIFE) as u16;
                if cx.race_stats(ss).ship_info.energy_level >= SPECIAL_ENERGY_COST
                    && cx.ship_state(ss).input & WEAPON as u8 == 0
                {
                    cx.ship_state_mut(ss).input |= SPECIAL as u8;
                }
            }
        }
    }

    pub(crate) fn initialize_plasma(
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
            flags: 0,
            pixoffs: MYCON_OFFSET,
            speed: MISSILE_SPEED,
            hit_points: MISSILE_DAMAGE as i16,
            damage: MISSILE_DAMAGE as i16,
            life: MISSILE_LIFE as u16,
            preprocess: Some(plasma_preprocess),
            blast_offs: MISSILE_OFFSET,
        };
        out.push(cx.missile(&mut block));
        if let Some(p) = out[0] {
            cx.element_mut(p).collide = Some(CollisionHook::Native(plasma_collision));
            cx.element_mut(p).turn = TRACK_WAIT + 2;
        }
        out
    }

    pub(crate) fn mycon_postprocess(cx: &mut Cx<'_>, ss: ShipId) {
        let e = cx.ship_element(ss);
        if cx.ship_state(ss).status & SPECIAL as u16 != 0
            && cx.ship_state(ss).special_counter == 0
            && cx.element(e).hits != cx.race_stats(ss).ship_info.max_crew
            && cx.delta_energy(e, -(SPECIAL_ENERGY_COST as SIZE))
        {
            cx.process_ship_sound(ss, 1, e);
            let mut add = REGENERATION_AMOUNT;
            let headroom =
                i32::from(cx.race_stats(ss).ship_info.max_crew) - i32::from(cx.element(e).hits);
            if add > headroom {
                add = headroom;
            }
            cx.delta_crew(e, add as SIZE);
            cx.ship_state_mut(ss).special_counter = cx.race_stats(ss).characteristics.special_wait;
        }
    }
}
