//! Ur-Quan live fusion and fighter callbacks, translated from ships/urquan/urquan.c.
#[cfg(not(test))]
pub(crate) mod engine {
    use crate::ships::native::{
        abi::*, angle_to_facing, cosine, facing_to_angle, normalize_angle, normalize_facing, sine,
        wrap_delta,
    };
    const fn color(r: u8, g: u8, b: u8) -> Color {
        Color {
            r: (r << 3) | (r >> 2),
            g: (g << 3) | (g >> 2),
            b: (b << 3) | (b >> 2),
            a: 255,
        }
    }
    fn descriptor(_world: &mut crate::engine::battle::World) -> crate::engine::battle::RaceDesc {
        let mut d = RaceTemplate::default();
        d.info.ship_flags = (FIRES_FORE as i32 | SEEKING_SPECIAL as i32) as _;
        d.info.ship_cost = 30;
        d.info.crew_level = (MAX_CREW_SIZE as i32) as _;
        d.info.max_crew = (MAX_CREW_SIZE as i32) as _;
        d.info.energy_level = (MAX_ENERGY_SIZE as i32) as _;
        d.info.max_energy = (MAX_ENERGY_SIZE as i32) as _;
        d.resources.strings = Some(c"ship.urquan.text");
        d.resources.icons = Some(c"ship.urquan.icons");
        d.resources.melee_icon = Some(c"ship.urquan.meleeicons");
        d.fleet_strength = (2666 / SPHERE_RADIUS_INCREMENT as i32 * 2) as u16;
        d.fleet_location = POINT { x: 5750, y: 6000 };
        d.characteristics.max_thrust = 30;
        d.characteristics.thrust_increment = 6;
        d.characteristics.energy_regeneration = 1;
        d.characteristics.weapon_energy_cost = 6;
        d.characteristics.special_energy_cost = 8;
        d.characteristics.energy_wait = 4;
        d.characteristics.turn_wait = 4;
        d.characteristics.thrust_wait = 6;
        d.characteristics.weapon_wait = 6;
        d.characteristics.special_wait = 9;
        d.characteristics.ship_mass = 10;
        d.resources.ship = [
            Some(c"ship.urquan.graphics.dreadnought.large"),
            Some(c"ship.urquan.graphics.dreadnought.medium"),
            Some(c"ship.urquan.graphics.dreadnought.small"),
        ];
        d.resources.weapon = [
            Some(c"ship.urquan.graphics.fusion.large"),
            Some(c"ship.urquan.graphics.fusion.medium"),
            Some(c"ship.urquan.graphics.fusion.small"),
        ];
        d.resources.special = [
            Some(c"ship.urquan.graphics.fighter.large"),
            Some(c"ship.urquan.graphics.fighter.medium"),
            Some(c"ship.urquan.graphics.fighter.small"),
        ];
        d.resources.captain = Some(c"ship.urquan.graphics.captain");
        d.resources.victory = Some(c"ship.urquan.ditty");
        d.resources.sounds = Some(c"ship.urquan.sounds");
        d.intelligence.weapon_range = (((20) << ONE_SHIFT as i32) * (20)) as u16;
        d.descriptor(&URQUAN)
    }
    use crate::engine::battle::views::Missile;
    use crate::engine::battle::{
        Concerns, Cx, ElementId, Race, RaceTemplate, ShipId, WeaponOrigin, WeaponSpawn,
    };
    pub static URQUAN: Race = Race {
        descriptor,
        preprocess: None,
        postprocess: Some(urquan_postprocess),
        init_weapon: Some(initialize_fusion),
        intelligence: Some(urquan_intelligence),
        uninit: None,
    };
    pub(crate) fn initialize_fusion(
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
            pixoffs: 32,
            speed: 20 << ONE_SHIFT,
            hit_points: 10,
            damage: 6,
            life: 20,
            preprocess: None,
            blast_offs: 8,
        };
        out.push(cx.missile(&mut block));
        out
    }
    pub(crate) fn fighter_postprocess(cx: &mut Cx<'_>, e: ElementId) {
        let ss = cx.owner(e);
        let face = COUNT::from(cx.element(e).thrust);
        let laser = crate::engine::battle::views::Laser {
            origin: cx.element(e).next.location,
            end: POINT {
                x: cosine(facing_to_angle(i32::from(face)), 44 << ONE_SHIFT),
                y: sine(facing_to_angle(i32::from(face)), 44 << ONE_SHIFT),
            },
            face,
            sender: cx.element(e).player,
            flags: IGNORE_SIMILAR as u16,
            pixoffs: 4,
            color: color(0x1f, 0x1f, 0x0a),
        };
        if let Some(laser) = cx.spawn_laser(laser) {
            cx.set_parent(laser, Some(ss));
            cx.process_ship_sound(ss, 2, laser);
            cx.put(laser);
        }
        cx.element_mut(e).post = None;
        cx.element_mut(e).thrust = 8;
    }
    pub(crate) fn fighter_preprocess(cx: &mut Cx<'_>, e: ElementId) {
        let ss = cx.owner(e);
        cx.race_stats_mut(ss).characteristics.special_wait = cx
            .race_stats(ss)
            .characteristics
            .special_wait
            .wrapping_add(1);
        if 400 - i32::from(cx.element(e).life) > 6 && cx.element(e).flags & CHANGING as u16 == 0 {
            let mut enroute = true;
            let mut dx = cx.race_stats(ss).ship_info.crew_level as SIZE;
            let mut dy = cx.element(e).life as SIZE;
            let orig = cx.frame_number(cx.element(e).current.image.frame);
            let mut face = orig;
            if (dy & 1 != 0 || cx.target(e).is_some() || cx.track_ship(e, &mut face) >= 0)
                && (dx == 0 || dy >= 125)
            {
                cx.element_mut(e).flags |= IGNORE_SIMILAR as u16;
            } else if dx != 0 {
                let ship = cx.ship_element(ss);
                let destination = cx.element(ship).current.location;
                let position = cx.element(e).current.location;
                let (width, height) = cx.screen_dimensions();
                dx = wrap_delta(
                    destination.x.wrapping_sub(position.x),
                    (width - 64) << (ONE_SHIFT + 3),
                );
                dy = wrap_delta(
                    destination.y.wrapping_sub(position.y),
                    height << (ONE_SHIFT + 3),
                );
                face = normalize_facing(angle_to_facing(i32::from(crate::math::arctan(dx, dy))));
                cx.element_mut(e).flags &= !(IGNORE_SIMILAR as u16);
                enroute = false;
            }
            if cx.element(e).thrust > 0 {
                cx.element_mut(e).thrust -= 1;
            }
            if cx.target(e).is_some() {
                let target = cx.target(e).unwrap_or_else(|| panic!("tracked target"));
                let destination = cx.element(target).current.location;
                let position = cx.element(e).current.location;
                let (width, height) = cx.screen_dimensions();
                dx = wrap_delta(
                    destination.x.wrapping_sub(position.x),
                    (width - 64) << (ONE_SHIFT + 3),
                );
                dy = wrap_delta(
                    destination.y.wrapping_sub(position.y),
                    height << (ONE_SHIFT + 3),
                );
                const RANGE: i32 = (44 << ONE_SHIFT) * 3 / 4;
                if cx.element(e).thrust == 0
                    && i32::from(dx).abs() < RANGE
                    && i32::from(dy).abs() < RANGE
                    && i32::from(dx) * i32::from(dx) + i32::from(dy) * i32::from(dy) < RANGE * RANGE
                {
                    cx.element_mut(e).thrust =
                        normalize_facing(angle_to_facing(i32::from(crate::math::arctan(dx, dy))))
                            as BYTE;
                    cx.element_mut(e).post = Some(ElementHook::Native(fighter_postprocess));
                }
                if enroute {
                    face = cx.frame_number(cx.element(target).current.image.frame);
                    let offset_face = if cx.element(e).turn & LEFT as u8 != 0 {
                        i32::from(face) - 4
                    } else {
                        i32::from(face) + 4
                    };
                    dx = dx.wrapping_add(cosine(facing_to_angle(offset_face), 30 << ONE_SHIFT));
                    dy = dy.wrapping_add(sine(facing_to_angle(offset_face), 30 << ONE_SHIFT));
                    face =
                        normalize_facing(angle_to_facing(i32::from(crate::math::arctan(dx, dy))));
                }
            }
            cx.element_mut(e).flags |= CHANGING as u16;
            if face != orig {
                cx.set_frame(
                    e,
                    true,
                    cx.absolute_frame(cx.element(e).next.image.frame, face),
                );
            }
            cx.element_mut(e).velocity.set_vector(8 << ONE_SHIFT, face);
        }
    }
    pub(crate) fn fighter_collision(
        cx: &mut Cx<'_>,
        a: ElementId,
        pa: &mut POINT,
        b: ElementId,
        pb: &mut POINT,
    ) {
        let ss = cx.owner(a);
        if u32::from(cx.element(b).mass) > MAX_SHIP_MASS * 10 {
            let h = cx.alloc_element();
            if let Some(fighter) = h {
                cx.copy_allocated_element(fighter, a);
                cx.element_mut(fighter).flags &= !(PRE_PROCESS as u16);
                cx.element_mut(fighter).flags |= CHANGING as u16;
                let current = cx.element(fighter).current;
                cx.copy_visual_to_next(fighter, current);
                let mut travel = cx.element(fighter).velocity.travel_angle;
                let delta = normalize_angle(
                    i32::from(crate::math::arctan(
                        (pb).x.wrapping_sub((pa).x),
                        (pb).y.wrapping_sub((pa).y),
                    )) - i32::from(travel),
                ) as SIZE;
                if (delta == 0 && cx.element(fighter).turn & LEFT as u8 != 0)
                    || (delta != 0 && i32::from(delta) <= HALF_CIRCLE as i32)
                {
                    travel = travel.wrapping_sub(QUADRANT as COUNT);
                } else {
                    travel = travel.wrapping_add(QUADRANT as COUNT);
                }
                travel = normalize_facing(angle_to_facing(normalize_angle(i32::from(travel))));
                cx.set_frame(
                    fighter,
                    true,
                    cx.absolute_frame(cx.element(fighter).next.image.frame, travel),
                );
                cx.element_mut(fighter)
                    .velocity
                    .set_vector(8 << ONE_SHIFT, travel);
                cx.put(fighter);
            }
            cx.element_mut(a).flags |= (DISAPPEARING | COLLISION) as u16;
        } else if cx.parent(a) != cx.parent(b) {
            cx.element_mut(a).thrust = 0;
            cx.weapon_collision(a, pa, b, pb);
            cx.element_mut(a).flags |= (DISAPPEARING | COLLISION) as u16;
        } else if cx.element(b).flags & PLAYER_SHIP as u16 != 0 {
            cx.process_ship_sound(ss, 3, b);
            cx.delta_crew(b, 1);
            cx.element_mut(a).flags |= (DISAPPEARING | COLLISION) as u16;
        }
        if cx.element(a).flags & DISAPPEARING as u16 != 0 {
            cx.element_mut(a).flags &= !(DISAPPEARING as u16);
            cx.element_mut(a).hits = 0;
            cx.element_mut(a).life = 0;
            cx.element_mut(a).flags |= NONSOLID as u16;
            cx.race_stats_mut(ss).characteristics.special_wait = cx
                .race_stats(ss)
                .characteristics
                .special_wait
                .wrapping_sub(1);
        }
    }
    pub(crate) fn spawn_fighters(cx: &mut Cx<'_>, e: ElementId) {
        let ss = cx.owner(e);
        let face =
            (i32::from(cx.ship_state(ss).facing) + angle_to_facing(HALF_CIRCLE as i32)) as COUNT;
        let dx = cosine(facing_to_angle(i32::from(face)), 14 << ONE_SHIFT);
        let dy = sine(facing_to_angle(i32::from(face)), 14 << ONE_SHIFT);
        let mut i = if cx.element(e).hits > 2 { 2 } else { 1 };
        while i != 0 {
            i -= 1;
            let Some(fighter) = cx.alloc_element() else {
                break;
            };
            cx.delta_crew(e, -1);
            cx.put(fighter);
            cx.element_mut(fighter).hits = 1;
            cx.element_mut(fighter).mass = 0;
            cx.element_mut(fighter).thrust = 7;
            cx.element_mut(fighter).player = cx.element(e).player;
            cx.element_mut(fighter).flags =
                (APPEARING | FINITE_LIFE | CREW_OBJECT | IGNORE_SIMILAR) as u16;
            cx.element_mut(fighter).life = 400;
            cx.primitive_style_mut(fighter).kind = gfx_object_STAMP_PRIM as GRAPHICS_PRIM;
            cx.element_mut(fighter).pre = Some(ElementHook::Native(fighter_preprocess));
            cx.element_mut(fighter).post = None;
            cx.element_mut(fighter).collide = Some(CollisionHook::Native(fighter_collision));
            cx.element_mut(fighter).death = None;
            cx.element_mut(fighter).current.location = cx.element(e).next.location;
            let fighter_face;
            if i == 1 {
                cx.element_mut(fighter).turn = LEFT as BYTE;
                fighter_face = normalize_facing(i32::from(face) + 2);
                cx.element_mut(fighter).current.location.x =
                    (i32::from(cx.element(fighter).current.location.x) + i32::from(dx)
                        - i32::from(dy)) as COORD;
                cx.element_mut(fighter).current.location.y =
                    (i32::from(cx.element(fighter).current.location.y)
                        + i32::from(dy)
                        + i32::from(dx)) as COORD;
            } else {
                cx.element_mut(fighter).turn = RIGHT as BYTE;
                fighter_face = normalize_facing(i32::from(face) - 2);
                cx.element_mut(fighter).current.location.x =
                    (i32::from(cx.element(fighter).current.location.x)
                        + i32::from(dx)
                        + i32::from(dy)) as COORD;
                cx.element_mut(fighter).current.location.y =
                    (i32::from(cx.element(fighter).current.location.y) + i32::from(dy)
                        - i32::from(dx)) as COORD;
            }
            let sx = cosine(
                facing_to_angle(i32::from(fighter_face)),
                (8 << ONE_SHIFT) << VELOCITY_SHIFT,
            );
            let sy = sine(
                facing_to_angle(i32::from(fighter_face)),
                (8 << ONE_SHIFT) << VELOCITY_SHIFT,
            );
            cx.element_mut(fighter)
                .velocity
                .set_components(i32::from(sx), i32::from(sy));
            let position = cx.element(fighter).current.location;
            cx.element_mut(fighter).current.location = POINT {
                x: position.x.wrapping_sub(sx >> VELOCITY_SHIFT),
                y: position.y.wrapping_sub(sy >> VELOCITY_SHIFT),
            };
            cx.set_frames(fighter, false, cx.ship_frame_set(ss, 2));
            cx.set_frame(
                fighter,
                false,
                cx.absolute_frame(cx.ship_frame(ss, 2), fighter_face),
            );
            cx.set_parent(fighter, Some(ss));
        }
    }
    pub(crate) fn urquan_intelligence(cx: &mut Cx<'_>, ss: ShipId, concerns: &mut Concerns) {
        let enemy = ENEMY_SHIP_INDEX as usize;
        concerns.objects[enemy].movement = MOVEMENT_STATE_PURSUE;
        let weapon = ENEMY_WEAPON_INDEX as usize;
        if concerns.objects[weapon].object.is_some()
            && concerns.objects[weapon].movement == MOVEMENT_STATE_ENTICE
            && (cx
                .element(
                    concerns.objects[weapon]
                        .object
                        .unwrap_or_else(|| panic!("AI concern")),
                )
                .flags
                & CREW_OBJECT as u16
                == 0
                || concerns.objects[weapon].which_turn <= 8)
            && (cx
                .element(
                    concerns.objects[weapon]
                        .object
                        .unwrap_or_else(|| panic!("AI concern")),
                )
                .flags
                & FINITE_LIFE as u16
                == 0
                || (cx
                    .element(
                        concerns.objects[weapon]
                            .object
                            .unwrap_or_else(|| panic!("AI concern")),
                    )
                    .mass
                    >= 4
                    && concerns.objects[weapon].which_turn == 2
                    && concerns.objects[enemy].which_turn > 16))
        {
            concerns.objects[weapon].movement = MOVEMENT_STATE_PURSUE;
        }
        cx.ship_intelligence(ss, concerns);
        let other = concerns.objects[enemy].object.map(|enemy| cx.owner(enemy));
        if cx.ship_state(ss).special_counter == 0
            && concerns.objects[enemy].object.is_some()
            && cx.race_stats(ss).ship_info.crew_level > cx.race_stats(ss).ship_info.max_crew >> 2
            && cx
                .race_stats(other.unwrap_or_else(|| panic!("enemy ship")))
                .ship_info
                .ship_flags
                & POINT_DEFENSE as u16
                == 0
            && (cx.race_stats(ss).characteristics.special_wait < 6
                || (cx
                    .race_stats(other.unwrap_or_else(|| panic!("enemy ship")))
                    .intelligence
                    .maneuverability
                    <= SLOW_SHIP as COUNT
                    && cx
                        .ship_state(other.unwrap_or_else(|| panic!("enemy ship")))
                        .status
                        & SHIP_BEYOND_MAX_SPEED as u16
                        == 0)
                || (concerns.objects[enemy].which_turn <= 12
                    && cx.ship_state(ss).input & (LEFT | RIGHT) as u8 != 0
                    && cx.race_stats(ss).ship_info.energy_level
                        >= cx.race_stats(ss).ship_info.max_energy >> 1))
        {
            cx.ship_state_mut(ss).input |= SPECIAL as u8;
        } else {
            cx.ship_state_mut(ss).input &= !(SPECIAL as u8);
        }
        cx.race_stats_mut(ss).characteristics.special_wait = 0;
    }
    pub(crate) fn urquan_postprocess(cx: &mut Cx<'_>, ss: ShipId) {
        let e = cx.ship_element(ss);
        if cx.ship_state(ss).status & SPECIAL as u16 != 0
            && cx.element(e).hits > 1
            && cx.ship_state(ss).special_counter == 0
            && cx.delta_energy(e, -8)
        {
            cx.process_ship_sound(ss, 1, e);
            spawn_fighters(cx, e);
            cx.ship_state_mut(ss).special_counter = 9;
        }
    }
}
