//! Hyperspace collision and transition callbacks.
#![cfg(not(test))]
#![allow(non_snake_case)]
use super::abi::*;
use super::globals::GlobData;
use super::hyperspace::live::{height, logx, logy, primitive, universe_x, universe_y, width};
use super::{get, set};
use crate::planets::starmap::{star_array, CurStarDescPtr};
use std::{mem, ptr};

pub(super) unsafe fn element_universe(element: *const ELEMENT) -> POINT {
    POINT {
        x: universe_x(
            GlobData.SIS_state.log_x + (*element).next.location.x as i32 - (width() >> 1),
        ),
        y: universe_y(
            GlobData.SIS_state.log_y + (*element).next.location.y as i32 - (height() >> 1),
        ),
    }
}
pub(super) unsafe fn cleanup() {
    let queue = ptr::addr_of_mut!(GlobData.Game_state.encounter_q);
    let mut handle = (*queue).head;
    while !handle.is_null() {
        let encounter = handle.cast::<ENCOUNTER>();
        handle = (*encounter).succ;
        if !(*encounter).hElement.is_null()
            && !(*(*encounter).hElement.cast::<ELEMENT>()).hTarget.is_null()
        {
            RemoveQueue(queue, encounter.cast());
            InsertQueue(queue, encounter.cast(), (*queue).head);
        }
        (*encounter).hElement = ptr::null_mut();
    }
}
unsafe fn interplanetary(element: *mut ELEMENT) {
    GlobData.Game_state.ip_planet = 0;
    GlobData.Game_state.in_orbit = 0;
    GlobData.Game_state.ShipFacing = 0;
    set("USED_BROADCASTER", 0);
    if get("ARILOU_SPACE_SIDE") <= 1 {
        GlobData.Game_state.CurrentActivity |= START_INTERPLANETARY as u16;
        set("ESCAPE_COUNTER", 0);
    } else {
        GlobData.Game_state.autopilot = POINT { x: -1, y: -1 };
        let mut point = element_universe(element);
        let star = FindStar(ptr::null_mut(), &mut point, 5, 5);
        CurStarDescPtr = star.cast();
        if (*star).star_pt.x == ARILOU_HOME_X as i16 && (*star).star_pt.y == ARILOU_HOME_Y as i16 {
            GlobData.Game_state.CurrentActivity |= START_ENCOUNTER as u16;
        } else {
            const PORTALS: [(i16, i16); 15] = [
                (4091, 7748),
                (3184, 4906),
                (9211, 6104),
                (5673, 1207),
                (1910, 926),
                (8607, 151),
                (50, 1647),
                (6117, 4131),
                (5658, 9712),
                (2302, 3988),
                (112, 9409),
                (7752, 8906),
                (368, 6332),
                (9735, 3153),
                (5850, 6213),
            ];
            let index = CurStarDescPtr.offset_from(star_array.add(503)) as usize;
            let (x, y) = PORTALS[index];
            GlobData.SIS_state.log_x = logx(x);
            GlobData.SIS_state.log_y = logy(y);
            set("ARILOU_SPACE_SIDE", 0);
            // The source portal is not a star at the HyperSpace destination.
            // Keeping it makes a save name a descriptor from the wrong catalog.
            CurStarDescPtr = ptr::null_mut();
        }
    }
}
unsafe fn arilou() {
    GlobData.Game_state.ShipFacing = 0;
    set("USED_BROADCASTER", 0);
    GlobData.Game_state.autopilot = POINT { x: -1, y: -1 };
    if get("ARILOU_SPACE_SIDE") <= 1 {
        GlobData.SIS_state.log_x = logx(QUASI_SPACE_X as i16);
        GlobData.SIS_state.log_y = logy(QUASI_SPACE_Y as i16);
        if get("PORTAL_COUNTER") != 0 {
            set("PORTAL_COUNTER", 0);
        }
        set("ARILOU_SPACE_SIDE", 3);
    } else {
        GlobData.SIS_state.log_x = logx(ARILOU_SPACE_X as i16);
        GlobData.SIS_state.log_y = logy(ARILOU_SPACE_Y as i16);
        set("ARILOU_SPACE_SIDE", 0);
    }
    CurStarDescPtr = ptr::null_mut();
}
pub(crate) unsafe extern "C" fn unhyper(element: *mut ELEMENT) {
    (*element).state_flags |= CHANGING as u16;
    let mut index = GetFrameIndex((*element).current.image.frame.cast());
    if index == 0 {
        index += 16;
    } else if index < 16 {
        index = (index + 1) & 15;
    } else {
        index += 1;
        if index == GetFrameCount((*element).current.image.frame) {
            cleanup();
            GlobData.Game_state.CurrentActivity &= !(IN_BATTLE as u16);
            match (*element).__bindgen_anon_4.turn_wait {
                0 => {
                    SaveSisHyperState();
                    GlobData.Game_state.CurrentActivity |= START_ENCOUNTER as u16;
                }
                1 => interplanetary(element),
                2 => arilou(),
                _ => unreachable!(),
            }
            (*element).velocity = mem::zeroed();
            (*primitive(element)).Type = gfx_object_NUM_PRIMS as u8;
            return;
        }
    }
    crate::collections::observation::image!(frame; (*element).next.image.frame, SetAbsFrameIndex((*element).current.image.frame, index));
}
pub(super) unsafe fn init_transition(source: *mut ELEMENT, ship: *mut ELEMENT, kind: u8) {
    let dx = (((*source).next.location.x as i32 - (*ship).next.location.x as i32) << 5) as i16;
    let dy = (((*source).next.location.y as i32 - (*ship).next.location.y as i32) << 5) as i16;
    (*ship).state_flags |= NONSOLID as u16;
    (*ship).preprocess_func = Some(ElementHook::Legacy(unhyper));
    (*ship).postprocess_func = None;
    (*ship).__bindgen_anon_4.turn_wait = kind;
    let starship = (*ship).pParent.cast::<STARSHIP>();
    let mut turns = (GetFrameCount((*ship).next.image.frame) as i32 - 16
        + ((16 - (*starship).ShipFacing as i32) & 15)) as i16;
    if turns == 0 {
        turns = 1;
    }
    SetVelocityComponents(&mut (*ship).velocity, dx / turns, dy / turns);
}
pub(crate) unsafe extern "C" fn hyper_preprocess(element: *mut ELEMENT) {
    hyper_transition(element);
}

