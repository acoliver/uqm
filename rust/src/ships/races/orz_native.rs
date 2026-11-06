//! Orz callbacks and descriptor translated from `sc2/src/uqm/ships/orz/orz.c`.

#[cfg(not(test))]
pub(crate) mod engine {
    #[cfg(not(test))]
    use crate::battle::context_access::race_access::RaceOps;
    #[cfg(not(test))]
    use crate::battle::context_access::ContextOps;
    use crate::engine::battle::views::Missile;
    use crate::engine::battle::{
        Concerns, Cx, ElementId, Race, RaceTemplate, ShipId, WeaponOrigin, WeaponSpawn,
    };
    use crate::ships::native::{
        abi::*, angle_to_facing, cosine, facing_to_angle, normalize_angle, normalize_facing, sine,
    };

    const MODIFY_SWAP: BYTE = 1 << 1;
    // The cast in element.h prevents bindgen from emitting this macro.
    const GRAVITY_THRESHOLD: COUNT = 255;
    const MAX_CREW: u16 = 16;
    const MAX_ENERGY: u8 = 20;
    const MISSILE_SPEED: i16 = 30 << ONE_SHIFT;
    const MISSILE_LIFE: u16 = 12;
    const TURRET_OFFSET: i32 = 14;
    const TURRET_WAIT: u8 = 3;
    const MAX_MARINES: u8 = 8;
    const MARINE_WAIT: u8 = 12;
    const MARINE_MAX_THRUST: u16 = 32;
    const MARINE_THRUST_INCREMENT: u16 = 8;
    const ION_LIFE: u8 = 1;

    const fn color(r: u8, g: u8, b: u8) -> Color {
        Color {
            r: (r << 3) | (r >> 2),
            g: (g << 3) | (g >> 2),
            b: (b << 3) | (b >> 2),
            a: 255,
        }
    }

    const ION_COLORS: [Color; 5] = [
        color(0x1f, 0x15, 0),
        color(0x1f, 0x0e, 0),
        color(0x1f, 0x03, 0),
        color(0x1b, 0, 0),
        color(0x13, 0, 0),
    ];

    fn descriptor(_world: &mut crate::engine::battle::World) -> crate::engine::battle::RaceDesc {
        let mut d = RaceTemplate::default();
        d.info.ship_flags = (FIRES_FORE | SEEKING_SPECIAL) as u16;
        d.info.ship_cost = 23;
        d.info.crew_level = MAX_CREW;
        d.info.max_crew = MAX_CREW;
        d.info.energy_level = MAX_ENERGY;
        d.info.max_energy = MAX_ENERGY;
        d.resources.strings = Some(c"ship.orz.text");
        d.resources.icons = Some(c"ship.orz.icons");
        d.resources.melee_icon = Some(c"ship.orz.meleeicons");
        d.fleet_strength = (333 / SPHERE_RADIUS_INCREMENT * 2) as u16;
        d.fleet_location = POINT { x: 3608, y: 2637 };
        d.characteristics = CHARACTERISTIC_STUFF {
            max_thrust: 35,
            thrust_increment: 5,
            energy_regeneration: 1,
            weapon_energy_cost: MAX_ENERGY / 3,
            special_energy_cost: 0,
            energy_wait: 6,
            turn_wait: 1,
            thrust_wait: 0,
            weapon_wait: 4,
            special_wait: 12,
            ship_mass: 4,
        };
        d.resources.ship = [
            Some(c"ship.orz.graphics.nemesis.large"),
            Some(c"ship.orz.graphics.nemesis.medium"),
            Some(c"ship.orz.graphics.nemesis.small"),
        ];
        d.resources.weapon = [
            Some(c"ship.orz.graphics.howitzer.large"),
            Some(c"ship.orz.graphics.howitzer.medium"),
            Some(c"ship.orz.graphics.howitzer.small"),
        ];
        d.resources.special = [
            Some(c"ship.orz.graphics.turret.large"),
            Some(c"ship.orz.graphics.turret.medium"),
            Some(c"ship.orz.graphics.turret.small"),
        ];
        d.resources.captain = Some(c"ship.orz.graphics.captain");
        d.resources.victory = Some(c"ship.orz.ditty");
        d.resources.sounds = Some(c"ship.orz.sounds");
        d.intelligence.weapon_range = (MISSILE_SPEED as i32 * MISSILE_LIFE as i32) as u16;
        d.descriptor(&ORZ)
    }

    pub static ORZ: Race = Race {
        descriptor,
        preprocess: Some(orz_preprocess),
        postprocess: None,
        init_weapon: Some(initialize_turret_missile),
        intelligence: Some(orz_intelligence),
        uninit: None,
    };

