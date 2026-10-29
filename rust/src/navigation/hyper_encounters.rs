//! Hyperspace encounter elements, transition animation, tracking and radar.
#![cfg(not(test))]
use super::abi::*;
use super::get;
use super::globals::GlobData;
use super::hyper_data::HYPERSTARS;
use super::hyper_transition::{init_transition, sis_element};
use super::hyperspace::live::{cosine, height, logx, logy, sine, universe_x, universe_y, width};
use crate::engine::battle::views::{Frame, Frames};
use crate::engine::battle::{Cx, ElementId};
use std::ptr;
use std::ptr::NonNull;

const MAKEUP: [u8; 23] = [
    0x51, 0, 0, 0x51, 0x51, 0, 0x51, 0x51, 0x51, 0x51, 0x51, 0x51, 0x11, 0x51, 0x51, 0x51, 0x11,
    0x51, 0x51, 0x51, 0, 0x51, 0x51,
];
const SPEED: [i16; 23] = [
    40, 27, 24, 40, 40, 35, 48, 40, 50, 30, 21, 30, 40, 20, 25, 27, 60, 18, 30, 40, 36, 30, 30,
];
pub(super) const XOFFS: i16 = (63 * 12 + 63 * 4) / 2;
pub(super) const YOFFS: i16 = (50 * 12 + 50 * 4) / 2;

