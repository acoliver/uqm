//! Earthling Cruiser callbacks used by the live battle engine.

#[cfg(not(test))]
pub(super) const MAX_CREW: u16 = 18;
#[cfg(not(test))]
pub(super) const MAX_ENERGY: u8 = 18;
#[cfg(not(test))]
pub(super) const ENERGY_REGENERATION: u8 = 1;
#[cfg(not(test))]
pub(super) const ENERGY_WAIT: u8 = 8;
pub(super) const MAX_THRUST: u16 = 24;
#[cfg(not(test))]
pub(super) const THRUST_INCREMENT: u16 = 3;
#[cfg(not(test))]
pub(super) const THRUST_WAIT: u8 = 4;
#[cfg(not(test))]
pub(super) const TURN_WAIT: u8 = 1;
#[cfg(not(test))]
pub(super) const SHIP_MASS: u8 = 6;

#[cfg(not(test))]
pub(super) const WEAPON_ENERGY_COST: u8 = 9;
#[cfg(not(test))]
pub(super) const WEAPON_WAIT: u8 = 10;
#[cfg(not(test))]
pub(super) const HUMAN_OFFSET: i16 = 42;
#[cfg(not(test))]
pub(super) const NUKE_OFFSET: i16 = 8;
pub(super) const MISSILE_LIFE: u16 = 60;
#[cfg(not(test))]
pub(super) const MISSILE_HITS: i16 = 1;
#[cfg(not(test))]
pub(super) const MISSILE_DAMAGE: i16 = 4;

