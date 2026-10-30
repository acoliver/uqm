//! Supox callbacks used by the live battle engine.
//!
//! Every value and expression below mirrors `sc2/src/uqm/ships/supox/supox.c`,
//! including C integer promotion and truncation, so battles replay identically.

#[cfg(not(test))]
pub(crate) mod engine {
    use crate::ships::native::abi::*;

    // Core characteristics
    const MAX_CREW: u16 = 12;
    const MAX_ENERGY: u8 = 16;
    const ENERGY_REGENERATION: u8 = 1;
    const ENERGY_WAIT: u8 = 4;
    const MAX_THRUST: u16 = 40;
    const THRUST_INCREMENT: u16 = 8;
    const THRUST_WAIT: u8 = 0;
    const TURN_WAIT: u8 = 1;
    const SHIP_MASS: u8 = 4;

    // Gob launcher
    const WEAPON_ENERGY_COST: u8 = 1;
    const WEAPON_WAIT: u8 = 2;
    const SUPOX_OFFSET: i16 = 23;
    const MISSILE_OFFSET: i16 = 2;
    /// DISPLAY_TO_WORLD (30)
    const MISSILE_SPEED: i16 = 30 << ONE_SHIFT;
    const MISSILE_LIFE: u16 = 10;
    const MISSILE_HITS: i16 = 1;
    const MISSILE_DAMAGE: i16 = 1;

    // Lateral/reverse thrust
    const SPECIAL_ENERGY_COST: u8 = 1;
    const SPECIAL_WAIT: u8 = 0;

    const FULL_CIRCLE_ANGLE: i32 = FULL_CIRCLE as i32;
    const HALF_CIRCLE_ANGLE: i32 = HALF_CIRCLE as i32;
    const QUADRANT_ANGLE: i32 = QUADRANT as i32;
    const OCTANT_ANGLE: i32 = OCTANT as i32;
    const FACING_MASK: i32 = (1 << FACING_SHIFT) - 1;

    /// units.h ANGLE_TO_FACING; evaluated in `int` like the C macro.
    const fn angle_to_facing(angle: i32) -> i32 {
        (angle + (1 << (CIRCLE_SHIFT - FACING_SHIFT - 1))) >> (CIRCLE_SHIFT - FACING_SHIFT)
    }

    /// units.h FACING_TO_ANGLE
    const fn facing_to_angle(facing: i32) -> i32 {
        facing << (CIRCLE_SHIFT - FACING_SHIFT)
    }

    /// units.h NORMALIZE_ANGLE
    const fn normalize_angle(angle: i32) -> i32 {
        angle & (FULL_CIRCLE_ANGLE - 1)
    }

    /// units.h NORMALIZE_FACING
    const fn normalize_facing(facing: i32) -> u16 {
        (facing & FACING_MASK) as u16
    }

