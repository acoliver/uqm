//! Campaign events and daily fleet movement, ported from gameev.c.

use super::abi::*;

fn grow_fleet(fleet: &mut FLEET_INFO) -> Option<u8> {
    if fleet.actual_strength == 0 || fleet.actual_strength == u16::MAX {
        return None;
    }
    let mut delta = fleet.growth as i8 as i16;
    if fleet.growth_err_term <= fleet.growth_fract {
        if delta <= 0 {
            delta -= 1;
        } else {
            delta += 1;
        }
    }
    fleet.growth_err_term = fleet.growth_err_term.wrapping_sub(fleet.growth_fract);
    delta = delta.wrapping_add(fleet.actual_strength as i16);
    if delta <= 0 {
        delta = 0;
        fleet.allied_state = DEAD_GUY as u16;
    } else if delta > MAX_FLEET_STRENGTH as i16 {
        delta = MAX_FLEET_STRENGTH as i16;
    }
    fleet.actual_strength = delta as u16;
    if fleet.actual_strength != 0 && fleet.days_left != 0 {
        fleet.loc.x = (fleet.loc.x as i32
            + (fleet.dest_loc.x as i32 - fleet.loc.x as i32) / fleet.days_left as i32)
            as i16;
        fleet.loc.y = (fleet.loc.y as i32
            + (fleet.dest_loc.y as i32 - fleet.loc.y as i32) / fleet.days_left as i32)
            as i16;
        fleet.days_left -= 1;
        if fleet.days_left == 0 && fleet.func_index != u8::MAX {
            return Some(fleet.func_index);
        }
    }
    None
}

#[cfg(not(test))]
#[allow(non_snake_case)]
mod live {
    use super::*;
    use crate::navigation::globals::{inHyperSpace, GlobData};
    use crate::navigation::{get, set};
    use crate::planets::starmap::CurStarDescPtr;
    use std::ptr;

    unsafe fn fleet(race: u32) -> *mut FLEET_INFO {
        GetStarShipFromIndex(
            ptr::addr_of_mut!(GlobData.Game_state.avail_race_q),
            race as u16,
        )
        .cast()
    }
    unsafe fn relative(month: u16, day: u16, year: u16, selector: u32) {
        AddEvent(EVENT_TYPE_RELATIVE_EVENT, month, day, year, selector as u8);
    }
    unsafe fn at_star(index: u32) -> bool {
        GlobData.Game_state.CurrentActivity as u8 as u32 == IN_INTERPLANETARY
            && !CurStarDescPtr.is_null()
            && (*CurStarDescPtr).index as u32 == index
    }

    /// # Safety
    /// Game-thread access to initialized game-clock and fleet queues.
    #[no_mangle]
    pub unsafe extern "C" fn AddInitialGameEvents() {
        relative(0, 1, 0, HYPERSPACE_ENCOUNTER_EVENT);
        AddEvent(
            EVENT_TYPE_ABSOLUTE_EVENT,
            3,
            17,
            START_YEAR as u16,
            ARILOU_ENTRANCE_EVENT as u8,
        );
        relative(
            0,
            0,
            YEARS_TO_KOHRAH_VICTORY as u16,
            KOHR_AH_VICTORIOUS_EVENT,
        );
        relative(0, 0, 0, SLYLANDRO_RAMP_UP);
    }

