//! Live utwig callbacks translated from ships/utwig/utwig.c.
#[cfg(not(test))]
pub(crate) mod engine {
    use crate::ships::native::{abi::*, cosine, facing_to_angle, sine};
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
        d.info.ship_flags = (FIRES_FORE as i32 | POINT_DEFENSE as i32 | SHIELD_DEFENSE as i32) as _;
        d.info.ship_cost = 22;
        d.info.crew_level = 20;
        d.info.max_crew = 20;
        d.info.energy_level = ((20) >> 1) as _;
        d.info.max_energy = 20;
        d.resources.strings = Some(c"ship.utwig.text");
        d.resources.icons = Some(c"ship.utwig.icons");
        d.resources.melee_icon = Some(c"ship.utwig.meleeicons");
        d.fleet_strength = (666 / SPHERE_RADIUS_INCREMENT as i32 * 2) as u16;
        d.fleet_location = POINT { x: 8534, y: 8797 };
        d.characteristics.max_thrust = 36;
        d.characteristics.thrust_increment = 6;
        d.characteristics.energy_regeneration = 0;
        d.characteristics.weapon_energy_cost = 0;
        d.characteristics.special_energy_cost = 1;
        d.characteristics.energy_wait = 255;
        d.characteristics.turn_wait = 1;
        d.characteristics.thrust_wait = 6;
        d.characteristics.weapon_wait = 7;
        d.characteristics.special_wait = 12;
        d.characteristics.ship_mass = 8;
        d.resources.ship = [
            Some(c"ship.utwig.graphics.jugger.large"),
            Some(c"ship.utwig.graphics.jugger.medium"),
            Some(c"ship.utwig.graphics.jugger.small"),
        ];
        d.resources.weapon = [
            Some(c"ship.utwig.graphics.lance.large"),
            Some(c"ship.utwig.graphics.lance.medium"),
            Some(c"ship.utwig.graphics.lance.small"),
        ];
        d.resources.special = [None, None, None];
        d.resources.captain = Some(c"ship.utwig.graphics.captain");
        d.resources.victory = Some(c"ship.utwig.ditty");
        d.resources.sounds = Some(c"ship.utwig.sounds");
        d.intelligence.weapon_range = (50 << ONE_SHIFT) as u16;
        d.descriptor(&UTWIG)
    }
    use crate::engine::battle::views::Missile;
    use crate::engine::battle::{
        Concerns, Cx, ElementId, Race, RaceTemplate, ShipId, WeaponOrigin, WeaponSpawn,
    };
    pub static UTWIG: Race = Race {
        descriptor,
        preprocess: Some(utwig_preprocess),
        postprocess: None,
        init_weapon: Some(initialize_lance),
        intelligence: Some(utwig_intelligence),
        uninit: None,
    };
    pub(crate) fn initialize_lance(
        cx: &mut Cx<'_>,
        ss: ShipId,
        origin: WeaponOrigin,
    ) -> WeaponSpawn {
        let mut out = Vec::new();
        let mut block = Missile {
            farray: cx.ship_frame_set(ss, 1),
            face: cx.ship_state(ss).facing,
            index: cx.ship_state(ss).facing,
            sender: origin.player,
            flags: IGNORE_SIMILAR as u16,
            speed: 30 << ONE_SHIFT,
            hit_points: 1,
            damage: 1,
            life: 10,
            preprocess: None,
            blast_offs: 1,
            pixoffs: 0,
            cx: 0,
            cy: 0,
        };
        for (x, y) in [(5, -18), (13, -9), (17, -4)] {
            let angle = (facing_to_angle(i32::from(block.face)) + QUADRANT as i32) as COUNT;
            let cc = cosine(i32::from(angle), x << ONE_SHIFT);
            let ssin = sine(i32::from(angle), y << ONE_SHIFT);
            let sc = sine(i32::from(angle), x << ONE_SHIFT);
            let cs = cosine(i32::from(angle), y << ONE_SHIFT);
            let cos0 = cc.wrapping_sub(ssin);
            let sin0 = sc.wrapping_add(cs);
            block.cx = origin.next.location.x.wrapping_add(cos0);
            block.cy = origin.next.location.y.wrapping_add(sin0);
            out.push(cx.missile(&mut block));
            let cos0 = (-(i32::from(cc)) - i32::from(ssin)) as SIZE;
            let sin0 = (-(i32::from(sc)) + i32::from(cs)) as SIZE;
            block.cx = origin.next.location.x.wrapping_add(cos0);
            block.cy = origin.next.location.y.wrapping_add(sin0);
            out.push(cx.missile(&mut block));
        }
        out
    }

    pub(crate) fn utwig_intelligence(cx: &mut Cx<'_>, ss: ShipId, concerns: &mut Concerns) {
        let weapon = ENEMY_WEAPON_INDEX as usize;
        let mut shield: SIZE = 0;
        if cx.race_stats(ss).ship_info.energy_level != 0 {
            shield = -1;
            if concerns.objects[weapon].object.is_some()
                && concerns.objects[weapon].movement == MOVEMENT_STATE_ENTICE
            {
                shield = 0;
                let object = concerns.objects[weapon]
                    .object
                    .unwrap_or_else(|| panic!("AI concern"));
                if cx.element(object).flags & FINITE_LIFE as u16 == 0 {
                    concerns.objects[weapon].movement = MOVEMENT_STATE_PURSUE;
                } else if cx.element(object).mass != 0
                    || cx.element(object).flags & CREW_OBJECT as u16 != 0
                {
                    concerns.objects[weapon].which_turn >>= 1;
                    if concerns.objects[weapon].which_turn == 0 {
                        concerns.objects[weapon].which_turn = 1;
                    }
                    if cx.element(object).mass != 0 {
                        concerns.objects[weapon].object = None;
                    } else {
                        concerns.objects[weapon].movement = MOVEMENT_STATE_PURSUE;
                    }
                    shield = 1;
                }
            }
        }
        if cx.ship_state(ss).special_counter == 0 {
            cx.ship_state_mut(ss).input &= !(SPECIAL as u8);
            if shield != 0 {
                if (shield > 0 || concerns.objects[weapon].object.is_some())
                    && concerns.objects[weapon].which_turn <= 2
                    && (shield > 0
                        || cx
                            .element(
                                concerns.objects[weapon]
                                    .object
                                    .unwrap_or_else(|| panic!("AI concern")),
                            )
                            .flags
                            & PLAYER_SHIP as u16
                            != 0
                        || cx.plot_intercept(
                            &cx.weapon_origin(
                                concerns.objects[weapon]
                                    .object
                                    .unwrap_or_else(|| panic!("AI concern")),
                            ),
                            &concerns.subject,
                            2,
                            0,
                        ) != 0)
                    && cx.random() & 3 != 0
                {
                    cx.ship_state_mut(ss).input |= SPECIAL as u8;
                    cx.ship_state_mut(ss).input &= !(WEAPON as u8);
                }
                concerns.objects[weapon].object = None;
            }
        }
        let enemy = ENEMY_SHIP_INDEX as usize;
        if cx.race_stats(ss).ship_info.energy_level != 0 && concerns.objects[enemy].object.is_some()
        {
            let other = cx.owner(
                concerns.objects[enemy]
                    .object
                    .unwrap_or_else(|| panic!("enemy ship")),
            );
            if cx.race_stats(other).ship_info.ship_flags & IMMEDIATE_WEAPON as u16 == 0 {
                concerns.objects[enemy].movement = MOVEMENT_STATE_PURSUE;
            }
        }
        cx.ship_intelligence(ss, concerns);
    }

    pub(crate) fn utwig_collision(
        cx: &mut Cx<'_>,
        a: ElementId,
        pa: &mut POINT,
        b: ElementId,
        pb: &mut POINT,
    ) {
        if u32::from(cx.element(a).life) > NORMAL_LIFE
            && cx.element(b).flags & FINITE_LIFE as u16 != 0
            && cx.element(b).mass != 0
        {
            cx.element_mut(a).life = cx
                .element(a)
                .life
                .wrapping_add(u16::from(cx.element(b).mass));
        }
        collision(cx, a, pa, b, pb);
    }

    pub(crate) fn utwig_preprocess(cx: &mut Cx<'_>, ss: ShipId) {
        let e = cx.ship_element(ss);
        if cx.element(e).flags & APPEARING as u16 != 0 {
            cx.element_mut(e).collide = Some(CollisionHook::Native(utwig_collision));
        }
        if u32::from(cx.element(e).life) > NORMAL_LIFE + 1 {
            cx.delta_energy(
                e,
                (u32::from(cx.element(e).life) - (NORMAL_LIFE + 1)) as SIZE,
            );
            cx.process_ship_sound(ss, 1, e);
        }
        if cx.ship_state(ss).status & SPECIAL as u16 == 0 {
            cx.ship_state_mut(ss).special_counter = 0;
        } else if cx.ship_state(ss).special_counter.is_multiple_of(6) {
            if !cx.delta_energy(e, -1) {
                cx.race_stats_mut(ss).ship_info.ship_flags &=
                    !((POINT_DEFENSE | SHIELD_DEFENSE) as u16);
            } else if cx.ship_state(ss).special_counter == 0 {
                cx.ship_state_mut(ss).special_counter =
                    cx.race_stats(ss).characteristics.special_wait;
                cx.process_ship_sound(ss, 2, e);
            }
        }
        if cx.ship_state(ss).special_counter == 0 {
            cx.primitive_style_mut(e).color = color(0x1f, 0x1c, 0);
            cx.element_mut(e).color_cycle = 0;
            cx.element_mut(e).life = NORMAL_LIFE as COUNT;
            cx.primitive_style_mut(e).kind = gfx_object_STAMP_PRIM as GRAPHICS_PRIM;
        } else {
            const COLORS: [Color; 7] = [
                color(0x1f, 0x15, 0),
                color(0x1f, 0x0e, 0),
                color(0x1f, 7, 0),
                color(0x1f, 0, 0),
                color(0x17, 0, 0),
                color(0x13, 0, 0),
                color(0x0f, 0, 0),
            ];
            if cx.ship_state(ss).weapon_counter == 0 {
                cx.ship_state_mut(ss).weapon_counter += 1;
            }
            cx.element_mut(e).color_cycle = cx.element(e).color_cycle.wrapping_add(1);
            if usize::from(cx.element(e).color_cycle) == COLORS.len() + 1 {
                cx.element_mut(e).color_cycle = 1;
            }
            cx.primitive_style_mut(e).color = COLORS[usize::from(cx.element(e).color_cycle) - 1];
            cx.element_mut(e).life = (NORMAL_LIFE + 1) as COUNT;
            cx.primitive_style_mut(e).kind = gfx_object_STAMPFILL_PRIM as GRAPHICS_PRIM;
        }
    }
}
