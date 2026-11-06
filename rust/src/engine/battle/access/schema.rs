//! Unchanged ABI storage declarations used at the Stage B boundary.
#![allow(non_snake_case, non_camel_case_types)]
use super::super::record::VELOCITY_DESC;
use crate::graphics::native_abi::DWORD as TimeCount;
use crate::graphics::native_abi::*;

pub type GETMELEE_STATE = getmelee_struct;

#[repr(C)]
pub struct getmelee_struct {
    pub InputFunc:
        ::std::option::Option<unsafe extern "C" fn(pInputState: *mut getmelee_struct) -> BOOLEAN>,
    pub Initialized: BOOLEAN,
    pub player: [getmelee_struct__bindgen_ty_1; 2usize],
    pub net: Option<std::rc::Rc<dyn crate::engine::battle::net::BattleNet>>,
}

#[repr(C)]
pub struct getmelee_struct__bindgen_ty_1 {
    pub timeIn: TimeCount,
    pub hBattleShip: *mut std::ffi::c_void,
    pub choice: COUNT,
    pub row: COUNT,
    pub col: COUNT,
    pub ships_left: COUNT,
    pub randomIndex: COUNT,
    pub selecting: BOOLEAN,
    pub done: BOOLEAN,
    pub flashContext: *mut crate::graphics::flash::FlashContext,
    pub remoteSelected: BOOLEAN,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct CLOCK_STATE {
    pub day_index: BYTE,
    pub month_index: BYTE,
    pub year_index: COUNT,
    pub tick_count: SIZE,
    pub day_in_ticks: SIZE,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct GAME_STATE {
    pub glob_flags: BYTE,
    pub crew_cost: BYTE,
    pub fuel_cost: BYTE,
    pub module_cost: [BYTE; 20usize],
    pub element_worth: [BYTE; 8usize],
    pub _display_array: *mut PRIMITIVE,
    pub game_clock: CLOCK_STATE,
    pub autopilot: POINT,
    pub ip_location: POINT,
    pub ship_stamp: STAMP,
    pub ship_facing: UWORD,
    pub ip_planet: BYTE,
    pub in_orbit: BYTE,
    pub velocity: VELOCITY_DESC,
    pub battle_group_ref: DWORD,
}
