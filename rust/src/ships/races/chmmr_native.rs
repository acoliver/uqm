//! Chmmr callbacks for the live engine, ported from `ships/chmmr/chmmr.c`.

fn tractor_shadow_flags() -> u16 {
    use crate::ships::native::abi::{
        CHANGING, FINITE_LIFE, IGNORE_SIMILAR, NONSOLID, POST_PROCESS,
    };
    // POST_PROCESS shadows skip preprocessing. Their first projection must not
    // reuse the previous occupant's primitive frame, even at zero velocity.
    (FINITE_LIFE | NONSOLID | IGNORE_SIMILAR | POST_PROCESS | CHANGING) as u16
}

#[cfg(test)]
mod tests {
    #[test]
    fn stationary_new_tractor_shadow_projects_a_fresh_primitive() {
        use crate::ships::native::abi::{APPEARING, CHANGING, POST_PROCESS};
        let flags = super::tractor_shadow_flags();
        assert_ne!(
            flags & POST_PROCESS as u16,
            0,
            "the shadow already bypassed preprocessing"
        );
        assert_ne!(flags & (APPEARING | CHANGING) as u16, 0,
            "a stationary first-use shadow must project its own frame before drawing a recycled primitive");
    }
}

#[cfg(not(test))]
pub(crate) mod engine {
    use crate::ships::native::{
        abi::*, angle_to_facing, cosine, facing_to_angle, normalize_angle, sine,
    };

