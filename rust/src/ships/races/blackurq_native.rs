//! Live black_urquan callbacks translated from ships/blackurq/blackurq.c.
#[cfg(not(test))]
pub(crate) mod engine {
    use crate::ships::native::{abi::*, angle_to_facing, normalize_facing, wrap_delta};

    fn descriptor(_world: &mut crate::engine::battle::World) -> crate::engine::battle::RaceDesc {
        let mut d = RaceTemplate::default();
        d.info.ship_flags = (FIRES_FORE as i32) as _;
        d.info.ship_cost = 30;
        d.info.crew_level = (MAX_CREW_SIZE as i32) as _;
        d.info.max_crew = (MAX_CREW_SIZE as i32) as _;
        d.info.energy_level = (MAX_ENERGY_SIZE as i32) as _;
        d.info.max_energy = (MAX_ENERGY_SIZE as i32) as _;
        d.resources.strings = Some(c"ship.kohrah.text");
        d.resources.icons = Some(c"ship.kohrah.icons");
        d.resources.melee_icon = Some(c"ship.kohrah.meleeicons");
        d.fleet_strength = (2666 / SPHERE_RADIUS_INCREMENT as i32 * 2) as u16;
        d.fleet_location = POINT { x: 6000, y: 6250 };
        d.characteristics.max_thrust = 30;
        d.characteristics.thrust_increment = 6;
        d.characteristics.energy_regeneration = 1;
        d.characteristics.weapon_energy_cost = 6;
        d.characteristics.special_energy_cost = (MAX_ENERGY_SIZE as i32 / 2) as _;
        d.characteristics.energy_wait = 4;
        d.characteristics.turn_wait = 4;
        d.characteristics.thrust_wait = 6;
        d.characteristics.weapon_wait = 6;
        d.characteristics.special_wait = 9;
        d.characteristics.ship_mass = 10;
        d.resources.ship = [
            Some(c"ship.kohrah.graphics.marauder.large"),
            Some(c"ship.kohrah.graphics.marauder.medium"),
            Some(c"ship.kohrah.graphics.marauder.small"),
        ];
        d.resources.weapon = [
            Some(c"ship.kohrah.graphics.buzzsaw.large"),
            Some(c"ship.kohrah.graphics.buzzsaw.medium"),
            Some(c"ship.kohrah.graphics.buzzsaw.small"),
        ];
        d.resources.special = [
            Some(c"ship.kohrah.graphics.gas.large"),
            Some(c"ship.kohrah.graphics.gas.medium"),
            Some(c"ship.kohrah.graphics.gas.small"),
        ];
        d.resources.captain = Some(c"ship.kohrah.graphics.captain");
        d.resources.victory = Some(c"ship.kohrah.ditty");
        d.resources.sounds = Some(c"ship.kohrah.sounds");
        d.intelligence.weapon_range = (50 << ONE_SHIFT) as u16;
        d.descriptor(&BLACKURQ)
    }
    use crate::engine::battle::views::Missile;
    use crate::engine::battle::{
        Concerns, Cx, ElementId, Race, RaceTemplate, ShipId, WeaponOrigin, WeaponSpawn,
    };
    pub static BLACKURQ: Race = Race {
        descriptor,
        preprocess: Some(black_urquan_preprocess),
        postprocess: Some(black_urquan_postprocess),
        init_weapon: Some(initialize_buzzsaw),
        intelligence: Some(black_urquan_intelligence),
        uninit: None,
    };
    pub(crate) fn spin_preprocess(cx: &mut Cx<'_>, e: ElementId) {
        let ss = cx.owner(e);
        let ship = cx.ship_element(ss);
        if cx.element(ship).hits != 0 && {
            let wait = cx
                .race_stats(ss)
                .characteristics
                .special_wait
                .wrapping_add(1);
            cx.race_stats_mut(ss).characteristics.special_wait = wait;
            wait > 8
        } {
            cx.element_mut(e).life = 1;
            cx.element_mut(e).flags |= DISAPPEARING as u16;
        } else {
            cx.element_mut(e).life = cx.element(e).life.wrapping_add(1);
            if cx.element(e).turn != 0 {
                cx.element_mut(e).turn -= 1;
            } else {
                cx.set_frame(
                    e,
                    true,
                    if cx.frame_number(cx.element(e).current.image.frame) < 1 {
                        cx.advance_frame(cx.element(e).current.image.frame)
                    } else {
                        cx.absolute_frame(cx.element(e).current.image.frame, 0)
                    },
                );
                cx.element_mut(e).flags |= CHANGING as u16;
                cx.element_mut(e).turn = 0;
            }
        }
    }