    /// # Safety
    /// Game-thread access to initialized game-clock and fleet queues.
    #[no_mangle]
    pub unsafe extern "C" fn EventHandler(selector: u8) {
        match selector as u32 {
            ARILOU_ENTRANCE_EVENT => {
                set("ARILOU_SPACE", ARILOU_GATE_STATE_OPENING as u8);
                relative(0, 3, 0, ARILOU_EXIT_EVENT);
            }
            ARILOU_EXIT_EVENT => {
                let mut year = GlobData.Game_state.GameClock.year_index;
                let month = GlobData.Game_state.GameClock.month_index as u16 % 12;
                if month == 0 {
                    year = year.wrapping_add(1);
                }
                set("ARILOU_SPACE", ARILOU_GATE_STATE_CLOSING as u8);
                AddEvent(
                    EVENT_TYPE_ABSOLUTE_EVENT,
                    month + 1,
                    17,
                    year,
                    ARILOU_ENTRANCE_EVENT as u8,
                );
            }
            HYPERSPACE_ENCOUNTER_EVENT => {
                let mut handle = GlobData.Game_state.avail_race_q.head;
                while !handle.is_null() {
                    let current = handle.cast::<FLEET_INFO>();
                    handle = (*current).succ;
                    if let Some(callback) = grow_fleet(&mut *current) {
                        EventHandler(callback);
                    }
                }
                if inHyperSpace() != 0 {
                    check_hyperspace_encounter();
                }
                relative(0, 1, 0, HYPERSPACE_ENCOUNTER_EVENT);
            }
            KOHR_AH_VICTORIOUS_EVENT if get("UTWIG_SUPOX_MISSION") != 0 => {
                relative(0, 0, 1, KOHR_AH_GENOCIDE_EVENT)
            }
            KOHR_AH_VICTORIOUS_EVENT | KOHR_AH_GENOCIDE_EVENT => {
                if get("KOHR_AH_FRENZY") == 0 && at_star(SAMATRA_DEFINED) {
                    relative(0, 7, 0, KOHR_AH_GENOCIDE_EVENT);
                } else {
                    genocide();
                }
            }
            ADVANCE_PKUNK_MISSION => pkunk(),
            ADVANCE_THRADD_MISSION => thradd(),
            ZOQFOT_DISTRESS_EVENT => {
                if at_star(ZOQFOT_DEFINED) {
                    relative(0, 7, 0, ZOQFOT_DISTRESS_EVENT);
                } else {
                    set("ZOQFOT_DISTRESS", 1);
                    relative(6, 0, 0, ZOQFOT_DEATH_EVENT);
                }
            }
            ZOQFOT_DEATH_EVENT => {
                if at_star(ZOQFOT_DEFINED) {
                    relative(0, 7, 0, ZOQFOT_DEATH_EVENT);
                } else if get("ZOQFOT_DISTRESS") != 0 {
                    let race = fleet(ZOQFOTPIK_SHIP);
                    (*race).actual_strength = 0;
                    (*race).allied_state = DEAD_GUY as u16;
                    set("ZOQFOT_DISTRESS", 2);
                }
            }
            SHOFIXTI_RETURN_EVENT => {
                SetRaceAllied(SHOFIXTI_SHIP as u16, 1);
                GlobData.Game_state.CrewCost = GlobData.Game_state.CrewCost.wrapping_sub(2);
                set("CREW_PURCHASED0", 0);
                set("CREW_PURCHASED1", 0);
            }
            ADVANCE_UTWIG_SUPOX_MISSION => utwig_supox(),
            SPATHI_SHIELD_EVENT => {
                if at_star(SPATHI_DEFINED) {
                    relative(0, 7, 0, SPATHI_SHIELD_EVENT);
                } else {
                    let race = fleet(SPATHI_SHIP);
                    if (*race).actual_strength != 0 {
                        SetRaceAllied(SPATHI_SHIP as u16, 0);
                        set("SPATHI_SHIELDED_SELVES", 1);
                        (*race).actual_strength = 0;
                    }
                }
            }
            ADVANCE_ILWRATH_MISSION => ilwrath(),
            ADVANCE_MYCON_MISSION => mycon(),
            ARILOU_UMGAH_CHECK => set("ARILOU_CHECKED_UMGAH", 2),
            YEHAT_REBEL_EVENT => {
                let rebel = fleet(YEHAT_REBEL_SHIP);
                let royal = fleet(YEHAT_SHIP);
                (*royal).actual_strength = ((*royal).actual_strength as u32 * 2 / 3) as u16;
                (*rebel).actual_strength = (*royal).actual_strength;
                (*rebel).loc = POINT { x: 5150, y: 0 };
                StartSphereTracking(YEHAT_REBEL_SHIP as u16);
            }
            SLYLANDRO_RAMP_UP => {
                if get("DESTRUCT_CODE_ON_SHIP") == 0 {
                    let factor = get("SLYLANDRO_MULTIPLIER").wrapping_add(1);
                    if factor <= 4 {
                        set("SLYLANDRO_MULTIPLIER", factor);
                        relative(0, 182, 0, SLYLANDRO_RAMP_UP);
                    }
                }
            }
            SLYLANDRO_RAMP_DOWN => {
                let factor = get("SLYLANDRO_MULTIPLIER").wrapping_sub(1);
                if factor != 0 {
                    relative(0, 23, 0, SLYLANDRO_RAMP_DOWN);
                }
                set("SLYLANDRO_MULTIPLIER", factor);
            }
            _ => {}
        }
    }