    // SIZE assignments happen both before and after the C WRAP_DELTA macros.
    fn wrap_delta(delta: SIZE, extent: i32) -> SIZE {
        let delta = i32::from(delta);
        if delta < 0 {
            if -delta <= extent >> 1 {
                delta as SIZE
            } else {
                (extent + delta) as SIZE
            }
        } else if delta <= extent >> 1 {
            delta as SIZE
        } else {
            (delta - extent) as SIZE
        }
    }

    fn wrapped_deltas(cx: &Cx<'_>, a: ElementId, b: ElementId) -> (SIZE, SIZE) {
        let dx = (i32::from(cx.element(a).current.location.x)
            - i32::from(cx.element(b).current.location.x)) as SIZE;
        let dy = (i32::from(cx.element(a).current.location.y)
            - i32::from(cx.element(b).current.location.y)) as SIZE;
        (
            wrap_delta(dx, ((cx.screen_dimensions().0 - 64) << ONE_SHIFT) << 3),
            wrap_delta(dy, (cx.screen_dimensions().1 << ONE_SHIFT) << 3),
        )
    }

    pub(crate) fn howitzer_collision(
        cx: &mut Cx<'_>,
        a: ElementId,
        pa: &mut POINT,
        b: ElementId,
        pb: &mut POINT,
    ) {
        if cx.element(a).player != cx.element(b).player {
            cx.weapon_collision(a, pa, b, pb);
        }
    }

    pub(crate) fn initialize_turret_missile(
        cx: &mut Cx<'_>,
        ss: ShipId,
        origin: WeaponOrigin,
    ) -> WeaponSpawn {
        let ship = cx.ship_element(ss);
        let turret = cx
            .elements()
            .succ(ship)
            .unwrap_or_else(|| panic!("Orz turret successor"));
        if cx.element(turret).turn == 0
            && cx.ship_state(ss).status & SPECIAL as u16 != 0
            && cx.ship_state(ss).status & (LEFT | RIGHT) as u16 != 0
        {
            if cx.ship_state(ss).status & RIGHT as u16 != 0 {
                cx.element_mut(turret).thrust = cx.element(turret).thrust.wrapping_add(1);
            } else {
                cx.element_mut(turret).thrust = cx.element(turret).thrust.wrapping_sub(1);
            }
            cx.element_mut(turret).turn = TURRET_WAIT + 1;
        }
        let facing = normalize_facing(
            i32::from(cx.ship_state(ss).facing) + i32::from(cx.element(turret).thrust),
        );
        let mut block = Missile {
            cx: origin.next.location.x,
            cy: origin.next.location.y,
            frames: cx.ship_frame_set(ss, 1),
            face: facing,
            index: facing,
            sender: origin.player,
            flags: IGNORE_SIMILAR as u16,
            pixoffs: TURRET_OFFSET as SIZE,
            speed: MISSILE_SPEED,
            hit_points: 2,
            damage: 3,
            life: MISSILE_LIFE,
            preprocess: None,
            blast_offs: 1,
        };
        let out = cx.missile(&mut block);
        if let Some(out) = out {
            cx.element_mut(out).collide = Some(CollisionHook::Native(howitzer_collision));
        }
        vec![out]
    }

    fn count_marines(cx: &Cx<'_>, ss: ShipId, find_spot: bool) -> BYTE {
        let mut id_use = [0u8; MAX_MARINES as usize];
        let mut count = 0u8;
        let mut h = cx.elements().tail();
        while let Some(entry) = h {
            let e = entry;
            let mut next = cx.elements().pred(e);
            if cx.element(e).current.image.frames == cx.ship_frame_set(ss, 2)
                && cx.element(e).life != 0
                && cx.element(e).flags & (FINITE_LIFE | DISAPPEARING) as u16 == 0
            {
                if cx.element(e).flags & NONSOLID as u16 != 0 {
                    id_use[cx.element(e).turn as usize] = 1;
                }
                count = count.wrapping_add(1);
                if count == MAX_MARINES {
                    next = None;
                }
            }
            h = next;
        }
        if find_spot {
            count = 0;
            while id_use[count as usize] != 0 {
                count = count.wrapping_add(1);
            }
        }
        count
    }