#[cfg(not(test))]
pub(super) const SPECIAL_ENERGY_COST: u8 = 4;
#[cfg(not(test))]
pub(super) const SPECIAL_WAIT: u8 = 9;

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
    use super::*;
    use super::{in_defense_range, nuke_speed, use_defense};
    use crate::ships::native::{
        abi::*, element, frame_index, object_cloaked, put_element, starship,
    };
    use std::ptr;

    const TRACK_WAIT: u8 = 3;

    const fn descriptor() -> RACE_DESC {
        // All zero fields are null handles, absent callbacks, or zero counters.
        let mut desc = RACE_DESC::EMPTY;
        desc.ship_info.ship_flags = (FIRES_FORE | SEEKING_WEAPON | POINT_DEFENSE) as u16;
        desc.ship_info.ship_cost = 11;
        desc.ship_info.crew_level = MAX_CREW;
        desc.ship_info.max_crew = MAX_CREW;
        desc.ship_info.energy_level = MAX_ENERGY;
        desc.ship_info.max_energy = MAX_ENERGY;
        desc.ship_info.race_strings_rsc = c"ship.earthling.text".as_ptr();
        desc.ship_info.icons_rsc = c"ship.earthling.icons".as_ptr();
        desc.ship_info.melee_icon_rsc = c"ship.earthling.meleeicons".as_ptr();
        desc.fleet.known_loc = POINT { x: 1752, y: 1450 };
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
        desc.ship_data.ship_rsc = [
            c"ship.earthling.graphics.human.large".as_ptr(),
            c"ship.earthling.graphics.human.medium".as_ptr(),
            c"ship.earthling.graphics.human.small".as_ptr(),
        ];
        desc.ship_data.weapon_rsc = [
            c"ship.earthling.graphics.saturn.large".as_ptr(),
            c"ship.earthling.graphics.saturn.medium".as_ptr(),
            c"ship.earthling.graphics.saturn.small".as_ptr(),
        ];
        desc.ship_data.captain_control.captain_rsc = c"ship.earthling.graphics.captain".as_ptr();
        desc.ship_data.victory_ditty_rsc = c"ship.earthling.ditty".as_ptr();
        desc.ship_data.ship_sounds_rsc = c"ship.earthling.sounds".as_ptr();
        desc.cyborg_control.WeaponRange = 1000 << ONE_SHIFT;
        desc.cyborg_control.intelligence_func = Some(human_intelligence);
        desc.postprocess_func = Some(human_postprocess);
        desc.init_weapon_func = Some(initialize_nuke);
        desc
    }

    static mut HUMAN_DESC: RACE_DESC = descriptor();

    /// Returns the static template copied by CaptureCodeRes for each ship.
    #[no_mangle]
    pub extern "C" fn init_human() -> *mut RACE_DESC {
        &raw mut HUMAN_DESC
    }

    pub(crate) unsafe extern "C" fn nuke_preprocess(missile: *mut ELEMENT) {
        let mut facing = frame_index((*missile).next.image.frame);
        if (*missile).__bindgen_anon_4.turn_wait > 0 {
            (*missile).__bindgen_anon_4.turn_wait -= 1;
        } else {
            if TrackShip(missile, &mut facing) > 0 {
                (*missile).next.image.frame = SetAbsFrameIndex((*missile).next.image.frame, facing);
                (*missile).state_flags |= CHANGING as u16;
            }
            (*missile).__bindgen_anon_4.turn_wait = TRACK_WAIT;
        }
        SetVelocityVector(
            &raw mut (*missile).velocity,
            nuke_speed((*missile).__bindgen_anon_1.life_span),
            facing,
        );
    }

    pub(crate) unsafe extern "C" fn initialize_nuke(
        ship: *mut ELEMENT,
        weapons: *mut HELEMENT,
    ) -> COUNT {
        let ss = starship(ship);
        let mut block = MISSILE_BLOCK {
            cx: (*ship).next.location.x,
            cy: (*ship).next.location.y,
            flags: 0,
            sender: (*ship).playerNr,
            pixoffs: HUMAN_OFFSET,
            speed: nuke_speed(MISSILE_LIFE),
            hit_points: MISSILE_HITS,
            damage: MISSILE_DAMAGE,
            face: (*ss).ShipFacing,
            index: (*ss).ShipFacing,
            life: MISSILE_LIFE,
            farray: (*(*ss).RaceDescPtr).ship_data.weapon.as_mut_ptr(),
            preprocess_func: Some(nuke_preprocess),
            blast_offs: NUKE_OFFSET,
        };
        let nuke = initialize_missile(&mut block);
        *weapons = nuke;
        if !nuke.is_null() {
            (*element(nuke)).__bindgen_anon_4.turn_wait = TRACK_WAIT;
        }
        1
    }

    pub(crate) unsafe extern "C" fn spawn_point_defense(source: *mut ELEMENT) {
        let ss = starship(source);
        if (*source).state_flags & PLAYER_SHIP as u16 != 0 {
            let defense = AllocElement();
            if !defense.is_null() {
                let e = element(defense);
                (*e).playerNr = (*source).playerNr;
                (*e).state_flags = (APPEARING | NONSOLID | FINITE_LIFE) as u16;
                (*e).death_func = Some(spawn_point_defense);
                (*e).pParent = ss.cast();
                put_element(defense);
            }
            return;
        }

        let ship = element((*ss).hShip);
        let mut paid = false;
        let mut handle = disp_q.tail;
        while !handle.is_null() {
            let object = element(handle);
            // Lasers append to the queue, so save the predecessor first.
            handle = (*object).pred;
            if object == ship
                || (*object).state_flags & (NONSOLID | DISAPPEARING) as u16 != 0
                || object_cloaked(object)
            {
                continue;
            }
            let dx = i32::from((*object).next.location.x) - i32::from((*ship).next.location.x);
            let dy = i32::from((*object).next.location.y) - i32::from((*ship).next.location.y);
            if !in_defense_range(dx, dy) {
                continue;
            }
            if !paid {
                if DeltaEnergy(ship, -i16::from(SPECIAL_ENERGY_COST)) == 0 {
                    break;
                }
                ProcessSound(
                    SetAbsStringTableIndex((*(*ss).RaceDescPtr).ship_data.ship_sounds, 1),
                    source,
                );
                (*ss).special_counter = (*(*ss).RaceDescPtr).characteristics.special_wait;
                paid = true;
            }
            let mut laser = LASER_BLOCK {
                cx: (*ship).next.location.x,
                cy: (*ship).next.location.y,
                ex: dx as i16,
                ey: dy as i16,
                flags: IGNORE_SIMILAR as u16,
                sender: (*ship).playerNr,
                pixoffs: 0,
                face: 0,
                color: Color {
                    r: 255,
                    g: 255,
                    b: 255,
                    a: 255,
                },
            };
            let beam = initialize_laser(&mut laser);
            if !beam.is_null() {
                (*element(beam)).pParent = ss.cast();
                (*element(beam)).hTarget = ptr::null_mut();
                put_element(beam);
            }
        }
    }

    pub(crate) unsafe extern "C" fn human_postprocess(ship: *mut ELEMENT) {
        let ss = starship(ship);
        if (*ss).cur_status_flags & SPECIAL as u16 != 0 && (*ss).special_counter == 0 {
            spawn_point_defense(ship);
        }
    }

    pub(crate) unsafe extern "C" fn human_intelligence(
        ship: *mut ELEMENT,
        concerns: *mut EVALUATE_DESC,
        count: COUNT,
    ) {
        let ss = starship(ship);
        let enemy = concerns.add(ENEMY_SHIP_INDEX as usize);
        let weapon = concerns.add(ENEMY_WEAPON_INDEX as usize);
        let enemy_turn = (!(*enemy).ObjectPtr.is_null()).then_some((*enemy).which_turn);
        let weapon_turn = (!(*weapon).ObjectPtr.is_null()).then_some((*weapon).which_turn);
        if use_defense((*ss).special_counter, weapon_turn, enemy_turn) {
            (*ss).ship_input_state |= SPECIAL as u8;
        } else {
            (*ss).ship_input_state &= !(SPECIAL as u8);
        }
        (*weapon).ObjectPtr = ptr::null_mut();
        ship_intelligence(ship, concerns, count);
        if (*ss).weapon_counter == 0
            && !(*enemy).ObjectPtr.is_null()
            && ((*ss).ship_input_state & (LEFT | RIGHT) as u8 == 0 || (*enemy).which_turn <= 12)
        {
            (*ss).ship_input_state |= WEAPON as u8;
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