    pub(crate) fn buzztrack_preprocess(cx: &mut Cx<'_>, e: ElementId) {
        if cx.element(e).thrust != 0 {
            cx.element_mut(e).thrust -= 1;
        } else {
            let mut facing = 0;
            if cx.target(e).is_none() && cx.track_ship(e, &mut facing) < 0 {
                cx.element_mut(e).velocity = VELOCITY_DESC::new();
            } else {
                let target = cx.target(e).unwrap_or_else(|| panic!("tracked target"));
                let mut dx = wrap_delta(
                    cx.element(target)
                        .current
                        .location
                        .x
                        .wrapping_sub(cx.element(e).current.location.x),
                    (cx.screen_dimensions().0 - 64) << (ONE_SHIFT + 3),
                );
                let mut dy = wrap_delta(
                    cx.element(target)
                        .current
                        .location
                        .y
                        .wrapping_sub(cx.element(e).current.location.y),
                    cx.screen_dimensions().1 << (ONE_SHIFT + 3),
                );
                facing = normalize_facing(angle_to_facing(i32::from(crate::math::arctan(dx, dy))));
                if dx < 0 {
                    dx = dx.wrapping_neg();
                }
                if dy < 0 {
                    dy = dy.wrapping_neg();
                }
                dx >>= ONE_SHIFT;
                dy >>= ONE_SHIFT;
                if dx >= 224
                    || dy >= 224
                    || (i32::from(dx as u16) * i32::from(dx)) as u32
                        + (i32::from(dy as u16) * i32::from(dy)) as u32
                        >= 224 * 224
                {
                    cx.element_mut(e).velocity = VELOCITY_DESC::new();
                } else {
                    cx.element_mut(e).thrust = 4;
                    cx.element_mut(e)
                        .velocity
                        .set_vector(2 << ONE_SHIFT, facing);
                }
            }
        }
        spin_preprocess(cx, e);
    }