    pub(crate) fn orz_intelligence(cx: &mut Cx<'_>, ss: ShipId, concerns: &mut Concerns) {
        let ship = cx.ship_element(ss);
        let turret = cx
            .elements()
            .succ(ship)
            .unwrap_or_else(|| panic!("Orz turret successor"));
        cx.element_mut(turret).turn = cx.element(turret).turn.wrapping_add(1);
        cx.ship_intelligence(ss, concerns);
        cx.element_mut(turret).turn = cx.element(turret).turn.wrapping_sub(1);
        let enemy = concerns.objects[ENEMY_SHIP_INDEX as usize];
        if enemy.object.is_none() {
            cx.ship_state_mut(ss).input &= !(SPECIAL as u8);
        } else if cx.ship_state(ss).special_counter != 1 {
            let target = enemy.object.unwrap_or_else(|| panic!("enemy ship"));
            if cx.element(ship).turn == 0
                && enemy.movement == MOVEMENT_STATE_ENTICE
                && enemy.which_turn < 24
                && cx.ship_state(ss).status & (SHIP_AT_MAX_SPEED | SHIP_BEYOND_MAX_SPEED) as u16
                    != 0
                && cx.ship_state(ss).input & THRUST as u8 == 0
                && normalize_angle(
                    i32::from(cx.element(ship).velocity.travel_angle)
                        - i32::from(crate::math::arctan(
                            (i32::from(cx.element(target).next.location.x)
                                - i32::from(cx.element(ship).next.location.x))
                                as SIZE,
                            (i32::from(cx.element(target).next.location.y)
                                - i32::from(cx.element(ship).next.location.y))
                                as SIZE,
                        ))
                        + (QUADRANT as i32 - (OCTANT as i32 >> 1)),
                ) >= ((QUADRANT as i32 - (OCTANT as i32 >> 1)) << 1)
            {
                cx.ship_state_mut(ss).input &= !((LEFT | RIGHT) as u8);
            }
            cx.ship_state_mut(ss).input &= !(SPECIAL as u8);
            if cx.element(ship).turn == 0
                && cx.ship_state(ss).input & (LEFT | RIGHT | WEAPON) as u8 == 0
                && cx.element(turret).turn == 0
            {
                let mut facing = normalize_facing(
                    i32::from(cx.ship_state(ss).facing) + i32::from(cx.element(turret).thrust),
                );
                let mut delta = cx.track_ship(turret, &mut facing);
                if delta > 0 {
                    cx.ship_state_mut(ss).input |= SPECIAL as u8;
                    if i32::from(delta) == angle_to_facing(HALF_CIRCLE as i32) {
                        delta = (i32::from(delta) + (((cx.random() as u8 & 1) as i32) << 1) - 1)
                            as SIZE;
                    }
                    if i32::from(delta) < angle_to_facing(HALF_CIRCLE as i32) {
                        cx.ship_state_mut(ss).input |= RIGHT as u8;
                    } else {
                        cx.ship_state_mut(ss).input |= LEFT as u8;
                    }
                }
            }
            let enemy_ss = cx.owner(target);
            if cx.ship_state(ss).special_counter == 0
                && cx.ship_state(ss).input & WEAPON as u8 == 0
                && cx.race_stats(ss).ship_info.crew_level
                    > (cx.race_stats(ss).ship_info.max_crew >> 2) as BYTE as u16
                && cx.race_stats(enemy_ss).ship_info.ship_flags & POINT_DEFENSE as u16 == 0
                && (u32::from(cx.race_stats(enemy_ss).intelligence.maneuverability) < SLOW_SHIP
                    || enemy.which_turn <= 12
                    || count_marines(cx, ss, false) < 2)
            {
                cx.ship_state_mut(ss).input |= (WEAPON | SPECIAL) as u8;
            }
        }
    }

    pub(crate) fn ion_preprocess(cx: &mut Cx<'_>, e: ElementId) {
        cx.element_mut(e).color_cycle = cx.element(e).color_cycle.wrapping_add(1);
        if usize::from(cx.element(e).color_cycle) != ION_COLORS.len() {
            cx.element_mut(e).life = u16::from(cx.element(e).thrust);
            cx.primitive_style_mut(e).color = ION_COLORS[usize::from(cx.element(e).color_cycle)];
            cx.element_mut(e).flags &= !(DISAPPEARING as u16);
            cx.element_mut(e).flags |= CHANGING as u16;
        }
    }

    fn silhouette(
        cx: &mut Cx<'_>,
        ship: ElementId,
        marine: ElementId,
        frame: crate::engine::battle::views::FrameSelection,
        flags: BYTE,
    ) {
        let origin = POINT {
            x: (16 + i32::from(cx.element(marine).turn & 3) * 9) as COORD,
            y: (14 + i32::from(cx.element(marine).turn >> 2) * 11) as COORD,
        };
        cx.silhouette_at(ship, origin, frame, flags);
    }

    fn owner_departed(cx: &Cx<'_>, owner: ElementId) -> bool {
        let owner = cx.element(owner);
        owner.hits == 0
            && owner.life == 1
            && owner.flags & (FINITE_LIFE | NONSOLID) as u16 == (FINITE_LIFE | NONSOLID) as u16
    }

