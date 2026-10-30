//! Chenjesu callbacks and descriptor translated from `sc2/src/uqm/ships/chenjesu/chenjesu.c`.

#[cfg(not(test))]
pub(crate) mod engine {
    use crate::engine::battle::views::Missile;
    use crate::engine::battle::{
        Concerns, Cx, ElementId, Race, RaceTemplate, ShipId, WeaponOrigin, WeaponSpawn,
    };
    use crate::ships::native::{
        abi::*, angle_to_facing, cosine, facing_to_angle, normalize_facing, sine,
    };

    const MAX_CREW: u16 = 36;
    const MAX_ENERGY: u8 = 30;
    const MAX_THRUST: u16 = 27;
    const THRUST_INCREMENT: u16 = 3;
    const ENERGY_REGENERATION: u8 = 1;
    const ENERGY_WAIT: u8 = 4;
    const TURN_WAIT: u8 = 6;
    const THRUST_WAIT: u8 = 4;
    const SHIP_MASS: u8 = 10;
    const WEAPON_ENERGY_COST: u8 = 5;
    const SPECIAL_ENERGY_COST: u8 = MAX_ENERGY;
    const WEAPON_WAIT: u8 = 0;
    const SPECIAL_WAIT: u8 = 0;
    const CHENJESU_OFFSET: i16 = 16;
    const MISSILE_OFFSET: i16 = 0;
    const MISSILE_SPEED: i16 = 16 << ONE_SHIFT;
    const MISSILE_LIFE: u16 = 90;
    const MISSILE_HITS: i16 = 10;
    const MISSILE_DAMAGE: i16 = 6;
    const NUM_SPARKLES: u16 = 8;
    const FRAGMENT_OFFSET: i16 = 2;
    const NUM_FRAGMENTS: i32 = 8;
    const FRAGMENT_LIFE: u16 = 10;
    const FRAGMENT_HITS: i16 = 1;
    const FRAGMENT_DAMAGE: i16 = 2;
    const DOGGY_OFFSET: i32 = 18;
    const DOGGY_SPEED: i16 = 8 << ONE_SHIFT;
    const ENERGY_DRAIN: u8 = 10;
    const MAX_DOGGIES: u8 = 4;
    const DOGGY_HITS: u16 = 3;
    const DOGGY_MASS: u8 = 4;
    const LONG_RANGE_WEAPON: u32 = 1000 << ONE_SHIFT;
    const FRAGMENT_RANGE: i32 = FRAGMENT_LIFE as i32 * MISSILE_SPEED as i32;

