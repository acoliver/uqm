//! Live engine callbacks for the Mmrnmhrm, mirroring `mmrnmhrm.c`.

#[cfg(not(test))]
pub(crate) mod engine {
    use crate::ships::native::{
        abi::*, cosine, facing_to_angle, normalize_angle, normalize_facing, sine,
    };

    const MAX_CREW: u16 = 20;
    const MAX_ENERGY: u8 = 10;
    const SHIP_MASS: u8 = 3;
    const ENERGY_REGENERATION: u8 = 2;
    const ENERGY_WAIT: u8 = 6;
    const MAX_THRUST: u16 = 20;
    const THRUST_INCREMENT: u16 = 5;
    const THRUST_WAIT: u8 = 1;
    const TURN_WAIT: u8 = 2;
    const YWING_ENERGY_REGENERATION: u8 = 1;
    const YWING_MAX_THRUST: u16 = 50;
    const YWING_THRUST_INCREMENT: u16 = 10;
    const YWING_THRUST_WAIT: u8 = 0;
    const YWING_TURN_WAIT: u8 = 14;
    const WEAPON_ENERGY_COST: u8 = 1;
    const WEAPON_WAIT: u8 = 0;
    const SPECIAL_ENERGY_COST: u8 = MAX_ENERGY;
    const SPECIAL_WAIT: u8 = 0;
    const YWING_WEAPON_ENERGY_COST: u8 = 1;
    const YWING_WEAPON_WAIT: u8 = 20;
    const MISSILE_LIFE: u16 = 40;
    const MISSILE_HITS: i16 = 1;
    const MISSILE_DAMAGE: i16 = 1;
    const TRACK_WAIT: u8 = 5;
    const CLOSE_RANGE_WEAPON: i32 = 50 << ONE_SHIFT;
    const LONG_RANGE_WEAPON: i32 = 1000 << ONE_SHIFT;
    const SLOW_SHIP: u32 = 25;

    const YWING_CHARACTERISTICS: CHARACTERISTIC_STUFF = CHARACTERISTIC_STUFF {
        max_thrust: YWING_MAX_THRUST,
        thrust_increment: YWING_THRUST_INCREMENT,
        energy_regeneration: YWING_ENERGY_REGENERATION,
        weapon_energy_cost: YWING_WEAPON_ENERGY_COST,
        special_energy_cost: SPECIAL_ENERGY_COST,
        energy_wait: ENERGY_WAIT,
        turn_wait: YWING_TURN_WAIT,
        thrust_wait: YWING_THRUST_WAIT,
        weapon_wait: YWING_WEAPON_WAIT,
        special_wait: SPECIAL_WAIT,
        ship_mass: SHIP_MASS,
    };