    pub fn intruder_preprocess(cx: &mut Cx<'_>, e: ElementId) {
        let ss = cx.owner(e);
        let owner = cx.ship_element(ss);
        if owner_departed(cx, owner) {
            cx.element_mut(e).life = 0;
            cx.element_mut(e).flags |= DISAPPEARING as u16;
        }
        if cx.element(e).thrust != 0 {
            cx.element_mut(e).thrust -= 1;
        }
        let mut h = cx.elements().head();
        while let Some(ship) = h {
            if cx.element(ship).flags & PLAYER_SHIP as u16 != 0
                && cx.element(ship).player != cx.element(e).player
            {
                if cx.element(e).thrust == MARINE_WAIT {
                    cx.element_mut(e).thrust -= 1;
                    let frame = cx.element_frame_set_first(e, true);
                    let frame = cx.absolute_frame(frame, cx.frame_count(frame).wrapping_sub(2));
                    silhouette(cx, ship, e, frame, 0);
                }
                cx.element_mut(e).next.location = cx.element(ship).next.location;
                let mut left_ship = cx.element(ship).hits == 0 || cx.element(e).life == 0;
                if left_ship {
                    h = None;
                } else if cx.element(e).thrust == 0 {
                    cx.element_mut(e).thrust = MARINE_WAIT;
                    let randval = cx.random() as BYTE;
                    if u16::from(randval) < 0x100 / 16 {
                        cx.element_mut(e).life = 0;
                        cx.element_mut(e).flags |= DISAPPEARING as u16;
                        cx.process_ship_sound(ss, 4, e);
                        left_ship = true;
                    } else if u16::from(randval) < 0x100 / 2 + 0x100 / 16 {
                        if !cx.delta_crew(ship, -1) {
                            cx.element_mut(ship).life = 0;
                        }
                        cx.element_mut(e).thrust = cx.element(e).thrust.wrapping_add(1);
                        let frame = cx.element_frame_set_first(e, true);
                        let frame = cx.absolute_frame(frame, cx.frame_count(frame).wrapping_sub(1));
                        silhouette(cx, ship, e, frame, 0);
                        cx.process_ship_sound(ss, 3, e);
                    }
                }
                if left_ship {
                    silhouette(
                        cx,
                        ship,
                        e,
                        cx.element(e).next.image.frame,
                        MODIFY_SWAP as BYTE,
                    );
                }
                break;
            }
            h = cx.elements().succ(ship);
        }
        if h.is_none() && cx.element(e).life != 0 {
            cx.element_mut(e).flags &= !(NONSOLID as u16);
            cx.element_mut(e).flags |= (CHANGING | CREW_OBJECT) as u16;
            cx.primitive_style_mut(e).kind = gfx_object_STAMP_PRIM as GRAPHICS_PRIM;
            let frame = cx.absolute_frame(cx.ship_frame(ss, 2), 21);
            cx.set_frame(e, false, frame.clone());
            cx.set_frame(e, true, frame);
            cx.element_mut(e).thrust = 0;
            cx.element_mut(e).turn =
                (normalize_facing(i32::from(cx.random() as BYTE)) << 4) as BYTE;
            cx.element_mut(e).pre = Some(ElementHook::Native(marine_preprocess));
        }
    }

    fn spawn_marine_ion_trail(cx: &mut Cx<'_>, e: ElementId, ss: ShipId, facing: COUNT) {
        let h = cx.alloc_element();
        if let Some(h) = h {
            let angle = (facing_to_angle(i32::from(facing)) + HALF_CIRCLE as i32) as COUNT;
            cx.insert_before(h, cx.elements().head());
            let ion = h;
            cx.element_mut(ion).player = NEUTRAL_PLAYER_NUM as SIZE;
            cx.element_mut(ion).flags = (APPEARING | FINITE_LIFE | NONSOLID) as u16;
            cx.element_mut(ion).thrust = ION_LIFE;
            cx.element_mut(ion).life = u16::from(ION_LIFE);
            cx.primitive_style_mut(ion).kind = gfx_object_POINT_PRIM as GRAPHICS_PRIM;
            cx.primitive_style_mut(ion).color = ION_COLORS[0];
            cx.element_mut(ion).color_cycle = 0;
            cx.element_mut(ion).current.location = cx.element(e).current.location;
            cx.element_mut(ion).current.location.x = (i32::from(cx.element(ion).current.location.x)
                + i32::from(cosine(i32::from(angle), 2 << ONE_SHIFT)))
                as COORD;
            cx.element_mut(ion).current.location.y = (i32::from(cx.element(ion).current.location.y)
                + i32::from(sine(i32::from(angle), 2 << ONE_SHIFT)))
                as COORD;
            cx.element_mut(ion).death = Some(ElementHook::Native(ion_preprocess));
            cx.set_parent(ion, Some(ss));
            cx.copy_current_to_next(ion);
            cx.element_mut(ion).life -= 1;
            cx.element_mut(ion).flags |= PRE_PROCESS as u16;
        }
    }