    /// # Safety
    /// `race` must identify an allocated fleet in the initialized world queue.
    #[no_mangle]
    pub unsafe extern "C" fn SetRaceDest(race: u8, x: i16, y: i16, days: u8, selector: u8) {
        let current = fleet(race as u32);
        (*current).dest_loc = POINT { x, y };
        (*current).days_left = days;
        (*current).func_index = selector;
    }

    unsafe fn genocide() {
        let black = fleet(BLACK_URQUAN_SHIP);
        let mut best_dist = -1i64;
        let mut best_dx = (SOL_X as i32 - (*black).loc.x as i32) as i16;
        let mut best_dy = (SOL_Y as i32 - (*black).loc.y as i32) as i16;
        let mut index = 0u8;
        let mut handle = GlobData.Game_state.avail_race_q.head;
        while !handle.is_null() {
            let current = handle.cast::<FLEET_INFO>();
            handle = (*current).succ;
            if index as u32 != BLACK_URQUAN_SHIP
                && index as u32 != URQUAN_SHIP
                && (*current).actual_strength != u16::MAX
            {
                let dx = (*current).loc.x.wrapping_sub((*black).loc.x);
                let dy = (*current).loc.y.wrapping_sub((*black).loc.y);
                if dx == 0 && dy == 0 {
                    (*current).allied_state = DEAD_GUY as u16;
                    (*current).actual_strength = 0;
                } else if (*current).actual_strength != 0 {
                    let dist = dx as i64 * dx as i64 + dy as i64 * dy as i64;
                    if best_dist < 0 || dist < best_dist || index as u32 == DRUUGE_SHIP {
                        best_dist = dist;
                        best_dx = dx;
                        best_dy = dy;
                        if index as u32 == DRUUGE_SHIP {
                            handle = ptr::null_mut();
                        }
                    }
                }
            }
            index = index.wrapping_add(1);
        }
        if best_dist < 0 && best_dx == 0 && best_dy == 0 {
            GlobData.Game_state.CurrentActivity &= !(IN_BATTLE as u16);
            GlobData.SIS_state.CrewEnlisted = u16::MAX;
            set("KOHR_AH_KILLED_ALL", 1);
        } else {
            if best_dist < 0 {
                best_dist = best_dx as i64 * best_dx as i64 + best_dy as i64 * best_dy as i64;
            }
            let speed = (square_root(best_dist as u32) / 158).clamp(1, 255) as u8;
            set("KOHR_AH_FRENZY", 1);
            for key in [
                "KOHR_AH_VISITS",
                "KOHR_AH_REASONS",
                "KOHR_AH_PLEAD",
                "KOHR_AH_INFO",
                "URQUAN_VISITS",
            ] {
                set(key, 0);
            }
            SetRaceDest(
                BLACK_URQUAN_SHIP as u8,
                (*black).loc.x.wrapping_add(best_dx),
                (*black).loc.y.wrapping_add(best_dy),
                speed,
                KOHR_AH_GENOCIDE_EVENT as u8,
            );
        }
    }

    unsafe fn pkunk() {
        let race = fleet(PKUNK_SHIP);
        if (*race).actual_strength == 0 {
            return;
        }
        let mission = get("PKUNK_MISSION");
        if (*race).days_left == 0 && mission != 0 {
            if mission & 1 != 0 || ((*race).loc.x == 4970 && (*race).loc.y == 400) {
                (*race).actual_strength = 0;
            } else if (*race).loc.x == 502 && (*race).loc.y == 401 && get("PKUNK_ON_THE_MOVE") != 0
            {
                set("PKUNK_ON_THE_MOVE", 0);
                relative(3, 0, 0, ADVANCE_PKUNK_MISSION);
                return;
            }
        }
        if (*race).actual_strength == 0 {
            set("YEHAT_ABSORBED_PKUNK", 1);
            (*race).allied_state = DEAD_GUY as u16;
            StartSphereTracking(YEHAT_SHIP as u16);
        } else {
            let (x, y) = if mission & 1 == 0 {
                (4970, 400)
            } else {
                (502, 401)
            };
            set("PKUNK_ON_THE_MOVE", 1);
            set("PKUNK_SWITCH", 0);
            SetRaceDest(
                PKUNK_SHIP as u8,
                x,
                y,
                (182u16.wrapping_sub((*race).days_left as u16)) as u8,
                ADVANCE_PKUNK_MISSION as u8,
            );
        }
        set("PKUNK_MISSION", mission.wrapping_add(1));
    }

