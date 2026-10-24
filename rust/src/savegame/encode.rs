//! Writes a [`SaveFile`] in the byte layout of `sc2/src/uqm/save.c`.

use super::error::EncodeError;
use super::format::*;
use super::types::*;

/// Encode a whole save in the order `SaveGame` writes it.
pub fn encode_save(save: &SaveFile) -> Result<Vec<u8>, EncodeError> {
    let mut out = Vec::new();
    put_u32(&mut out, SAVEFILE_TAG);

    chunk(&mut out, SUMMARY_TAG, summary_body(&save.summary))?;
    chunk(&mut out, GLOBAL_STATE_TAG, global_body(&save.global))?;
    chunk(&mut out, GAME_STATE_TAG, save.game_state.to_vec())?;

    queue_chunk(&mut out, RACE_Q_TAG, &save.race_queue, race_entry)?;
    queue_chunk(&mut out, NPC_SHIP_Q_TAG, &save.npc_ship_queue, ship_entry)?;
    queue_chunk(&mut out, IP_GRP_Q_TAG, &save.ip_group_queue, ip_group)?;
    queue_chunk(&mut out, SHIP_Q_TAG, &save.ship_queue, ship_entry)?;
    queue_chunk(&mut out, EVENTS_TAG, &save.events, event_entry)?;
    queue_chunk(&mut out, ENCOUNTERS_TAG, &save.encounters, encounter_entry)?;

    if let Some(words) = &save.scan {
        let mut body = Vec::with_capacity(words.len() * 4);
        for &word in words {
            put_u32(&mut body, word);
        }
        chunk(&mut out, SCAN_TAG, body)?;
    }
    queue_chunk(&mut out, CANNED_NODES_TAG, &save.canned_nodes, canned_node)?;
    if let Some(list) = &save.group_list {
        chunk(&mut out, GROUP_LIST_TAG, group_list_body(list)?)?;
    }
    for group in &save.battle_groups {
        chunk(&mut out, BATTLE_GROUP_TAG, battle_group_body(group)?)?;
    }

    chunk(&mut out, STAR_TAG, star_body(&save.star))?;
    Ok(out)
}

pub(super) fn encode_summary(summary: &Summary) -> Vec<u8> {
    let mut out = Vec::new();
    chunk(&mut out, SUMMARY_TAG, summary_body(summary)).expect("summary fits a chunk");
    out
}

fn put_u8(out: &mut Vec<u8>, v: u8) {
    out.push(v);
}

fn put_u16(out: &mut Vec<u8>, v: u16) {
    out.extend_from_slice(&v.to_le_bytes());
}

fn put_i16(out: &mut Vec<u8>, v: i16) {
    out.extend_from_slice(&v.to_le_bytes());
}

fn put_u32(out: &mut Vec<u8>, v: u32) {
    out.extend_from_slice(&v.to_le_bytes());
}

fn put_i32(out: &mut Vec<u8>, v: i32) {
    out.extend_from_slice(&v.to_le_bytes());
}

fn put_point(out: &mut Vec<u8>, p: Point) {
    put_i16(out, p.x);
    put_i16(out, p.y);
}

fn put_extent(out: &mut Vec<u8>, e: Extent) {
    put_i16(out, e.width);
    put_i16(out, e.height);
}

fn chunk(out: &mut Vec<u8>, tag: u32, body: Vec<u8>) -> Result<(), EncodeError> {
    let size = u32::try_from(body.len()).map_err(|_| EncodeError::CountOverflow {
        tag,
        count: body.len() as u64,
    })?;
    put_u32(out, tag);
    put_u32(out, size);
    out.extend_from_slice(&body);
    Ok(())
}

/// Writes nothing for an empty queue, as `SaveGame` does.
fn queue_chunk<T>(
    out: &mut Vec<u8>,
    tag: u32,
    entries: &[T],
    write_entry: fn(&mut Vec<u8>, &T),
) -> Result<(), EncodeError> {
    if entries.is_empty() {
        return Ok(());
    }
    check_count(tag, entries.len(), MAX_QUEUE_ENTRIES)?;
    let mut body = Vec::new();
    for entry in entries {
        write_entry(&mut body, entry);
    }
    chunk(out, tag, body)
}

