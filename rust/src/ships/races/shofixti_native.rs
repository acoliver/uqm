//! Shofixti live callbacks translated from `sc2/src/uqm/ships/shofixti/shofixti.c`.
#[cfg(not(test))]
pub(crate) mod engine {
    use crate::engine::battle::{Concerns, Cx, ElementId, ShipId, WeaponOrigin, WeaponSpawn};
    use crate::ships::native::abi::*;
    const MAX_CREW: u16 = 6;
    const MAX_ENERGY: u8 = 4;
    const MAX_THRUST: u16 = 35;
    const THRUST_INCREMENT: u16 = 5;
    const DESTRUCT_RANGE: i32 = 180;
    const MAX_DESTRUCTION: i32 = DESTRUCT_RANGE / 10;
    // element.h: NUM_EXPLOSION_FRAMES is twelve, including for the ship flash.
    const DESTRUCT_SWITCH: u16 = 12 * 3 - 3;
    use crate::ships::races::orz_native::engine::intruder_preprocess;

    use crate::ships::races::orz_native::engine::marine_collision;

    // BUILD_COLOR ignores the palette index; MAKE_RGB15 expands each channel.
    const fn rgb15(r: u8, g: u8, b: u8) -> Color {
        Color {
            r: (r << 3) | (r >> 2),
            g: (g << 3) | (g >> 2),
            b: (b << 3) | (b >> 2),
            a: 255,
        }
    }

    fn same_color(a: Color, b: Color) -> bool {
        a.r == b.r && a.g == b.g && a.b == b.b && a.a == b.a
    }

    fn descriptor(world: &mut crate::engine::battle::World) -> crate::engine::battle::RaceDesc {
        let mut d = crate::engine::battle::template::RaceTemplate {
            info: crate::engine::battle::views::Info {
                ship_flags: FIRES_FORE as u16,
                ship_cost: 5,
                crew_level: MAX_CREW,
                max_crew: MAX_CREW,
                energy_level: MAX_ENERGY,
                max_energy: MAX_ENERGY,
            },
            resources: crate::engine::battle::template::RaceResources {
                strings: Some(c"ship.shofixti.text"),
                icons: Some(c"ship.shofixti.icons"),
                melee_icon: Some(c"ship.shofixti.meleeicons"),
                ship: [
                    Some(c"ship.shofixti.graphics.scout.large"),
                    Some(c"ship.shofixti.graphics.scout.medium"),
                    Some(c"ship.shofixti.graphics.scout.small"),
                ],
                weapon: [
                    Some(c"ship.shofixti.graphics.missile.large"),
                    Some(c"ship.shofixti.graphics.missile.medium"),
                    Some(c"ship.shofixti.graphics.missile.small"),
                ],
                special: [
                    Some(c"ship.shofixti.graphics.destruct.large"),
                    Some(c"ship.shofixti.graphics.destruct.medium"),
                    Some(c"ship.shofixti.graphics.destruct.small"),
                ],
                captain: Some(c"ship.shofixti.graphics.captain"),
                victory: Some(c"ship.shofixti.ditty"),
                sounds: Some(c"ship.shofixti.sounds"),
            },
            characteristics: CHARACTERISTIC_STUFF {
                max_thrust: MAX_THRUST,
                thrust_increment: THRUST_INCREMENT,
                energy_regeneration: 1,
                weapon_energy_cost: 1,
                special_energy_cost: 0,
                energy_wait: 9,
                turn_wait: 1,
                thrust_wait: 0,
                weapon_wait: 3,
                special_wait: 0,
                ship_mass: 1,
            },
            intelligence: crate::engine::battle::views::Intelligence {
                weapon_range: ((24 << ONE_SHIFT) * 10) as COUNT,
                ..Default::default()
            },
            ..Default::default()
        };
        if world.activity() as u8 == IN_ENCOUNTER as u8
            && world.story_state(crate::engine::battle::race::StoryKey::ShofixtiRecruited) == 0
        {
            configure_old_scout(&mut d);
        }

        d.descriptor(&SHOFIXTI)
    }

    fn configure_old_scout(d: &mut crate::engine::battle::template::RaceTemplate) {
        d.resources.ship = [
            Some(c"ship.shofixti.graphics.oldscout.large"),
            Some(c"ship.shofixti.graphics.oldscout.medium"),
            Some(c"ship.shofixti.graphics.oldscout.small"),
        ];
        d.resources.special = [None; 3];
        d.resources.captain = Some(c"ship.shofixti.graphics.oldcaptain");
        d.characteristics.weapon_wait = 10;
        for _ in 0..3 {
            let c = &mut d.characteristics;
            c.turn_wait = c.turn_wait.wrapping_add(1);
            if c.turn_wait == 0 {
                c.turn_wait = c.turn_wait.wrapping_sub(1);
            }
            c.thrust_wait = c.thrust_wait.wrapping_add(1);
            if c.thrust_wait == 0 {
                c.thrust_wait = c.thrust_wait.wrapping_sub(1);
            }
            if c.thrust_increment <= 1 << ONE_SHIFT {
                c.max_thrust = c.thrust_increment << 1;
            } else {
                let num_thrusts = c.max_thrust / c.thrust_increment;
                c.thrust_increment -= 1;
                c.max_thrust = c.thrust_increment * num_thrusts;
            }
        }
    }

