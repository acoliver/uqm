//! Pointer-free campaign fields at mode checkpoints.
use super::parity::Fields;
use super::parity_state::value;
use crate::savegame::types::GlobalState;

#[cfg(not(test))]
pub(super) fn capture(
    fields: &mut Fields,
    game: &crate::comm::locdata::CGameState,
    journal: &mut super::parity::Journal,
) {
    global(
        fields,
        &crate::savegame::runtime::global::capture_global_state(game),
    );
    value(fields, "mode", "battle_group_ref", game.battle_group_ref);
    story(fields);
    events(fields, &game.game_clock.event_q);
    groups(fields, &game.ip_group_q, journal);
}

pub(super) fn global(fields: &mut Fields, state: &GlobalState) {
    for (name, number) in [
        ("flags", i64::from(state.glob_flags)),
        ("crew_cost", i64::from(state.crew_cost)),
        ("fuel_cost", i64::from(state.fuel_cost)),
        ("activity", i64::from(state.current_activity)),
        ("clock.day", i64::from(state.clock.day_index)),
        ("clock.month", i64::from(state.clock.month_index)),
        ("clock.year", i64::from(state.clock.year_index)),
        ("clock.tick", i64::from(state.clock.tick_count)),
        ("clock.day_in_ticks", i64::from(state.clock.day_in_ticks)),
        ("autopilot.x", i64::from(state.autopilot.x)),
        ("autopilot.y", i64::from(state.autopilot.y)),
        ("ip_location.x", i64::from(state.ip_location.x)),
        ("ip_location.y", i64::from(state.ip_location.y)),
        ("ship_stamp.x", i64::from(state.ship_stamp_origin.x)),
        ("ship_stamp.y", i64::from(state.ship_stamp_origin.y)),
        ("facing", i64::from(state.ship_facing)),
        ("planet", i64::from(state.ip_planet)),
        ("in_orbit", i64::from(state.in_orbit)),
    ] {
        value(fields, "mode", name, number);
    }
    for (index, amount) in state.module_cost.iter().enumerate() {
        value(fields, "mode", &format!("module_cost.{index}"), amount);
    }
    for (index, amount) in state.element_worth.iter().enumerate() {
        value(fields, "mode", &format!("element_worth.{index}"), amount);
    }
    velocity(fields, state);
}

fn velocity(fields: &mut Fields, state: &GlobalState) {
    value(
        fields,
        "mode",
        "velocity.angle",
        state.velocity.travel_angle,
    );
    for (name, extent) in [
        ("vector", state.velocity.vector),
        ("fract", state.velocity.fract),
        ("error", state.velocity.error),
        ("incr", state.velocity.incr),
    ] {
        value(
            fields,
            "mode.velocity",
            &format!("{name}.width"),
            extent.width,
        );
        value(
            fields,
            "mode.velocity",
            &format!("{name}.height"),
            extent.height,
        );
    }
}

pub(super) fn story(fields: &mut Fields) {
    match crate::state::ffi::snapshot_game_state() {
        Ok(bytes) => {
            value(fields, "mode.story", "initialized", true);
            for (index, byte) in bytes.iter().enumerate() {
                value(fields, "mode.story", &index.to_string(), byte);
            }
        }
        Err(error) => value(fields, "mode.story", "unavailable", error),
    }
}

#[cfg(not(test))]
fn events(fields: &mut Fields, queue: &crate::collections::queue::Queue) {
    value(
        fields,
        "mode.events",
        "initialized",
        !queue.pq_tab.is_null(),
    );
    if queue.pq_tab.is_null() {
        return;
    }
    let events = crate::savegame::runtime::events::capture_events(queue)
        .unwrap_or_else(|error| panic!("live event queue: {error}"));
    value(fields, "mode.events", "count", events.len());
    for (index, event) in events.iter().enumerate() {
        let prefix = format!("mode.events.{index}");
        value(fields, &prefix, "day", event.day_index);
        value(fields, &prefix, "month", event.month_index);
        value(fields, &prefix, "year", event.year_index);
        value(fields, &prefix, "function", event.func_index);
    }
}

#[cfg(not(test))]
fn groups(
    fields: &mut Fields,
    queue: &crate::comm::locdata::CQueue,
    journal: &mut super::parity::Journal,
) {
    use crate::savegame::runtime::{
        groups::CIpGroup,
        queues::{walk, QueueKind},
    };
    if queue.pq_tab.is_null() {
        return;
    }
    for pointer in walk::<CIpGroup>(queue, QueueKind::IpGroups)
        .unwrap_or_else(|error| panic!("IP group queue: {error}"))
    {
        // SAFETY: the pool walker checked membership, size, alignment and link order.
        let group = unsafe { &*pointer };
        journal.register_parent(pointer as usize, format!("group:{}", group.group_id));
        journal.owner(pointer as usize, -1, u32::from(group.race_id) + 1);
        let prefix = format!("mode.groups.{}", group.group_id);
        for (name, number) in [
            ("counter", i32::from(group.group_counter)),
            ("race", i32::from(group.race_id)),
            ("sys_loc", i32::from(group.sys_loc)),
            ("task", i32::from(group.task)),
            ("in_system", i32::from(group.in_system)),
            ("dest_loc", i32::from(group.dest_loc)),
            ("orbit_pos", i32::from(group.orbit_pos)),
            ("id", i32::from(group.group_id)),
            ("x", i32::from(group.loc.x)),
            ("y", i32::from(group.loc.y)),
        ] {
            value(fields, &prefix, name, number);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn campaign_fields_include_costs_clock_and_velocity() {
        // SAFETY: GlobalState contains only integers and integer arrays/structs.
        let mut state: GlobalState = unsafe { std::mem::zeroed() };
        state.crew_cost = 7;
        state.clock.tick_count = -17;
        state.module_cost[2] = 11;
        state.velocity.incr.width = -31;
        let mut fields = Fields::new();
        global(&mut fields, &state);
        assert_eq!(fields["mode.crew_cost"], "7");
        assert_eq!(fields["mode.clock.tick"], "-17");
        assert_eq!(fields["mode.module_cost.2"], "11");
        assert_eq!(fields["mode.velocity.incr.width"], "-31");
    }

    #[test]
    fn story_reports_the_current_state_owner() {
        let mut fields = Fields::new();
        story(&mut fields);
        // Other state tests may initialize the shared owner. Either state must
        // produce a complete result without requiring native game initialization.
        if fields.contains_key("mode.story.initialized") {
            assert_eq!(fields["mode.story.initialized"], "true");
            assert!(fields.contains_key("mode.story.0"));
        } else {
            assert!(fields.contains_key("mode.story.unavailable"));
        }
    }
}