/// # Safety
/// `element` is the live flagship element on the game thread.
#[no_mangle]
pub unsafe extern "C" fn hyper_transition(element: *mut ELEMENT) -> BOOLEAN {
    if (*element).state_flags & APPEARING as u16 != 0 {
        if LastActivity & CHECK_LOAD as u16 != 0 {
            LastActivity &= !(CHECK_LOAD as u16);
            crate::collections::observation::image!(copy; (*element).current, (*element).next);
            SetUpElement(element);
            (*element).state_flags |= DEFY_PHYSICS as u16;
            return 0;
        } else {
            (*element).preprocess_func = Some(ElementHook::Legacy(hyper_preprocess));
            (*element).postprocess_func = None;
            (*element).state_flags |= NONSOLID as u16;
            crate::collections::observation::image!(frame; (*element).next.image.frame, SetAbsFrameIndex(
                (*element).current.image.frame,
                GetFrameCount((*element).current.image.frame) - 1,
            ));
        }
    } else {
        let old = GetFrameIndex((*element).current.image.frame.cast());
        let mut index = old.wrapping_sub(1);
        if old <= 16 {
            if index == 15 {
                index = 0;
            } else {
                index &= 15;
            }
            let ship = (*element).pParent.cast::<STARSHIP>();
            if index == (*ship).ShipFacing {
                (*element).preprocess_func = Some(ElementHook::Native(ship_preprocess));
                (*element).postprocess_func = Some(ElementHook::Native(ship_postprocess));
                (*element).state_flags &= !(NONSOLID as u16);
            }
        }
        (*element).state_flags |= CHANGING as u16;
        crate::collections::observation::image!(frame; (*element).next.image.frame, SetAbsFrameIndex((*element).current.image.frame, index));
        if (*element).state_flags & NONSOLID as u16 == 0 {
            crate::collections::observation::image!(copy; (*element).current, (*element).next);
            SetUpElement(element);
            (*element).state_flags |= DEFY_PHYSICS as u16;
        }
    }
    1
}
pub(crate) fn star_collision(
    cx: &mut crate::engine::battle::Cx<'_>,
    source: crate::engine::battle::ElementId,
    _p0: &mut POINT,
    ship: crate::engine::battle::ElementId,
    _p1: &mut POINT,
) {
    if cx.b.elements[ship].state_flags & PLAYER_SHIP as u16 == 0 || cx.portal_counter() != 0 {
        return;
    }
    let contact = cx.star_contact(source);
    let parent = cx.owner(ship);
    let (dx, dy) = cx.b.elements[ship].velocity.get_current_components();
    if contact.current
        || cx.b.elements[ship].state_flags & APPEARING as u16 != 0
        || (dx == 0
            && dy == 0
            && cx.ship(parent).cur_status_flags
                & ((LEFT | RIGHT | THRUST | WEAPON | SPECIAL) as u16)
                == 0)
    {
        cx.select_star(Some(contact));
        cx.b.elements[source].state_flags |= (DEFY_PHYSICS | COLLISION) as u16;
    } else {
        let autopilot = cx.autopilot();
        if cx.world.activity() & IN_BATTLE as u16 != 0
            && (autopilot.x == -1
                || autopilot.y == -1
                || (autopilot.x == contact.position.x && autopilot.y == contact.position.y))
        {
            cx.select_star(Some(contact));
            cx.b.elements[source].state_flags |= COLLISION as u16;
            cx.init_hyper_transition(source, ship, 1);
        }
    }
}
pub(crate) fn star_death(
    cx: &mut crate::engine::battle::Cx<'_>,
    element: crate::engine::battle::ElementId,
) {
    if cx.b.elements[element].state_flags & DEFY_PHYSICS as u16 == 0
        && cx.world.activity() & IN_BATTLE as u16 != 0
    {
        cx.select_star(None);
    }
}
pub(crate) unsafe extern "C" fn portal_death(element: *mut ELEMENT) {
    if (*element).state_flags & DEFY_PHYSICS as u16 == 0 || get("ARILOU_SPACE_COUNTER") == 0 {
        set(
            "ARILOU_SPACE_SIDE",
            if get("ARILOU_SPACE_SIDE") <= 1 { 0 } else { 3 },
        );
    }
}
pub(crate) unsafe extern "C" fn portal_collision(
    source: *mut ELEMENT,
    _p0: *mut POINT,
    ship: *mut ELEMENT,
    _p1: *mut POINT,
) {
    if (*ship).state_flags & PLAYER_SHIP as u16 == 0 {
        return;
    }
    let side = get("ARILOU_SPACE_SIDE");
    if side == 0 || side == 3 {
        if (*ship).state_flags & DEFY_PHYSICS as u16 != 0 {
            set("ARILOU_SPACE_SIDE", side ^ 1);
        } else {
            init_transition(source, ship, 2);
        }
    }
    (*source).state_flags |= (DEFY_PHYSICS | COLLISION) as u16;
}
pub(super) unsafe fn sis_element() -> HELEMENT {
    let queue = ptr::addr_of_mut!(race_q)
        .cast::<QUEUE>()
        .add(RPG_PLAYER_NUM as usize);
    let ship = (*queue).head.cast::<STARSHIP>();
    if ship.is_null() {
        return ptr::null_mut();
    }
    (*ship).hShip
}
/// # Safety
/// Game-thread access to the live flagship queue and world state.
pub unsafe fn SaveSisHyperState() {
    let element = sis_element().cast::<ELEMENT>();
    if element.is_null() {
        return;
    }
    let ship = (*element).pParent.cast::<STARSHIP>();
    GlobData.Game_state.ShipFacing = (*ship).ShipFacing + 1;
}

