//! Earthling Cruiser callbacks used by the live battle engine.

pub(super) const MAX_THRUST: u16 = 24;
pub(super) const MISSILE_LIFE: u16 = 60;

fn nuke_speed(life: u16) -> i16 {
    let initial = i32::from(MAX_THRUST).max(10 << 2);
    (initial + (i32::from(MISSILE_LIFE) - i32::from(life)) * 4).min(20 << 2) as i16
}

fn in_defense_range(dx: i32, dy: i32) -> bool {
    let x = dx.abs() >> 2;
    let y = dy.abs() >> 2;
    x <= 100 && y <= 100 && x * x + y * y <= 100 * 100
}

fn use_defense(cooldown: u8, weapon_turn: Option<u16>, enemy_turn: Option<u16>) -> bool {
    cooldown == 0
        && (weapon_turn.is_some_and(|turn| turn <= 2) || enemy_turn.is_some_and(|turn| turn <= 4))
}

#[cfg(not(test))]
pub(crate) mod engine {
    use super::{in_defense_range, nuke_speed, use_defense, MISSILE_LIFE};
    use crate::engine::battle::{Concerns, Cx, ElementId, ShipId, WeaponOrigin, WeaponSpawn};
    use crate::ships::native::abi::*;

    const TRACK_WAIT: u8 = 3;
    const SPECIAL_ENERGY_COST: u8 = 4;

    fn descriptor(_world: &mut crate::engine::battle::World) -> crate::engine::battle::RaceDesc {
        let d = crate::engine::battle::template::RaceTemplate {
            info: crate::engine::battle::views::Info {
                ship_flags: (FIRES_FORE | SEEKING_WEAPON | POINT_DEFENSE) as u16,
                ship_cost: 11,
                crew_level: 18,
                max_crew: 18,
                energy_level: 18,
                max_energy: 18,
            },
            resources: crate::engine::battle::template::RaceResources {
                strings: Some(c"ship.earthling.text"),
                icons: Some(c"ship.earthling.icons"),
                melee_icon: Some(c"ship.earthling.meleeicons"),
                ship: [
                    Some(c"ship.earthling.graphics.human.large"),
                    Some(c"ship.earthling.graphics.human.medium"),
                    Some(c"ship.earthling.graphics.human.small"),
                ],
                weapon: [
                    Some(c"ship.earthling.graphics.saturn.large"),
                    Some(c"ship.earthling.graphics.saturn.medium"),
                    Some(c"ship.earthling.graphics.saturn.small"),
                ],
                captain: Some(c"ship.earthling.graphics.captain"),
                victory: Some(c"ship.earthling.ditty"),
                sounds: Some(c"ship.earthling.sounds"),
                ..Default::default()
            },
            fleet_location: POINT { x: 1752, y: 1450 },
            characteristics: CHARACTERISTIC_STUFF {
                max_thrust: 24,
                thrust_increment: 3,
                energy_regeneration: 1,
                weapon_energy_cost: 9,
                special_energy_cost: 4,
                energy_wait: 8,
                turn_wait: 1,
                thrust_wait: 4,
                weapon_wait: 10,
                special_wait: 9,
                ship_mass: 6,
            },
            intelligence: crate::engine::battle::views::Intelligence {
                weapon_range: 1000 << ONE_SHIFT,
                ..Default::default()
            },
            ..Default::default()
        };
        d.descriptor(&HUMAN)
    }

    pub static HUMAN: crate::engine::battle::Race = crate::engine::battle::Race {
        descriptor,
        preprocess: None,
        postprocess: Some(human_postprocess),
        init_weapon: Some(initialize_nuke),
        intelligence: Some(human_intelligence),
        uninit: None,
    };

    pub(crate) fn nuke_preprocess(cx: &mut Cx<'_>, missile: ElementId) {
        let mut facing = cx.frame_number(cx.element(missile).next.image.frame);
        if cx.element(missile).turn_wait() > 0 {
            let wait = cx.element(missile).turn_wait() - 1;
            cx.element_mut(missile).set_turn_wait(wait);
        } else {
            if cx.track_ship(missile, &mut facing) > 0 {
                let frame = cx.absolute_frame(cx.element(missile).next.image.frame, facing);
                cx.set_frame(missile, true, frame);
                cx.element_mut(missile).flags |= CHANGING as u16;
            }
            cx.element_mut(missile).set_turn_wait(TRACK_WAIT);
        }
        let speed = nuke_speed(cx.element(missile).life_span());
        cx.element_mut(missile)
            .velocity
            .set_vector(i32::from(speed), facing);
    }

    pub(crate) fn initialize_nuke(
        cx: &mut Cx<'_>,
        ss: ShipId,
        origin: WeaponOrigin,
    ) -> WeaponSpawn {
        let facing = cx.ship_state(ss).facing;
        let mut block = crate::engine::battle::views::Missile {
            cx: origin.next.location.x,
            cy: origin.next.location.y,
            flags: 0,
            sender: origin.player,
            pixoffs: 42,
            speed: nuke_speed(MISSILE_LIFE),
            hit_points: 1,
            damage: 4,
            face: facing,
            index: facing,
            life: MISSILE_LIFE,
            farray: cx.ship_frame_set(ss, 1),
            preprocess: Some(nuke_preprocess),
            blast_offs: 8,
        };
        let nuke = cx.missile(&mut block);
        if let Some(nuke) = nuke {
            cx.element_mut(nuke).set_turn_wait(TRACK_WAIT);
        }
        vec![nuke]
    }

