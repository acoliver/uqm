//! VUX behavior over the battle context's ids and per-ship descriptor.
#[cfg(not(test))]
pub(crate) mod engine {
    use crate::collections::observation::image;
    use crate::engine::battle::{
        Concerns, Cx, ElementId, Race, RaceState, ShipId, WeaponOrigin, WeaponSpawn,
    };
    use crate::ships::native::{
        abi::*, angle_to_facing, cosine, facing_to_angle, normalize_facing, sine,
    };
    pub fn descriptor() -> RACE_DESC {
        let mut d = RACE_DESC::EMPTY;
        d.ship_info.ship_flags =
            (FIRES_FORE as i32 | SEEKING_SPECIAL as i32 | IMMEDIATE_WEAPON as i32) as _;
        d.ship_info.ship_cost = 12;
        d.ship_info.crew_level = 20;
        d.ship_info.max_crew = 20;
        d.ship_info.energy_level = 40;
        d.ship_info.max_energy = 40;
        d.ship_info.race_strings_rsc = c"ship.vux.text".as_ptr();
        d.ship_info.icons_rsc = c"ship.vux.icons".as_ptr();
        d.ship_info.melee_icon_rsc = c"ship.vux.meleeicons".as_ptr();
        d.fleet.strength = (900 / SPHERE_RADIUS_INCREMENT as i32 * 2) as u16;
        d.fleet.known_loc = POINT { x: 4412, y: 1558 };
        d.characteristics.max_thrust = 21;
        d.characteristics.thrust_increment = 7;
        d.characteristics.energy_regeneration = 1;
        d.characteristics.weapon_energy_cost = 1;
        d.characteristics.special_energy_cost = 2;
        d.characteristics.energy_wait = 8;
        d.characteristics.turn_wait = 6;
        d.characteristics.thrust_wait = 4;
        d.characteristics.weapon_wait = 0;
        d.characteristics.special_wait = 7;
        d.characteristics.ship_mass = 6;
        d.ship_data.ship_rsc = [
            c"ship.vux.graphics.intruder.large".as_ptr(),
            c"ship.vux.graphics.intruder.medium".as_ptr(),
            c"ship.vux.graphics.intruder.small".as_ptr(),
        ];
        d.ship_data.weapon_rsc = [
            c"ship.vux.graphics.slime".as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
        ];
        d.ship_data.special_rsc = [
            c"ship.vux.graphics.limpets.large".as_ptr(),
            c"ship.vux.graphics.limpets.medium".as_ptr(),
            c"ship.vux.graphics.limpets.small".as_ptr(),
        ];
        d.ship_data.captain_control.captain_rsc = c"ship.vux.graphics.captain".as_ptr();
        d.ship_data.victory_ditty_rsc = c"ship.vux.ditty".as_ptr();
        d.ship_data.ship_sounds_rsc = c"ship.vux.sounds".as_ptr();
        d.cyborg_control.WeaponRange = (50 << ONE_SHIFT) as u16;
        d.race = Some(&VUX);
        d.race_state = RaceState::Vux { entry_done: false };
        d
    }

    pub static VUX: Race = Race {
        descriptor,
        preprocess: Some(vux_preprocess),
        postprocess: Some(vux_postprocess),
        init_weapon: initialize_horrific_laser,
        intelligence: vux_intelligence,
        uninit: None,
    };

    pub(crate) fn limpet_preprocess(cx: &mut Cx<'_>, e: ElementId) {
        let original = normalize_facing(angle_to_facing(i32::from(
            cx.b.elements[e].velocity.travel_angle,
        )));
        let mut face = original;
        let delta = cx.track_ship(e, &mut face);
        if delta > 0 {
            face = (i32::from(original) + i32::from(delta)) as COUNT;
            cx.b.elements[e].velocity.set_vector(25, face);
        }
        let frame = cx.inc_frame(cx.b.elements[e].next.image.frame);
        image!(frame; cx.b.elements[e].next.image.frame, frame);
        cx.b.elements[e].state_flags |= CHANGING as u16;
    }

