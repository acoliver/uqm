//! Slylandro live callbacks translated from `sc2/src/uqm/ships/slylandr/slylandr.c`.
#[cfg(not(test))]
pub(crate) mod engine {
    use crate::ships::native::{
        abi::*, angle_to_facing, cosine, facing_to_angle, normalize_facing, sine,
    };

    use crate::engine::battle::views::Laser;
    use crate::engine::battle::{
        Concerns, Cx, ElementId, Race, RaceTemplate, ShipId, WeaponOrigin, WeaponSpawn,
    };
    pub static SLYLANDRO: Race = Race {
        descriptor,
        preprocess: Some(slylandro_preprocess),
        postprocess: Some(slylandro_postprocess),
        init_weapon: Some(initialize_lightning),
        intelligence: Some(slylandro_intelligence),
        uninit: None,
    };
    const MAX_CREW: u16 = 12;
    const MAX_ENERGY: u8 = 20;
    const WEAPON_WAIT: u8 = 17;
    const LASER_LENGTH: u32 = 32;
    const HARVEST_RANGE: i32 = 208 * 3 / 8;
    const CLOSE_RANGE_WEAPON: u16 = 50 << ONE_SHIFT;

    fn descriptor(_world: &mut crate::engine::battle::World) -> crate::engine::battle::RaceDesc {
        let mut d = RaceTemplate::default();
        d.info.ship_flags = (SEEKING_WEAPON | CREW_IMMUNE) as u16;
        d.info.ship_cost = 17;
        d.info.crew_level = MAX_CREW;
        d.info.max_crew = MAX_CREW;
        d.info.energy_level = MAX_ENERGY;
        d.info.max_energy = MAX_ENERGY;
        d.resources.strings = Some(c"ship.slylandro.text");
        d.resources.icons = Some(c"ship.slylandro.icons");
        d.resources.melee_icon = Some(c"ship.slylandro.meleeicons");
        // races.h: INFINITE_RADIUS is (~0), stored in COUNT.
        d.fleet_strength = u16::MAX;
        d.fleet_location = POINT { x: 333, y: 9812 };
        d.characteristics = CHARACTERISTIC_STUFF {
            max_thrust: 60,
            thrust_increment: 60,
            energy_regeneration: 0,
            weapon_energy_cost: 2,
            special_energy_cost: 0,
            energy_wait: 10,
            turn_wait: 0,
            thrust_wait: 0,
            weapon_wait: WEAPON_WAIT,
            special_wait: 20,
            ship_mass: 1,
        };
        d.resources.ship = [
            Some(c"ship.slylandro.graphics.probe.large"),
            Some(c"ship.slylandro.graphics.probe.medium"),
            Some(c"ship.slylandro.graphics.probe.small"),
        ];
        d.resources.captain = Some(c"ship.slylandro.graphics.captain");
        d.resources.victory = Some(c"ship.slylandro.ditty");
        d.resources.sounds = Some(c"ship.slylandro.sounds");
        d.intelligence.weapon_range = CLOSE_RANGE_WEAPON << 1;
        d.descriptor(&SLYLANDRO)
    }
    const fn rgb15(r: u8, g: u8, b: u8) -> Color {
        Color {
            r: (r << 3) | (r >> 2),
            g: (g << 3) | (g >> 2),
            b: (b << 3) | (b >> 2),
            a: 255,
        }
    }

    pub(crate) fn lightning_postprocess(cx: &mut Cx<'_>, e: ElementId) {
        if cx.element(e).turn != 0 && cx.element(e).flags & COLLISION as u16 == 0 {
            if let Some(h) = lightning_segment(cx, e) {
                cx.put(h);
            }
        }
    }