    fn descriptor(_world: &mut crate::engine::battle::World) -> crate::engine::battle::RaceDesc {
        let mut desc = RaceTemplate::default();
        desc.info.ship_flags = (FIRES_FORE | IMMEDIATE_WEAPON) as u16;
        desc.info.ship_cost = 19;
        desc.info.crew_level = MAX_CREW;
        desc.info.max_crew = MAX_CREW;
        desc.info.energy_level = MAX_ENERGY;
        desc.info.max_energy = MAX_ENERGY;
        desc.resources.strings = Some(c"ship.mmrnmhrm.text");
        desc.resources.icons = Some(c"ship.mmrnmhrm.icons");
        desc.resources.melee_icon = Some(c"ship.mmrnmhrm.meleeicons");
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
            Some(c"ship.mmrnmhrm.graphics.xform.large"),
            Some(c"ship.mmrnmhrm.graphics.xform.medium"),
            Some(c"ship.mmrnmhrm.graphics.xform.small"),
        ];
        desc.resources.weapon = [
            Some(c"ship.mmrnmhrm.graphics.torpedo.large"),
            Some(c"ship.mmrnmhrm.graphics.torpedo.medium"),
            Some(c"ship.mmrnmhrm.graphics.torpedo.small"),
        ];
        desc.resources.special = [
            Some(c"ship.mmrnmhrm.graphics.ywing.large"),
            Some(c"ship.mmrnmhrm.graphics.ywing.medium"),
            Some(c"ship.mmrnmhrm.graphics.ywing.small"),
        ];
        desc.resources.captain = Some(c"ship.mmrnmhrm.graphics.captain");
        desc.resources.victory = Some(c"ship.mmrnmhrm.ditty");
        desc.resources.sounds = Some(c"ship.mmrnmhrm.sounds");
        desc.intelligence.weapon_range = CLOSE_RANGE_WEAPON as u16;
        desc.state = RaceState::Mmrnmhrm {
            alternate: YWING_CHARACTERISTICS,
        };
        desc.descriptor(&MMRNMHRM)
    }
    use crate::engine::battle::views::Missile;
    use crate::engine::battle::{
        Concerns, Cx, ElementId, Race, RaceState, RaceTemplate, ShipId, WeaponOrigin, WeaponSpawn,
    };
    pub static MMRNMHRM: Race = Race {
        descriptor,
        preprocess: Some(mmrnmhrm_preprocess),
        postprocess: Some(mmrnmhrm_postprocess),
        init_weapon: Some(initialize_dual_weapons),
        intelligence: Some(mmrnmhrm_intelligence),
        uninit: Some(uninit_mmrnmhrm),
    };
    pub(crate) fn missile_preprocess(cx: &mut Cx<'_>, element: ElementId) {
        if cx.element(element).turn > 0 {
            cx.element_mut(element).turn -= 1;
        } else {
            let mut facing = cx.frame_number(cx.element(element).next.image.frame);
            if cx.track_ship(element, &mut facing) > 0 {
                cx.set_frame(
                    element,
                    true,
                    cx.absolute_frame(cx.element(element).next.image.frame, facing),
                );
                cx.element_mut(element).flags |= CHANGING as u16;
                cx.element_mut(element)
                    .velocity
                    .set_vector(20 << ONE_SHIFT, facing);
            }
            cx.element_mut(element).turn = TRACK_WAIT;
        }
    }

    pub(crate) fn mmrnmhrm_intelligence(cx: &mut Cx<'_>, ss: ShipId, concerns: &mut Concerns) {
        let enemy_eval = ENEMY_SHIP_INDEX as usize;
        let enemy = concerns.objects[enemy_eval].object;
        let can_transform = cx.ship_state(ss).special_counter == 0
            && cx.race_stats(ss).ship_info.energy_level
                >= cx.race_stats(ss).characteristics.special_energy_cost;
        let enemy_ss = enemy.map(|enemy| cx.owner(enemy));

        cx.ship_intelligence(ss, concerns);
        cx.ship_state_mut(ss).input &= !(SPECIAL as u8);
        if can_transform && enemy.is_some() && cx.ship_state(ss).input & WEAPON as u8 == 0 {
            let other = cx.element(enemy.unwrap_or_else(|| panic!("enemy ship")));
            let (x, y) = other.velocity.get_current_components();
            let (mut dx, mut dy) = (x as SIZE, y as SIZE);
            let (direction_angle, mut travel_angle);
            if dx == 0 && dy == 0 {
                direction_angle = 0;
                travel_angle = 0;
            } else {
                dx = (other.current.location.x as i32 - concerns.subject.current.location.x as i32)
                    as SIZE;
                dy = (other.current.location.y as i32 - concerns.subject.current.location.y as i32)
                    as SIZE;
                direction_angle = crate::math::arctan(-i32::from(dx) as _, -i32::from(dy) as _);
                travel_angle = other.velocity.travel_angle;
            }

            if concerns.subject.next.image.frames == cx.ship_frame_set(ss, 0) {
                if concerns.objects[enemy_eval].which_turn > 8
                    && (u32::from(
                        cx.race_stats(enemy_ss.unwrap_or_else(|| panic!("enemy ship")))
                            .intelligence
                            .maneuverability,
                    ) <= SLOW_SHIP
                        || normalize_angle(
                            direction_angle as i32 - travel_angle as i32 + QUADRANT as i32,
                        ) > HALF_CIRCLE as i32)
                {
                    cx.ship_state_mut(ss).input |= SPECIAL as u8;
                }
            } else {
                let (x, y) = concerns.subject.velocity.get_current_components();
                let (ship_dx, ship_dy) = (x as SIZE, y as SIZE);
                dx = dx.wrapping_sub(ship_dx);
                dy = dy.wrapping_sub(ship_dy);
                travel_angle = crate::math::arctan(dx, dy);
                if concerns.objects[enemy_eval].which_turn < 16 {
                    if concerns.objects[enemy_eval].which_turn <= 8
                        || normalize_angle(
                            direction_angle as i32 - travel_angle as i32 + OCTANT as i32,
                        ) <= QUADRANT as i32
                    {
                        cx.ship_state_mut(ss).input |= SPECIAL as u8;
                    }
                } else if concerns.objects[enemy_eval].which_turn > 32
                    && normalize_angle(
                        direction_angle as i32 - travel_angle as i32 + QUADRANT as i32,
                    ) > HALF_CIRCLE as i32
                {
                    cx.ship_state_mut(ss).input |= SPECIAL as u8;
                }
            }
        }
        if concerns.subject.current.image.frames == cx.ship_frame_set(ss, 2) {
            if cx.ship_state(ss).input & SPECIAL as u8 == 0 && enemy.is_some() {
                cx.ship_state_mut(ss).input |= WEAPON as u8;
            } else {
                cx.ship_state_mut(ss).input &= !(WEAPON as u8);
            }
        }
    }

    pub(crate) fn twin_laser_collision(
        cx: &mut Cx<'_>,
        e0: ElementId,
        p0: &mut POINT,
        e1: ElementId,
        p1: &mut POINT,
    ) {
        if cx.element(e1).flags & PLAYER_SHIP as u16 == 0
            || cx.element(e0).player != cx.element(e1).player
        {
            cx.weapon_collision(e0, p0, e1, p1);
        }
    }

    pub(crate) fn initialize_dual_weapons(
        cx: &mut Cx<'_>,
        ss: ShipId,
        origin: WeaponOrigin,
    ) -> WeaponSpawn {
        let facing = cx.ship_state(ss).facing;
        let angle = facing_to_angle(i32::from(facing));
        let position = origin.next.location;
        let x = position.x.wrapping_add(cosine(angle, 4 << ONE_SHIFT));
        let y = position.y.wrapping_add(sine(angle, 4 << ONE_SHIFT));
        let mut out = Vec::new();
        if origin.next.image.frames == cx.ship_frame_set(ss, 0) {
            let ex = x.wrapping_add(cosine(angle, 141 << ONE_SHIFT));
            let ey = y.wrapping_add(sine(angle, 141 << ONE_SHIFT));
            let ox = -sine(angle, 10 << ONE_SHIFT);
            let oy = cosine(angle, 10 << ONE_SHIFT);
            for sign in [1i32, -1] {
                let bx = x.wrapping_add((i32::from(ox) * sign) as _);
                let by = y.wrapping_add((i32::from(oy) * sign) as _);
                let laser = cx.spawn_laser(crate::engine::battle::views::Laser {
                    origin: POINT { x: bx, y: by },
                    end: POINT {
                        x: ex.wrapping_sub(bx),
                        y: ey.wrapping_sub(by),
                    },
                    sender: origin.player,
                    flags: 0,
                    pixoffs: 0,
                    face: facing,
                    color: Color {
                        r: 255,
                        g: 82,
                        b: 82,
                        a: 255,
                    },
                });
                if let Some(laser) = laser {
                    cx.element_mut(laser).collide =
                        Some(CollisionHook::Native(twin_laser_collision));
                }
                out.push(laser);
            }
        } else {
            let mut block = Missile {
                cx: 0,
                cy: 0,
                farray: cx.ship_frame_set(ss, 1),
                sender: origin.player,
                flags: IGNORE_SIMILAR as u16,
                pixoffs: 0,
                speed: 20 << ONE_SHIFT,
                hit_points: MISSILE_HITS,
                damage: MISSILE_DAMAGE,
                life: MISSILE_LIFE,
                preprocess: Some(missile_preprocess),
                blast_offs: 0,
                face: normalize_facing(i32::from(facing) - 1),
                index: normalize_facing(i32::from(facing) - 1),
            };
            let mut ox = -sine(facing_to_angle(i32::from(block.face)), 4 << ONE_SHIFT);
            let mut oy = cosine(facing_to_angle(i32::from(block.face)), 4 << ONE_SHIFT);
            for index in 0..2 {
                if index == 1 {
                    block.face = normalize_facing(i32::from(facing) + 1);
                    block.index = block.face;
                }
                block.cx = x.wrapping_add(ox);
                block.cy = y.wrapping_add(oy);
                let missile = cx.missile(&mut block);
                if let Some(missile) = missile {
                    cx.element_mut(missile).turn = TRACK_WAIT;
                }
                out.push(missile);
                ox = -ox;
                oy = -oy;
            }
        }
        out
    }

    pub(crate) fn mmrnmhrm_postprocess(cx: &mut Cx<'_>, ss: ShipId) {
        let ship = cx.ship_element(ss);
        if cx.element(ship).next.image.frames == cx.element(ship).current.image.frames {
            return;
        }
        cx.process_ship_sound(ss, 1, ship);
        cx.ship_state_mut(ss).weapon_counter = 0;
        let RaceState::Mmrnmhrm { alternate } = cx.race_stats(ss).state else {
            panic!("Mmrnmhrm alternate characteristics missing");
        };
        let current = cx.race_stats(ss).characteristics;
        cx.race_stats_mut(ss).state = RaceState::Mmrnmhrm { alternate: current };
        cx.race_stats_mut(ss).characteristics = alternate;
        cx.race_stats_mut(ss).intelligence.maneuverability = 0;
        if cx.element(ship).next.image.frames == cx.ship_frame_set(ss, 2) {
            cx.race_stats_mut(ss).intelligence.weapon_range = LONG_RANGE_WEAPON as u16 - 1;
            cx.race_stats_mut(ss).ship_info.ship_flags &= !(IMMEDIATE_WEAPON as u16);
            cx.race_stats_mut(ss).ship_info.ship_flags |= SEEKING_WEAPON as u16;
            cx.set_ship_sound_bank(ss, 2);
            cx.ship_state_mut(ss).status &= !((SHIP_AT_MAX_SPEED | SHIP_BEYOND_MAX_SPEED) as u16);
        } else {
            cx.race_stats_mut(ss).intelligence.weapon_range = CLOSE_RANGE_WEAPON as u16;
            cx.race_stats_mut(ss).ship_info.ship_flags &= !(SEEKING_WEAPON as u16);
            cx.race_stats_mut(ss).ship_info.ship_flags |= IMMEDIATE_WEAPON as u16;
            cx.set_ship_sound_bank(ss, 0);
            if cx.ship_state(ss).status & (SHIP_AT_MAX_SPEED | SHIP_BEYOND_MAX_SPEED) as u16 != 0 {
                cx.ship_state_mut(ss).status |= (SHIP_AT_MAX_SPEED | SHIP_BEYOND_MAX_SPEED) as u16;
            }
        }
    }

    pub(crate) fn mmrnmhrm_preprocess(cx: &mut Cx<'_>, ss: ShipId) {
        let ship = cx.ship_element(ss);
        if cx.element(ship).flags & APPEARING as u16 == 0
            && cx.ship_state(ss).status & SPECIAL as u16 != 0
            && cx.ship_state(ss).special_counter == 0
            && cx.delta_energy(
                ship,
                -i16::from(cx.race_stats(ss).characteristics.special_energy_cost),
            )
        {
            if cx.element(ship).next.image.frames == cx.ship_frame_set(ss, 0) {
                cx.set_frames(ship, true, cx.ship_frame_set(ss, 2));
            } else {
                cx.set_frames(ship, true, cx.ship_frame_set(ss, 0));
            }
            cx.set_frame(
                ship,
                true,
                cx.equivalent_frame(
                    cx.element_frame_set_first(ship, true),
                    cx.element(ship).next.image.frame,
                ),
            );
            cx.element_mut(ship).flags |= CHANGING as u16;
            cx.ship_state_mut(ss).special_counter = cx.race_stats(ss).characteristics.special_wait;
        }
    }
    pub(crate) fn uninit_mmrnmhrm(
        _world: &mut crate::engine::battle::World,
        stats: &mut crate::engine::battle::views::RaceStats,
    ) {
        stats.state = RaceState::None;
    }
}