    pub(crate) fn limpet_collision(
        cx: &mut Cx<'_>,
        a: ElementId,
        _pa: &mut POINT,
        b: ElementId,
        _pb: &mut POINT,
    ) {
        if cx.b.elements[b].state_flags & PLAYER_SHIP as u16 != 0 {
            let other = cx.owner(b);
            let d = cx.desc_mut(other);
            d.characteristics.turn_wait = d.characteristics.turn_wait.wrapping_add(1);
            if d.characteristics.turn_wait == 0 {
                d.characteristics.turn_wait = d.characteristics.turn_wait.wrapping_sub(1);
            }
            d.characteristics.thrust_wait = d.characteristics.thrust_wait.wrapping_add(1);
            if d.characteristics.thrust_wait == 0 {
                d.characteristics.thrust_wait = d.characteristics.thrust_wait.wrapping_sub(1);
            }
            if d.characteristics.thrust_increment <= 1 << ONE_SHIFT {
                d.characteristics.max_thrust = d.characteristics.thrust_increment.wrapping_shl(1);
            } else {
                let n = d.characteristics.max_thrust / d.characteristics.thrust_increment;
                d.characteristics.thrust_increment -= 1;
                d.characteristics.max_thrust = d.characteristics.thrust_increment.wrapping_mul(n);
            }
            d.cyborg_control.ManeuverabilityIndex = 0;
            let ss = cx.owner(a);
            cx.process_ship_sound(ss, 2, b);
            let index = (cx.rng)() as COUNT;
            let frame = cx.abs_frame(cx.desc(ss).ship_data.weapon[0], index);
            cx.silhouette(b, frame, MODIFY_IMAGE as BYTE);
        }
        cx.b.elements[a].set_hit_points(0);
        cx.b.elements[a].set_life_span(0);
        cx.b.elements[a].state_flags |= (COLLISION | DISAPPEARING) as u16;
    }

    pub(crate) fn spawn_limpets(cx: &mut Cx<'_>, e: ElementId) {
        let ss = cx.owner(e);
        let mut block = MISSILE_BLOCK {
            farray: cx.ship_frames(ss, 2),
            // The original expression adds HALF_CIRCLE directly to the facing.
            face: (i32::from(cx.ship(ss).ShipFacing) + HALF_CIRCLE as i32) as COUNT,
            index: 0,
            sender: cx.b.elements[e].playerNr,
            flags: IGNORE_SIMILAR as u16,
            pixoffs: 8,
            speed: 25,
            hit_points: 1,
            damage: 0,
            life: 80,
            preprocess_func: Some(ElementHook::Native(limpet_preprocess)),
            blast_offs: 0,
            cx: cx.b.elements[e].next.location.x,
            cy: cx.b.elements[e].next.location.y,
        };
        if let Some(limpet) = cx.missile(&mut block) {
            cx.b.elements[limpet].collision_func = Some(CollisionHook::Native(limpet_collision));
            cx.set_parent(limpet, Some(ss));
            cx.put(limpet);
        }
    }

    pub(crate) fn initialize_horrific_laser(
        cx: &mut Cx<'_>,
        ss: ShipId,
        origin: WeaponOrigin,
    ) -> WeaponSpawn {
        let face = cx.ship(ss).ShipFacing;
        let mut block = LASER_BLOCK {
            face,
            cx: origin.location.x,
            cy: origin.location.y,
            ex: cosine(facing_to_angle(i32::from(face)), 162 << ONE_SHIFT),
            ey: sine(facing_to_angle(i32::from(face)), 162 << ONE_SHIFT),
            sender: origin.player,
            flags: IGNORE_SIMILAR as u16,
            pixoffs: 12,
            color: Color {
                r: 82,
                g: 255,
                b: 82,
                a: 255,
            },
        };
        vec![cx.laser(&mut block)]
    }

    pub(crate) fn vux_intelligence(cx: &mut Cx<'_>, ss: ShipId, concerns: &mut Concerns) {
        let enemy = ENEMY_SHIP_INDEX as usize;
        let weapon = ENEMY_WEAPON_INDEX as usize;
        concerns.objects[enemy].movement = MOVEMENT_STATE_PURSUE;
        if let Some(threat) = concerns.objects[weapon].object {
            if concerns.objects[weapon].movement == MOVEMENT_STATE_ENTICE {
                let flags = cx.b.elements[threat].state_flags;
                concerns.objects[weapon].movement =
                    if flags & FINITE_LIFE as u16 != 0 && flags & CREW_OBJECT as u16 == 0 {
                        MOVEMENT_STATE_AVOID
                    } else {
                        MOVEMENT_STATE_PURSUE
                    };
            }
        }
        cx.ship_intelligence(ss, concerns);
        if cx.ship(ss).special_counter == 0
            && concerns.objects[enemy].object.is_some()
            && concerns.objects[enemy].which_turn <= 12
            && cx.ship(ss).ship_input_state & (LEFT | RIGHT) as u8 != 0
            && cx.desc(ss).ship_info.energy_level >= cx.desc(ss).ship_info.max_energy >> 1
        {
            cx.ship_mut(ss).ship_input_state |= SPECIAL as u8;
        } else {
            cx.ship_mut(ss).ship_input_state &= !(SPECIAL as u8);
        }
    }

