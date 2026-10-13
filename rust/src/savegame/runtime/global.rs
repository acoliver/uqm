//! Capture and restore of the fixed "GlSt" section against the live
//! `GAME_STATE` record (`GlobData.Game_state`).
//!
//! Field order and widths are `SaveGameState` (save.c) and `LoadGameState`
//! (load.c). Only the fields those write are touched: `DisplayArray`,
//! `ShipStamp.frame`, `BattleGroupRef`, the queues and the clock's event queue
//! belong to other sections or to the running game.

use crate::battle::velocity::{Extent as LiveExtent, VelocityDesc};
use crate::comm::locdata::{CGameState, CPoint};
use crate::savegame::types::{ClockState, Extent, GlobalState, Point, Velocity};

fn point(live: CPoint) -> Point {
    Point {
        x: live.x,
        y: live.y,
    }
}

fn live_point(saved: Point) -> CPoint {
    CPoint {
        x: saved.x,
        y: saved.y,
    }
}

fn extent(live: LiveExtent) -> Extent {
    Extent {
        width: live.width,
        height: live.height,
    }
}

fn live_extent(saved: Extent) -> LiveExtent {
    LiveExtent {
        width: saved.width,
        height: saved.height,
    }
}

/// The "GlSt" section for the game as it is now.
///
/// Precondition: the caller is on the game thread and nothing is ticking the
/// clock meanwhile. This takes no lock; the C clock mutex is not reachable
/// from Rust, and reading the record in one call is what keeps the date and
/// tick count consistent with each other.
pub fn capture_global_state(game: &CGameState) -> GlobalState {
    let clock = &game.game_clock;
    GlobalState {
        glob_flags: game.glob_flags,
        crew_cost: game.crew_cost,
        fuel_cost: game.fuel_cost,
        module_cost: game.module_cost,
        element_worth: game.element_worth,
        current_activity: game.current_activity,
        clock: ClockState {
            day_index: clock.day_index,
            month_index: clock.month_index,
            year_index: clock.year_index,
            tick_count: clock.tick_count,
            day_in_ticks: clock.day_in_ticks,
        },
        autopilot: point(game.autopilot),
        ip_location: point(game.ip_location),
        ship_stamp_origin: point(game.ship_stamp.origin),
        ship_facing: game.ship_facing,
        ip_planet: game.ip_planet,
        in_orbit: game.in_orbit,
        velocity: Velocity {
            travel_angle: game.velocity.travel_angle,
            vector: extent(game.velocity.vector),
            fract: extent(game.velocity.fract),
            error: extent(game.velocity.error),
            incr: extent(game.velocity.incr),
        },
    }
}

/// Write `saved` into the live record and return the saved activity.
///
/// `LoadGame` keeps the activity the game is in while the load runs and hands
/// the saved one back as `NextActivity` for the main loop to switch to, so
/// `current_activity` is left alone here and the caller decides what the
/// returned value means.
pub fn apply_global_state(game: &mut CGameState, saved: &GlobalState) -> u16 {
    game.glob_flags = saved.glob_flags;
    game.crew_cost = saved.crew_cost;
    game.fuel_cost = saved.fuel_cost;
    game.module_cost = saved.module_cost;
    game.element_worth = saved.element_worth;

    let clock = &mut game.game_clock;
    clock.day_index = saved.clock.day_index;
    clock.month_index = saved.clock.month_index;
    clock.year_index = saved.clock.year_index;
    clock.tick_count = saved.clock.tick_count;
    clock.day_in_ticks = saved.clock.day_in_ticks;

    game.autopilot = live_point(saved.autopilot);
    game.ip_location = live_point(saved.ip_location);
    game.ship_stamp.origin = live_point(saved.ship_stamp_origin);
    game.ship_facing = saved.ship_facing;
    game.ip_planet = saved.ip_planet;
    game.in_orbit = saved.in_orbit;
    game.velocity = VelocityDesc {
        travel_angle: saved.velocity.travel_angle,
        vector: live_extent(saved.velocity.vector),
        fract: live_extent(saved.velocity.fract),
        error: live_extent(saved.velocity.error),
        incr: live_extent(saved.velocity.incr),
    };
    saved.current_activity
}