    pub(crate) fn marine_preprocess(cx: &mut Cx<'_>, e: ElementId) {
        let ss = cx.owner(e);
        let ship = cx.ship_element(ss);
        if owner_departed(cx, ship) {
            cx.element_mut(e).life = 0;
            cx.element_mut(e).flags |= (DISAPPEARING | NONSOLID) as u16;
            cx.element_mut(e).turn = 1;
        }
        if cx.element(e).turn & 0xf != 0 {
            cx.element_mut(e).turn -= 1;
            return;
        }
        cx.element_mut(e).thrust &= !((SHIP_IN_GRAVITY_WELL >> 6) as BYTE);
        let mut pfacing: COUNT = 0;
        let mut target = None;
        let mut h = cx.elements().head();
        while let Some(entry) = h {
            let object = entry;
            let mut next = cx.elements().succ(object);
            if u32::from(cx.element(object).mass) > MAX_SHIP_MASS * 10 {
                let (dx, dy) = wrapped_deltas(cx, object, e);
                if i64::from(dx) * i64::from(dx) + i64::from(dy) * i64::from(dy)
                    <= i64::from((GRAVITY_THRESHOLD as i32) << ONE_SHIFT).pow(2)
                {
                    pfacing = angle_to_facing(i32::from(crate::math::arctan(dx, dy))) as COUNT;
                    let delta = normalize_facing(
                        i32::from(pfacing)
                            - angle_to_facing(i32::from(cx.element(e).velocity.travel_angle))
                            + angle_to_facing(OCTANT as i32),
                    ) as SIZE;
                    if i32::from(delta) <= angle_to_facing(QUADRANT as i32) {
                        target = Some(entry);
                        next = None;
                    }
                    cx.element_mut(e).thrust |= (SHIP_IN_GRAVITY_WELL >> 6) as BYTE;
                }
            } else if cx.element(object).flags & PLAYER_SHIP as u16 != 0
                && cx.element(object).hits != 0
                && !cx.object_cloaked(object)
            {
                if cx.element(object).player != cx.element(e).player {
                    if cx.element(e).flags & IGNORE_SIMILAR as u16 != 0 {
                        target = Some(entry);
                    }
                } else if target.is_none() {
                    target = Some(entry);
                }
            }
            h = next;
        }
        let mut facing = COUNT::from(cx.element(e).turn >> 4);
        let mut delta: SIZE;
        if target.is_none() {
            delta = -1;
        } else {
            let object = target.unwrap_or_else(|| panic!("marine target"));
            let (mut dx, mut dy) = wrapped_deltas(cx, object, e);
            if u32::from(cx.element(object).mass) > MAX_SHIP_MASS * 10 {
                delta = normalize_facing(
                    i32::from(pfacing) - i32::from(facing) + angle_to_facing(OCTANT as i32),
                ) as SIZE;
                if i32::from(delta) > angle_to_facing(QUADRANT as i32) {
                    delta = 0;
                } else if i32::from(delta) == angle_to_facing(OCTANT as i32) {
                    facing = (i32::from(facing) + (((cx.random() as SIZE & 1) as i32) << 1) - 1)
                        as COUNT;
                } else if i32::from(delta) < angle_to_facing(OCTANT as i32) {
                    facing = facing.wrapping_add(1);
                } else {
                    facing = facing.wrapping_sub(1);
                }
            } else {
                if cx.element(object).player == cx.element(e).player
                    && cx.element(e).flags & IGNORE_SIMILAR as u16 != 0
                {
                    cx.set_frame(e, true, cx.absolute_frame(cx.ship_frame(ss, 2), 21));
                    cx.element_mut(e).flags &= !(IGNORE_SIMILAR as u16);
                    cx.element_mut(e).flags |= CHANGING as u16;
                }
                let mut frames = (crate::math::square_root(
                    (i64::from(dx) * i64::from(dx) + i64::from(dy) * i64::from(dy)) as DWORD,
                ) >> 6) as COUNT;
                if frames == 0 {
                    frames = 1;
                }
                let mut velocity = cx.element(object).velocity;
                let (vx, vy) = velocity.get_next_components(frames);
                dx = (i32::from(cx.element(object).current.location.x) + i32::from(vx as SIZE)
                    - i32::from(cx.element(e).current.location.x)) as SIZE;
                dy = (i32::from(cx.element(object).current.location.y) + i32::from(vy as SIZE)
                    - i32::from(cx.element(e).current.location.y)) as SIZE;
                delta = normalize_facing(
                    angle_to_facing(i32::from(crate::math::arctan(dx, dy))) - i32::from(facing),
                ) as SIZE;
                if delta > 0 {
                    if i32::from(delta) == angle_to_facing(HALF_CIRCLE as i32) {
                        facing = (i32::from(facing) + (((cx.random() as BYTE & 1) as i32) << 1) - 1)
                            as COUNT;
                    } else if i32::from(delta) < angle_to_facing(HALF_CIRCLE as i32) {
                        facing = facing.wrapping_add(1);
                    } else {
                        facing = facing.wrapping_sub(1);
                    }
                }
            }
        }
        cx.element_mut(e).turn = (normalize_facing(i32::from(facing)) << 4) as BYTE;
        if delta == 0
            || (cx.element(e).thrust & (SHIP_BEYOND_MAX_SPEED >> 6) as BYTE != 0
                && cx.element(e).thrust & (SHIP_IN_GRAVITY_WELL >> 6) as BYTE == 0)
        {
            let old_facing = cx.ship_state(ss).facing;
            let old_status = cx.ship_state(ss).status;
            let old_increment = cx.race_stats(ss).characteristics.thrust_increment;
            let old_thrust = cx.race_stats(ss).characteristics.max_thrust;
            cx.ship_state_mut(ss).facing = facing;
            cx.ship_state_mut(ss).status = u16::from(cx.element(e).thrust) << 6;
            cx.race_stats_mut(ss).characteristics.thrust_increment = MARINE_THRUST_INCREMENT;
            cx.race_stats_mut(ss).characteristics.max_thrust = MARINE_MAX_THRUST;
            let thrust_status = cx.apply_inertial_thrust(e);
            cx.race_stats_mut(ss).characteristics.max_thrust = old_thrust;
            cx.race_stats_mut(ss).characteristics.thrust_increment = old_increment;
            cx.ship_state_mut(ss).status = old_status;
            cx.ship_state_mut(ss).facing = old_facing;
            if cx.element(e).thrust & (SHIP_IN_GRAVITY_WELL >> 6) as BYTE != 0
                || delta != 0
                || thrust_status & (SHIP_AT_MAX_SPEED | SHIP_BEYOND_MAX_SPEED) as u16 == 0
            {
                spawn_marine_ion_trail(cx, e, ss, facing);
            }
            cx.element_mut(e).thrust = (thrust_status >> 6) as BYTE;
        }
    }

