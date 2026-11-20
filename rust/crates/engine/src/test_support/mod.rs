//! Safe fixtures over the native graphics and resource state, for tests in this
//! crate's dependents. Gated behind `test-support`.

pub mod campaign_display;
pub mod conversation_display;
pub mod native_probe;
pub mod native_process;
pub mod native_session;

use crate::battle::access::schema::{ClockRecord, GameRecord};
use crate::{battle::record::VelocityDesc, graphics::geometry::Point};

/// An all-zero campaign state record, as the C initializer produced one.
impl Default for GameRecord {
    fn default() -> Self {
        Self {
            glob_flags: 0,
            crew_cost: 0,
            fuel_cost: 0,
            module_cost: [0; 20],
            element_worth: [0; 8],
            game_clock: ClockRecord {
                day_index: 0,
                month_index: 0,
                year_index: 0,
                tick_count: 0,
                day_in_ticks: 0,
            },
            autopilot: Point::default(),
            ip_location: Point::default(),
            ship_stamp: Default::default(),
            ship_facing: 0,
            ip_planet: 0,
            in_orbit: 0,
            velocity: VelocityDesc::default(),
            battle_group_ref: 0,
        }
    }
}