    pub(crate) fn lightning_collision(
        cx: &mut Cx<'_>,
        a: ElementId,
        pa: &mut POINT,
        b: ElementId,
        pb: &mut POINT,
    ) {
        let ss = cx.owner(a);
        if cx.ship_state(ss).weapon_counter > WEAPON_WAIT >> 1 {
            cx.ship_state_mut(ss).weapon_counter =
                WEAPON_WAIT.wrapping_sub(cx.ship_state(ss).weapon_counter);
        }
        cx.ship_state_mut(ss).weapon_counter = cx
            .ship_state(ss)
            .weapon_counter
            .wrapping_sub(cx.element(a).turn);
        cx.element_mut(a).turn = 0;
        cx.weapon_collision(a, pa, b, pb);
    }

    pub(crate) fn initialize_lightning(
        cx: &mut Cx<'_>,
        ss: ShipId,
        origin: WeaponOrigin,
    ) -> WeaponSpawn {
        vec![lightning(cx, ss, origin).0]
    }

    /// Spawn a segment from a live source, retaining the source's tracking changes.
    pub fn lightning_segment(cx: &mut Cx<'_>, source: ElementId) -> Option<ElementId> {
        let owner = cx.owner(source);
        let origin = cx.weapon_origin(source);
        let (out, origin) = lightning(cx, owner, origin);
        cx.set_target(source, origin.target);
        out
    }

    fn lightning(
        cx: &mut Cx<'_>,
        ss: ShipId,
        mut origin: WeaponOrigin,
    ) -> (Option<ElementId>, WeaponOrigin) {
        let block = Laser {
            origin: origin.next.location,
            end: POINT { x: 0, y: 0 },
            sender: origin.player,
            flags: IGNORE_SIMILAR as u16,
            face: 0,
            pixoffs: 0,
            // The C block leaves color uninitialized; it is overwritten below
            // before the new element can be drawn.
            color: rgb15(0, 0, 0),
        };
        let out = cx.spawn_laser(block);
        if let Some(laser) = out {
            cx.element_mut(laser).post = Some(ElementHook::Native(lightning_postprocess));
            cx.element_mut(laser).collide = Some(CollisionHook::Native(lightning_collision));
            // This draw precedes TrackShip's optional opposite-facing draw.
            let random = cx.random();
            let mut angle: COUNT;
            let delta: SIZE;
            if origin.flags & PLAYER_SHIP as u16 == 0 {
                angle = origin.velocity.travel_angle;
                let mut facing = normalize_facing(angle_to_facing(i32::from(angle)));
                delta = cx.track_subject(&mut origin, &mut facing);
                cx.element_mut(laser).turn = origin.turn.wrapping_sub(1);
                cx.primitive_style_mut(laser).color = cx.subject_style(&origin).color;
                // Child segments keep their travel angle, not the rounded and
                // turned facing returned by TrackShip.
            } else {
                let mut facing = cx.ship_state(ss).facing;
                origin.target = None;
                delta = cx.track_subject(&mut origin, &mut facing);
                origin.target = None;
                angle = facing_to_angle(i32::from(facing)) as COUNT;
                let mut wait = cx.ship_state(ss).weapon_counter;
                if wait == 0 {
                    wait = WEAPON_WAIT;
                }
                if wait > WEAPON_WAIT >> 1 {
                    wait = WEAPON_WAIT.wrapping_sub(wait);
                }
                cx.element_mut(laser).turn = wait;
                cx.primitive_style_mut(laser).color = match (random >> 8) & 3 {
                    0 => rgb15(31, 31, 31),
                    1 => rgb15(22, 23, 31),
                    2 => rgb15(6, 7, 31),
                    _ => rgb15(0, 0, 24),
                };
            }
            if delta == -1 || i32::from(delta) == angle_to_facing(HALF_CIRCLE as i32) {
                angle = angle.wrapping_add(random as COUNT);
            } else if delta == 0 {
                angle = if random as COUNT & 1 != 0 {
                    angle.wrapping_sub(1)
                } else {
                    angle.wrapping_add(1)
                };
            } else if i32::from(delta) < angle_to_facing(HALF_CIRCLE as i32) {
                angle = angle.wrapping_add(random as COUNT & (QUADRANT as COUNT - 1));
            } else {
                angle = angle.wrapping_sub(random as COUNT & (QUADRANT as COUNT - 1));
            }
            let length = ((((random >> 16) & (LASER_LENGTH - 1)) + 4)
                << (ONE_SHIFT + VELOCITY_SHIFT)) as SIZE;
            cx.element_mut(laser).velocity.set_components(
                i32::from((cosine(i32::from(angle), i32::from(length))) as SIZE),
                i32::from((sine(i32::from(angle), i32::from(length))) as SIZE),
            );
            cx.set_parent(laser, Some(ss));
        }
        (out, origin)
    }