    pub fn marine_collision(
        cx: &mut Cx<'_>,
        a: ElementId,
        _pa: &mut POINT,
        b: ElementId,
        _pb: &mut POINT,
    ) {
        if cx.element(a).life == 0
            || cx.element(a).flags & (NONSOLID | COLLISION) as u16 != 0
            || cx.element(b).flags & FINITE_LIFE as u16 != 0
        {
            return;
        }
        if cx.element(a).player != cx.element(b).player {
            cx.element_mut(a).turn = 5 | (cx.element(a).turn & 0xf0);
            cx.element_mut(a).thrust &=
                !(((SHIP_AT_MAX_SPEED | SHIP_BEYOND_MAX_SPEED) >> 6) as BYTE);
            cx.element_mut(a).flags |= COLLISION as u16;
        }
        if u32::from(cx.element(b).mass) > MAX_SHIP_MASS * 10 {
            cx.element_mut(a).flags |= (NONSOLID | FINITE_LIFE) as u16;
            cx.element_mut(a).hits = 0;
            cx.element_mut(a).life = 0;
        } else if cx.element(b).flags & PLAYER_SHIP as u16 != 0
            && (cx.element(b).flags & FINITE_LIFE as u16 != 0
                || u32::from(cx.element(b).life) == NORMAL_LIFE)
        {
            cx.element_mut(b).flags &= !(COLLISION as u16);
            if cx.element(a).flags & COLLISION as u16 == 0 {
                cx.delta_crew(b, 1);
                cx.element_mut(a).flags |= (DISAPPEARING | NONSOLID | FINITE_LIFE) as u16;
                cx.element_mut(a).hits = 0;
                cx.element_mut(a).life = 0;
            } else if cx.element(a).flags & IGNORE_SIMILAR as u16 != 0 && cx.element(b).hits != 0 {
                let ss = cx.owner(a);
                if !cx.delta_crew(b, -1) {
                    cx.element_mut(b).life = 0;
                } else {
                    cx.element_mut(a).turn = count_marines(cx, ss, true);
                    cx.element_mut(a).thrust = MARINE_WAIT;
                    cx.set_frame(
                        a,
                        true,
                        cx.absolute_frame(
                            cx.element_frame_set_first(a, true),
                            22 + u16::from(cx.element(a).turn),
                        ),
                    );
                    cx.element_mut(a).flags |= NONSOLID as u16;
                    cx.element_mut(a).flags &= !(CREW_OBJECT as u16);
                    cx.primitive_style_mut(a).kind = gfx_object_NUM_PRIMS as GRAPHICS_PRIM;
                    cx.element_mut(a).pre = Some(ElementHook::Native(intruder_preprocess));
                    silhouette(cx, b, a, cx.element(a).next.image.frame, 0);
                }
                cx.process_ship_sound(ss, 2, b);
            }
            cx.element_mut(a).flags &= !(COLLISION as u16);
        }
    }