    use crate::engine::battle::views::Laser;
    use crate::engine::battle::{
        Concerns, Cx, ElementId, Race, RaceTemplate, Selection, ShipId, WeaponOrigin, WeaponSpawn,
    };
    pub static CHMMR: Race = Race {
        descriptor,
        preprocess: Some(chmmr_preprocess),
        postprocess: Some(chmmr_postprocess),
        init_weapon: Some(initialize_megawatt_laser),
        intelligence: Some(chmmr_intelligence),
        uninit: None,
    };
    const CHMMR_OFFSET: i32 = 18;
    const LASER_RANGE: i32 = 150 << ONE_SHIFT;
    const SATELLITE_OFFSET: i32 = 64 << ONE_SHIFT;
    const DEFENSE_RANGE: u16 = 64;

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
        d.info.ship_flags =
            (FIRES_FORE | IMMEDIATE_WEAPON | SEEKING_SPECIAL | POINT_DEFENSE) as u16;
        d.info.ship_cost = 30;
        d.info.crew_level = MAX_CREW_SIZE as u16;
        d.info.max_crew = MAX_CREW_SIZE as u16;
        d.info.energy_level = MAX_ENERGY_SIZE as u8;
        d.info.max_energy = MAX_ENERGY_SIZE as u8;
        d.resources.strings = Some(c"ship.chmmr.text");
        d.resources.icons = Some(c"ship.chmmr.icons");
        d.resources.melee_icon = Some(c"ship.chmmr.meleeicons");
        d.characteristics = CHARACTERISTIC_STUFF {
            max_thrust: 35,
            thrust_increment: 7,
            energy_regeneration: 1,
            weapon_energy_cost: 2,
            special_energy_cost: 1,
            energy_wait: 1,
            turn_wait: 3,
            thrust_wait: 5,
            weapon_wait: 0,
            special_wait: 0,
            ship_mass: 10,
        };
        d.resources.ship = [
            Some(c"ship.chmmr.graphics.avatar.large"),
            Some(c"ship.chmmr.graphics.avatar.medium"),
            Some(c"ship.chmmr.graphics.avatar.small"),
        ];
        d.resources.weapon = [
            Some(c"ship.chmmr.graphics.muzzle.large"),
            Some(c"ship.chmmr.graphics.muzzle.medium"),
            Some(c"ship.chmmr.graphics.muzzle.small"),
        ];
        d.resources.special = [
            Some(c"ship.chmmr.graphics.satellite.large"),
            Some(c"ship.chmmr.graphics.satellite.medium"),
            Some(c"ship.chmmr.graphics.satellite.small"),
        ];
        d.resources.captain = Some(c"ship.chmmr.graphics.captain");
        d.resources.victory = Some(c"ship.chmmr.ditty");
        d.resources.sounds = Some(c"ship.chmmr.sounds");
        d.intelligence.weapon_range = (50 << ONE_SHIFT) as u16;
        d.descriptor(&CHMMR)
    }
    pub(crate) fn laser_death(cx: &mut Cx<'_>, e: ElementId) {
        let ss = cx.owner(e);
        cx.ship_state_mut(ss).special_counter = cx.element(e).turn;
        if let Some(ship) = cx.optional_ship_element(ss) {
            let dx = cx
                .element(e)
                .current
                .location
                .x
                .wrapping_sub(cx.element(ship).current.location.x);
            let dy = cx
                .element(e)
                .current
                .location
                .y
                .wrapping_sub(cx.element(ship).current.location.y);
            if cx.random() as u8 & 7 != 0 {
                let dist = i64::from(dx) * i64::from(dx) + i64::from(dy) * i64::from(dy);
                let minimum = (CHMMR_OFFSET + 10) << ONE_SHIFT;
                if dist >= i64::from(minimum) * i64::from(minimum) {
                    if let Some(spot) = cx.alloc_element() {
                        cx.element_mut(spot).player = cx.element(e).player;
                        cx.element_mut(spot).flags =
                            (FINITE_LIFE | NONSOLID | IGNORE_SIMILAR | APPEARING) as u16;
                        cx.element_mut(spot).turn = 0;
                        cx.element_mut(spot).thrust = 0;
                        cx.element_mut(spot).life = 9;
                        let angle = i32::from(crate::math::arctan(dx, dy));
                        let magnitude = (i32::from(cx.random() as COUNT)
                            % (i32::from(crate::math::square_root(dist as DWORD)) + 1 - minimum)
                            + minimum) as COUNT;
                        cx.element_mut(spot).current.location.x = cx
                            .element(ship)
                            .current
                            .location
                            .x
                            .wrapping_add(cosine(angle, i32::from(magnitude)));
                        cx.element_mut(spot).current.location.y = cx
                            .element(ship)
                            .current
                            .location
                            .y
                            .wrapping_add(sine(angle, i32::from(magnitude)));
                        cx.set_frames(spot, false, cx.ship_frame_set(ss, 1));
                        let frame = cx.absolute_frame(
                            cx.ship_frame(ss, 1),
                            (angle_to_facing(FULL_CIRCLE as i32) << 1) as COUNT,
                        );
                        cx.set_frame(spot, false, frame);
                        cx.element_mut(spot).pre = Some(ElementHook::Native(animation_preprocess));
                        cx.set_parent(spot, Some(ss));
                        cx.primitive_style_mut(spot).kind = gfx_object_STAMP_PRIM as GRAPHICS_PRIM;
                        cx.put(spot);
                    }
                }
            }
        }
    }

    pub(crate) fn initialize_megawatt_laser(
        cx: &mut Cx<'_>,
        ss: ShipId,
        origin: WeaponOrigin,
    ) -> WeaponSpawn {
        let face = cx.ship_state(ss).facing;
        let rect = cx.frame_rect(cx.absolute_frame(cx.ship_frame(ss, 1), face));
        let block = Laser {
            origin: POINT {
                x: ((i32::from(origin.next.location.x) & !((1 << ONE_SHIFT) - 1))
                    + (i32::from(rect.corner.x) << ONE_SHIFT)) as COORD,
                y: ((i32::from(origin.next.location.y) & !((1 << ONE_SHIFT) - 1))
                    + (i32::from(rect.corner.y) << ONE_SHIFT)) as COORD,
            },
            end: POINT {
                x: cosine(facing_to_angle(i32::from(face)), LASER_RANGE),
                y: sine(facing_to_angle(i32::from(face)), LASER_RANGE),
            },
            sender: origin.player,
            flags: IGNORE_SIMILAR as u16,
            pixoffs: 0,
            face,
            color: [
                color(0x17, 0, 0),
                color(0x1f, 3, 0),
                color(0x1f, 0x11, 0),
                color(0x1f, 3, 0),
            ][cx.ship_state(ss).special_counter as usize],
        };
        let out = cx.spawn_laser(block);
        if let Some(laser) = out {
            cx.element_mut(laser).mass = 2;
            cx.element_mut(laser).death = Some(ElementHook::Native(laser_death));
            cx.element_mut(laser).turn =
                ((u32::from(cx.ship_state(ss).special_counter) + 1) % 4) as u8;
        }
        vec![out]
    }

    pub(crate) fn chmmr_intelligence(cx: &mut Cx<'_>, ss: ShipId, concerns: &mut Concerns) {
        cx.ship_intelligence(ss, concerns);
        cx.ship_state_mut(ss).input &= !(SPECIAL as u8);
        let enemy = ENEMY_SHIP_INDEX as usize;
        if cx.ship_state(ss).special_counter == 0
            && concerns.objects[enemy].object.is_some()
            && cx.ship_state(ss).input & WEAPON as u8 == 0
            && concerns.objects[enemy].which_turn > 12
            && normalize_angle(
                i32::from(concerns.subject.velocity.travel_angle)
                    - (i32::from(
                        cx.element(
                            concerns.objects[enemy]
                                .object
                                .unwrap_or_else(|| panic!("AI concern")),
                        )
                        .velocity
                        .travel_angle,
                    ) + HALF_CIRCLE as i32)
                    + QUADRANT as i32,
            ) > HALF_CIRCLE as i32
        {
            cx.ship_state_mut(ss).input |= SPECIAL as u8;
        }
    }

    pub(crate) fn chmmr_postprocess(cx: &mut Cx<'_>, ss: ShipId) {
        let ship = cx.ship_element(ss);
        if cx.ship_state(ss).status & WEAPON as u16 != 0 {
            let last = cx
                .elements()
                .tail()
                .unwrap_or_else(|| panic!("ship queue tail"));
            if last != ship
                && cx.element(last).player == cx.element(ship).player
                && cx.element(last).flags & APPEARING as u16 != 0
                && cx.primitive_style(last).kind == gfx_object_LINE_PRIM as GRAPHICS_PRIM
                && cx.ship_state(ss).special_counter & 1 == 0
            {
                if let Some(flash) = cx.alloc_element() {
                    cx.element_mut(flash).player = cx.element(ship).player;
                    cx.element_mut(flash).flags =
                        (FINITE_LIFE | NONSOLID | IGNORE_SIMILAR | APPEARING) as u16;
                    cx.element_mut(flash).life = 1;
                    cx.element_mut(flash).current.location = cx.element(ship).next.location;
                    cx.copy_image_to_current(flash, cx.element(ship).next.image);
                    cx.set_frames(flash, false, cx.ship_frame_set(ss, 1));
                    let frame = cx.absolute_frame(
                        cx.ship_frame(ss, 1),
                        (cx.ship_state(ss).facing as i32 + angle_to_facing(FULL_CIRCLE as i32))
                            as COUNT,
                    );
                    cx.set_frame(flash, false, frame);
                    cx.set_parent(flash, Some(ss));
                    cx.primitive_style_mut(flash).kind = gfx_object_STAMP_PRIM as GRAPHICS_PRIM;
                    cx.put(flash);
                }
            }
        }
        if cx.ship_state(ss).status & SPECIAL as u16 != 0 && cx.delta_energy(ship, -1) {
            // C plays the sound at the old target before TrackShip selects one.
            cx.process_ship_sound(ss, 1, cx.target(ship));
            let mut facing: COUNT = 0;
            if cx.track_ship(ship, &mut facing) >= 0 {
                let target = cx
                    .target(ship)
                    .unwrap_or_else(|| panic!("tracked ship target"));
                let collision_mass = i32::from(cx.element(target).mass) + 1;
                if collision_mass <= MAX_SHIP_MASS as i32 * 10 {
                    let a = facing_to_angle(i32::from(cx.ship_state(ss).facing));
                    let dx = (i32::from(cx.element(ship).next.location.x)
                        + i32::from(cosine(a, LASER_RANGE / 3 + (CHMMR_OFFSET << ONE_SHIFT)))
                        - i32::from(cx.element(target).next.location.x))
                        as SIZE;
                    let dy = (i32::from(cx.element(ship).next.location.y)
                        + i32::from(sine(a, LASER_RANGE / 3 + (CHMMR_OFFSET << ONE_SHIFT)))
                        - i32::from(cx.element(target).next.location.y))
                        as SIZE;
                    let angle = i32::from(crate::math::arctan(dx, dy));
                    let magnitude =
                        ((12 << VELOCITY_SHIFT) / i32::from(cx.element(target).mass)) as COUNT;
                    cx.element_mut(target).velocity.delta_components(
                        i32::from((cosine(angle, i32::from(magnitude))) as SIZE),
                        i32::from((sine(angle, i32::from(magnitude))) as SIZE),
                    );
                    let (vx, vy) = cx.element(target).velocity.get_current_components();
                    let (vx, vy) = (vx as SIZE, vy as SIZE);
                    let enemy_ss = cx.owner(target);
                    let speed =
                        (i64::from(vx) * i64::from(vx) + i64::from(vy) * i64::from(vy)) as DWORD;
                    let max_velocity =
                        (i32::from(cx.race_stats(enemy_ss).characteristics.max_thrust)
                            << VELOCITY_SHIFT) as SIZE;
                    let max_speed = (i64::from(max_velocity) * i64::from(max_velocity)) as DWORD;
                    cx.ship_state_mut(enemy_ss).status &=
                        !((SHIP_AT_MAX_SPEED | SHIP_BEYOND_MAX_SPEED) as u16);
                    if speed > max_speed {
                        cx.ship_state_mut(enemy_ss).status |=
                            (SHIP_AT_MAX_SPEED | SHIP_BEYOND_MAX_SPEED) as u16;
                    } else if speed == max_speed {
                        cx.ship_state_mut(enemy_ss).status |= SHIP_AT_MAX_SPEED as u16;
                    }
                    let offsets = [8, 17, 28, 42, 60];
                    let colors = [0x10, 0x0e, 0x0c, 9, 7];
                    for i in 0..5 {
                        if let Some(shadow) = cx.alloc_element() {
                            cx.element_mut(shadow).player = cx.element(target).player;
                            cx.element_mut(shadow).flags = super::tractor_shadow_flags();
                            cx.element_mut(shadow).life = 1;
                            cx.element_mut(shadow).current.location =
                                cx.element(target).next.location;
                            cx.copy_image_to_current(shadow, cx.element(target).next.image);
                            cx.element_mut(shadow).current.location.x = cx
                                .element(shadow)
                                .current
                                .location
                                .x
                                .wrapping_add(cosine(angle, offsets[i] << ONE_SHIFT));
                            cx.element_mut(shadow).current.location.y = cx
                                .element(shadow)
                                .current
                                .location
                                .y
                                .wrapping_add(sine(angle, offsets[i] << ONE_SHIFT));
                            cx.copy_visual_to_next(shadow, cx.element(shadow).current);
                            cx.set_parent(shadow, Some(enemy_ss));
                            cx.element_mut(shadow)
                                .velocity
                                .set_components(i32::from((vx) as SIZE), i32::from((vy) as SIZE));
                            cx.primitive_style_mut(shadow).kind =
                                gfx_object_STAMPFILL_PRIM as GRAPHICS_PRIM;
                            cx.primitive_style_mut(shadow).color = color(0, 0, colors[i]);
                            cx.insert_before(shadow, cx.elements().head());
                        }
                    }
                }
            }
        }
        cx.ship_state_mut(ss).special_counter = 0;
    }

    pub(crate) fn satellite_preprocess(cx: &mut Cx<'_>, e: ElementId) {
        cx.element_mut(e).life = cx.element(e).life.wrapping_add(1);
        let frame = cx.absolute_frame(
            cx.element(e).current.image.frame,
            (cx.frame_number(cx.element(e).current.image.frame)
                .wrapping_add(1))
                & 7,
        );
        cx.set_frame(e, true, frame);
        cx.element_mut(e).flags |= CHANGING as u16;
        cx.element_mut(e).turn = normalize_angle(i32::from(cx.element(e).turn) + 1) as u8;
        let ss = cx.owner(e);
        if let Some(ship) = cx.optional_ship_element(ss) {
            cx.race_stats_mut(ss).ship_info.ship_flags |= POINT_DEFENSE as u16;
            let mut dx = (i32::from(cx.element(ship).next.location.x)
                + i32::from(cosine(i32::from(cx.element(e).turn), SATELLITE_OFFSET))
                - i32::from(cx.element(e).current.location.x)) as SIZE;
            let mut dy = (i32::from(cx.element(ship).next.location.y)
                + i32::from(sine(i32::from(cx.element(e).turn), SATELLITE_OFFSET))
                - i32::from(cx.element(e).current.location.y)) as SIZE;
            let (width, height) = cx.screen_dimensions();
            let width = (width - 64) << (ONE_SHIFT + 3);
            let height = height << (ONE_SHIFT + 3);
            // WRAP_DELTA performs one adjustment, after the SIZE assignment.
            dx = if i32::from(dx).abs() <= width >> 1 {
                dx
            } else if dx < 0 {
                (width + i32::from(dx)) as SIZE
            } else {
                (i32::from(dx) - width) as SIZE
            };
            dy = if i32::from(dy).abs() <= height >> 1 {
                dy
            } else if dy < 0 {
                (height + i32::from(dy)) as SIZE
            } else {
                (i32::from(dy) - height) as SIZE
            };
            if i64::from(dx) * i64::from(dx) + i64::from(dy) * i64::from(dy)
                <= i64::from(20 << ONE_SHIFT).pow(2)
            {
                cx.element_mut(e).velocity.set_components(
                    i32::from(((i32::from(dx) << VELOCITY_SHIFT) as SIZE) as SIZE),
                    i32::from(((i32::from(dy) << VELOCITY_SHIFT) as SIZE) as SIZE),
                );
            } else {
                let angle = i32::from(crate::math::arctan(dx, dy));
                cx.element_mut(e).velocity.set_components(
                    i32::from((cosine(angle, (20 << ONE_SHIFT) << VELOCITY_SHIFT)) as SIZE),
                    i32::from((sine(angle, (20 << ONE_SHIFT) << VELOCITY_SHIFT)) as SIZE),
                );
            }
        }
    }

    pub(crate) fn spawn_point_defense(cx: &mut Cx<'_>, e: ElementId) {
        let ss = cx.owner(e);
        let ship = cx.ship_element(ss);
        let satellite = cx.target(e).unwrap_or_else(|| panic!("defense satellite"));
        let mut best = None;
        let mut best_dist: u16 = DEFENSE_RANGE + 1;
        let mut weakest: u8 = 255;
        let mut h = cx.elements().pred(e);
        while let Some(object) = h {
            let previous = cx.elements().pred(object);
            let shared_flags = cx.element(ship).flags & cx.element(object).flags;
            // CollisionPossible is a macro, including its asymmetric solidity test.
            if cx.element(object).player != cx.element(ship).player
                && i32::from(cx.element(object).player) != NEUTRAL_PLAYER_NUM
                && cx.element(object).flags & (NONSOLID | DISAPPEARING) as u16 == 0
                && shared_flags & COLLISION as u16 == 0
                && (shared_flags & IGNORE_SIMILAR as u16 == 0
                    || cx.parent(ship) != cx.parent(object))
                && (cx.element(ship).mass != 0 || cx.element(object).mass != 0)
                && !cx.object_cloaked(object)
            {
                let mut dx = cx
                    .element(object)
                    .next
                    .location
                    .x
                    .wrapping_sub(cx.element(satellite).next.location.x);
                let mut dy = cx
                    .element(object)
                    .next
                    .location
                    .y
                    .wrapping_sub(cx.element(satellite).next.location.y);
                if dx < 0 {
                    dx = dx.wrapping_neg();
                }
                if dy < 0 {
                    dy = dy.wrapping_neg();
                }
                let x = (dx >> ONE_SHIFT) as UWORD;
                let y = (dy >> ONE_SHIFT) as UWORD;
                if x <= DEFENSE_RANGE && y <= DEFENSE_RANGE {
                    let dist = (u32::from(x) * u32::from(x) + u32::from(y) * u32::from(y)) as UWORD;
                    let hp = cx.element(object).hits;
                    if dist <= DEFENSE_RANGE * DEFENSE_RANGE
                        && (hp < u16::from(weakest)
                            || (hp == u16::from(weakest) && dist < best_dist))
                    {
                        best = Some(object);
                        best_dist = dist;
                        weakest = hp as u8;
                    }
                }
            }
            h = previous;
        }
        if let Some(object) = best {
            let beam = cx.spawn_laser(Laser {
                origin: cx.element(satellite).next.location,
                end: POINT {
                    x: cx
                        .element(object)
                        .next
                        .location
                        .x
                        .wrapping_sub(cx.element(satellite).next.location.x),
                    y: cx
                        .element(object)
                        .next
                        .location
                        .y
                        .wrapping_sub(cx.element(satellite).next.location.y),
                },
                sender: cx.element(satellite).player,
                flags: IGNORE_SIMILAR as u16,
                pixoffs: 0,
                face: 0,
                color: color(0, 1, 0x1f),
            });
            if let Some(beam) = beam {
                cx.set_parent(beam, Some(ss));
                cx.set_target(beam, None);
                cx.put(beam);
                cx.element_mut(satellite).thrust = 2;
            }
        }
    }

    pub(crate) fn satellite_postprocess(cx: &mut Cx<'_>, e: ElementId) {
        if cx.element(e).thrust != 0 || cx.element(e).life == 0 {
            cx.element_mut(e).thrust = cx.element(e).thrust.wrapping_sub(1);
        } else if let Some(defense) = cx.alloc_element() {
            cx.put(defense);
            cx.element_mut(defense).player = cx.element(e).player;
            cx.element_mut(defense).flags = (APPEARING | NONSOLID | FINITE_LIFE) as u16;
            let successor = cx
                .elements()
                .succ(e)
                .unwrap_or_else(|| panic!("queued satellite successor"));
            cx.set_target(defense, cx.elements().pred(successor));
            cx.element_mut(defense).death = Some(ElementHook::Native(spawn_point_defense));
            cx.set_parent(defense, Some(cx.owner(e)));
        }
    }

    pub(crate) fn satellite_collision(
        _: &mut Cx<'_>,
        _: ElementId,
        _: &mut POINT,
        _: ElementId,
        _: &mut POINT,
    ) {
    }

    pub(crate) fn satellite_death(cx: &mut Cx<'_>, e: ElementId) {
        let ss = cx.owner(e);
        cx.race_stats_mut(ss).ship_info.ship_flags &= !(POINT_DEFENSE as u16);
        cx.element_mut(e).flags &= !(DISAPPEARING as u16);
        cx.element_mut(e).flags |= (NONSOLID | FINITE_LIFE | CHANGING) as u16;
        cx.element_mut(e).life = 4;
        cx.element_mut(e).turn = 1;
        cx.element_mut(e).thrust = 0;
        let frame = cx.absolute_frame(cx.element(e).current.image.frame, 8);
        cx.set_frame(e, true, frame);
        cx.element_mut(e).pre = Some(ElementHook::Native(animation_preprocess));
        cx.element_mut(e).death = None;
        cx.element_mut(e).post = None;
        cx.element_mut(e).collide = None;
    }

    pub(crate) fn spawn_satellites(cx: &mut Cx<'_>, e: ElementId) {
        let ss = cx.owner(e);
        if let Some(ship) = cx.optional_ship_element(ss) {
            for i in 0..3 {
                if let Some(satellite) = cx.alloc_element() {
                    cx.element_mut(satellite).player = cx.element(ship).player;
                    cx.element_mut(satellite).flags =
                        (IGNORE_SIMILAR | APPEARING | FINITE_LIFE) as u16;
                    cx.element_mut(satellite).life = (NORMAL_LIFE + 1) as COUNT;
                    cx.element_mut(satellite).hits = 10;
                    cx.element_mut(satellite).mass = 10;
                    let angle = (i * FULL_CIRCLE as i32 + 1) / 3;
                    cx.element_mut(satellite).turn = angle as u8;
                    cx.element_mut(satellite).current.location.x = cx
                        .element(ship)
                        .next
                        .location
                        .x
                        .wrapping_add(cosine(angle, SATELLITE_OFFSET));
                    cx.element_mut(satellite).current.location.y = cx
                        .element(ship)
                        .next
                        .location
                        .y
                        .wrapping_add(sine(angle, SATELLITE_OFFSET));
                    cx.set_frames(satellite, false, cx.ship_frame_set(ss, 2));
                    let index = cx.random() as COUNT & 7;
                    let frame = cx.absolute_frame(cx.ship_frame(ss, 2), index);
                    cx.set_frame(satellite, false, frame);
                    cx.element_mut(satellite).pre = Some(ElementHook::Native(satellite_preprocess));
                    cx.element_mut(satellite).post =
                        Some(ElementHook::Native(satellite_postprocess));
                    cx.element_mut(satellite).death = Some(ElementHook::Native(satellite_death));
                    cx.element_mut(satellite).collide =
                        Some(CollisionHook::Native(satellite_collision));
                    cx.set_parent(satellite, Some(ss));
                    cx.primitive_style_mut(satellite).kind = gfx_object_STAMP_PRIM as GRAPHICS_PRIM;
                    cx.put(satellite);
                }
            }
        }
    }

    pub(crate) fn chmmr_preprocess(cx: &mut Cx<'_>, ss: ShipId) {
        let e = cx.ship_element(ss);
        if let Some(satellite) = cx.alloc_element() {
            cx.element_mut(satellite).player = cx.element(e).player;
            cx.element_mut(satellite).flags =
                (FINITE_LIFE | NONSOLID | IGNORE_SIMILAR | APPEARING) as u16;
            cx.element_mut(satellite).life = (HYPERJUMP_LIFE + 1) as COUNT;
            cx.element_mut(satellite).death = Some(ElementHook::Native(spawn_satellites));
            cx.set_parent(satellite, Some(ss));
            cx.primitive_style_mut(satellite).kind = gfx_object_NUM_PRIMS as GRAPHICS_PRIM;
            cx.put(satellite);
        }
        cx.set_preprocess(ss, Selection::Disabled);
    }
}