    pub(crate) fn decelerate_preprocess(cx: &mut Cx<'_>, e: ElementId) {
        let (x, y) = cx.element(e).velocity.get_current_components();
        let (mut dx, mut dy) = (x as SIZE, y as SIZE);
        dx /= 2;
        dy /= 2;
        cx.element_mut(e)
            .velocity
            .set_components(i32::from(dx), i32::from(dy));
        if dx == 0 && dy == 0 {
            cx.element_mut(e).pre = Some(ElementHook::Native(buzztrack_preprocess));
        }
        spin_preprocess(cx, e);
    }
    pub(crate) fn splinter_preprocess(cx: &mut Cx<'_>, e: ElementId) {
        cx.set_frame(e, true, cx.advance_frame(cx.element(e).current.image.frame));
        cx.element_mut(e).flags |= CHANGING as u16;
    }
    pub(crate) fn buzzsaw_collision(
        cx: &mut Cx<'_>,
        a: ElementId,
        pa: &mut POINT,
        b: ElementId,
        pb: &mut POINT,
    ) {
        cx.weapon_collision(a, pa, b, pb);
        if cx.element(a).flags & DISAPPEARING as u16 != 0 {
            cx.element_mut(a).flags &= !(DISAPPEARING as u16);
            cx.element_mut(a).flags |= (NONSOLID | CHANGING) as u16;
            cx.element_mut(a).life = 5;
            cx.set_frame(
                a,
                true,
                cx.absolute_frame(cx.element(a).current.image.frame, 2),
            );
            cx.element_mut(a).pre = Some(ElementHook::Native(splinter_preprocess));
        }
    }
    pub(crate) fn buzzsaw_preprocess(cx: &mut Cx<'_>, e: ElementId) {
        let ss = cx.owner(e);
        if cx.ship_state(ss).status & WEAPON as u16 == 0 {
            cx.element_mut(e).life >>= 1;
            cx.element_mut(e).pre = Some(ElementHook::Native(decelerate_preprocess));
        }
        spin_preprocess(cx, e);
    }
    pub(crate) fn buzzsaw_postprocess(cx: &mut Cx<'_>, e: ElementId) {
        cx.element_mut(e).post = None;
        if let Some(list) = cx.alloc_element() {
            cx.copy_allocated_element(list, e);
            let next = cx.element(list).next;
            cx.element_mut(list).current.location = next.location;
            cx.copy_image_to_current(list, next.image);
            cx.init_intersect(list);
            cx.element_mut(list).flags = (cx.element(list).flags
                & !((PRE_PROCESS | CHANGING | APPEARING) as u16))
                | POST_PROCESS as u16;
            let ss = cx.owner(e);
            let before = cx.elements().succ(cx.ship_element(ss));
            cx.insert_before(list, before);
            cx.element_mut(e).life = 0;
        }
    }
    pub(crate) fn initialize_buzzsaw(
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
            flags: IGNORE_SIMILAR as u16,
            pixoffs: 28,
            speed: 64,
            hit_points: 10,
            damage: 4,
            life: 64,
            preprocess: Some(buzzsaw_preprocess),
            blast_offs: 9,
        };
        out.push(cx.missile(&mut block));
        if let Some(e) = out[0] {
            cx.element_mut(e).turn = 0;
            cx.element_mut(e).thrust = 0;
            cx.element_mut(e).post = Some(ElementHook::Native(buzzsaw_postprocess));
            cx.element_mut(e).collide = Some(CollisionHook::Native(buzzsaw_collision));
        }
        out
    }
    pub(crate) fn black_urquan_intelligence(cx: &mut Cx<'_>, ss: ShipId, concerns: &mut Concerns) {
        let weapon = ENEMY_WEAPON_INDEX as usize;
        if concerns.objects[weapon].object.is_some()
            && concerns.objects[weapon].movement == MOVEMENT_STATE_ENTICE
            && cx
                .element(
                    concerns.objects[weapon]
                        .object
                        .unwrap_or_else(|| panic!("AI concern")),
                )
                .flags
                & CREW_OBJECT as u16
                != 0
            && concerns.objects[weapon].which_turn <= 8
        {
            concerns.objects[weapon].movement = MOVEMENT_STATE_PURSUE;
        }
        cx.ship_intelligence(ss, concerns);
        cx.ship_state_mut(ss).input &= !(SPECIAL as u8);
        if cx.ship_state(ss).special_counter == 0
            && cx.race_stats(ss).ship_info.energy_level >= (MAX_ENERGY_SIZE / 2) as u8
            && concerns.objects[weapon].object.is_some()
            && concerns.objects[weapon].which_turn <= 8
        {
            cx.ship_state_mut(ss).input |= SPECIAL as u8;
        }
        let enemy = ENEMY_SHIP_INDEX as usize;
        if let Some(target) = concerns.objects[enemy].object {
            let mut h = if cx.ship_state(ss).old_status & WEAPON as u16 != 0 {
                cx.elements().succ(cx.ship_element(ss))
            } else {
                None
            };
            while let Some(saw) = h {
                let next = cx.elements().succ(saw);
                if cx.element(saw).flags & NONSOLID as u16 == 0
                    && cx.element(saw).next.image.frames == cx.ship_frame_set(ss, 1)
                    && cx.element(saw).life > 64 * 3 / 4
                    && cx.element(saw).player == concerns.subject.player
                {
                    if cx.plot_intercept(
                        &cx.weapon_origin(saw),
                        &cx.weapon_origin(target),
                        cx.element(saw).life,
                        640 / 2,
                    ) == 0
                    {
                        cx.ship_state_mut(ss).input &= !(WEAPON as u8);
                    } else if cx.ship_state(ss).weapon_counter == 0 {
                        cx.ship_state_mut(ss).input |= WEAPON as u8;
                    }
                    break;
                }
                h = next;
            }
            if h.is_none() {
                if cx.ship_state(ss).old_status & WEAPON as u16 != 0 {
                    cx.ship_state_mut(ss).input &= !(WEAPON as u8);
                } else if cx.ship_state(ss).weapon_counter == 0
                    && cx.ship_weapons(&concerns.subject, &cx.weapon_origin(target), 640 / 2) != 0
                {
                    cx.ship_state_mut(ss).input |= WEAPON as u8;
                }
                if cx.ship_state(ss).special_counter == 0
                    && cx.ship_state(ss).input & WEAPON as u8 == 0
                    && concerns.objects[enemy].which_turn <= 8
                    && cx.ship_state(ss).input & (LEFT | RIGHT) as u8 != 0
                    && cx.race_stats(ss).ship_info.energy_level >= (MAX_ENERGY_SIZE / 2) as u8
                {
                    cx.ship_state_mut(ss).input |= SPECIAL as u8;
                }
            }
        }
    }
    pub(crate) fn gas_cloud_preprocess(cx: &mut Cx<'_>, e: ElementId) {
        if cx.element(e).turn != 0 {
            cx.element_mut(e).turn -= 1;
        } else {
            cx.set_frame(e, true, cx.advance_frame(cx.element(e).current.image.frame));
            cx.element_mut(e).flags |= CHANGING as u16;
            cx.element_mut(e).turn = 2;
        }
    }
    pub(crate) fn gas_cloud_collision(
        cx: &mut Cx<'_>,
        a: ElementId,
        pa: &mut POINT,
        b: ElementId,
        pb: &mut POINT,
    ) {
        cx.element_mut(a).mass = if cx.element(b).flags & PLAYER_SHIP as u16 != 0 {
            3
        } else {
            50
        };
        cx.weapon_collision(a, pa, b, pb);
    }
    pub(crate) fn spawn_gas_cloud(cx: &mut Cx<'_>, e: ElementId) {
        let ss = cx.owner(e);
        let mut block = Missile {
            cx: cx.element(e).next.location.x,
            cy: cx.element(e).next.location.y,
            farray: cx.ship_frame_set(ss, 2),
            face: 0,
            index: 0,
            sender: cx.element(e).player,
            flags: IGNORE_SIMILAR as u16,
            pixoffs: 20,
            speed: 16,
            hit_points: 100,
            damage: 3,
            life: (i32::from(cx.frame_count(cx.ship_frame(ss, 2))) * 3 - 1) as COUNT,
            preprocess: Some(gas_cloud_preprocess),
            blast_offs: 2,
        };
        let (x, y) = cx.element(e).velocity.get_current_components();
        let (dx, dy) = (x as SIZE, y as SIZE);
        while i32::from(block.face) < angle_to_facing(FULL_CIRCLE as i32) {
            let h = cx.missile(&mut block);
            if let Some(gas) = h {
                cx.set_parent(gas, Some(ss));
                cx.set_target(gas, None);
                cx.element_mut(gas).turn = 1;
                cx.element_mut(gas).collide = Some(CollisionHook::Native(gas_cloud_collision));
                cx.element_mut(gas)
                    .velocity
                    .delta_components(i32::from(dx), i32::from(dy));
                cx.put(gas);
            }
            block.face =
                (i32::from(block.face) + angle_to_facing(FULL_CIRCLE as i32) / 16) as COUNT;
        }
    }
    pub(crate) fn black_urquan_postprocess(cx: &mut Cx<'_>, ss: ShipId) {
        let e = cx.ship_element(ss);
        if cx.ship_state(ss).status & SPECIAL as u16 != 0
            && cx.ship_state(ss).special_counter == 0
            && cx.delta_energy(e, -((MAX_ENERGY_SIZE / 2) as SIZE))
        {
            spawn_gas_cloud(cx, e);
            cx.process_ship_sound(ss, 1, e);
            cx.ship_state_mut(ss).special_counter = 9;
        }
    }
    pub(crate) fn black_urquan_preprocess(cx: &mut Cx<'_>, ss: ShipId) {
        cx.race_stats_mut(ss).characteristics.special_wait = 0;
        if cx.ship_state(ss).weapon_counter == 0
            && cx.ship_state(ss).status & cx.ship_state(ss).old_status & WEAPON as u16 != 0
        {
            cx.ship_state_mut(ss).weapon_counter += 1;
        }
    }
}