    fn descriptor(_world: &mut crate::engine::battle::World) -> crate::engine::battle::RaceDesc {
        let mut d = RaceTemplate::default();
        d.info.ship_flags = (FIRES_FORE | SEEKING_SPECIAL | SEEKING_WEAPON) as u16;
        d.info.ship_cost = 28;
        d.info.crew_level = MAX_CREW;
        d.info.max_crew = MAX_CREW;
        d.info.energy_level = MAX_ENERGY;
        d.info.max_energy = MAX_ENERGY;
        d.resources.strings = Some(c"ship.chenjesu.text");
        d.resources.icons = Some(c"ship.chenjesu.icons");
        d.resources.melee_icon = Some(c"ship.chenjesu.meleeicons");
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
            special_wait: SPECIAL_WAIT,
            ship_mass: SHIP_MASS,
        };
        d.resources.ship = [
            Some(c"ship.chenjesu.graphics.broodhome.large"),
            Some(c"ship.chenjesu.graphics.broodhome.medium"),
            Some(c"ship.chenjesu.graphics.broodhome.small"),
        ];
        d.resources.weapon = [
            Some(c"ship.chenjesu.graphics.spark.large"),
            Some(c"ship.chenjesu.graphics.spark.medium"),
            Some(c"ship.chenjesu.graphics.spark.small"),
        ];
        d.resources.special = [
            Some(c"ship.chenjesu.graphics.doggy.large"),
            Some(c"ship.chenjesu.graphics.doggy.medium"),
            Some(c"ship.chenjesu.graphics.doggy.small"),
        ];
        d.resources.captain = Some(c"ship.chenjesu.graphics.captain");
        d.resources.victory = Some(c"ship.chenjesu.ditty");
        d.resources.sounds = Some(c"ship.chenjesu.sounds");
        d.intelligence.weapon_range = LONG_RANGE_WEAPON as u16;
        d.descriptor(&CHENJESU)
    }

    pub static CHENJESU: Race = Race {
        descriptor,
        preprocess: Some(chenjesu_preprocess),
        postprocess: Some(chenjesu_postprocess),
        init_weapon: Some(initialize_crystal),
        intelligence: Some(chenjesu_intelligence),
        uninit: None,
    };

    pub(crate) fn crystal_postprocess(cx: &mut Cx<'_>, e: ElementId) {
        let ss = cx.owner(e);
        let mut block = Missile {
            cx: cx.element(e).next.location.x,
            cy: cx.element(e).next.location.y,
            farray: cx.ship_frame_set(ss, 1),
            face: 0,
            index: 1,
            sender: cx.element(e).player,
            flags: IGNORE_SIMILAR as u16,
            pixoffs: 0,
            speed: MISSILE_SPEED,
            hit_points: FRAGMENT_HITS,
            damage: FRAGMENT_DAMAGE,
            life: FRAGMENT_LIFE,
            preprocess: None,
            blast_offs: FRAGMENT_OFFSET,
        };
        let facing_circle = angle_to_facing(FULL_CIRCLE as i32);
        while i32::from(block.face) < facing_circle {
            let h = cx.missile(&mut block);
            if let Some(h) = h {
                cx.set_parent(h, Some(ss));
                cx.put(h);
            }
            block.face = (i32::from(block.face) + facing_circle / NUM_FRAGMENTS) as COUNT;
        }
        cx.process_ship_sound(ss, 1, e);
    }

    pub(crate) fn crystal_preprocess(cx: &mut Cx<'_>, e: ElementId) {
        let ss = cx.owner(e);
        if cx.ship_state(ss).status & WEAPON as u16 != 0 {
            cx.element_mut(e).life = cx.element(e).life.wrapping_add(1);
        } else {
            cx.element_mut(e).life = 1;
            cx.element_mut(e).post = Some(ElementHook::Native(crystal_postprocess));
        }
    }

    pub(crate) fn crystal_collision(
        cx: &mut Cx<'_>,
        a: ElementId,
        pa: &mut POINT,
        b: ElementId,
        pb: &mut POINT,
    ) {
        let h = cx.weapon_collision(a, pa, b, pb);
        if let Some(h) = h {
            let blast = h;
            cx.element_mut(blast).current.location = cx.element(b).current.location;
            cx.element_mut(blast).life = NUM_SPARKLES;
            cx.element_mut(blast).turn = 0;
            cx.element_mut(blast).thrust = 0;
            cx.element_mut(blast).pre = Some(ElementHook::Native(animation_preprocess));
            cx.set_frames(blast, false, cx.element(a).next.image.frames);
            cx.set_frame(
                blast,
                false,
                cx.absolute_frame(cx.element_frame_set_first(blast, false), 2),
            );
        }
    }

    pub(crate) fn doggy_preprocess(cx: &mut Cx<'_>, e: ElementId) {
        let ss = cx.owner(e);
        cx.ship_state_mut(ss).special_counter = cx.ship_state(ss).special_counter.wrapping_add(1);
        if cx.element(e).thrust > 0 {
            cx.element_mut(e).thrust -= 1;
        } else {
            let original = normalize_facing(angle_to_facing(i32::from(
                cx.element(e).velocity.travel_angle,
            )));
            let mut facing = original;
            if cx.track_ship(e, &mut facing) < 0 {
                facing = normalize_facing(cx.random() as i32);
            } else {
                let target = cx
                    .target(e)
                    .unwrap_or_else(|| panic!("tracked DOGI target"));
                // ARCTAN takes SIZE: C promotes the subtraction to int, then
                // truncates each argument to signed 16 bits at the call.
                let dx = (i32::from(cx.element(target).current.location.x)
                    - i32::from(cx.element(e).current.location.x)) as SIZE;
                let dy = (i32::from(cx.element(target).current.location.y)
                    - i32::from(cx.element(e).current.location.y)) as SIZE;
                facing = normalize_facing(angle_to_facing(i32::from(crate::math::arctan(dx, dy))));
                let delta_facing = normalize_facing(
                    i32::from(facing)
                        - i32::from(cx.frame_number(cx.element(target).current.image.frame)),
                );
                if i32::from(delta_facing) > angle_to_facing(HALF_CIRCLE as i32 - OCTANT as i32)
                    && i32::from(delta_facing) < angle_to_facing(HALF_CIRCLE as i32 + OCTANT as i32)
                {
                    if i32::from(delta_facing) >= angle_to_facing(HALF_CIRCLE as i32) {
                        facing = (i32::from(facing) - angle_to_facing(QUADRANT as i32)) as COUNT;
                    } else {
                        facing = (i32::from(facing) + angle_to_facing(QUADRANT as i32)) as COUNT;
                    }
                }
                facing = normalize_facing(i32::from(facing));
            }
            if facing != original {
                cx.element_mut(e)
                    .velocity
                    .set_vector(i32::from(DOGGY_SPEED), facing);
            }
        }
    }

    pub(crate) fn doggy_death(cx: &mut Cx<'_>, e: ElementId) {
        cx.process_ship_sound(cx.owner(e), 3, e);
        cx.element_mut(e).flags &= !(DISAPPEARING as u16);
        cx.element_mut(e).flags |= (NONSOLID | FINITE_LIFE) as u16;
        cx.element_mut(e).life = 6;
        cx.element_mut(e).pre = Some(ElementHook::Native(animation_preprocess));
        cx.element_mut(e).death = None;
        cx.element_mut(e).collide = None;
        cx.element_mut(e).velocity = VELOCITY_DESC::new();
        cx.element_mut(e).turn = 0;
        cx.element_mut(e).thrust = 0;
    }

    pub(crate) fn doggy_collision(
        cx: &mut Cx<'_>,
        a: ElementId,
        pa: &mut POINT,
        b: ElementId,
        pb: &mut POINT,
    ) {
        crate::battle::ship_native::collision(cx, a, pa, b, pb);
        if cx.element(b).flags & PLAYER_SHIP as u16 != 0
            && cx.element(a).player != cx.element(b).player
        {
            cx.process_ship_sound(cx.owner(a), 2, a);
            let enemy = cx.owner(b);
            let energy = cx.race_stats(enemy).ship_info.energy_level;
            cx.delta_energy(b, -i16::from(energy.min(ENERGY_DRAIN)));
        }
        if u32::from(cx.element(a).thrust) <= COLLISION_THRUST_WAIT {
            cx.element_mut(a).thrust =
                (u32::from(cx.element(a).thrust) + (COLLISION_THRUST_WAIT << 1)) as u8;
        }
    }

    pub(crate) fn spawn_doggy(cx: &mut Cx<'_>, e: ElementId) {
        let h = cx.alloc_element();
        if let Some(h) = h {
            cx.element_mut(e).flags |= DEFY_PHYSICS as u16;
            cx.put(h);
            let dog = h;
            cx.element_mut(dog).hits = DOGGY_HITS;
            cx.element_mut(dog).mass = DOGGY_MASS;
            cx.element_mut(dog).thrust = 0;
            cx.element_mut(dog).player = cx.element(e).player;
            cx.element_mut(dog).flags = APPEARING as u16;
            cx.element_mut(dog).life = NORMAL_LIFE as COUNT;
            cx.primitive_style_mut(dog).kind = gfx_object_STAMP_PRIM as GRAPHICS_PRIM;
            cx.element_mut(dog).pre = Some(ElementHook::Native(doggy_preprocess));
            cx.element_mut(dog).post = None;
            cx.element_mut(dog).collide = Some(CollisionHook::Native(doggy_collision));
            cx.element_mut(dog).death = Some(ElementHook::Native(doggy_death));
            let ss = cx.owner(e);
            let angle = (facing_to_angle(i32::from(cx.ship_state(ss).facing)) + HALF_CIRCLE as i32)
                as COUNT;
            let offset = (i32::from(CHENJESU_OFFSET) + DOGGY_OFFSET) << ONE_SHIFT;
            cx.element_mut(dog).current.location.x = (i32::from(cx.element(e).next.location.x)
                + i32::from(cosine(i32::from(angle), offset)))
                as COORD;
            cx.element_mut(dog).current.location.y = (i32::from(cx.element(e).next.location.y)
                + i32::from(sine(i32::from(angle), offset)))
                as COORD;
            cx.set_frames(dog, false, cx.ship_frame_set(ss, 2));
            cx.set_frame(dog, false, cx.ship_frame(ss, 2));
            cx.element_mut(dog).velocity.set_vector(
                i32::from(DOGGY_SPEED),
                normalize_facing(angle_to_facing(i32::from(angle))),
            );
            cx.set_parent(dog, Some(ss));
            cx.process_ship_sound(ss, 4, dog);
        }
    }

    fn crystal_misses(cx: &mut Cx<'_>, crystal: ElementId, target: ElementId) -> bool {
        let crystal_origin = cx.weapon_origin(crystal);
        let target_origin = cx.weapon_origin(target);
        let life = cx.element(crystal).life;
        let turn = cx.plot_intercept(
            &crystal_origin,
            &target_origin,
            life,
            (FRAGMENT_RANGE / 2) as COUNT,
        );
        turn == 0 || (turn == 1 && cx.plot_intercept(&crystal_origin, &target_origin, life, 0) == 0)
    }

    pub(crate) fn chenjesu_intelligence(cx: &mut Cx<'_>, ss: ShipId, concerns: &mut Concerns) {
        cx.ship_state_mut(ss).input &= !(SPECIAL as u8);
        let enemy = ENEMY_SHIP_INDEX as usize;
        let weapon = ENEMY_WEAPON_INDEX as usize;
        let mut eval = enemy;
        if let Some(other) = concerns.objects[enemy].object {
            let enemy_ss = cx.owner(other);
            let control = cx.race_stats(enemy_ss).intelligence;
            if (concerns.objects[enemy].which_turn <= 16
                && u32::from(control.maneuverability) >= MEDIUM_SHIP)
                || (u32::from(control.maneuverability) <= SLOW_SHIP
                    && u32::from(control.weapon_range) >= LONG_RANGE_WEAPON * 3 / 4
                    && cx.race_stats(enemy_ss).ship_info.ship_flags & SEEKING_WEAPON as u16 != 0)
            {
                concerns.objects[enemy].movement = MOVEMENT_STATE_PURSUE;
            }
        }
        if cx.ship_state(ss).special_counter == 1
            && concerns.objects[weapon].object.is_some()
            && concerns.objects[weapon].movement == MOVEMENT_STATE_ENTICE
            && concerns.objects[weapon].which_turn <= 8
        {
            eval = weapon;
        }
        cx.ship_intelligence(ss, concerns);
        if let Some(selected) = concerns.objects[eval].object {
            let mut h = if cx.ship_state(ss).old_status & WEAPON as u16 != 0 {
                cx.elements().tail()
            } else {
                None
            };
            while let Some(crystal) = h {
                let mut next = cx.elements().pred(crystal);
                let state = cx.element(crystal);
                if state.flags & NONSOLID as u16 == 0
                    && state.next.image.frames == cx.ship_frame_set(ss, 1)
                    && state.pre.is_some()
                    && state.life > 0
                    && state.player == concerns.subject.player
                {
                    if let Some(target) = concerns.objects[enemy].object {
                        if crystal_misses(cx, crystal, target) {
                            cx.ship_state_mut(ss).input &= !(WEAPON as u8);
                        } else if cx.ship_state(ss).weapon_counter == 0 {
                            cx.ship_state_mut(ss).input |= WEAPON as u8;
                            eval = enemy;
                        }
                        break;
                    }
                    next = None;
                }
                h = next;
            }
            if h.is_none() {
                if cx.ship_state(ss).old_status & WEAPON as u16 != 0 {
                    cx.ship_state_mut(ss).input &= !(WEAPON as u8);
                    if eval == weapon {
                        cx.ship_state_mut(ss).weapon_counter = 3;
                    }
                } else if cx.ship_state(ss).weapon_counter == 0
                    && cx.ship_weapons(
                        &concerns.subject,
                        &cx.weapon_origin(selected),
                        (FRAGMENT_RANGE / 2) as COUNT,
                    ) != 0
                {
                    cx.ship_state_mut(ss).input |= WEAPON as u8;
                }
            }
        }
        if cx.ship_state(ss).special_counter < MAX_DOGGIES
            && concerns.objects[eval].object.is_some()
            && cx.race_stats(ss).ship_info.energy_level <= SPECIAL_ENERGY_COST
            && cx.ship_state(ss).input & WEAPON as u8 == 0
        {
            cx.ship_state_mut(ss).input |= SPECIAL as u8;
        }
    }

    pub(crate) fn initialize_crystal(
        cx: &mut Cx<'_>,
        ss: ShipId,
        origin: WeaponOrigin,
    ) -> WeaponSpawn {
        let mut block = Missile {
            cx: origin.next.location.x,
            cy: origin.next.location.y,
            farray: cx.ship_frame_set(ss, 1),
            face: cx.ship_state(ss).facing,
            index: 0,
            sender: origin.player,
            flags: IGNORE_SIMILAR as u16,
            pixoffs: CHENJESU_OFFSET,
            speed: MISSILE_SPEED,
            hit_points: MISSILE_HITS,
            damage: MISSILE_DAMAGE,
            life: MISSILE_LIFE,
            preprocess: Some(crystal_preprocess),
            blast_offs: MISSILE_OFFSET,
        };
        let out = cx.missile(&mut block);
        if let Some(out) = out {
            cx.element_mut(out).collide = Some(CollisionHook::Native(crystal_collision));
        }
        vec![out]
    }

    pub(crate) fn chenjesu_postprocess(cx: &mut Cx<'_>, ss: ShipId) {
        let e = cx.ship_element(ss);
        if cx.ship_state(ss).status & SPECIAL as u16 != 0
            && cx.ship_state(ss).special_counter < MAX_DOGGIES
            && cx.delta_energy(e, -i16::from(SPECIAL_ENERGY_COST))
        {
            spawn_doggy(cx, e);
        }
        cx.ship_state_mut(ss).special_counter = 1;
    }

    pub(crate) fn chenjesu_preprocess(cx: &mut Cx<'_>, ss: ShipId) {
        if cx.ship_state(ss).special_counter > 1 {
            cx.ship_state_mut(ss).special_counter =
                cx.ship_state(ss).special_counter.wrapping_add(MAX_DOGGIES);
        }
        if cx.ship_state(ss).status & cx.ship_state(ss).old_status & WEAPON as u16 != 0 {
            cx.ship_state_mut(ss).weapon_counter = cx.ship_state(ss).weapon_counter.wrapping_add(1);
        }
    }
}