    pub static SHOFIXTI: crate::engine::battle::Race = crate::engine::battle::Race {
        descriptor,
        preprocess: None,
        postprocess: Some(shofixti_postprocess),
        init_weapon: Some(initialize_standard_missile),
        intelligence: Some(shofixti_intelligence),
        uninit: None,
    };

    pub(crate) fn initialize_standard_missile(
        cx: &mut Cx<'_>,
        ss: ShipId,
        origin: WeaponOrigin,
    ) -> WeaponSpawn {
        let facing = cx.ship_state(ss).facing;
        vec![cx.missile(&mut crate::engine::battle::views::Missile {
            cx: origin.next.location.x,
            cy: origin.next.location.y,
            farray: cx.ship_frame_set(ss, 1),
            face: facing,
            index: facing,
            sender: origin.player,
            flags: IGNORE_SIMILAR as u16,
            pixoffs: 15,
            speed: 24 << ONE_SHIFT,
            hit_points: 1,
            damage: 1,
            life: 10,
            preprocess: None,
            blast_offs: 1,
        })]
    }

    pub(crate) fn destruct_preprocess(cx: &mut Cx<'_>, e: ElementId) {
        cx.element_mut(e).flags |= CHANGING as u16;
        let life = cx.element(e).life;
        if life > DESTRUCT_SWITCH {
            let mut primitive = cx.primitive_style_mut(e);
            primitive.kind = gfx_object_STAMPFILL_PRIM as GRAPHICS_PRIM;
            primitive.color = if life == DESTRUCT_SWITCH + 2 {
                rgb15(31, 31, 10)
            } else {
                rgb15(31, 31, 31)
            };
        } else if life < DESTRUCT_SWITCH {
            let color = cx.primitive_style(e).color;
            let frame = cx.advance_frame(cx.element(e).current.image.frame);
            cx.set_frame(e, true, frame);
            for (from, to) in [
                (rgb15(31, 31, 31), rgb15(31, 31, 10)),
                (rgb15(31, 31, 10), rgb15(31, 10, 10)),
                (rgb15(31, 10, 10), rgb15(20, 10, 0)),
                (rgb15(20, 10, 0), rgb15(20, 0, 0)),
            ] {
                if same_color(color, from) {
                    cx.primitive_style_mut(e).color = to;
                    break;
                }
            }
        } else {
            cx.primitive_style_mut(e).kind = gfx_object_NUM_PRIMS as GRAPHICS_PRIM;
            cx.element_mut(e).pre = None;
            if let Some(d) = cx.alloc_element() {
                let ss = cx.owner(e);
                cx.put(d);
                cx.set_parent(d, Some(ss));
                cx.element_mut(d).hits = 0;
                cx.element_mut(d).mass = 0;
                cx.element_mut(d).player = NEUTRAL_PLAYER_NUM as SIZE;
                cx.element_mut(d).flags = (APPEARING | FINITE_LIFE | NONSOLID) as u16;
                cx.primitive_style_mut(d).kind = gfx_object_STAMPFILL_PRIM as GRAPHICS_PRIM;
                cx.primitive_style_mut(d).color = rgb15(31, 31, 31);
                let frame = cx.ship_frame(ss, 2);
                cx.set_frames(d, false, cx.ship_frame_set(ss, 2));
                cx.set_frame(d, false, frame);
                cx.element_mut(d).life = cx.frame_count(frame);
                cx.element_mut(d).current.location = cx.element(e).current.location;
                cx.element_mut(d).pre = Some(ElementHook::Native(destruct_preprocess));
                cx.element_mut(d).post = None;
                cx.element_mut(d).death = None;
                cx.element_mut(d).velocity = VELOCITY_DESC::default();
            }
        }
    }