fn check_count(tag: u32, count: usize, max: u64) -> Result<(), EncodeError> {
    if count as u64 > max {
        return Err(EncodeError::CountOverflow {
            tag,
            count: count as u64,
        });
    }
    Ok(())
}

fn sis_state(out: &mut Vec<u8>, s: &SisState) {
    put_i32(out, s.log_x);
    put_i32(out, s.log_y);
    put_u32(out, s.res_units);
    put_u32(out, s.fuel_on_board);
    put_u16(out, s.crew_enlisted);
    put_u16(out, s.total_element_mass);
    put_u16(out, s.total_bio_mass);
    out.extend_from_slice(&s.module_slots);
    out.extend_from_slice(&s.drive_slots);
    out.extend_from_slice(&s.jet_slots);
    put_u8(out, s.num_landers);
    for &amount in &s.element_amounts {
        put_u16(out, amount);
    }
    out.extend_from_slice(&s.ship_name);
    out.extend_from_slice(&s.commander_name);
    out.extend_from_slice(&s.planet_name);
}

fn summary_body(s: &Summary) -> Vec<u8> {
    let mut b = Vec::new();
    sis_state(&mut b, &s.sis);
    put_u8(&mut b, s.activity);
    put_u8(&mut b, s.flags);
    put_u8(&mut b, s.day_index);
    put_u8(&mut b, s.month_index);
    put_u16(&mut b, s.year_index);
    put_u8(&mut b, s.mcredit_lo);
    put_u8(&mut b, s.mcredit_hi);
    put_u8(&mut b, s.num_ships);
    put_u8(&mut b, s.num_devices);
    b.extend_from_slice(&s.ship_list);
    b.extend_from_slice(&s.device_list);
    b.extend_from_slice(s.save_name.as_bytes());
    b
}

fn global_body(g: &GlobalState) -> Vec<u8> {
    let mut b = Vec::new();
    put_u8(&mut b, g.glob_flags);
    put_u8(&mut b, g.crew_cost);
    put_u8(&mut b, g.fuel_cost);
    b.extend_from_slice(&g.module_cost);
    b.extend_from_slice(&g.element_worth);
    put_u16(&mut b, g.current_activity);
    put_u8(&mut b, g.clock.day_index);
    put_u8(&mut b, g.clock.month_index);
    put_u16(&mut b, g.clock.year_index);
    put_i16(&mut b, g.clock.tick_count);
    put_i16(&mut b, g.clock.day_in_ticks);
    put_point(&mut b, g.autopilot);
    put_point(&mut b, g.ip_location);
    put_point(&mut b, g.ship_stamp_origin);
    put_u16(&mut b, g.ship_facing);
    put_u8(&mut b, g.ip_planet);
    put_u8(&mut b, g.in_orbit);
    put_u16(&mut b, g.velocity.travel_angle);
    put_extent(&mut b, g.velocity.vector);
    put_extent(&mut b, g.velocity.fract);
    put_extent(&mut b, g.velocity.error);
    put_extent(&mut b, g.velocity.incr);
    b
}

fn canned_node(b: &mut Vec<u8>, node: &crate::state::planet_info::CannedNode) {
    assert!(node.is_valid(), "invalid internal canister record");
    put_u16(b, node.star);
    put_u8(b, node.planet);
    put_u8(b, node.moon);
    put_u8(b, node.node);
    put_u8(b, node.value);
    put_i16(b, node.x);
    put_i16(b, node.y);
}

fn race_entry(b: &mut Vec<u8>, e: &RaceEntry) {
    put_u16(b, e.index);
    put_u16(b, e.allied_state);
    put_u8(b, e.days_left);
    put_u8(b, e.growth_fract);
    put_u16(b, e.crew_level);
    put_u16(b, e.max_crew);
    put_u8(b, e.growth);
    put_u8(b, e.max_energy);
    put_point(b, e.loc);
    put_u16(b, e.actual_strength);
    put_u16(b, e.known_strength);
    put_point(b, e.known_loc);
    put_u8(b, e.growth_err_term);
    put_u8(b, e.func_index);
    put_point(b, e.dest_loc);
}

