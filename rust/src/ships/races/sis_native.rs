//! Live flagship callbacks translated from ships/sis_ship/sis_ship.c.
#[cfg(not(test))]
pub(crate) mod engine {
    use crate::engine::battle::race::{FlagshipWeapon, SisState};
    use crate::engine::battle::views::{Laser, Missile};
    use crate::engine::battle::{
        Concerns, Cx, ElementId, Race, RaceDesc, RaceState, RaceTemplate, ShipId, StoryKey,
        WeaponOrigin, WeaponSpawn, World,
    };

    use crate::ships::native::{abi::*, angle_to_facing, normalize_facing};

    fn template() -> RaceTemplate {
        let mut d = RaceTemplate::default();
        d.info.ship_cost = 16;
        d.info.crew_level = MAX_CREW_SIZE as COUNT;
        d.info.max_crew = MAX_CREW_SIZE as COUNT;
        d.info.energy_level = MAX_ENERGY_SIZE as u8;
        d.info.max_energy = MAX_ENERGY_SIZE as u8;
        d.resources.icons = Some(c"ship.flagship.icons");
        d.characteristics.max_thrust = 10;
        d.characteristics.thrust_increment = 4;
        d.characteristics.energy_regeneration = 1;
        d.characteristics.weapon_energy_cost = 1;
        d.characteristics.special_energy_cost = 0;
        d.characteristics.energy_wait = 10;
        d.characteristics.turn_wait = 17;
        d.characteristics.thrust_wait = 6;
        d.characteristics.weapon_wait = 6;
        d.characteristics.special_wait = 9;
        d.characteristics.ship_mass = MAX_SHIP_MASS as u8;
        d.resources.ship = [
            Some(c"ship.flagship.graphics.flagship.large"),
            Some(c"ship.flagship.graphics.flagship.medium"),
            Some(c"ship.flagship.graphics.flagship.small"),
        ];
        d.resources.weapon = [
            Some(c"ship.flagship.graphics.blaster.large"),
            Some(c"ship.flagship.graphics.blaster.medium"),
            Some(c"ship.flagship.graphics.blaster.small"),
        ];
        d.resources.captain = Some(c"ship.flagship.graphics.captain");
        d.resources.victory = Some(c"ship.flagship.ditty");
        d.resources.sounds = Some(c"ship.flagship.sounds");
        d.intelligence.weapon_range = (24 << ONE_SHIFT) * 12;
        d
    }
    fn universe_position(position: (i32, i32), dimensions: (i32, i32)) -> (i16, i16) {
        let (pixel_width, pixel_height) = dimensions;
        // units.h uses the original, resolution-dependent hyperspace factors.
        let log_units_x = (((pixel_width - 64) << (ONE_SHIFT + 3)) >> 4) * 195;
        let log_units_y = ((pixel_height << (ONE_SHIFT + 3)) >> 4) * 25;
        let ux = (position.0.wrapping_mul(6250).wrapping_add(log_units_x / 2) / log_units_x) as i16;
        let uy = (9999 - position.1.wrapping_mul(625).wrapping_add(log_units_y / 2) / log_units_y)
            as i16;
        (ux, uy)
    }
    pub(crate) fn sis_hyper_preprocess(cx: &mut Cx<'_>, ss: ShipId) {
        let config = cx.flagship_config();
        let (pixel_width, pixel_height) = cx.screen_dimensions();
        let e = cx.ship_element(ss);
        if cx.element(e).flags & APPEARING as u16 != 0 {
            cx.element_mut(e).velocity = cx.flagship_velocity();
        }
        cx.ship_state_mut(ss).weapon_counter = cx.ship_state(ss).weapon_counter.wrapping_add(1);
        let (mut udx, mut udy): (SIZE, SIZE) = (0, 0);
        let (mut dx, mut dy): (SIZE, SIZE) = (0, 0);
        let mut accelerate = 0;
        let mut leave = cx.autopilot().x == -1
            || cx.autopilot().y == -1
            || cx.ship_state(ss).status & (LEFT | RIGHT | THRUST) as u16 != 0;
        if !leave {
            let (ux, uy) = universe_position(config.position, (pixel_width, pixel_height));
            udx = cx.autopilot().x.wrapping_sub(ux);
            udy = cx.autopilot().y.wrapping_sub(uy).wrapping_neg();
            dx = udx.wrapping_abs();
            dy = udy.wrapping_abs();
            leave = dx <= 1 && dy <= 1;
            if !leave {
                let mut facing =
                    normalize_facing(angle_to_facing(crate::math::arctan(udx, udy) as i32));
                if cx.ship_state(ss).status & SHIP_AT_MAX_SPEED as u16 != 0 {
                    if normalize_facing(
                        i32::from(cx.ship_state(ss).facing) + angle_to_facing(QUADRANT as i32)
                            - i32::from(facing),
                    ) > angle_to_facing(HALF_CIRCLE as i32) as u16
                    {
                        leave = true;
                    }
                    facing = cx.ship_state(ss).facing;
                } else if facing != cx.ship_state(ss).facing && cx.element(e).turn == 0 {
                    if normalize_facing(i32::from(cx.ship_state(ss).facing) - i32::from(facing))
                        >= angle_to_facing(HALF_CIRCLE as i32) as u16
                    {
                        facing = normalize_facing(i32::from(facing) - 1);
                        cx.ship_state_mut(ss).status |= RIGHT as u16;
                    } else if cx.ship_state(ss).facing != facing {
                        facing = normalize_facing(i32::from(facing) + 1);
                        cx.ship_state_mut(ss).status |= LEFT as u16;
                    }
                    if facing == cx.ship_state(ss).facing {
                        cx.element_mut(e).velocity =
                            crate::engine::battle::record::VELOCITY_DESC::default();
                    }
                }
                if !leave {
                    let (x, y) = cx.element(e).velocity.get_current_components();
                    dx = x as SIZE;
                    dy = y as SIZE;
                    if (config.fuel != 0 || cx.story_state(StoryKey::ArilouSpaceSide) > 1)
                        && facing == cx.ship_state(ss).facing
                    {
                        cx.ship_state_mut(ss).status |= SHIP_AT_MAX_SPEED as u16;
                        accelerate = 1;
                    } else {
                        accelerate = -1;
                        udx = dx.wrapping_shl(4);
                        udy = dy.wrapping_shl(4);
                    }
                }
            }
        }
        if leave {
            cx.set_autopilot(POINT { x: -1, y: -1 });
            if cx.ship_state(ss).status & THRUST as u16 == 0
                || (config.fuel == 0 && cx.story_state(StoryKey::ArilouSpaceSide) <= 1)
            {
                accelerate = -1;
                let (x, y) = cx.element(e).velocity.get_current_components();
                dx = x as SIZE;
                dy = y as SIZE;
                udx = dx.wrapping_shl(4);
                udy = dy.wrapping_shl(4);
                cx.ship_state_mut(ss).status &= !(THRUST as u16);
            }
        }
        if cx.element(e).thrust == 0 && accelerate != 0 {
            let c = cx.race_stats(ss).characteristics;
            let increment = (i32::from(c.thrust_increment) << VELOCITY_SHIFT) as i16;
            let dist = square_root(
                (i64::from(udx) * i64::from(udx) + i64::from(udy) * i64::from(udy)) as u32,
            )
            .max(1);
            let mut speed =
                square_root((i64::from(dx) * i64::from(dx) + i64::from(dy) * i64::from(dy)) as u32)
                    as i16;
            if accelerate < 0 {
                dy = ((i32::from(speed) / i32::from(increment) - 1) * i32::from(increment)) as i16;
                if i32::from(dy) < i32::from(speed) - i32::from(increment) {
                    dy = speed.wrapping_sub(increment);
                }
                speed = dy.max(0);
                cx.ship_state_mut(ss).status &= !(SHIP_AT_MAX_SPEED as u16);
            } else {
                let max_velocity = (i32::from(c.max_thrust) << VELOCITY_SHIFT) as i16;
                dy = ((i32::from(speed) / i32::from(increment) + 1) * i32::from(increment)) as i16;
                if i32::from(dy) < i32::from(speed) + i32::from(increment) {
                    dy = speed.wrapping_add(increment);
                }
                speed = dy;
                if speed > max_velocity {
                    speed = max_velocity;
                    cx.ship_state_mut(ss).status |= SHIP_AT_MAX_SPEED as u16;
                }
            }
            dx = (i64::from(udx) * i64::from(speed) / i64::from(dist)) as i16;
            dy = (i64::from(udy) * i64::from(speed) / i64::from(dist)) as i16;
            cx.element_mut(e)
                .velocity
                .set_components(i32::from(dx), i32::from(dy));
            cx.element_mut(e).thrust = c.thrust_wait;
        }
    }

