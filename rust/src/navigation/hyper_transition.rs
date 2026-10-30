//! Hyperspace collision and transition callbacks.
#![cfg(not(test))]
use super::abi::*;
use super::{get, set};
use std::ptr;

pub(crate) fn element_universe(cx: &Cx<'_>, element: ElementId) -> POINT {
    let point = cx.element(element).next.location;
    let (width, height) = cx.screen_dimensions();
    POINT {
        x: crate::navigation::hyper_units::universe_x(
            cx.navigation().SIS_state.log_x + i32::from(point.x) - (((width - 64) << 5) >> 1),
            width - 64,
        ),
        y: crate::navigation::hyper_units::universe_y(
            cx.navigation().SIS_state.log_y + i32::from(point.y) - ((height << 5) >> 1),
            height,
        ),
    }
}
pub(super) fn cleanup(cx: &mut Cx<'_>) {
    let mut handle = cx.navigation().Game_state.encounter_q.head;
    while let Some(encounter) = NonNull::new(handle.cast::<ENCOUNTER>()) {
        handle = cx.encounter_record(encounter).succ;
        let element = cx.elements().id(cx.encounter_record(encounter).hElement);
        if element.is_some_and(|id| cx.target(id).is_some()) {
            cx.promote_hyper_encounter(encounter);
        }
        cx.encounter_record_mut(encounter).hElement = ptr::null_mut();
    }
}
pub(crate) fn unhyper(cx: &mut Cx<'_>, element: ElementId) {
    cx.element_mut(element).flags |= CHANGING as u16;
    let current = cx.element(element).current.image.frame;
    let mut index = cx.frame_number(current);
    if index == 0 {
        index += 16;
    } else if index < 16 {
        index = (index + 1) & 15;
    } else {
        index += 1;
        if index == cx.frame_count(current) {
            cleanup(cx);
            cx.set_activity(cx.world.activity() & !(IN_BATTLE as u16));
            match cx.element(element).turn {
                0 => {
                    save_sis_facing(cx);
                    cx.set_activity(cx.world.activity() | START_ENCOUNTER as u16);
                }
                kind @ (1 | 2) => {
                    let point = element_universe(cx, element);
                    cx.leave_hyperspace(kind, point);
                }
                _ => unreachable!(),
            }
            cx.element_mut(element).velocity.zero();
            cx.primitive_style_mut(element).kind = gfx_object_NUM_PRIMS as u8;
            return;
        }
    }
    let frame = cx.absolute_frame(current, index);
    cx.set_frame(element, true, frame);
}
pub(crate) fn init_transition(cx: &mut Cx<'_>, source: ElementId, ship: ElementId, kind: u8) {
    let from = cx.element(source).next.location;
    let to = cx.element(ship).next.location;
    let dx = ((i32::from(from.x) - i32::from(to.x)) << 5) as i16;
    let dy = ((i32::from(from.y) - i32::from(to.y)) << 5) as i16;
    cx.element_mut(ship).flags |= NONSOLID as u16;
    cx.element_mut(ship).pre = Some(ElementHook::Native(unhyper));
    cx.element_mut(ship).post = None;
    cx.element_mut(ship).turn = kind;
    let mut turns = (i32::from(cx.frame_count(cx.element(ship).next.image.frame)) - 16
        + ((16 - i32::from(cx.ship_state(cx.owner(ship)).facing)) & 15)) as i16;
    if turns == 0 {
        turns = 1;
    }
    cx.element_mut(ship)
        .velocity
        .set_components(i32::from(dx / turns), i32::from(dy / turns));
}
pub(crate) fn hyper_preprocess(
    cx: &mut crate::engine::battle::Cx<'_>,
    element: crate::engine::battle::ElementId,
) {
    hyper_transition(cx, element);
}