fn ip_group(b: &mut Vec<u8>, g: &IpGroup) {
    put_u16(b, g.group_counter);
    put_u8(b, g.race_id);
    put_u8(b, g.sys_loc);
    put_u8(b, g.task);
    put_u8(b, g.in_system);
    put_u8(b, g.dest_loc);
    put_u8(b, g.orbit_pos);
    put_u8(b, g.group_id);
    put_point(b, g.loc);
}

fn ship_entry(b: &mut Vec<u8>, e: &ShipFragmentEntry) {
    put_u16(b, e.race_index);
    put_u8(b, e.captains_name_index);
    put_u8(b, e.race_id);
    put_u8(b, e.index);
    put_u16(b, e.crew_level);
    put_u16(b, e.max_crew);
    put_u8(b, e.energy_level);
    put_u8(b, e.max_energy);
}

fn event_entry(b: &mut Vec<u8>, e: &EventEntry) {
    put_u8(b, e.day_index);
    put_u8(b, e.month_index);
    put_u16(b, e.year_index);
    put_u8(b, e.func_index);
}

fn encounter_entry(b: &mut Vec<u8>, e: &EncounterEntry) {
    put_i16(b, e.transition_state);
    put_point(b, e.origin);
    put_u16(b, e.radius);
    put_point(b, e.loc_pt);
    put_u8(b, e.race_id);
    put_u8(b, e.num_ships);
    put_u8(b, e.flags);
    for ship in &e.ship_list {
        put_u8(b, ship.race_id);
        put_u16(b, ship.crew_level);
        put_u16(b, ship.max_crew);
        put_u8(b, ship.max_energy);
    }
    put_i32(b, e.log_x);
    put_i32(b, e.log_y);
}

fn star_body(s: &StarDesc) -> Vec<u8> {
    let mut b = Vec::new();
    put_point(&mut b, s.star_pt);
    put_u8(&mut b, s.star_type);
    put_u8(&mut b, s.index);
    put_u8(&mut b, s.prefix);
    put_u8(&mut b, s.postfix);
    b
}

fn group_list_body(list: &GroupList) -> Result<Vec<u8>, EncodeError> {
    check_count(GROUP_LIST_TAG, list.entries.len(), MAX_BYTE_COUNT)?;
    let mut b = Vec::new();
    put_u8(&mut b, list.last_encounter);
    for entry in &list.entries {
        put_u8(&mut b, entry.race_outer);
        ip_group(&mut b, &entry.group);
    }
    Ok(b)
}

fn battle_group_body(g: &BattleGroup) -> Result<Vec<u8>, EncodeError> {
    check_count(BATTLE_GROUP_TAG, g.fleets.len(), MAX_BYTE_COUNT)?;
    let mut b = Vec::new();
    put_u32(&mut b, g.encounter_id);
    put_u8(&mut b, u8::from(g.current));
    put_u16(&mut b, g.star_index);
    put_u8(&mut b, g.day_index);
    put_u8(&mut b, g.month_index);
    put_u16(&mut b, g.year_index);
    put_u8(&mut b, g.fleets.len() as u8);
    for fleet in &g.fleets {
        check_count(BATTLE_GROUP_TAG, fleet.ships.len(), MAX_BYTE_COUNT)?;
        put_u8(&mut b, fleet.icon);
        put_u8(&mut b, fleet.ships.len() as u8);
        for ship in &fleet.ships {
            put_u8(&mut b, ship.race_outer);
            put_u8(&mut b, ship.captains_name_index);
            put_u8(&mut b, ship.race_id);
            put_u8(&mut b, ship.index);
            put_u16(&mut b, ship.crew_level);
            put_u16(&mut b, ship.max_crew);
            put_u8(&mut b, ship.energy_level);
            put_u8(&mut b, ship.max_energy);
        }
    }
    Ok(b)
}