    pub(crate) fn slylandro_intelligence(cx: &mut Cx<'_>, ss: ShipId, concerns: &mut Concerns) {
        if cx.world.activity() as u8 == IN_ENCOUNTER as u8 {
            concerns.objects[ENEMY_WEAPON_INDEX as usize].object = None;
        }
        let enemy = ENEMY_SHIP_INDEX as usize;
        if cx.ship_state(ss).special_counter == 0
            && cx.race_stats(ss).ship_info.energy_level == 0
            && concerns.objects[GRAVITY_MASS_INDEX as usize]
                .object
                .is_none()
        {
            concerns.count = FIRST_EMPTY_INDEX as COUNT + 1;
        }
        if concerns.objects[enemy].object.is_some()
            && concerns.objects[enemy].movement == MOVEMENT_STATE_PURSUE
            && concerns.objects[enemy].which_turn <= 6
        {
            concerns.objects[enemy].movement = MOVEMENT_STATE_ENTICE;
        }
        concerns.subject.thrust = concerns.subject.thrust.wrapping_add(1);
        cx.ship_intelligence(ss, concerns);
        concerns.subject.thrust = concerns.subject.thrust.wrapping_sub(1);
        if concerns.objects[enemy].object.is_some() && concerns.objects[enemy].which_turn <= 14 {
            cx.ship_state_mut(ss).input |= WEAPON as u8;
        } else {
            cx.ship_state_mut(ss).input &= !(WEAPON as u8);
        }
        cx.ship_state_mut(ss).input &= !(SPECIAL as u8);
        if cx.race_stats(ss).ship_info.energy_level < cx.race_stats(ss).ship_info.max_energy {
            let junk = FIRST_EMPTY_INDEX as usize;
            if concerns.objects[junk].object.is_some() && concerns.objects[junk].which_turn <= 14 {
                cx.ship_state_mut(ss).input |= SPECIAL as u8;
            }
        }
    }

    // CollisionPossible checks solidity only on the first argument.
    fn collision_possible(cx: &Cx<'_>, a: ElementId, b: ElementId) -> bool {
        let shared = cx.element(a).flags & cx.element(b).flags;
        cx.element(a).flags & (NONSOLID | DISAPPEARING) as u16 == 0
            && shared & COLLISION as u16 == 0
            && (shared & IGNORE_SIMILAR as u16 == 0 || cx.parent(a) != cx.parent(b))
            && (cx.element(a).mass != 0 || cx.element(b).mass != 0)
    }