    fn turret_muzzle_flash(
        cx: &mut Cx<'_>,
        e: ElementId,
        turret: ElementId,
        ss: ShipId,
        facing: COUNT,
    ) {
        let effect = cx
            .elements()
            .tail()
            .unwrap_or_else(|| panic!("Orz queue tail"));
        if effect != e
            && cx.element(effect).player == cx.element(e).player
            && cx.element(effect).flags & APPEARING as u16 != 0
            && cx.primitive_style(effect).kind == gfx_object_STAMP_PRIM as GRAPHICS_PRIM
        {
            let effect_h = cx.alloc_element();
            if let Some(effect_h) = effect_h {
                cx.element_mut(turret).current.location.x =
                    (i32::from(cx.element(turret).current.location.x)
                        - i32::from(cosine(i32::from(facing), 2 << ONE_SHIFT)))
                        as COORD;
                cx.element_mut(turret).current.location.y =
                    (i32::from(cx.element(turret).current.location.y)
                        - i32::from(sine(i32::from(facing), 2 << ONE_SHIFT)))
                        as COORD;
                let effect = effect_h;
                cx.element_mut(effect).player = cx.element(e).player;
                cx.element_mut(effect).flags =
                    (FINITE_LIFE | NONSOLID | IGNORE_SIMILAR | APPEARING) as u16;
                cx.element_mut(effect).life = 4;
                cx.element_mut(effect).current.location.x =
                    (i32::from(cx.element(turret).current.location.x)
                        + i32::from(cosine(i32::from(facing), TURRET_OFFSET << ONE_SHIFT)))
                        as COORD;
                cx.element_mut(effect).current.location.y =
                    (i32::from(cx.element(turret).current.location.y)
                        + i32::from(sine(i32::from(facing), TURRET_OFFSET << ONE_SHIFT)))
                        as COORD;
                cx.set_frames(effect, false, cx.ship_frame_set(ss, 2));
                cx.set_frame(
                    effect,
                    false,
                    cx.absolute_frame(
                        cx.ship_frame(ss, 2),
                        angle_to_facing(FULL_CIRCLE as i32) as COUNT,
                    ),
                );
                cx.element_mut(effect).pre = Some(ElementHook::Native(animation_preprocess));
                cx.set_parent(effect, Some(ss));
                cx.primitive_style_mut(effect).kind = gfx_object_STAMP_PRIM as GRAPHICS_PRIM;
                cx.put(effect_h);
            }
        }
    }

