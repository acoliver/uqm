//! Druuge callbacks registered with the live battle engine.

#[cfg(not(test))]
pub(crate) mod engine {
    use crate::ships::native::{abi::*, cosine, facing_to_angle, sine};

    const MAX_CREW: u16 = 14;
    const MAX_ENERGY: u8 = 32;
    const ENERGY_REGENERATION: u8 = 1;
    const ENERGY_WAIT: u8 = 50;
    const MAX_THRUST: u16 = 20;
    const THRUST_INCREMENT: u16 = 2;
    const THRUST_WAIT: u8 = 1;
    const TURN_WAIT: u8 = 4;
    const SHIP_MASS: u8 = 5;
    const WEAPON_ENERGY_COST: u8 = 4;
    const WEAPON_WAIT: u8 = 10;
    const DRUUGE_OFFSET: i16 = 24;
    const MISSILE_OFFSET: i16 = 6;
    const MISSILE_SPEED: i16 = 30 << ONE_SHIFT;
    const MISSILE_LIFE: u16 = 20;
    const MISSILE_RANGE: i32 = MISSILE_SPEED as i32 * MISSILE_LIFE as i32;
    const MISSILE_HITS: i16 = 4;
    const MISSILE_DAMAGE: i16 = 6;
    const RECOIL_VELOCITY: i32 = (6 << ONE_SHIFT) << VELOCITY_SHIFT;
    const MAX_RECOIL_VELOCITY: i32 = RECOIL_VELOCITY * 4;
    const SPECIAL_ENERGY_COST: u8 = 16;
    const SPECIAL_WAIT: u8 = 30;

