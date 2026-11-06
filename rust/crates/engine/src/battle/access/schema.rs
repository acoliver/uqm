//! Storage declarations used at the Stage B boundary.
#![allow(non_snake_case, non_camel_case_types)]
use super::super::record::VELOCITY_DESC;
use crate::graphics::native_abi::*;

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
