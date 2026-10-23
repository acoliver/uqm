//! Hyperspace encounter elements, transition animation, tracking and radar.
#![cfg(not(test))]
use super::abi::*;
use super::get;
use super::globals::GlobData;
use super::hyper_data::HYPERSTARS;
use super::hyper_transition::{init_transition, sis_element};
use super::hyperspace::live::{
    cosine, height, logx, logy, primitive, sine, universe_x, universe_y, width,
};
use std::ptr;

const MAKEUP: [u8; 23] = [
    0x51, 0, 0, 0x51, 0x51, 0, 0x51, 0x51, 0x51, 0x51, 0x51, 0x51, 0x11, 0x51, 0x51, 0x51, 0x11,
    0x51, 0x51, 0x51, 0, 0x51, 0x51,
];
const SPEED: [i16; 23] = [
    40, 27, 24, 40, 40, 35, 48, 40, 50, 30, 21, 30, 40, 20, 25, 27, 60, 18, 30, 40, 36, 30, 30,
];
pub(super) const XOFFS: i16 = (63 * 12 + 63 * 4) / 2;
pub(super) const YOFFS: i16 = (50 * 12 + 50 * 4) / 2;

pub(super) unsafe fn allocate(point: POINT) -> HELEMENT {
    let handle = AllocElement();
    if handle.is_null() {
        return handle;
    }
    let element = handle.cast::<ELEMENT>();
    (*element).playerNr = NEUTRAL_PLAYER_NUM as i16;
    (*element).state_flags = (CHANGING | FINITE_LIFE) as u16;
    (*element).__bindgen_anon_1.life_span = 1;
    (*element).__bindgen_anon_3.mass_points = 1;
    let lx = logx(point.x) + (width() >> 1) - GlobData.SIS_state.log_x;
    let ly = logy(point.y) + (height() >> 1) - GlobData.SIS_state.log_y;
    (*element).current.location = POINT {
        x: wrap(lx, width()),
        y: wrap(ly, height()),
    };
    (*primitive(element)).Type = gfx_object_STAMP_PRIM as u8;
    (*element).current.image.farray = ptr::addr_of_mut!(HYPERSTARS)
        .cast::<FRAME>()
        .add(1 + (get("ARILOU_SPACE_SIDE") >> 1) as usize);
    handle
}
fn wrap(value: i32, limit: i32) -> i16 {
    if value < 0 {
        (value + limit) as i16
    } else if value >= limit {
        (value - limit) as i16
    } else {
        value as i16
    }
}