    pub(crate) fn sis_hyper_postprocess(cx: &mut Cx<'_>, ss: ShipId) {
        let e = cx.ship_element(ss);
        cx.set_flagship_velocity(cx.element(e).velocity);
        if (cx.ship_state(ss).status & WEAPON as u16 != 0 || cx.hyperspace_cancel_pressed())
            && cx.ship_state(ss).special_counter == 0
        {
            cx.hyperspace_menu();
            cx.ship_state_mut(ss).status &= !(SHIP_AT_MAX_SPEED as u16);
            cx.ship_state_mut(ss).special_counter = 10;
        }
    }

    pub(crate) fn spawn_point_defense(cx: &mut Cx<'_>, e: ElementId) {
        let ss = cx.owner(e);
        if cx.element(e).flags & PLAYER_SHIP as u16 != 0 {
            if let Some(d) = cx.alloc_element() {
                cx.element_mut(d).player = cx.element(e).player;
                cx.element_mut(d).flags = (APPEARING | NONSOLID | FINITE_LIFE) as u16;
                cx.element_mut(d).death = Some(ElementHook::Native(spawn_point_defense));
                cx.set_parent(d, Some(ss));
                cx.put(d);
            }
        } else {
            let colors = [3, 7, 10, 14, 17, 21, 24, 28];
            let cost = cx.race_stats(ss).characteristics.special_energy_cost;
            let g = colors[cost as usize];
            let color = Color {
                r: 255,
                g: (g << 3) | (g >> 2),
                b: 0,
                a: 255,
            };
            let ship = cx.ship_element(ss);
            let mut paid = false;
            let mut h = cx.elements().tail();
            while let Some(o) = h {
                h = cx.elements().pred(o);
                if o != ship
                    && cx.element(o).flags & (NONSOLID | DISAPPEARING) as u16 == 0
                    && !object_cloaked(cx, o)
                {
                    let location = cx.element(o).next.location;
                    let origin = cx.element(ship).next.location;
                    let dx = location.x.wrapping_sub(origin.x).wrapping_abs() >> ONE_SHIFT;
                    let dy = location.y.wrapping_sub(origin.y).wrapping_abs() >> ONE_SHIFT;
                    if dx as u16 <= 100
                        && dy as u16 <= 100
                        && u32::from(dx as u16) * u32::from(dx as u16)
                            + u32::from(dy as u16) * u32::from(dy as u16)
                            <= 10000
                    {
                        if !paid {
                            if !cx.delta_energy(ship, -(i16::from(cost) << 2)) {
                                break;
                            }
                            cx.process_ship_sound(ss, 1, e);
                            cx.ship_state_mut(ss).special_counter =
                                cx.race_stats(ss).characteristics.special_wait;
                            paid = true;
                        }
                        let b = Laser {
                            origin: cx.element(ship).next.location,
                            face: 0,
                            end: POINT {
                                x: cx
                                    .element(o)
                                    .next
                                    .location
                                    .x
                                    .wrapping_sub(cx.element(ship).next.location.x),
                                y: cx
                                    .element(o)
                                    .next
                                    .location
                                    .y
                                    .wrapping_sub(cx.element(ship).next.location.y),
                            },
                            sender: cx.element(ship).player,
                            flags: IGNORE_SIMILAR as u16,
                            pixoffs: 0,
                            color,
                        };
                        if let Some(p) = cx.spawn_laser(b) {
                            cx.element_mut(p).mass = cost;
                            cx.set_parent(p, Some(ss));
                            cx.set_target(p, None);
                            cx.put(p);
                        }
                    }
                }
            }
        }
    }

