//! Storage declarations used at the Stage B boundary.
use super::super::record::VelocityDesc;
use crate::graphics::{geometry::Point, intersection::Stamp};

#[repr(C)]
#[derive(Debug, Copy, Clone, Default)]
pub struct ClockRecord {
    pub day_index: u8,
    pub month_index: u8,
    pub year_index: u16,
    pub tick_count: i16,
    pub day_in_ticks: i16,
}

/// The default is the all-zero record the C initializer produced.
#[repr(C)]
#[derive(Debug, Copy, Clone, Default)]
pub struct GameRecord {
    pub glob_flags: u8,
    pub crew_cost: u8,
    pub fuel_cost: u8,
    pub module_cost: [u8; 20usize],
    pub element_worth: [u8; 8usize],
    pub game_clock: ClockRecord,
    pub autopilot: Point,
    pub ip_location: Point,
    pub ship_stamp: Stamp,
    pub ship_facing: u16,
    pub ip_planet: u8,
    pub in_orbit: u8,
    pub velocity: VelocityDesc,
    pub battle_group_ref: u32,
}