    pub(crate) fn turret_postprocess(cx: &mut Cx<'_>, e: ElementId) {
        if cx.element(e).life != 0 {
            return;
        }
        cx.primitive_style_mut(e).kind = gfx_object_NUM_PRIMS as GRAPHICS_PRIM;
        let ss = cx.owner(e);
        if cx.optional_ship_element(ss).is_none() {
            return;
        }
        let ship = cx.ship_element(ss);
        let h = cx.alloc_element();
        // C leaves facing uninitialized on allocation failure. Define that
        // path as zero while retaining TrackShip's call order and RNG effects.
        let mut facing: COUNT = 0;
        if let Some(h) = h {
            let turret = h;
            cx.element_mut(turret).player = cx.element(e).player;
            cx.element_mut(turret).flags =
                (FINITE_LIFE | NONSOLID | IGNORE_SIMILAR | CHANGING | PRE_PROCESS | POST_PROCESS)
                    as u16;
            cx.element_mut(turret).life = 1;
            cx.copy_image_to_current(turret, cx.element(e).current.image);
            cx.element_mut(turret).current.location = cx.element(ship).next.location;
            cx.element_mut(turret).turn = cx.element(e).turn;
            cx.element_mut(turret).thrust = cx.element(e).thrust;
            if cx.element(turret).turn != 0 {
                cx.element_mut(turret).turn -= 1;
            } else if cx.ship_state(ss).status & SPECIAL as u16 != 0
                && cx.ship_state(ss).status & (LEFT | RIGHT) as u16 != 0
            {
                if cx.ship_state(ss).status & RIGHT as u16 != 0 {
                    cx.element_mut(turret).thrust = cx.element(turret).thrust.wrapping_add(1);
                } else {
                    cx.element_mut(turret).thrust = cx.element(turret).thrust.wrapping_sub(1);
                }
                cx.element_mut(turret).turn = TURRET_WAIT;
            }
            facing = normalize_facing(
                i32::from(cx.ship_state(ss).facing) + i32::from(cx.element(turret).thrust),
            );
            cx.race_stats_mut(ss).ship_info.ship_flags &=
                !((FIRES_FORE | FIRES_RIGHT | FIRES_AFT | FIRES_LEFT) as u16);
            cx.race_stats_mut(ss).ship_info.ship_flags |= (FIRES_FORE
                << (i32::from(normalize_facing(
                    i32::from(facing) + angle_to_facing(OCTANT as i32),
                )) / angle_to_facing(QUADRANT as i32)))
                as u16;
            cx.set_frame(
                turret,
                false,
                cx.absolute_frame(cx.element(turret).current.image.frame.view(), facing),
            );
            facing = facing_to_angle(i32::from(facing)) as COUNT;
            if cx.ship_state(ss).status & WEAPON as u16 != 0 {
                turret_muzzle_flash(cx, e, turret, ss, facing);
            }
            cx.copy_current_to_next(turret);
            cx.primitive_style_mut(turret).kind = cx.primitive_style(ship).kind;
            cx.primitive_style_mut(turret).color = cx.primitive_style(ship).color;
            cx.element_mut(turret).post = cx.element(e).post;
            cx.set_parent(turret, Some(ss));
            cx.insert_before(h, cx.elements().succ(e));
        }
        if cx.ship_state(ss).special_counter == 0
            && cx.ship_state(ss).status & SPECIAL as u16 != 0
            && cx.ship_state(ss).status & WEAPON as u16 != 0
            && cx.element(ship).hits > 1
            && count_marines(cx, ss, false) < MAX_MARINES
            && cx.track_ship(ship, &mut facing) >= 0
        {
            let h = cx.alloc_element();
            if let Some(h) = h {
                let marine = h;
                cx.element_mut(marine).player = cx.element(e).player;
                cx.element_mut(marine).flags = (IGNORE_SIMILAR | APPEARING | CREW_OBJECT) as u16;
                cx.element_mut(marine).life = NORMAL_LIFE as COUNT;
                cx.element_mut(marine).hits = 3;
                cx.element_mut(marine).mass = 1;
                facing = facing_to_angle(i32::from(cx.ship_state(ss).facing)) as COUNT;
                cx.element_mut(marine).current.location.x =
                    (i32::from(cx.element(ship).current.location.x)
                        - i32::from(cosine(i32::from(facing), TURRET_OFFSET << ONE_SHIFT)))
                        as COORD;
                cx.element_mut(marine).current.location.y =
                    (i32::from(cx.element(ship).current.location.y)
                        - i32::from(sine(i32::from(facing), TURRET_OFFSET << ONE_SHIFT)))
                        as COORD;
                cx.set_frames(marine, false, cx.ship_frame_set(ss, 2));
                cx.set_frame(marine, false, cx.absolute_frame(cx.ship_frame(ss, 2), 20));
                cx.element_mut(marine).turn = (normalize_facing(angle_to_facing(
                    i32::from(facing) + HALF_CIRCLE as i32,
                )) << 4) as BYTE;
                cx.element_mut(marine).pre = Some(ElementHook::Native(marine_preprocess));
                cx.element_mut(marine).collide = Some(CollisionHook::Native(marine_collision));
                cx.set_parent(marine, Some(ss));
                cx.primitive_style_mut(marine).kind = gfx_object_STAMP_PRIM as GRAPHICS_PRIM;
                cx.put(h);
                cx.delta_crew(ship, -1);
                cx.process_ship_sound(ss, 1, marine);
                cx.ship_state_mut(ss).special_counter =
                    cx.race_stats(ss).characteristics.special_wait;
            }
        }
    }

    pub(crate) fn orz_preprocess(cx: &mut Cx<'_>, ss: ShipId) {
        let e = cx.ship_element(ss);
        if cx.element(e).flags & APPEARING as u16 == 0 {
            if (cx.ship_state(ss).status | cx.ship_state(ss).old_status) & SPECIAL as u16 != 0
                && cx.ship_state(ss).status & (LEFT | RIGHT) as u16 != 0
                && cx.element(e).turn == 0
            {
                cx.element_mut(e).turn += 1;
            }
            if cx.ship_state(ss).status & SPECIAL as u16 != 0
                && cx.ship_state(ss).status & WEAPON as u16 != 0
                && cx.ship_state(ss).weapon_counter == 0
            {
                cx.ship_state_mut(ss).weapon_counter += 1;
            }
        } else {
            let h = cx.alloc_element();
            if let Some(h) = h {
                let turret = h;
                cx.element_mut(turret).player = cx.element(e).player;
                cx.element_mut(turret).flags = (FINITE_LIFE | NONSOLID | IGNORE_SIMILAR) as u16;
                cx.element_mut(turret).life = 1;
                cx.set_frames(turret, false, cx.ship_frame_set(ss, 2));
                cx.set_frame(
                    turret,
                    false,
                    cx.absolute_frame(cx.ship_frame(ss, 2), cx.ship_state(ss).facing),
                );
                cx.element_mut(turret).post = Some(ElementHook::Native(turret_postprocess));
                cx.set_parent(turret, Some(ss));
                cx.insert_before(h, cx.elements().succ(e));
            }
        }
    }
}