#[derive(Clone, Copy)]
pub(crate) struct StarContact {
    address: usize,
    position: crate::graphics::native_abi::POINT,
    current: bool,
}
impl crate::engine::battle::Cx<'_> {
    fn portal_counter(&self) -> u8 {
        // SAFETY: story state is accessed only on the game thread.
        get("PORTAL_COUNTER")
    }
    fn autopilot(&self) -> crate::graphics::native_abi::POINT {
        // SAFETY: this is a value projection of the game-thread-owned world.
        unsafe { GlobData.Game_state.autopilot }
    }
    fn star_contact(&self, id: crate::engine::battle::ElementId) -> StarContact {
        let element = self.b.elements.raw(id).cast();
        // SAFETY: lookup reads the checked element and loaded star catalog synchronously.
        unsafe {
            let mut point = element_universe(element);
            let star = FindStar(ptr::null_mut(), &mut point, 5, 5);
            assert!(!star.is_null(), "star collision has no catalog entry");
            StarContact {
                address: star as usize,
                position: (*star).star_pt,
                current: star.cast() == CurStarDescPtr,
            }
        }
    }
    fn select_star(&mut self, contact: Option<StarContact>) {
        // SAFETY: catalog addresses originate only in star_contact; None clears selection.
        unsafe {
            CurStarDescPtr = contact.map_or(ptr::null_mut(), |c| c.address as *mut _);
        }
    }
    fn init_hyper_transition(
        &mut self,
        source: crate::engine::battle::ElementId,
        ship: crate::engine::battle::ElementId,
        kind: u8,
    ) {
        let source = self.b.elements.raw(source).cast();
        let ship = self.b.elements.raw(ship).cast();
        // SAFETY: checked elements remain live and no record references cross the call.
        unsafe {
            init_transition(source, ship, kind);
        }
    }
}