    fn descriptor(_world: &mut crate::engine::battle::World) -> crate::engine::battle::RaceDesc {
        let mut desc = RaceTemplate::default();
        desc.info.ship_flags = FIRES_FORE as u16;
        desc.info.ship_cost = 17;
        desc.info.crew_level = MAX_CREW;
        desc.info.max_crew = MAX_CREW;
        desc.info.energy_level = MAX_ENERGY;
        desc.info.max_energy = MAX_ENERGY;
        desc.resources.strings = Some(c"ship.druuge.text");
        desc.resources.icons = Some(c"ship.druuge.icons");
        desc.resources.melee_icon = Some(c"ship.druuge.meleeicons");
        desc.fleet_strength = (1400 / SPHERE_RADIUS_INCREMENT * 2) as u16;
        desc.fleet_location = POINT { x: 9500, y: 2792 };
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
            Some(c"ship.druuge.graphics.mauler.large"),
            Some(c"ship.druuge.graphics.mauler.medium"),
            Some(c"ship.druuge.graphics.mauler.small"),
        ];
        desc.resources.weapon = [
            Some(c"ship.druuge.graphics.cannon.large"),
            Some(c"ship.druuge.graphics.cannon.medium"),
            Some(c"ship.druuge.graphics.cannon.small"),
        ];
        desc.resources.captain = Some(c"ship.druuge.graphics.captain");
        desc.resources.victory = Some(c"ship.druuge.ditty");
        desc.resources.sounds = Some(c"ship.druuge.sounds");
        desc.intelligence.weapon_range = MISSILE_RANGE as u16;
        desc.descriptor(&DRUUGE)
    }
    use crate::engine::battle::views::Missile;
    use crate::engine::battle::{
        Concerns, Cx, ElementId, Race, RaceTemplate, ShipId, WeaponOrigin, WeaponSpawn,
    };
    pub static DRUUGE: Race = Race {
        descriptor,
        preprocess: Some(druuge_preprocess),
        postprocess: Some(druuge_postprocess),
        init_weapon: Some(initialize_cannon),
        intelligence: Some(druuge_intelligence),
        uninit: None,
    };
    pub(crate) fn initialize_cannon(
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
            pixoffs: DRUUGE_OFFSET,
            speed: MISSILE_SPEED,
            hit_points: MISSILE_HITS,
            damage: MISSILE_DAMAGE,
            life: MISSILE_LIFE,
            preprocess: None,
            blast_offs: MISSILE_OFFSET,
        };
        out.push(cx.missile(&mut block));
        if let Some(cannon) = out[0] {
            cx.element_mut(cannon).collide = Some(CollisionHook::Native(cannon_collision));
        }
        out
    }

    pub(crate) fn cannon_collision(
        cx: &mut Cx<'_>,
        cannon: ElementId,
        point0: &mut POINT,
        other: ElementId,
        point1: &mut POINT,
    ) {
        cx.weapon_collision(cannon, point0, other, point1);
        let collision_mass = i32::from(cx.element(other).mass) + 1;
        if cx.element(other).flags & PLAYER_SHIP as u16 != 0
            && cx.element(other).hits != 0
            && collision_mass <= MAX_SHIP_MASS as i32 * 10
        {
            let ss = cx.owner(other);
            cx.ship_state_mut(ss).status &= !((SHIP_AT_MAX_SPEED | SHIP_BEYOND_MAX_SPEED) as u16);
            let angle = facing_to_angle(i32::from(
                cx.frame_number(cx.element(cannon).next.image.frame),
            ));
            cx.element_mut(other).velocity.delta_components(
                i32::from(cosine(angle, RECOIL_VELOCITY) as SIZE),
                i32::from(sine(angle, RECOIL_VELOCITY) as SIZE),
            );
            let (x, y) = cx.element(other).velocity.get_current_components();
            let (dx, dy) = (x as SIZE, y as SIZE);
            let x = i64::from(dx);
            let y = i64::from(dy);
            let limit = i64::from(MAX_RECOIL_VELOCITY);
            if x * x + y * y > limit * limit {
                let a = crate::math::arctan(dx, dy);
                cx.element_mut(other).velocity.set_components(
                    i32::from(cosine(i32::from(a), MAX_RECOIL_VELOCITY) as SIZE),
                    i32::from(sine(i32::from(a), MAX_RECOIL_VELOCITY) as SIZE),
                );
            }
        }
    }

    pub(crate) fn druuge_intelligence(cx: &mut Cx<'_>, ss: ShipId, concerns: &mut Concerns) {
        let enemy = ENEMY_SHIP_INDEX as usize;
        let mut old_flags = 0u16;
        let mut enemy_ss = None;
        if cx.ship_state(ss).status & SHIP_BEYOND_MAX_SPEED as u16 != 0 {
            concerns.objects[enemy].movement = MOVEMENT_STATE_ENTICE;
        } else if concerns.objects[enemy].object.is_some()
            && concerns.objects[enemy].which_turn <= ((MISSILE_RANGE * 3 / 4) >> 6) as u16
        {
            let other = cx.owner(
                concerns.objects[enemy]
                    .object
                    .unwrap_or_else(|| panic!("enemy ship")),
            );
            enemy_ss = Some(other);
            old_flags = cx.race_stats(other).ship_info.ship_flags;
            cx.race_stats_mut(other).ship_info.ship_flags &=
                !((FIRES_FORE | FIRES_RIGHT | FIRES_AFT | FIRES_LEFT) as u16);
            concerns.objects[enemy].movement = MOVEMENT_STATE_PURSUE;
            if concerns.subject.thrust == 0 {
                concerns.subject.thrust += 1;
            }
        }
        cx.ship_intelligence(ss, concerns);
        if let Some(other) = enemy_ss {
            cx.race_stats_mut(other).ship_info.ship_flags = old_flags;
        }
        if cx.ship_state(ss).status & SHIP_BEYOND_MAX_SPEED as u16 == 0
            && (concerns.objects[enemy].which_turn <= 12
                || (concerns.objects[ENEMY_WEAPON_INDEX as usize]
                    .object
                    .is_some()
                    && concerns.objects[ENEMY_WEAPON_INDEX as usize].which_turn <= 6))
        {
            cx.ship_state_mut(ss).input |= WEAPON as u8;
            if concerns.subject.thrust < WEAPON_WAIT + 1 {
                concerns.subject.thrust = WEAPON_WAIT + 1;
            }
        }
        if cx.ship_state(ss).input & WEAPON as u8 != 0
            && cx.race_stats(ss).ship_info.energy_level < WEAPON_ENERGY_COST
            && concerns.subject.hits > 1
        {
            cx.ship_state_mut(ss).input |= SPECIAL as u8;
        } else {
            cx.ship_state_mut(ss).input &= !(SPECIAL as u8);
        }
    }

    pub(crate) fn druuge_postprocess(cx: &mut Cx<'_>, ss: ShipId) {
        let ship = cx.ship_element(ss);
        if cx.ship_state(ss).status & WEAPON as u16 == 0
            || cx.ship_state(ss).weapon_counter != cx.race_stats(ss).characteristics.weapon_wait
        {
            return;
        }
        cx.ship_state_mut(ss).status &= !(SHIP_AT_MAX_SPEED as u16);
        let angle = facing_to_angle(i32::from(cx.ship_state(ss).facing)) + HALF_CIRCLE as i32;
        cx.element_mut(ship).velocity.delta_components(
            i32::from(cosine(angle, RECOIL_VELOCITY) as SIZE),
            i32::from(sine(angle, RECOIL_VELOCITY) as SIZE),
        );
        let (x, y) = cx.element(ship).velocity.get_current_components();
        let (dx, dy) = (x as SIZE, y as SIZE);
        let x = i64::from(dx);
        let y = i64::from(dy);
        let limit = i64::from(MAX_RECOIL_VELOCITY);
        if x * x + y * y > limit * limit {
            let a = crate::math::arctan(dx, dy);
            cx.element_mut(ship).velocity.set_components(
                i32::from(cosine(i32::from(a), MAX_RECOIL_VELOCITY) as SIZE),
                i32::from(sine(i32::from(a), MAX_RECOIL_VELOCITY) as SIZE),
            );
        }
    }

    pub(crate) fn druuge_preprocess(cx: &mut Cx<'_>, ss: ShipId) {
        let ship = cx.ship_element(ss);
        if cx.ship_state(ss).status & SPECIAL as u16 == 0 {
            return;
        }
        if cx.ship_state(ss).special_counter != 0
            || cx.element(ship).hits == 1
            || cx.race_stats(ss).ship_info.energy_level == cx.race_stats(ss).ship_info.max_energy
        {
            cx.ship_state_mut(ss).status &= !(SPECIAL as u16);
            return;
        }
        cx.process_ship_sound(ss, 1, ship);
        cx.delta_crew(ship, -1);
        cx.delta_energy(ship, i16::from(SPECIAL_ENERGY_COST));
        cx.ship_state_mut(ss).special_counter = cx.race_stats(ss).characteristics.special_wait;
    }
}