    pub(crate) fn self_destruct_kill_objects(cx: &mut Cx<'_>, e: ElementId) {
        let mut h = cx.elements().head();
        while let Some(obj) = h {
            h = cx.elements().succ(obj);
            let object = cx.element(obj);
            let flags = object.flags;
            let marine = object.pre.map(|f| f.address())
                == Some(intruder_preprocess as *const () as usize)
                && object.collide.map(|f| f.address())
                    == Some(marine_collision as *const () as usize);
            if flags & (NONSOLID | DISAPPEARING) as u16 == 0 || marine {
                let mut dx = (i32::from(object.next.location.x)
                    - i32::from(cx.element(e).next.location.x))
                    as SIZE;
                let mut dy = (i32::from(object.next.location.y)
                    - i32::from(cx.element(e).next.location.y))
                    as SIZE;
                if dx < 0 {
                    dx = dx.wrapping_neg();
                }
                if dy < 0 {
                    dy = dy.wrapping_neg();
                }
                dx >>= ONE_SHIFT;
                dy >>= ONE_SHIFT;
                let dist = (i32::from(dx) * i32::from(dx) + i32::from(dy) * i32::from(dy)) as DWORD;
                if i32::from(dx) <= DESTRUCT_RANGE
                    && i32::from(dy) <= DESTRUCT_RANGE
                    && dist <= (DESTRUCT_RANGE * DESTRUCT_RANGE) as DWORD
                {
                    let destruction = 1 + MAX_DESTRUCTION
                        * (DESTRUCT_RANGE - i32::from(crate::math::square_root(dist)))
                        / DESTRUCT_RANGE;
                    if flags & PLAYER_SHIP as u16 != 0 {
                        if !cx.delta_crew(obj, -destruction as SIZE) {
                            cx.element_mut(obj).life = 0;
                        }
                    } else if u32::from(object.mass) <= MAX_SHIP_MASS * 10 {
                        if destruction < i32::from(object.hits) {
                            cx.element_mut(obj).hits =
                                object.hits.wrapping_sub(destruction as COUNT);
                        } else {
                            cx.element_mut(obj).hits = 0;
                            cx.element_mut(obj).life = 0;
                        }
                    }
                }
            }
        }
    }

    pub(crate) fn self_destruct(cx: &mut Cx<'_>, e: ElementId) {
        if let Some(d) = cx.alloc_element() {
            cx.element_mut(d).player = cx.element(e).player;
            cx.element_mut(d).flags = (APPEARING | NONSOLID | FINITE_LIFE) as u16;
            cx.element_mut(d).next.location = cx.element(e).next.location;
            cx.element_mut(d).life = 0;
            cx.set_parent(d, Some(cx.owner(e)));
            cx.set_target(d, None);
            cx.element_mut(d).death = Some(ElementHook::Native(self_destruct_kill_objects));
            cx.put(d);
        }
        cx.delta_crew(e, -(i32::from(cx.element(e).hits)) as SIZE);
        cx.element_mut(e).flags |= NONSOLID as u16;
        cx.element_mut(e).life = 0;
        cx.element_mut(e).death = Some(ElementHook::Native(shofixti_destruct_death));
    }

    pub(crate) fn shofixti_destruct_death(cx: &mut Cx<'_>, e: ElementId) {
        let ss = cx.owner(e);
        cx.stop_battle_music();
        cx.start_ship_explosion(e, false);
        cx.element_mut(e).pre = Some(ElementHook::Native(destruct_preprocess));
        cx.play_ship_sound(ss, 1, e, 3);
        cx.select_self_destruct_winner(e);
        cx.record_ship_death(e);
    }

    pub(crate) fn shofixti_intelligence(cx: &mut Cx<'_>, ss: ShipId, concerns: &mut Concerns) {
        cx.ship_intelligence(ss, concerns);
        if cx.ship_state(ss).special_counter != 0 {
            return;
        }
        if cx.ship_state(ss).input & SPECIAL as u8 != 0 {
            cx.ship_state_mut(ss).input &= !(SPECIAL as u8);
        } else {
            let weapon = concerns.objects[ENEMY_WEAPON_INDEX as usize];
            let enemy = concerns.objects[ENEMY_SHIP_INDEX as usize];
            let captain = cx.captain_special(ss);
            if !cx.ship_frame(ss, 2).is_absent()
                && (i32::from(cx.frame_count(captain)) - i32::from(cx.frame_number(captain)) > 5
                    || (enemy.object.is_some() && enemy.which_turn <= 4)
                    || weapon.object.is_some_and(|threat| {
                        (cx.element(threat).flags & PLAYER_SHIP as u16 != 0
                            && concerns.subject.hits == 1)
                            || (cx.plot_intercept(
                                &cx.weapon_origin(threat),
                                &concerns.subject,
                                2,
                                0,
                            ) != 0
                                && u16::from(cx.element(threat).mass) >= concerns.subject.hits
                                && cx.random() & 1 != 0)
                    }))
            {
                cx.ship_state_mut(ss).input |= SPECIAL as u8;
            }
        }
    }

    pub(crate) fn shofixti_postprocess(cx: &mut Cx<'_>, ss: ShipId) {
        if (cx.ship_state(ss).status ^ cx.ship_state(ss).old_status) & SPECIAL as u16 != 0 {
            let captain = cx.advance_frame(cx.captain_special(ss));
            cx.set_captain_special(ss, captain);
            if i32::from(cx.frame_count(captain)) - i32::from(cx.frame_number(captain)) == 3 {
                self_destruct(cx, cx.ship_element(ss));
            }
        }
    }
}