    pub(crate) fn sis_battle_preprocess(cx: &mut Cx<'_>, ss: ShipId) {
        if cx.race_stats(ss).characteristics.special_energy_cost == 0 {
            cx.ship_state_mut(ss).status &= !(SPECIAL as u16);
            cx.ship_state_mut(ss).special_counter = 2;
        }
        if cx.race_stats(ss).ship_info.ship_flags
            & (FIRES_FORE | FIRES_RIGHT | FIRES_AFT | FIRES_LEFT) as u16
            == 0
        {
            cx.ship_state_mut(ss).status &= !(WEAPON as u16);
            cx.ship_state_mut(ss).weapon_counter = 2;
        }
    }
    pub(crate) fn sis_battle_postprocess(cx: &mut Cx<'_>, ss: ShipId) {
        let e = cx.ship_element(ss);
        if cx.ship_state(ss).status & SPECIAL as u16 != 0
            && cx.ship_state(ss).special_counter == 0
            && cx.race_stats(ss).characteristics.special_energy_cost != 0
        {
            spawn_point_defense(cx, e);
        }
    }
    pub(crate) fn blaster_collision(
        cx: &mut Cx<'_>,
        e: ElementId,
        p: &mut POINT,
        o: ElementId,
        q: &mut POINT,
    ) {
        let h = cx.weapon_collision(e, p, o, q);
        if let Some(b) = h {
            match cx.element(e).mass {
                2 => {
                    cx.element_mut(b).life = 2;
                    cx.set_frame(
                        b,
                        false,
                        cx.absolute_frame(cx.element(e).current.image.frame, 0),
                    );
                    cx.element_mut(b).pre = None;
                }
                4 => {
                    cx.element_mut(b).life = 6;
                    cx.set_frame(
                        b,
                        false,
                        cx.advance_frame(cx.element(e).current.image.frame),
                    );
                }
                6 => {
                    cx.element_mut(b).life = 7;
                    cx.set_frame(
                        b,
                        false,
                        cx.absolute_frame(cx.element(e).current.image.frame, 20),
                    );
                }
                _ => {}
            }
        }
    }
    pub(crate) fn blaster_preprocess(cx: &mut Cx<'_>, e: ElementId) {
        let frame = cx.frame_number(cx.element(e).current.image.frame);
        match cx.element(e).mass {
            2 if frame < 8 => {
                cx.set_frame(e, true, cx.advance_frame(cx.element(e).current.image.frame));
                cx.element_mut(e).flags |= CHANGING as u16;
            }
            6 => {
                cx.set_frame(
                    e,
                    true,
                    if frame < 19 {
                        cx.advance_frame(cx.element(e).current.image.frame)
                    } else {
                        cx.absolute_frame(cx.element(e).current.image.frame, 16)
                    },
                );
                cx.element_mut(e).flags |= CHANGING as u16;
            }
            _ => {}
        }
        if cx.element(e).turn & 15 != 0 {
            cx.element_mut(e).turn -= 1;
        } else {
            let wait = cx.element(e).turn >> 4;
            if wait != 0 {
                let mut face =
                    normalize_facing(angle_to_facing(cx.element(e).velocity.travel_angle as i32));
                if cx.track_ship(e, &mut face) > 0 {
                    cx.element_mut(e).velocity.set_vector(24 << ONE_SHIFT, face);
                }
                cx.element_mut(e).turn = (wait << 4) | wait;
            }
        }
    }
    pub(crate) fn initialize_blasters(
        cx: &mut Cx<'_>,
        ss: ShipId,
        origin: WeaponOrigin,
    ) -> WeaponSpawn {
        let RaceState::Sis(data) = cx.race_stats(ss).state else {
            panic!("SIS weapon state");
        };
        let nt = (4 - data.trackers) as u8 & 3;
        let mut out = Vec::with_capacity(usize::from(data.blasters));
        for weapon in &data.missiles[..usize::from(data.blasters)] {
            let weapon = weapon.unwrap_or_else(|| panic!("configured SIS blaster"));
            let mut b = Missile {
                cx: origin.next.location.x,
                cy: origin.next.location.y,
                farray: cx.ship_frame_set(ss, 1),
                sender: origin.player,
                face: normalize_facing(
                    i32::from(cx.ship_state(ss).facing) + i32::from(weapon.facing),
                ),
                flags: weapon.flags,
                pixoffs: weapon.pixel_offset,
                speed: weapon.speed,
                hit_points: weapon.hit_points,
                damage: weapon.damage,
                life: weapon.life,
                index: weapon.index,
                preprocess: weapon.preprocess,
                blast_offs: weapon.blast_offset,
            };
            let missile = cx.missile(&mut b);
            out.push(missile);
            if let Some(m) = missile {
                cx.element_mut(m).collide = Some(CollisionHook::Native(blaster_collision));
                cx.element_mut(m).turn = (nt << 4) | nt;
            }
        }
        out
    }
    pub(crate) fn sis_intelligence(cx: &mut Cx<'_>, ss: ShipId, concerns: &mut Concerns) {
        let RaceState::Sis(data) = cx.race_stats(ss).state else {
            panic!("SIS AI state");
        };
        let w = ENEMY_WEAPON_INDEX as usize;
        let enemy = ENEMY_SHIP_INDEX as usize;
        if let Some(threat) = concerns.objects[w].object {
            if cx.race_stats(ss).characteristics.special_energy_cost != 0 {
                if cx.ship_state(ss).special_counter == 0
                    && (concerns.objects[w].which_turn <= 2
                        || (concerns.objects[enemy].object.is_some()
                            && concerns.objects[enemy].which_turn <= 4))
                {
                    cx.ship_state_mut(ss).input |= SPECIAL as u8;
                } else {
                    cx.ship_state_mut(ss).input &= !(SPECIAL as u8);
                }
                concerns.objects[w].object = None;
            } else if cx.race_stats(ss).intelligence.maneuverability < MEDIUM_SHIP as u16
                && concerns.objects[w].movement == MOVEMENT_STATE_ENTICE
                && (cx.element(threat).flags & CREW_OBJECT as u16 == 0
                    || concerns.objects[w].which_turn <= 8)
                && (cx.element(threat).flags & FINITE_LIFE as u16 == 0
                    || (cx.element(threat).mass >= 4
                        && concerns.objects[w].which_turn == 2
                        && concerns.objects[enemy].which_turn > 16))
            {
                concerns.objects[w].movement = MOVEMENT_STATE_PURSUE;
            }
        }
        cx.ship_intelligence(ss, concerns);
        if data.trackers != 0
            && cx.ship_state(ss).weapon_counter == 0
            && cx.ship_state(ss).input & WEAPON as u8 == 0
            && concerns.objects[enemy].which_turn <= 16
        {
            let Some(target) = concerns.objects[enemy].object else {
                return;
            };
            let dx = cx
                .element(target)
                .current
                .location
                .x
                .wrapping_sub(concerns.subject.current.location.x);
            let dy = cx
                .element(target)
                .current
                .location
                .y
                .wrapping_sub(concerns.subject.current.location.y);
            let direction = normalize_facing(angle_to_facing(crate::math::arctan(dx, dy) as i32));
            let mut flags = FIRES_FORE as u16;
            let mut facing = cx.ship_state(ss).facing;
            while flags <= FIRES_LEFT as u16 {
                if cx.race_stats(ss).ship_info.ship_flags & flags != 0
                    && normalize_facing(
                        i32::from(direction) - i32::from(facing) + angle_to_facing(OCTANT as i32),
                    ) <= angle_to_facing(QUADRANT as i32) as u16
                {
                    cx.ship_state_mut(ss).input |= WEAPON as u8;
                    break;
                }
                flags <<= 1;
                facing = facing.wrapping_add(QUADRANT as u16);
            }
        }
    }