pub(super) unsafe fn ambient() {
    let handle = AllocElement();
    if handle.is_null() {
        return;
    }
    let element = handle.cast::<ELEMENT>();
    (*element).playerNr = NEUTRAL_PLAYER_NUM as i16;
    (*element).state_flags = (APPEARING | FINITE_LIFE | NONSOLID) as u16;
    (*primitive(element)).Type = gfx_object_STAMP_PRIM as u8;
    (*element).preprocess_func = Some(animation_preprocess);
    let random = TFB_Random();
    let dx = (random as u8 as i32 % (ScreenWidth - 64)) - ((ScreenWidth - 64) >> 1);
    let dy = ((random >> 8) as u8 as i32 % ScreenHeight) - (ScreenHeight >> 1);
    (*element).current.location = POINT {
        x: ((width() >> 1) + (dx << 2)) as i16,
        y: ((height() >> 1) + (dy << 2)) as i16,
    };
    (*element).current.image.farray = ptr::addr_of_mut!(stars_in_space);
    if (random >> 16) & 7 != 0 {
        (*element).__bindgen_anon_1.life_span = 14;
        (*element).current.image.frame = stars_in_space;
    } else {
        (*element).__bindgen_anon_1.life_span = 12;
        (*element).current.image.frame = SetAbsFrameIndex(stars_in_space, 14);
    }
    InsertQueue(ptr::addr_of_mut!(disp_q), handle, disp_q.head);
}
unsafe extern "C" fn transition(element: *mut ELEMENT) {
    (*element).state_flags &= !(DISAPPEARING as u16);
    (*element).__bindgen_anon_1.life_span = 1;
    if (*element).__bindgen_anon_4.turn_wait != 0 {
        (*element).__bindgen_anon_4.turn_wait -= 1;
    } else {
        if (*element).__bindgen_anon_2.hit_points != 0 {
            (*element).next.image.frame = DecFrameIndex((*element).current.image.frame);
        } else {
            let frame = IncFrameIndex((*element).current.image.frame);
            if frame != *(*element).current.image.farray {
                (*element).next.image.frame = frame;
            } else {
                (*element).death_func = None;
            }
        }
        (*element).__bindgen_anon_4.turn_wait = 1;
    }
}
unsafe extern "C" fn collision(
    source: *mut ELEMENT,
    _p0: *mut POINT,
    ship: *mut ELEMENT,
    _p1: *mut POINT,
) {
    if (*ship).state_flags & PLAYER_SHIP as u16 == 0
        || GlobData.Game_state.CurrentActivity & IN_BATTLE as u16 == 0
    {
        return;
    }
    init_transition(source, ship, 0);
    let mut handle = GlobData.Game_state.encounter_q.head;
    while !handle.is_null() {
        let encounter = handle.cast::<ENCOUNTER>();
        handle = (*encounter).succ;
        if !(*encounter).hElement.is_null() {
            (*(*encounter).hElement.cast::<ELEMENT>()).state_flags |=
                (NONSOLID | IGNORE_SIMILAR) as u16;
        }
    }
    (*source).hTarget = sis_element();
    (*source).velocity = std::mem::zeroed();
}
unsafe fn add(encounter: *mut ENCOUNTER, universe: POINT) -> HELEMENT {
    if get("ARILOU_SPACE_SIDE") >= 2 {
        return ptr::null_mut();
    }
    if (*encounter).flags & ENCOUNTER_REFORMING as u8 != 0 {
        (*encounter).flags &= !(ENCOUNTER_REFORMING as u8);
        (*encounter).transition_state = 100;
        if (*encounter).flags & ONE_SHOT_ENCOUNTER as u8 != 0 || (*encounter).num_ships == 0 {
            return ptr::null_mut();
        }
    }
    let new = (*encounter).num_ships == 0;
    let mut point = (*encounter).loc_pt;
    if new {
        let race = (*encounter).race_id as usize;
        let makeup = MAKEUP[race];
        let mut ships = makeup & 15;
        for _ in 0..((makeup >> 4) - ships) {
            if TFB_Random() as u16 % 100 < 50 {
                ships += 1;
            }
        }
        ships = ships.min(MAX_HYPER_SHIPS as u8);
        (*encounter).num_ships = ships;
        for i in 0..ships as usize {
            let fleet = GetStarShipFromIndex(
                ptr::addr_of_mut!(GlobData.Game_state.avail_race_q),
                race as u16,
            )
            .cast::<FLEET_INFO>();
            let brief = &mut (*encounter).ShipList[i];
            brief.race_id = race as u8;
            brief.crew_level = (*fleet).crew_level;
            brief.max_crew = (*fleet).max_crew;
            brief.max_energy = (*fleet).max_energy;
        }
        loop {
            let random = TFB_Random();
            point = POINT {
                x: (universe.x as i32 + (random as u16 % (XOFFS as u16 * 2)) as i32 - XOFFS as i32)
                    .clamp(0, 9999) as i16,
                y: (universe.y as i32 + ((random >> 16) % (YOFFS as u32 * 2)) as i32 - YOFFS as i32)
                    .clamp(0, 9999) as i16,
            };
            let dx = point.x.wrapping_sub((*encounter).origin.x) as i64;
            let dy = point.y.wrapping_sub((*encounter).origin.y) as i64;
            if dx * dx + dy * dy <= (*encounter).radius as i64 * (*encounter).radius as i64 {
                break;
            }
        }
        (*encounter).loc_pt = point;
        (*encounter).log_x = logx(point.x);
        (*encounter).log_y = logy(point.y);
    }
    let handle = allocate(point);
    if !handle.is_null() {
        let element = handle.cast::<ELEMENT>();
        let mut i = (*encounter).transition_state;
        if i != 0 || new {
            if i < 0 {
                i = -i;
                (*element).__bindgen_anon_2.hit_points = 1;
            }
            if i == 0 || i > 9 {
                i = 9;
            }
            (*element).current.image.frame = SetRelFrameIndex(*(*element).current.image.farray, -i);
            (*element).death_func = Some(transition);
        } else {
            (*element).current.image.frame = DecFrameIndex(*(*element).current.image.farray);
        }
        (*element).__bindgen_anon_4.turn_wait = 1;
        (*element).preprocess_func = None;
        (*element).postprocess_func = None;
        (*element).collision_func = Some(collision);
        SetUpElement(element);
        (*element).IntersectControl.IntersectStamp.frame = DecFrameIndex(stars_in_space);
        (*primitive(element)).Type = gfx_object_NUM_PRIMS as u8;
        (*element).state_flags |= (NONSOLID | IGNORE_VELOCITY) as u16;
        InsertQueue(ptr::addr_of_mut!(disp_q), handle, disp_q.tail);
    }
    (*encounter).hElement = handle;
    handle
}
unsafe fn process(
    encounter: *mut ENCOUNTER,
    universe: POINT,
    ox: i16,
    oy: i16,
    stamp: *mut STAMP,
) -> bool {
    if (*encounter).hElement.is_null() && add(encounter, universe).is_null() {
        return false;
    }
    let element = (*encounter).hElement.cast::<ELEMENT>();
    if (*element).death_func.is_some() {
        if (*encounter).transition_state != 0 && (*element).__bindgen_anon_4.turn_wait == 0 {
            (*encounter).transition_state -= 1;
            if (*encounter).transition_state >= 9 {
                (*element).__bindgen_anon_4.turn_wait += 1;
            } else if (*encounter).transition_state == -9 {
                (*element).death_func = None;
                return false;
            } else {
                (*primitive(element)).Type = gfx_object_STAMP_PRIM as u8;
            }
        }
    } else {
        (*element).__bindgen_anon_1.life_span = 1;
        let (mut dx, mut dy) = (0, 0);
        GetNextVelocityComponents(&mut (*element).velocity, &mut dx, &mut dy, 1);
        if (*element).__bindgen_anon_5.thrust_wait != 0 {
            (*element).__bindgen_anon_5.thrust_wait -= 1;
        } else if (*element).hTarget.is_null() {
            let mut facing = ((*element).velocity.TravelAngle + 2) >> 2;
            let target = (ARCTAN(
                universe.x.wrapping_sub((*encounter).loc_pt.x),
                universe.y.wrapping_sub((*encounter).loc_pt.y),
            ) + 2)
                >> 2;
            let delta = facing.wrapping_sub(target) & 15;
            if delta != 0 || (dx == 0 && dy == 0) {
                let mut speed = SPEED[(*encounter).race_id as usize] * 16;
                if delta < 8 {
                    facing = facing.wrapping_sub(1);
                } else {
                    facing = facing.wrapping_add(1);
                }
                if (delta + 2) & 15 > 4 {
                    if delta < 8 {
                        facing = facing.wrapping_sub(1);
                    } else {
                        facing = facing.wrapping_add(1);
                    }
                    speed >>= 1;
                }
                let angle = facing.wrapping_shl(2);
                SetVelocityComponents(
                    &mut (*element).velocity,
                    cosine(angle, speed as i32),
                    sine(angle, speed as i32),
                );
                GetNextVelocityComponents(&mut (*element).velocity, &mut dx, &mut dy, 1);
                (*element).__bindgen_anon_5.thrust_wait = 3;
            }
        }
        (*encounter).log_x += dx as i32;
        (*encounter).log_y -= dy as i32;
        (*encounter).loc_pt = POINT {
            x: universe_x((*encounter).log_x),
            y: universe_y((*encounter).log_y),
        };
        let radius = (*encounter).radius.wrapping_add(100);
        let dx = (*encounter)
            .loc_pt
            .x
            .wrapping_sub((*encounter).origin.x)
            .wrapping_abs() as u16;
        let dy = (*encounter)
            .loc_pt
            .y
            .wrapping_sub((*encounter).origin.y)
            .wrapping_abs() as u16;
        if dx >= radius
            || dy >= radius
            || dx as u32 * dx as u32 + dy as u32 * dy as u32 >= radius as u32 * radius as u32
        {
            (*element).state_flags |= NONSOLID as u16;
            (*element).__bindgen_anon_1.life_span = 0;
            if (*encounter).transition_state == 0 {
                (*element).death_func = Some(transition);
                (*encounter).transition_state = -1;
                (*element).__bindgen_anon_2.hit_points = 1;
            } else {
                (*element).death_func = None;
                return false;
            }
        }
    }
    let point = (*encounter).loc_pt;
    let dx = point.x as i32 - universe.x as i32;
    let dy = point.y as i32 - universe.y as i32;
    if (-63..=63).contains(&dx) && (-50..=50).contains(&dy) {
        (*element).next.location = POINT {
            x: (((*encounter).log_x - GlobData.SIS_state.log_x) as i16 as i32 + (width() >> 1))
                as i16,
            y: (((*encounter).log_y - GlobData.SIS_state.log_y) as i16 as i32 + (height() >> 1))
                as i16,
        };
        if (*element).state_flags & NONSOLID as u16 != 0 && (*encounter).transition_state == 0 {
            (*element).current.location = (*element).next.location;
            (*primitive(element)).Type = gfx_object_STAMP_PRIM as u8;
            if (*element).death_func.is_none() {
                (*element).IntersectControl.IntersectStamp.origin = POINT {
                    x: (*element).current.location.x >> 2,
                    y: (*element).current.location.y >> 2,
                };
                (*element).state_flags &= !(NONSOLID as u16);
            }
        }
    } else {
        (*element).state_flags |= NONSOLID as u16;
        if dx < -(XOFFS as i32) || dx > XOFFS as i32 || dy < -(YOFFS as i32) || dy > YOFFS as i32 {
            (*element).__bindgen_anon_1.life_span = 0;
            (*element).death_func = None;
            return false;
        }
        (*primitive(element)).Type = gfx_object_NUM_PRIMS as u8;
    }
    (*stamp).origin = POINT {
        x: ((point.x as i32 * 56 / (63 * 12)) as i16).wrapping_sub(ox),
        y: (((9999 - point.y as i32) * 53 / (50 * 12)) as i16).wrapping_sub(oy),
    };
    DrawStamp(stamp);
    true
}
pub(super) unsafe fn process_all(universe: POINT, ox: i16, oy: i16) {
    let mut stamp = STAMP {
        origin: POINT { x: 0, y: 0 },
        frame: SetAbsFrameIndex(stars_in_space, 91),
    };
    let queue = ptr::addr_of_mut!(GlobData.Game_state.encounter_q);
    let mut handle = (*queue).head;
    while !handle.is_null() {
        let encounter = handle.cast::<ENCOUNTER>();
        handle = (*encounter).succ;
        if !process(encounter, universe, ox, oy, &mut stamp) {
            RemoveQueue(queue, encounter.cast());
            FreeLink(queue, encounter.cast());
        }
    }
}