    pub(crate) fn spawn_point_defense(cx: &mut Cx<'_>, source: ElementId) {
        let ss = cx.owner(source);
        if cx.element(source).flags & PLAYER_SHIP as u16 != 0 {
            if let Some(defense) = cx.alloc_element() {
                cx.element_mut(defense).player = cx.element(source).player;
                cx.element_mut(defense).flags = (APPEARING | NONSOLID | FINITE_LIFE) as u16;
                cx.element_mut(defense).death = Some(ElementHook::Native(spawn_point_defense));
                cx.set_parent(defense, Some(ss));
                cx.put(defense);
            }
            return;
        }

        let ship = cx.ship_element(ss);
        let mut paid = false;
        let mut handle = cx.elements().tail();
        while let Some(object) = handle {
            // Lasers append to the queue, so save the predecessor first.
            handle = cx.elements().pred(object);
            if object == ship
                || cx.element(object).flags & (NONSOLID | DISAPPEARING) as u16 != 0
                || cx.object_cloaked(object)
            {
                continue;
            }
            let dx = i32::from(cx.element(object).next.location.x)
                - i32::from(cx.element(ship).next.location.x);
            let dy = i32::from(cx.element(object).next.location.y)
                - i32::from(cx.element(ship).next.location.y);
            if !in_defense_range(dx, dy) {
                continue;
            }
            if !paid {
                if !cx.delta_energy(ship, -i16::from(SPECIAL_ENERGY_COST)) {
                    break;
                }
                cx.process_ship_sound(ss, 1, source);
                cx.ship_state_mut(ss).special_counter =
                    cx.race_stats(ss).characteristics.special_wait;
                paid = true;
            }
            let laser = crate::engine::battle::views::Laser {
                origin: cx.element(ship).next.location,
                end: POINT {
                    x: dx as i16,
                    y: dy as i16,
                },
                flags: IGNORE_SIMILAR as u16,
                sender: cx.element(ship).player,
                pixoffs: 0,
                face: 0,
                color: Color {
                    r: 255,
                    g: 255,
                    b: 255,
                    a: 255,
                },
            };
            if let Some(beam) = cx.spawn_laser(laser) {
                cx.set_parent(beam, Some(ss));
                cx.set_target(beam, None);
                cx.put(beam);
            }
        }
    }

    pub(crate) fn human_postprocess(cx: &mut Cx<'_>, ss: ShipId) {
        if cx.ship_state(ss).status & SPECIAL as u16 != 0 && cx.ship_state(ss).special_counter == 0
        {
            spawn_point_defense(cx, cx.ship_element(ss));
        }
    }

    pub(crate) fn human_intelligence(cx: &mut Cx<'_>, ss: ShipId, concerns: &mut Concerns) {
        let enemy = ENEMY_SHIP_INDEX as usize;
        let weapon = ENEMY_WEAPON_INDEX as usize;
        let enemy_turn = concerns.objects[enemy]
            .object
            .map(|_| concerns.objects[enemy].which_turn);
        let weapon_turn = concerns.objects[weapon]
            .object
            .map(|_| concerns.objects[weapon].which_turn);
        if use_defense(cx.ship_state(ss).special_counter, weapon_turn, enemy_turn) {
            cx.ship_state_mut(ss).input |= SPECIAL as u8;
        } else {
            cx.ship_state_mut(ss).input &= !(SPECIAL as u8);
        }
        concerns.objects[weapon].object = None;
        cx.ship_intelligence(ss, concerns);
        if cx.ship_state(ss).weapon_counter == 0
            && concerns.objects[enemy].object.is_some()
            && (cx.ship_state(ss).input & (LEFT | RIGHT) as u8 == 0
                || concerns.objects[enemy].which_turn <= 12)
        {
            cx.ship_state_mut(ss).input |= WEAPON as u8;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nuke_accelerates_and_caps_at_twenty_display_units() {
        assert_eq!(nuke_speed(60), 40);
        assert_eq!(nuke_speed(59), 44);
        assert_eq!(nuke_speed(50), 80);
        assert_eq!(nuke_speed(1), 80);
    }

    #[test]
    fn defense_range_is_a_circle_in_display_coordinates() {
        assert!(in_defense_range(400, 0));
        assert!(in_defense_range(-240, 320));
        assert!(in_defense_range(0, -403));
        assert!(!in_defense_range(404, 0));
        assert!(!in_defense_range(400, 400));
    }

    #[test]
    fn defense_requires_a_nearby_threat_and_no_cooldown() {
        assert!(use_defense(0, Some(2), None));
        assert!(use_defense(0, None, Some(4)));
        assert!(!use_defense(1, Some(1), Some(1)));
        assert!(!use_defense(0, Some(3), Some(5)));
        assert!(!use_defense(0, None, None));
    }
}