    unsafe fn thradd() {
        let race = fleet(THRADDASH_SHIP);
        let mut mission = get("THRADD_MISSION");
        if (*race).actual_strength != 0 && mission < 3 {
            let (x, y) = if mission < 2 {
                (4879, 7201)
            } else {
                (2535, 8358)
            };
            if mission == 1 {
                let loss = ((*race).actual_strength >> 1) as i32;
                (*race).growth = (-loss / 14) as u8;
                (*race).growth_fract = (((loss % 14) << 8) / 14) as u8;
                (*race).growth_err_term = 127;
            } else if mission != 0 {
                (*race).growth = 0;
                (*race).growth_fract = 0;
            }
            SetRaceDest(THRADDASH_SHIP as u8, x, y, 14, ADVANCE_THRADD_MISSION as u8);
        }
        mission = mission.wrapping_add(1);
        set("THRADD_MISSION", mission);
        if mission == 4 && get("ILWRATH_FIGHT_THRADDASH") != 0 {
            relative(0, 0, 0, ADVANCE_ILWRATH_MISSION);
        }
    }

    unsafe fn ilwrath() {
        let ilwrath = fleet(ILWRATH_SHIP);
        let thradd = fleet(THRADDASH_SHIP);
        if (*ilwrath).loc.x == (2500 + 2535) / 2 && (*ilwrath).loc.y == (8070 + 8358) / 2 {
            (*ilwrath).actual_strength = 0;
            (*thradd).actual_strength = 0;
            (*ilwrath).allied_state = DEAD_GUY as u16;
            (*thradd).allied_state = DEAD_GUY as u16;
        } else if (*ilwrath).actual_strength != 0 {
            if get("ILWRATH_FIGHT_THRADDASH") == 0
                && ((*ilwrath).dest_loc.x != 2500 || (*ilwrath).dest_loc.y != 8070)
            {
                SetRaceDest(
                    ILWRATH_SHIP as u8,
                    2500,
                    8070,
                    90,
                    ADVANCE_ILWRATH_MISSION as u8,
                );
            } else {
                if (*ilwrath).days_left == 0 {
                    set("ILWRATH_FIGHT_THRADDASH", 1);
                    set("HELIX_UNPROTECTED", 1);
                    let loss = (*ilwrath).actual_strength as i32;
                    (*ilwrath).growth = (-loss / 128) as u8;
                    (*ilwrath).growth_fract = (((loss % 128) << 8) / 128) as u8;
                    SetRaceDest(
                        ILWRATH_SHIP as u8,
                        (2500 + 2535) / 2,
                        (8070 + 8358) / 2,
                        127,
                        ADVANCE_ILWRATH_MISSION as u8,
                    );
                    let loss = (*thradd).actual_strength as i32;
                    (*thradd).growth = (-loss / 128) as u8;
                    (*thradd).growth_fract = (((loss % 128) << 8) / 128) as u8;
                    set("THRADD_VISITS", 0);
                    if (*thradd).allied_state == GOOD_GUY as u16 {
                        SetRaceAllied(THRADDASH_SHIP as u16, 0);
                    }
                }
                let mission = get("THRADD_MISSION");
                if mission == 0 || mission > 3 {
                    SetRaceDest(
                        THRADDASH_SHIP as u8,
                        (2500 + 2535) / 2,
                        (8070 + 8358) / 2,
                        (*ilwrath).days_left.wrapping_add(1),
                        u8::MAX,
                    );
                } else if mission < 3 {
                    SetRaceDest(
                        THRADDASH_SHIP as u8,
                        2535,
                        8358,
                        10,
                        ADVANCE_THRADD_MISSION as u8,
                    );
                    set("THRADD_MISSION", 3);
                }
            }
        }
    }