    fn descriptor(_world: &mut crate::engine::battle::World) -> crate::engine::battle::RaceDesc {
        // All zero fields are null handles, absent callbacks, or zero counters.
        let mut desc = RaceTemplate::default();
        desc.info.ship_flags = FIRES_FORE as u16;
        desc.info.ship_cost = 16;
        desc.info.crew_level = MAX_CREW;
        desc.info.max_crew = MAX_CREW;
        desc.info.energy_level = MAX_ENERGY;
        desc.info.max_energy = MAX_ENERGY;
        desc.resources.strings = Some(c"ship.supox.text");
        desc.resources.icons = Some(c"ship.supox.icons");
        desc.resources.melee_icon = Some(c"ship.supox.meleeicons");
        // Initial SoI radius: 333 / SPHERE_RADIUS_INCREMENT * 2
        desc.fleet_strength = (333 / SPHERE_RADIUS_INCREMENT * 2) as u16;
        desc.fleet_location = POINT { x: 7468, y: 9246 };
        desc.characteristics = CHARACTERISTIC_STUFF {
            max_thrust: MAX_THRUST,
            thrust_increment: THRUST_INCREMENT,
            energy_regeneration: ENERGY_REGENERATION,
            weapon_energy_cost: WEAPON_ENERGY_COST,
            special_energy_cost: SPECIAL_ENERGY_COST,
            energy_wait: ENERGY_WAIT,
            turn_wait: TURN_WAIT,
            thrust_wait: THRUST_WAIT,
            weapon_wait: WEAPON_WAIT,
            special_wait: SPECIAL_WAIT,
            ship_mass: SHIP_MASS,
        };
        desc.resources.ship = [
            Some(c"ship.supox.graphics.blade.large"),
            Some(c"ship.supox.graphics.blade.medium"),
            Some(c"ship.supox.graphics.blade.small"),
        ];
        desc.resources.weapon = [
            Some(c"ship.supox.graphics.glob.large"),
            Some(c"ship.supox.graphics.glob.medium"),
            Some(c"ship.supox.graphics.glob.small"),
        ];
        // special_rsc stays NULL_RESOURCE for all three views.
        desc.resources.captain = Some(c"ship.supox.graphics.captain");
        desc.resources.victory = Some(c"ship.supox.ditty");
        desc.resources.sounds = Some(c"ship.supox.sounds");
        desc.intelligence.maneuverability = 0;
        desc.intelligence.weapon_range = ((MISSILE_SPEED as i32 * MISSILE_LIFE as i32) >> 1) as u16;
        desc.descriptor(&SUPOX)
    }
    use crate::engine::battle::views::Missile;
    use crate::engine::battle::{
        Concerns, Cx, Race, RaceTemplate, ShipId, WeaponOrigin, WeaponSpawn,
    };
    pub static SUPOX: Race = Race {
        descriptor,
        preprocess: Some(supox_preprocess),
        postprocess: None,
        init_weapon: Some(initialize_horn),
        intelligence: Some(supox_intelligence),
        uninit: None,
    };
    pub(crate) fn supox_intelligence(cx: &mut Cx<'_>, ss: ShipId, concerns: &mut Concerns) {
        let enemy = ENEMY_SHIP_INDEX as usize;
        if cx.ship_state(ss).special_counter != 0 || concerns.objects[enemy].object.is_none() {
            cx.ship_state_mut(ss).input &= !(SPECIAL as u8);
        } else {
            let target = concerns.objects[enemy]
                .object
                .unwrap_or_else(|| panic!("AI concern"));
            let delta_x = (i32::from(cx.element(target).next.location.x)
                - i32::from(concerns.subject.next.location.x)) as SIZE;
            let delta_y = (i32::from(cx.element(target).next.location.y)
                - i32::from(concerns.subject.next.location.y)) as SIZE;
            let direction_angle = crate::math::arctan(delta_x, delta_y);

            let ship_angle = facing_to_angle(i32::from(cx.ship_state(ss).facing));
            let lined_up = normalize_angle(
                normalize_angle(i32::from(direction_angle) - ship_angle) + QUADRANT_ANGLE,
            ) <= HALF_CIRCLE_ANGLE;

            if !lined_up
                || concerns.objects[enemy].which_turn > 20
                || normalize_angle(
                    i32::from(concerns.objects[enemy].facing) - (ship_angle + HALF_CIRCLE_ANGLE)
                        + OCTANT_ANGLE,
                ) > QUADRANT_ANGLE
            {
                cx.ship_state_mut(ss).input &= !(SPECIAL as u8);
            } else if lined_up && concerns.objects[enemy].which_turn <= 12 {
                cx.ship_state_mut(ss).input |= SPECIAL as u8;
            }

            if cx.ship_state(ss).input & SPECIAL as u8 != 0 {
                concerns.objects[enemy].movement = MOVEMENT_STATE_PURSUE;
            }
        }

        cx.ship_intelligence(ss, concerns);

        if cx.ship_state(ss).input & SPECIAL as u8 != 0 {
            cx.ship_state_mut(ss).input |= (THRUST | WEAPON) as u8;
        }

        let enemy_weapon = ENEMY_WEAPON_INDEX as usize;
        if cx.ship_state(ss).special_counter == 0
            && concerns.objects[enemy_weapon].object.is_some()
            && concerns.objects[enemy_weapon].movement == MOVEMENT_STATE_AVOID
            && concerns.subject.turn == 0
        {
            cx.ship_state_mut(ss).input &= !(THRUST as u8);
            cx.ship_state_mut(ss).input |= SPECIAL as u8;
            if cx.ship_state(ss).status & (LEFT | RIGHT) as u16 == 0 {
                cx.ship_state_mut(ss).input |= 1u8 << (cx.random() as u8 & 1);
            } else {
                cx.ship_state_mut(ss).input |=
                    (cx.ship_state(ss).status & (LEFT | RIGHT) as u16) as u8;
            }
        }
    }

    pub(crate) fn initialize_horn(
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
            index: cx.ship_state(ss).facing,
            sender: origin.player,
            flags: IGNORE_SIMILAR as u16,
            pixoffs: SUPOX_OFFSET,
            speed: MISSILE_SPEED,
            hit_points: MISSILE_HITS,
            damage: MISSILE_DAMAGE,
            life: MISSILE_LIFE,
            preprocess: None,
            blast_offs: MISSILE_OFFSET,
        };
        out.push(cx.missile(&mut block));
        out
    }

    pub(crate) fn supox_preprocess(cx: &mut Cx<'_>, ss: ShipId) {
        let ship = cx.ship_element(ss);
        if cx.ship_state(ss).status & SPECIAL as u16 == 0 {
            return;
        }

        let mut add_facing: i32 = 0;
        if cx.ship_state(ss).status & THRUST as u16 != 0 {
            if cx.element(ship).thrust == 0 {
                cx.element_mut(ship).thrust += 1;
            }

            add_facing = angle_to_facing(HALF_CIRCLE_ANGLE);
        }
        if cx.ship_state(ss).status & LEFT as u16 != 0 {
            if cx.element(ship).turn == 0 {
                cx.element_mut(ship).turn += 1;
            }

            if add_facing != 0 {
                add_facing += angle_to_facing(OCTANT_ANGLE);
            } else {
                add_facing = -angle_to_facing(QUADRANT_ANGLE);
            }
        } else if cx.ship_state(ss).status & RIGHT as u16 != 0 {
            if cx.element(ship).turn == 0 {
                cx.element_mut(ship).turn += 1;
            }

            if add_facing != 0 {
                add_facing -= angle_to_facing(OCTANT_ANGLE);
            } else {
                add_facing = angle_to_facing(QUADRANT_ANGLE);
            }
        }

        if add_facing != 0 {
            let facing = cx.ship_state(ss).facing;
            cx.ship_state_mut(ss).facing = normalize_facing(i32::from(facing) + add_facing);
            let thrust_status = cx.apply_inertial_thrust(ship);
            cx.ship_state_mut(ss).status &=
                !((SHIP_AT_MAX_SPEED | SHIP_BEYOND_MAX_SPEED | SHIP_IN_GRAVITY_WELL) as u16);
            cx.ship_state_mut(ss).status |= thrust_status;
            cx.ship_state_mut(ss).facing = facing;
        }
    }
}