pub(super) unsafe fn allocate(cx: &mut Cx<'_>, point: POINT) -> Option<ElementId> {
    let element = cx.alloc_element()?;
    cx.element_mut(element).player = NEUTRAL_PLAYER_NUM as i16;
    cx.element_mut(element).flags = (CHANGING | FINITE_LIFE) as u16;
    cx.element_mut(element).life = 1;
    cx.element_mut(element).mass = 1;
    let lx = logx(point.x) + (width() >> 1) - GlobData.SIS_state.log_x;
    let ly = logy(point.y) + (height() >> 1) - GlobData.SIS_state.log_y;
    cx.element_mut(element).current.location = POINT {
        x: wrap(lx, width()),
        y: wrap(ly, height()),
    };
    cx.primitive_style_mut(element).kind = gfx_object_STAMP_PRIM as u8;
    let frames = ptr::addr_of_mut!(HYPERSTARS)
        .cast::<FRAME>()
        .wrapping_add(1 + (get("ARILOU_SPACE_SIDE") >> 1) as usize);
    cx.set_single_frames(
        element,
        Frames::from_native(
            NonNull::new(frames).unwrap_or_else(|| panic!("retained navigation authority")),
        ),
        Frame::from_native(*frames),
    );
    Some(element)
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

pub(super) unsafe fn ambient(cx: &mut Cx<'_>) {
    let Some(element) = cx.alloc_element() else {
        return;
    };
    cx.element_mut(element).player = NEUTRAL_PLAYER_NUM as i16;
    cx.element_mut(element).flags = (APPEARING | FINITE_LIFE | NONSOLID) as u16;
    cx.primitive_style_mut(element).kind = gfx_object_STAMP_PRIM as u8;
    cx.element_mut(element).pre = Some(ElementHook::Native(animation_preprocess));
    let random = TFB_Random();
    let dx = (random as u8 as i32 % (ScreenWidth - 64)) - ((ScreenWidth - 64) >> 1);
    let dy = ((random >> 8) as u8 as i32 % ScreenHeight) - (ScreenHeight >> 1);
    cx.element_mut(element).current.location = POINT {
        x: ((width() >> 1) + (dx << 2)) as i16,
        y: ((height() >> 1) + (dy << 2)) as i16,
    };
    let stars = stars_in_space;
    cx.set_single_frames(
        element,
        Frames::from_native(
            NonNull::new(&raw mut stars_in_space)
                .unwrap_or_else(|| panic!("retained navigation authority")),
        ),
        Frame::from_native(stars),
    );
    if (random >> 16) & 7 != 0 {
        cx.element_mut(element).life = 14;
        cx.set_frame(element, false, Frame::from_native(stars));
    } else {
        cx.element_mut(element).life = 12;
        cx.set_frame(
            element,
            false,
            Frame::from_native(SetAbsFrameIndex(stars, 14)),
        );
    }
    let head = cx.elements().head();
    cx.insert_before(element, head);
}
pub fn transition(
    cx: &mut crate::engine::battle::Cx<'_>,
    element: crate::engine::battle::ElementId,
) {
    cx.element_mut(element).flags &= !(DISAPPEARING as u16);
    cx.element_mut(element).life = 1;
    let state = cx.element(element);
    if state.turn != 0 {
        cx.element_mut(element).turn -= 1;
    } else {
        if state.hits != 0 {
            let frame = cx.retreat_frame(state.current.image.frame);
            cx.set_frame(element, true, frame);
        } else {
            let frame = cx.advance_frame(state.current.image.frame);
            if frame != cx.element_frame_set_first(element, false) {
                cx.set_frame(element, true, frame);
            } else {
                cx.element_mut(element).death = None;
            }
        }
        cx.element_mut(element).turn = 1;
    }
}
pub(crate) fn collision(
    cx: &mut Cx<'_>,
    source: ElementId,
    _: &mut POINT,
    ship: ElementId,
    _: &mut POINT,
) {
    if cx.element(ship).flags & PLAYER_SHIP as u16 == 0
        || cx.world.activity() & IN_BATTLE as u16 == 0
    {
        return;
    }
    init_transition(cx, source, ship, 0);
    let mut handle = cx.navigation().Game_state.encounter_q.head;
    while let Some(encounter) = NonNull::new(handle.cast::<ENCOUNTER>()) {
        let encounter = *cx.encounter_record(encounter);
        handle = encounter.succ;
        if let Some(element) = cx.elements().id(encounter.hElement) {
            cx.element_mut(element).flags |= (NONSOLID | IGNORE_SIMILAR) as u16;
        }
    }
    let target = sis_element(cx);
    cx.set_target(source, target);
    cx.element_mut(source).velocity.zero();
}
unsafe fn add(cx: &mut Cx<'_>, encounter: *mut ENCOUNTER, universe: POINT) -> Option<ElementId> {
    if get("ARILOU_SPACE_SIDE") >= 2 {
        return None;
    }
    if (*encounter).flags & ENCOUNTER_REFORMING as u8 != 0 {
        (*encounter).flags &= !(ENCOUNTER_REFORMING as u8);
        (*encounter).transition_state = 100;
        if (*encounter).flags & ONE_SHOT_ENCOUNTER as u8 != 0 || (*encounter).num_ships == 0 {
            return None;
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
    let handle = allocate(cx, point);
    if let Some(element) = handle {
        let mut i = (*encounter).transition_state;
        if i != 0 || new {
            if i < 0 {
                i = -i;
                cx.element_mut(element).hits = 1;
            }
            if i == 0 || i > 9 {
                i = 9;
            }
            cx.set_frame(
                element,
                false,
                Frame::from_native(SetRelFrameIndex(
                    cx.element_frame_set_first(element, false).raw(),
                    -i,
                )),
            );
            cx.element_mut(element).death = Some(ElementHook::Native(transition));
        } else {
            cx.set_frame(
                element,
                false,
                Frame::from_native(DecFrameIndex(
                    cx.element_frame_set_first(element, false).raw(),
                )),
            );
        }
        cx.element_mut(element).turn = 1;
        cx.element_mut(element).pre = None;
        cx.element_mut(element).post = None;
        cx.element_mut(element).collide = Some(CollisionHook::Native(collision));
        cx.setup_element(element);
        cx.set_intersect_frame(element, Frame::from_native(DecFrameIndex(stars_in_space)));
        cx.primitive_style_mut(element).kind = gfx_object_NUM_PRIMS as u8;
        cx.element_mut(element).flags |= (NONSOLID | IGNORE_VELOCITY) as u16;
        let tail = cx.elements().tail();
        cx.insert_before(element, tail);
    }
    (*encounter).hElement = handle.map_or(ptr::null_mut(), |id| cx.elements().raw(id));
    handle
}
unsafe fn process(
    cx: &mut Cx<'_>,
    encounter: *mut ENCOUNTER,
    universe: POINT,
    ox: i16,
    oy: i16,
    stamp: *mut STAMP,
) -> bool {
    if (*encounter).hElement.is_null() && add(cx, encounter, universe).is_none() {
        return false;
    }
    let element = cx
        .elements()
        .id((*encounter).hElement)
        .unwrap_or_else(|| panic!("encounter element"));
    if cx.element_mut(element).death.is_some() {
        if (*encounter).transition_state != 0 && cx.element_mut(element).turn == 0 {
            (*encounter).transition_state -= 1;
            if (*encounter).transition_state >= 9 {
                cx.element_mut(element).turn += 1;
            } else if (*encounter).transition_state == -9 {
                cx.element_mut(element).death = None;
                return false;
            } else {
                cx.primitive_style_mut(element).kind = gfx_object_STAMP_PRIM as u8;
            }
        }
    } else {
        cx.element_mut(element).life = 1;
        let components = cx.element_mut(element).velocity.get_next_components(1);
        let (mut dx, mut dy) = (components.0 as i16, components.1 as i16);
        if cx.element_mut(element).thrust != 0 {
            cx.element_mut(element).thrust -= 1;
        } else if cx.target(element).is_none() {
            let mut facing = (cx.element_mut(element).velocity.travel_angle + 2) >> 2;
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
                cx.element_mut(element).velocity.set_components(
                    i32::from(cosine(angle, speed as i32)),
                    i32::from(sine(angle, speed as i32)),
                );
                let components = cx.element_mut(element).velocity.get_next_components(1);
                (dx, dy) = (components.0 as i16, components.1 as i16);
                cx.element_mut(element).thrust = 3;
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
            cx.element_mut(element).flags |= NONSOLID as u16;
            cx.element_mut(element).life = 0;
            if (*encounter).transition_state == 0 {
                cx.element_mut(element).death = Some(ElementHook::Native(transition));
                (*encounter).transition_state = -1;
                cx.element_mut(element).hits = 1;
            } else {
                cx.element_mut(element).death = None;
                return false;
            }
        }
    }
    let point = (*encounter).loc_pt;
    let dx = point.x as i32 - universe.x as i32;
    let dy = point.y as i32 - universe.y as i32;
    if (-63..=63).contains(&dx) && (-50..=50).contains(&dy) {
        cx.element_mut(element).next.location = POINT {
            x: (((*encounter).log_x - GlobData.SIS_state.log_x) as i16 as i32 + (width() >> 1))
                as i16,
            y: (((*encounter).log_y - GlobData.SIS_state.log_y) as i16 as i32 + (height() >> 1))
                as i16,
        };
        if cx.element_mut(element).flags & NONSOLID as u16 != 0
            && (*encounter).transition_state == 0
        {
            let next = cx.element(element).next.location;
            cx.element_mut(element).current.location = next;
            cx.primitive_style_mut(element).kind = gfx_object_STAMP_PRIM as u8;
            if cx.element_mut(element).death.is_none() {
                cx.init_intersect_start(element);
                cx.element_mut(element).flags &= !(NONSOLID as u16);
            }
        }
    } else {
        cx.element_mut(element).flags |= NONSOLID as u16;
        if dx < -(XOFFS as i32) || dx > XOFFS as i32 || dy < -(YOFFS as i32) || dy > YOFFS as i32 {
            cx.element_mut(element).life = 0;
            cx.element_mut(element).death = None;
            return false;
        }
        cx.primitive_style_mut(element).kind = gfx_object_NUM_PRIMS as u8;
    }
    (*stamp).origin = POINT {
        x: ((point.x as i32 * 56 / (63 * 12)) as i16).wrapping_sub(ox),
        y: (((9999 - point.y as i32) * 53 / (50 * 12)) as i16).wrapping_sub(oy),
    };
    DrawStamp(stamp);
    true
}
pub(super) unsafe fn process_all(cx: &mut Cx<'_>, universe: POINT, ox: i16, oy: i16) {
    let mut stamp = STAMP {
        origin: POINT { x: 0, y: 0 },
        frame: SetAbsFrameIndex(stars_in_space, 91),
    };
    let queue = ptr::addr_of_mut!(GlobData.Game_state.encounter_q);
    let mut handle = (*queue).head;
    while !handle.is_null() {
        let encounter = handle.cast::<ENCOUNTER>();
        handle = (*encounter).succ;
        if !process(cx, encounter, universe, ox, oy, &mut stamp) {
            RemoveQueue(queue, encounter.cast());
            FreeLink(queue, encounter.cast());
        }
    }
}