    fn harvest_space_junk(cx: &mut Cx<'_>, ship: ElementId) -> bool {
        let mut found = false;
        let mut h = cx.elements().head();
        while let Some(obj) = h {
            let next = cx.elements().succ(obj);
            if cx.element(obj).flags & (APPEARING | PLAYER_SHIP | FINITE_LIFE) as u16 == 0
                && cx.element(obj).player == NEUTRAL_PLAYER_NUM as SIZE
                && u32::from(cx.element(obj).mass) <= MAX_SHIP_MASS * 10
                && collision_possible(cx, obj, ship)
            {
                let mut dx = (i32::from(cx.element(obj).next.location.x)
                    - i32::from(cx.element(ship).next.location.x))
                    as SIZE;
                let mut dy = (i32::from(cx.element(obj).next.location.y)
                    - i32::from(cx.element(ship).next.location.y))
                    as SIZE;
                if dx < 0 {
                    dx = dx.wrapping_neg();
                }
                if dy < 0 {
                    dy = dy.wrapping_neg();
                }
                dx >>= ONE_SHIFT;
                dy >>= ONE_SHIFT;
                if i32::from(dx) <= HARVEST_RANGE
                    && i32::from(dy) <= HARVEST_RANGE
                    && i32::from(dx) * i32::from(dx) + i32::from(dy) * i32::from(dy)
                        <= HARVEST_RANGE * HARVEST_RANGE
                {
                    cx.element_mut(obj).life = 0;
                    cx.element_mut(obj).flags |= NONSOLID as u16;
                    if !found {
                        found = true;
                        let ss = cx.owner(ship);
                        cx.process_ship_sound(ss, 1, ship);
                        cx.delta_energy(ship, i16::from(MAX_ENERGY));
                    }
                }
            }
            h = next;
        }
        found
    }

    pub(crate) fn slylandro_postprocess(cx: &mut Cx<'_>, ss: ShipId) {
        let ship = cx.ship_element(ss);
        if cx.ship_state(ss).weapon_counter != 0 && cx.ship_state(ss).weapon_counter < WEAPON_WAIT {
            if let Some(h) = lightning_segment(cx, ship) {
                cx.put(h);
            }
        }
        if cx.ship_state(ss).special_counter == 0
            && cx.ship_state(ss).status & SPECIAL as u16 != 0
            && harvest_space_junk(cx, ship)
        {
            cx.ship_state_mut(ss).special_counter = cx.race_stats(ss).characteristics.special_wait;
        }
    }

    pub(crate) fn slylandro_preprocess(cx: &mut Cx<'_>, ss: ShipId) {
        let ship = cx.ship_element(ss);
        if cx.element(ship).flags & (APPEARING | NONSOLID) as u16 == 0 {
            if cx.ship_state(ss).status & THRUST as u16 != 0
                && cx.ship_state(ss).old_status & THRUST as u16 == 0
            {
                cx.ship_state_mut(ss).facing = cx
                    .ship_state(ss)
                    .facing
                    .wrapping_add(angle_to_facing(HALF_CIRCLE as i32) as COUNT);
            }
            if cx.element(ship).turn == 0 {
                cx.element_mut(ship).turn = (u16::from(cx.element(ship).turn)
                    + u16::from(cx.race_stats(ss).characteristics.turn_wait)
                    + 1) as BYTE;
                if cx.ship_state(ss).status & LEFT as u16 != 0 {
                    cx.ship_state_mut(ss).facing = cx.ship_state(ss).facing.wrapping_sub(1);
                } else if cx.ship_state(ss).status & RIGHT as u16 != 0 {
                    cx.ship_state_mut(ss).facing = cx.ship_state(ss).facing.wrapping_add(1);
                }
            }
            cx.ship_state_mut(ss).facing = normalize_facing(i32::from(cx.ship_state(ss).facing));
            if cx.element(ship).thrust == 0 {
                cx.element_mut(ship).thrust = (u16::from(cx.element(ship).thrust)
                    + u16::from(cx.race_stats(ss).characteristics.thrust_wait)
                    + 1) as BYTE;
                let thrust = cx.race_stats(ss).characteristics.max_thrust as SIZE;
                let facing = cx.ship_state(ss).facing;
                cx.element_mut(ship)
                    .velocity
                    .set_vector(i32::from(thrust), facing);
                cx.ship_state_mut(ss).status |= SHIP_AT_MAX_SPEED as u16;
                cx.ship_state_mut(ss).status &= !(SHIP_IN_GRAVITY_WELL as u16);
            }
            cx.set_frame(
                ship,
                true,
                cx.advance_frame(cx.element(ship).next.image.frame),
            );
            cx.element_mut(ship).flags |= CHANGING as u16;
        }
    }
}