    unsafe fn utwig_supox() {
        let utwig = fleet(UTWIG_SHIP);
        let supox = fleet(SUPOX_SHIP);
        let mut mission = get("UTWIG_SUPOX_MISSION");
        if (*utwig).actual_strength != 0 && mission < 5 {
            if mission == 1 {
                relative(0, 80, 0, ADVANCE_UTWIG_SUPOX_MISSION);
                let loss = ((*utwig).actual_strength >> 1) as i32;
                (*utwig).growth = (-loss / 160) as u8;
                (*utwig).growth_fract = (((loss % 160) << 8) / 160) as u8;
                (*utwig).growth_err_term = 127;
                let loss = ((*supox).actual_strength >> 1) as i32;
                if loss != 0 {
                    (*supox).growth = (-loss / 160) as u8;
                    (*supox).growth_fract = (((loss % 160) << 8) / 160) as u8;
                    (*supox).growth_err_term = 127;
                }
                set("UTWIG_WAR_NEWS", 0);
                set("SUPOX_WAR_NEWS", 0);
            } else if mission == 2 {
                relative(0, 80, 0, ADVANCE_UTWIG_SUPOX_MISSION);
                mission = mission.wrapping_add(1);
            } else {
                let (ux, uy, sx, sy) = if mission == 0 {
                    (7208, 7000, 6479, 7541)
                } else {
                    (*utwig).growth = 0;
                    (*utwig).growth_fract = 0;
                    (*supox).growth = 0;
                    (*supox).growth_fract = 0;
                    set("UTWIG_WAR_NEWS", 0);
                    set("SUPOX_WAR_NEWS", 0);
                    (8534, 8797, 7468, 9246)
                };
                for key in ["UTWIG_VISITS", "UTWIG_INFO", "SUPOX_VISITS", "SUPOX_INFO"] {
                    set(key, 0);
                }
                SetRaceDest(
                    UTWIG_SHIP as u8,
                    ux,
                    uy,
                    21,
                    ADVANCE_UTWIG_SUPOX_MISSION as u8,
                );
                SetRaceDest(SUPOX_SHIP as u8, sx, sy, 21, u8::MAX);
            }
        }
        set("UTWIG_SUPOX_MISSION", mission.wrapping_add(1));
    }

    unsafe fn mycon() {
        let race = fleet(MYCON_SHIP);
        if (*race).actual_strength == 0 {
            return;
        }
        if (*race).growth != 0 {
            set("MYCON_KNOW_AMBUSH", 1);
            SetRaceDest(MYCON_SHIP as u8, 6392, 2200, 30, u8::MAX);
            (*race).growth = 0;
            (*race).growth_fract = 0;
        } else if (*race).loc.x != 6858 || (*race).loc.y != 577 {
            SetRaceDest(MYCON_SHIP as u8, 6858, 577, 30, ADVANCE_MYCON_MISSION as u8);
        } else {
            relative(0, 14, 0, ADVANCE_MYCON_MISSION);
            let loss = ((*race).actual_strength >> 1) as i32;
            (*race).growth = (-loss / 14) as u8;
            (*race).growth_fract = (((loss % 14) << 8) / 14) as u8;
            (*race).growth_err_term = 127;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn growth_fraction_and_destination_arrival_match_the_original_integer_math() {
        let mut fleet: FLEET_INFO = unsafe { std::mem::zeroed() };
        fleet.actual_strength = 100;
        fleet.growth = (-3i8) as u8;
        fleet.growth_fract = 128;
        fleet.growth_err_term = 127;
        fleet.days_left = 2;
        fleet.dest_loc = POINT { x: 5, y: -5 };
        fleet.func_index = 4;
        assert_eq!(grow_fleet(&mut fleet), None);
        assert_eq!(
            (
                fleet.actual_strength,
                fleet.growth_err_term,
                fleet.loc.x,
                fleet.loc.y
            ),
            (96, 255, 2, -2)
        );
        assert_eq!(grow_fleet(&mut fleet), Some(4));
        assert_eq!(
            (fleet.actual_strength, fleet.loc.x, fleet.loc.y),
            (93, 5, -5)
        );
    }

    #[test]
    fn extinct_and_infinite_fleets_do_not_move_and_losses_can_kill_a_fleet() {
        let mut fleet: FLEET_INFO = unsafe { std::mem::zeroed() };
        fleet.actual_strength = u16::MAX;
        fleet.days_left = 3;
        assert_eq!(grow_fleet(&mut fleet), None);
        assert_eq!(fleet.days_left, 3);
        fleet.actual_strength = 1;
        fleet.growth = (-2i8) as u8;
        fleet.allied_state = GOOD_GUY as u16;
        assert_eq!(grow_fleet(&mut fleet), None);
        assert_eq!(fleet.actual_strength, 0);
        assert_eq!(fleet.allied_state, DEAD_GUY as u16);
        assert_eq!(fleet.days_left, 3);
    }
}