pub fn hyper_transition(
    cx: &mut crate::engine::battle::Cx<'_>,
    element: crate::engine::battle::ElementId,
) -> bool {
    if cx.element(element).flags & APPEARING as u16 != 0 {
        let last = cx.previous_activity();
        if last & CHECK_LOAD as u16 != 0 {
            cx.set_previous_activity(last & !(CHECK_LOAD as u16));
            cx.copy_next_to_current(element);
            cx.setup_element(element);
            cx.element_mut(element).flags |= DEFY_PHYSICS as u16;
            return false;
        } else {
            cx.element_mut(element).pre = Some(ElementHook::Native(hyper_preprocess));
            cx.element_mut(element).post = None;
            cx.element_mut(element).flags |= NONSOLID as u16;
            let current = cx.element(element).current.image.frame;
            let frame = cx.absolute_frame(current, cx.frame_count(current) - 1);
            cx.set_frame(element, true, frame);
        }
    } else {
        let current = cx.element(element).current.image.frame;
        let old = cx.frame_number(current);
        let mut index = old.wrapping_sub(1);
        if old <= 16 {
            if index == 15 {
                index = 0;
            } else {
                index &= 15;
            }
            if index == cx.ship_state(cx.owner(element)).facing {
                cx.element_mut(element).pre = Some(ElementHook::Native(ship_preprocess));
                cx.element_mut(element).post = Some(ElementHook::Native(ship_postprocess));
                cx.element_mut(element).flags &= !(NONSOLID as u16);
            }
        }
        cx.element_mut(element).flags |= CHANGING as u16;
        let frame = cx.absolute_frame(current, index);
        cx.set_frame(element, true, frame);
        if cx.element(element).flags & NONSOLID as u16 == 0 {
            cx.copy_next_to_current(element);
            cx.setup_element(element);
            cx.element_mut(element).flags |= DEFY_PHYSICS as u16;
        }
    }
    true
}
pub(crate) fn star_collision(
    cx: &mut crate::engine::battle::Cx<'_>,
    source: crate::engine::battle::ElementId,
    _p0: &mut POINT,
    ship: crate::engine::battle::ElementId,
    _p1: &mut POINT,
) {
    if cx.element(ship).flags & PLAYER_SHIP as u16 == 0 || cx.portal_counter() != 0 {
        return;
    }
    let contact = cx.star_contact(source);
    let parent = cx.owner(ship);
    let (dx, dy) = cx.element(ship).velocity.get_current_components();
    if contact.current
        || cx.element(ship).flags & APPEARING as u16 != 0
        || (dx == 0
            && dy == 0
            && cx.ship_state(parent).status & ((LEFT | RIGHT | THRUST | WEAPON | SPECIAL) as u16)
                == 0)
    {
        cx.select_star(Some(contact));
        cx.element_mut(source).flags |= (DEFY_PHYSICS | COLLISION) as u16;
    } else {
        let autopilot = cx.autopilot();
        if cx.world.activity() & IN_BATTLE as u16 != 0
            && (autopilot.x == -1
                || autopilot.y == -1
                || (autopilot.x == contact.position.x && autopilot.y == contact.position.y))
        {
            cx.select_star(Some(contact));
            cx.element_mut(source).flags |= COLLISION as u16;
            cx.init_hyper_transition(source, ship, 1);
        }
    }
}
pub(crate) fn star_death(
    cx: &mut crate::engine::battle::Cx<'_>,
    element: crate::engine::battle::ElementId,
) {
    if cx.element(element).flags & DEFY_PHYSICS as u16 == 0
        && cx.world.activity() & IN_BATTLE as u16 != 0
    {
        cx.select_star(None);
    }
}
pub(crate) fn portal_death(
    cx: &mut crate::engine::battle::Cx<'_>,
    element: crate::engine::battle::ElementId,
) {
    if cx.element(element).flags & DEFY_PHYSICS as u16 == 0 || get("ARILOU_SPACE_COUNTER") == 0 {
        set(
            "ARILOU_SPACE_SIDE",
            if get("ARILOU_SPACE_SIDE") <= 1 { 0 } else { 3 },
        );
    }
}
pub(crate) fn portal_collision(
    cx: &mut crate::engine::battle::Cx<'_>,
    source: crate::engine::battle::ElementId,
    _p0: &mut POINT,
    ship: crate::engine::battle::ElementId,
    _p1: &mut POINT,
) {
    if cx.element(ship).flags & PLAYER_SHIP as u16 == 0 {
        return;
    }
    let side = get("ARILOU_SPACE_SIDE");
    if side == 0 || side == 3 {
        if cx.element(ship).flags & DEFY_PHYSICS as u16 != 0 {
            set("ARILOU_SPACE_SIDE", side ^ 1);
        } else {
            cx.init_hyper_transition(source, ship, 2);
        }
    }
    cx.element_mut(source).flags |= (DEFY_PHYSICS | COLLISION) as u16;
}
pub(super) fn sis_element(cx: &Cx<'_>) -> Option<ElementId> {
    let ship = cx.first_ship(RPG_PLAYER_NUM as usize)?;
    cx.optional_ship_element(ship)
}
fn save_sis_facing(cx: &mut Cx<'_>) {
    if let Some(element) = sis_element(cx) {
        cx.navigation_mut().Game_state.ShipFacing = cx.ship_state(cx.owner(element)).facing + 1;
    }
}
/// Retain the flagship facing while the navigation context owns the live queues.
pub fn save_sis_hyper_state(cx: &mut Cx<'_>) {
    save_sis_facing(cx);
}

use crate::engine::battle::{Cx, ElementId};
use std::ptr::NonNull;