    fn init_weapon_slots(d: &mut RaceTemplate, data: &mut SisState, slots: &[u8; 16]) {
        data.blasters = 0;
        for i in [0, 1, 2, 15] {
            let gun = slots[15 - i];
            if !(GUN_WEAPON as u8..=CANNON_WEAPON as u8).contains(&gun) {
                continue;
            }
            let gun = gun - (GUN_WEAPON as u8 - 1);
            d.characteristics.weapon_energy_cost =
                d.characteristics.weapon_energy_cost.wrapping_add(gun * 2);
            let mut b = FlagshipWeapon {
                flags: IGNORE_SIMILAR as u16,
                blast_offset: 8,
                speed: 24 << ONE_SHIFT,
                preprocess: Some(blaster_preprocess),
                hit_points: i16::from(2 * gun),
                damage: i16::from(2 * gun),
                life: 12 + 3 * (u16::from(gun) - 1),
                index: match gun {
                    1 => 0,
                    2 => 9,
                    _ => 16,
                },
                ..FlagshipWeapon::default()
            };
            match i {
                0 => {
                    d.info.ship_flags |= FIRES_FORE as u16;
                    b.pixel_offset = 28;
                    b.facing = 0;
                }
                1 => {
                    d.info.ship_flags |= FIRES_FORE as u16;
                    b.pixel_offset = 28;
                    b.facing = 1;
                }
                2 => {
                    d.info.ship_flags |= (FIRES_LEFT | FIRES_RIGHT) as u16;
                    b.pixel_offset = 20;
                    b.facing = angle_to_facing(QUADRANT as i32) as u16;
                }
                _ => {
                    d.info.ship_flags |= FIRES_AFT as u16;
                    b.pixel_offset = 28;
                    b.facing = angle_to_facing(HALF_CIRCLE as i32) as u16;
                }
            }
            data.missiles[usize::from(data.blasters)] = Some(b);
            data.blasters += 1;
            if i == 1 || i == 2 {
                b.facing = normalize_facing(-i32::from(b.facing));
                data.missiles[usize::from(data.blasters)] = Some(b);
                data.blasters += 1;
            }
        }
    }
    fn init_module_slots(d: &mut RaceTemplate, data: &mut SisState, slots: &[u8; 16]) {
        d.info.max_crew = 0;
        let mut trackers: u16 = 0;
        for &m in slots.iter().rev() {
            match u32::from(m) {
                CREW_POD => d.info.max_crew += CREW_POD_CAPACITY as COUNT,
                TRACKING_SYSTEM => trackers += 1,
                ANTIMISSILE_DEFENSE => d.characteristics.special_energy_cost += 1,
                SHIVA_FURNACE => d.characteristics.energy_regeneration += 1,
                DYNAMO_UNIT => {
                    d.characteristics.energy_wait = (d.characteristics.energy_wait - 2).max(4)
                }
                _ => {}
            }
        }
        trackers = trackers.min(3);
        d.characteristics.weapon_energy_cost += (trackers * 3) as u8;
        data.trackers = trackers;
        if d.characteristics.special_energy_cost != 0 {
            d.info.ship_flags |= POINT_DEFENSE as u16;
            d.characteristics.special_energy_cost = d.characteristics.special_energy_cost.min(8);
        }
    }