    pub(crate) fn vux_postprocess(cx: &mut Cx<'_>, ss: ShipId) {
        let e = cx.ship_element(ss);
        if cx.ship(ss).cur_status_flags & SPECIAL as u16 != 0
            && cx.ship(ss).special_counter == 0
            && cx.delta_energy(e, -2)
        {
            cx.process_ship_sound(ss, 1, e);
            spawn_limpets(cx, e);
            cx.ship_mut(ss).special_counter = cx.desc(ss).characteristics.special_wait;
        }
    }

    pub(crate) fn vux_preprocess(cx: &mut Cx<'_>, ss: ShipId) {
        let e = cx.ship_element(ss);
        if cx.b.elements[e].state_flags & APPEARING as u16 != 0 {
            let mut face = cx.ship(ss).ShipFacing;
            if cx.world.activity() as u8 != IN_ENCOUNTER as u8 && cx.track_ship(e, &mut face) >= 0 {
                let other = cx.target(e).expect("TrackShip found no target");
                const ENTRY: i32 = ((150 + 12 + 46) * 2) << ONE_SHIFT;
                loop {
                    cx.b.elements[e].current.location.x =
                        (i32::from(cx.b.elements[other].current.location.x) - (ENTRY >> 1)
                            + i32::from((cx.rng)() as COUNT) % ENTRY)
                            as COORD;
                    cx.b.elements[e].current.location.y =
                        (i32::from(cx.b.elements[other].current.location.y) - (ENTRY >> 1)
                            + i32::from((cx.rng)() as COUNT) % ENTRY)
                            as COORD;
                    let dx = cx.b.elements[other]
                        .current
                        .location
                        .x
                        .wrapping_sub(cx.b.elements[e].current.location.x);
                    let dy = cx.b.elements[other]
                        .current
                        .location
                        .y
                        .wrapping_sub(cx.b.elements[e].current.location.y);
                    face =
                        normalize_facing(angle_to_facing(i32::from(crate::math::arctan(dx, dy))));
                    let frame = cx.abs_frame(cx.b.elements[e].current.image.frame, face);
                    image!(frame; cx.b.elements[e].current.image.frame, frame);
                    let x =
                        i32::from(cx.b.elements[e].current.location.x) & !((1 << ONE_SHIFT) - 1);
                    let y =
                        i32::from(cx.b.elements[e].current.location.y) & !((1 << ONE_SHIFT) - 1);
                    let (pixel_width, pixel_height) = cx.screen_dimensions();
                    let width = (pixel_width - 64) << (ONE_SHIFT + 3);
                    let height = pixel_height << (ONE_SHIFT + 3);
                    cx.b.elements[e].current.location.x = (if x < 0 {
                        x + width
                    } else if x >= width {
                        x - width
                    } else {
                        x
                    }) as COUNT as COORD;
                    cx.b.elements[e].current.location.y = (if y < 0 {
                        y + height
                    } else if y >= height {
                        y - height
                    } else {
                        y
                    }) as COUNT as COORD;
                    if !cx.gravity(e) && !cx.matter_conflict(e) {
                        break;
                    }
                }
                cx.set_target(e, None);
                let current = cx.b.elements[e].current;
                image!(copy; cx.b.elements[e].next, current);
                cx.init_intersect(e);
                let frame = cx.equ_frame(cx.frame_at(e, true), cx.b.elements[e].next.image.frame);
                image!(frame; cx.b.elements[e].IntersectControl.IntersectStamp.frame, frame);
                cx.ship_mut(ss).ShipFacing = face;
            }
            cx.desc_mut(ss).race_state = RaceState::Vux { entry_done: true };
        }
    }
}