    pub static SIS: Race = Race {
        descriptor,
        preprocess: Some(sis_battle_preprocess),
        postprocess: Some(sis_battle_postprocess),
        init_weapon: Some(initialize_blasters),
        intelligence: Some(sis_intelligence),
        uninit: Some(uninit_sis),
    };
    static SIS_HYPER: Race = Race {
        descriptor,
        preprocess: Some(sis_hyper_preprocess),
        postprocess: Some(sis_hyper_postprocess),
        init_weapon: None,
        intelligence: None,
        uninit: Some(uninit_sis),
    };
    fn descriptor(world: &mut World) -> RaceDesc {
        let config = world.flagship_config();
        let mut d = template();
        let race = if config.hyperspace {
            d.resources = crate::engine::battle::RaceResources::default();
            d.resources.ship[0] = Some(c"ship.flagship.graphics.hyperspace");
            d.characteristics.max_thrust -= 4;
            &SIS_HYPER
        } else {
            if world.story_state(StoryKey::ChmmrBombState) == 3 {
                world.set_story_state(StoryKey::BombCarrier, 1);
            }
            &SIS
        };
        let mut data = SisState::default();
        init_module_slots(&mut d, &mut data, &config.modules);
        init_weapon_slots(&mut d, &mut data, &config.modules);
        d.state = RaceState::Sis(data);
        d.characteristics.thrust_wait = 0;
        for drive in config.drives {
            if u32::from(drive) == FUSION_THRUSTER {
                d.characteristics.max_thrust += 2;
                d.characteristics.thrust_wait += 1;
            }
        }
        d.characteristics.thrust_wait = 6 - (d.characteristics.thrust_wait >> 1);
        d.characteristics.max_thrust =
            (d.characteristics.max_thrust / d.characteristics.thrust_increment + 1)
                * d.characteristics.thrust_increment;
        for jet in config.jets {
            if u32::from(jet) == TURNING_JETS {
                d.characteristics.turn_wait -= 2;
            }
        }
        if world.activity() & 255 == SUPER_MELEE as u16 {
            d.info.crew_level = d.info.max_crew;
        } else {
            d.info.max_crew += 1;
            d.info.crew_level = config.crew + 1;
            d.info.ship_flags |= PLAYER_CAPTAIN as u16;
        }
        d.info.energy_level = d.info.max_energy;
        d.descriptor(race)
    }
    pub(crate) fn uninit_sis(world: &mut World, d: &mut crate::engine::battle::views::RaceStats) {
        if !world.flagship_config().hyperspace {
            let crew = if d.ship_info.ship_flags & PLAYER_CAPTAIN as u16 != 0 {
                d.ship_info.crew_level.wrapping_sub(1)
            } else {
                d.ship_info.crew_level
            };
            world.set_flagship_crew(crew);
        }
        d.state = RaceState::None;
    }
    fn object_cloaked(cx: &Cx<'_>, e: ElementId) -> bool {
        let p = cx.primitive_style(e);
        p.kind >= gfx_object_NUM_PRIMS as u8
            || (p.kind == gfx_object_STAMPFILL_PRIM as u8
                && p.color.r == 0
                && p.color.g == 0
                && p.color.b == 0
                && p.color.a == 255)
    }
}
