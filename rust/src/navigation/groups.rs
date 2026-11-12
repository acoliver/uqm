//! Save-compatible battle-group records and encounter group construction.

use crate::engine::world::queues::{IpGroup as IP_GROUP, ShipFragment as SHIP_FRAGMENT};
#[cfg(not(test))]
use crate::engine::StoryKey;
use crate::engine::World;
use uqm_save::statefiles::GroupHeader;

use crate::state::c_api::{read_state_file, write_state_file, GameStateFile};

unsafe fn read<const N: usize>(world: &mut World, file: *mut GameStateFile) -> [u8; N] {
    let mut bytes = [0; N];
    read_state_file(world, bytes.as_mut_ptr().cast(), 1, N as u16, file);
    bytes
}
unsafe fn write(world: &mut World, file: *mut GameStateFile, bytes: &[u8]) {
    write_state_file(world, bytes.as_ptr().cast(), 1, bytes.len() as u16, file);
}

unsafe fn read_group_header(world: &mut World, file: *mut GameStateFile, header: &mut GroupHeader) {
    decode_header(&read::<268>(world, file), header);
}

/// # Safety
/// `file` is an open state-file handle and `header` is readable.
pub unsafe fn write_group_header(
    world: &mut World,
    file: *mut GameStateFile,
    header: &GroupHeader,
) {
    write(world, file, &encode_header(header));
}

unsafe fn read_ship_fragment(
    world: &mut World,
    file: *mut GameStateFile,
    fragment: &mut SHIP_FRAGMENT,
) {
    decode_fragment(&read::<16>(world, file), fragment);
}
unsafe fn write_ship_fragment(
    world: &mut World,
    file: *mut GameStateFile,
    fragment: &SHIP_FRAGMENT,
) {
    write(world, file, &encode_fragment(fragment));
}
unsafe fn read_ip_group(world: &mut World, file: *mut GameStateFile, group: &mut IP_GROUP) {
    decode_group(&read::<16>(world, file), group);
}
unsafe fn write_ip_group(world: &mut World, file: *mut GameStateFile, group: &IP_GROUP) {
    write(world, file, &encode_group(group));
}

fn decode_header(bytes: &[u8; 268], header: &mut GroupHeader) {
    header.num_groups = bytes[0];
    header.day_index = bytes[1];
    header.month_index = bytes[2];
    header.star_index = u16::from_ne_bytes([bytes[4], bytes[5]]);
    header.year_index = u16::from_ne_bytes([bytes[6], bytes[7]]);
    for (offset, bytes) in header
        .group_offset
        .iter_mut()
        .zip(bytes[8..].as_chunks::<4>().0)
    {
        *offset = u32::from_ne_bytes(*bytes);
    }
}

fn encode_header(header: &GroupHeader) -> [u8; 268] {
    let mut bytes = [0; 268];
    bytes[..3].copy_from_slice(&[header.num_groups, header.day_index, header.month_index]);
    bytes[4..6].copy_from_slice(&header.star_index.to_ne_bytes());
    bytes[6..8].copy_from_slice(&header.year_index.to_ne_bytes());
    for (bytes, offset) in bytes[8..]
        .as_chunks_mut::<4>()
        .0
        .iter_mut()
        .zip(header.group_offset)
    {
        *bytes = offset.to_ne_bytes();
    }
    bytes
}

fn decode_fragment(bytes: &[u8; 16], fragment: &mut SHIP_FRAGMENT) {
    fragment.captains_name_index = bytes[2];
    fragment.race_id = bytes[6];
    fragment.index = bytes[7];
    fragment.crew_level = bytes[8] as u16;
    fragment.max_crew = bytes[9] as u16;
    fragment.energy_level = bytes[10];
    fragment.max_energy = bytes[11];
}

fn encode_fragment(fragment: &SHIP_FRAGMENT) -> [u8; 16] {
    [
        0,
        0,
        fragment.captains_name_index,
        0,
        0,
        0,
        fragment.race_id,
        fragment.index,
        fragment.crew_level as u8,
        fragment.max_crew as u8,
        fragment.energy_level,
        fragment.max_energy,
        0,
        0,
        0,
        0,
    ]
}

fn decode_group(bytes: &[u8; 16], group: &mut IP_GROUP) {
    group.group_counter = u16::from_ne_bytes([bytes[4], bytes[5]]);
    group.race_id = bytes[6];
    group.sys_loc = bytes[7] & 15;
    group.task = bytes[7] >> 4;
    group.in_system = bytes[8];
    group.dest_loc = bytes[10] & 15;
    group.orbit_pos = bytes[10] >> 4;
    group.group_id = bytes[11];
    group.loc.x = i16::from_ne_bytes([bytes[12], bytes[13]]);
    group.loc.y = i16::from_ne_bytes([bytes[14], bytes[15]]);
}

fn encode_group(group: &IP_GROUP) -> [u8; 16] {
    assert!(group.sys_loc < 16 && group.task < 16);
    assert!(group.dest_loc < 16 && group.orbit_pos < 16);
    let mut bytes = [0; 16];
    bytes[4..6].copy_from_slice(&group.group_counter.to_ne_bytes());
    bytes[6..12].copy_from_slice(&[
        group.race_id,
        group.sys_loc | (group.task << 4),
        group.in_system,
        0,
        group.dest_loc | (group.orbit_pos << 4),
        group.group_id,
    ]);
    bytes[12..14].copy_from_slice(&group.loc.x.to_ne_bytes());
    bytes[14..16].copy_from_slice(&group.loc.y.to_ne_bytes());
    bytes
}

fn sine(angle: u32, magnitude: i32) -> i32 {
    (crate::math::sinetab[(angle & 63) as usize] as i32 * magnitude) >> 14
}
fn cosine(angle: u32, magnitude: i32) -> i32 {
    sine(angle + 16, magnitude)
}

#[cfg(not(test))]
pub(crate) mod live {
    use super::*;
    use crate::comm::dispatch::ship::{SLYLANDRO_SHIP, SUPOX_SHIP, UTWIG_SHIP};
    use crate::engine::world::queues::IpGroup as IP_GROUP;
    use crate::engine::{
        graphics::native_abi::POINT,
        math::battle_types::{CIRCLE_SHIFT, FACING_SHIFT},
        save::groups::MAX_BATTLE_GROUPS,
        world::{
            fleet::SPHERE_RADIUS_INCREMENT,
            queues::{FleetInfo as FLEET_INFO, MAX_REVOLUTIONS, STATION_RADIUS},
        },
    };
    use crate::math::{random as TFB_Random, square_root};
    use crate::navigation::get;
    use crate::planets::generate::identities::*;

    use crate::state::c_api::{
        close_state_file, delete_state_file, length_state_file, open_state_file, seek_state_file,
    };
    use std::mem;

    const ENCOUNTER_PERCENT: [u8; 24] = [
        0, 0, 0, 20, 20, 0, 10, 20, 20, 20, 20, 40, 0, 20, 60, 20, 5, 20, 40, 0, 0, 40, 40, 0,
    ];
    const HOME_SYSTEMS: [u8; 23] = [
        0,
        0,
        0,
        ORZ_DEFINED,
        PKUNK_DEFINED,
        0,
        SPATHI_DEFINED,
        SUPOX_DEFINED,
        THRADD_DEFINED,
        UTWIG_DEFINED,
        VUX_DEFINED,
        YEHAT_DEFINED,
        0,
        DRUUGE_DEFINED,
        ILWRATH_DEFINED,
        MYCON_DEFINED,
        0,
        TALKING_PET_DEFINED,
        0,
        ZOQFOT_DEFINED,
        0,
        0,
        0,
    ];
    const ENCOUNTER_MAKEUP: [u8; 23] = [
        0x51, 0, 0, 0x51, 0x51, 0, 0x51, 0x51, 0x51, 0x51, 0x51, 0x51, 0x11, 0x51, 0x51, 0x51,
        0x11, 0x51, 0x51, 0x51, 0, 0x51, 0x51,
    ];
    unsafe fn open(world: &mut World, offset: u32, group: u8) -> *mut GameStateFile {
        open_state_file(
            world,
            if offset != 0 && group != 0 { 2 } else { 1 },
            c"r+b".as_ptr(),
        )
    }
    fn empty_header() -> GroupHeader {
        let mut header: GroupHeader = unsafe { mem::zeroed() };
        header.star_index = u16::MAX;
        header
    }
    unsafe fn seek(world: &mut World, file: *mut GameStateFile, offset: u32) {
        seek_state_file(world, file, offset as libc::c_long, 0);
    }
    fn validate_date(
        world: &World,
        relative: bool,
        month: &mut u16,
        day: &mut u16,
        year: &mut u16,
    ) -> bool {
        let clock = &world.data.game.game_clock;
        let now = (
            clock.year_index,
            u16::from(clock.month_index),
            u16::from(clock.day_index),
        );
        let date = crate::time::clock_abi::event_date(now, relative, (*year, *month, *day));
        if relative {
            (*year, *month, *day) = date;
        }
        date >= now
    }

    /// # Safety
    /// State storage is initialized; game-thread access only.
    pub unsafe fn init_group_info(world: &mut crate::engine::World, first: u32) {
        let file = open_state_file(world, 1, c"wb".as_ptr());
        if !file.is_null() {
            super::write_group_header(world, file, &empty_header());
            close_state_file(world, file);
        }
        if first != 0 {
            let file = open_state_file(world, 2, c"wb".as_ptr());
            if !file.is_null() {
                write_state_file(world, [0u8].as_ptr().cast(), 1, 1, file);
                close_state_file(world, file);
            }
        }
    }
    /// # Safety
    /// State files must no longer be in use.
    pub fn uninit_group_info(world: &mut crate::engine::World) {
        delete_state_file(world, 2);
        delete_state_file(world, 1);
    }

    pub fn build_group(
        world: &mut World,
        race: u8,
    ) -> Option<crate::engine::battle::link_queue::LinkId<crate::engine::world::queues::Group>>
    {
        let fleets = world.avail_races_ref();
        let icon = fleets.get(fleets.at(usize::from(race))?).melee_icon;
        let mut groups = world.ip_groups();
        let id = groups.alloc()?;
        groups.get_mut(id).race_id = race;
        groups.get_mut(id).melee_icon = icon;
        groups.put(id);
        Some(id)
    }
    fn clone_npc(
        world: &mut World,
        race: u8,
    ) -> Option<crate::engine::battle::link_queue::LinkId<crate::engine::world::queues::Fragment>>
    {
        let fleets = world.avail_races_ref();
        let template = *fleets.get(fleets.at(usize::from(race))?);
        world.npc_built_ships().clone_fragment(
            &template,
            race,
            0,
            crate::comm::dispatch::production_captain_name,
        )
    }

    fn encounter_chance(fleet: &FLEET_INFO, universe: POINT, base: u16) -> Option<u16> {
        let infinite = fleet.actual_strength == u16::MAX;
        let radius = if infinite {
            20000
        } else {
            ((fleet.actual_strength as u32 * SPHERE_RADIUS_INCREMENT) >> 1) as u16
        };
        let dx = universe.x.wrapping_sub(fleet.loc.x).wrapping_abs() as u16;
        let dy = universe.y.wrapping_sub(fleet.loc.y).wrapping_abs() as u16;
        let squared = dx as u32 * dx as u32 + dy as u32 * dy as u32;
        if dx >= radius || dy >= radius || squared >= radius as u32 * radius as u32 {
            return None;
        }
        Some(if infinite {
            base
        } else {
            70 - (square_root(squared) as u32 * 60 / radius as u32) as u16
        })
    }

    unsafe fn select_encounter(world: &World, home: &[u8; 23], percent: &[u8; 24]) -> (usize, u16) {
        let universe = (*world.star_pointer()).star_pt;
        let mut best_index = 0usize;
        let mut best_percent = 0u16;
        let mut handle = world.avail_races_ref().head();
        let mut index = 0usize;
        while let Some(id) = handle {
            let fleets = world.avail_races_ref();
            let fleet = fleets.get(id);
            let next = fleets.succ(id);
            let radius = fleet.actual_strength;
            let chance = percent[index] as u16;
            if radius != 0 && chance != 0 {
                if home[index] != 0 && (*world.star_pointer()).index == home[index] {
                    best_index = index;
                    best_percent = if home[index] == SPATHI_DEFINED || home[index] == SUPOX_DEFINED
                    {
                        2
                    } else {
                        70
                    };
                    break;
                }
                if let Some(mut chance) = encounter_chance(fleet, universe, chance) {
                    let random = TFB_Random();
                    if (random as u16 % 100) < chance
                        && (best_percent == 0
                            || ((random >> 16) % (chance + best_percent) as u32) < chance as u32)
                    {
                        if fleet.actual_strength == u16::MAX {
                            chance = 4;
                        }
                        best_percent = chance;
                        best_index = index;
                    }
                }
            }
            handle = next;
            index += 1;
        }
        (best_index, best_percent)
    }

    /// # Safety
    /// A current star, solar system, and initialized world queues are required.
    pub unsafe fn build_groups(
        world: &mut World,
        solar: &crate::engine::world::generation::SolarSnapshot,
    ) {
        let mut home = HOME_SYSTEMS;
        let mut percent = ENCOUNTER_PERCENT;
        percent[SLYLANDRO_SHIP as usize] = percent[SLYLANDRO_SHIP as usize]
            .wrapping_mul(get(world, StoryKey::SLYLANDRO_MULTIPLIER));
        let mission = get(world, StoryKey::UTWIG_SUPOX_MISSION);
        if mission > 1 && mission < 5 {
            home[UTWIG_SHIP as usize] = 0;
            home[SUPOX_SHIP as usize] = 0;
        }
        let (best_index, best_percent) = select_encounter(world, &home, &percent);
        if best_percent != 0 {
            let mut number = ((TFB_Random() as u16 % (best_percent >> 1)) + 1) as u8;
            if number > MAX_BATTLE_GROUPS {
                number = MAX_BATTLE_GROUPS;
            } else if number < 5
                && home[best_index] != 0
                && (*world.star_pointer()).index == home[best_index]
            {
                number = 5;
            }
            let mut which = 0u8;
            while number != 0 {
                let makeup = ENCOUNTER_MAKEUP[best_index];
                for ships in (1..=makeup >> 4).rev() {
                    if ships <= makeup & 15 || (TFB_Random() as u16 % 100) < 50 {
                        clone_npc(world, best_index as u8);
                    }
                }
                which += 1;
                put_group_info(world, 0, which);
                world.npc_built_ships().reinit();
                number -= 1;
            }
        }
        initialize_solar_groups(world, 0, solar);
    }

    /// # Safety
    /// The game thread retains the active solar system and the group's queue record.
    unsafe fn initialize_group(
        group: &mut IP_GROUP,
        which: u8,
        solar: &crate::engine::world::generation::SolarSnapshot,
    ) {
        group.group_id = which;
        group.in_system = 1;
        let random = TFB_Random();
        let mut task = random as u8 % IP_GROUP::ON_STATION;
        if task == IP_GROUP::FLEE {
            task = IP_GROUP::ON_STATION;
        }
        group.orbit_pos = (random >> 16) as u8 & 15;
        let mut location = solar.planet_count;
        if location == 1 && task == IP_GROUP::EXPLORE {
            task = IP_GROUP::IN_ORBIT;
        } else {
            location = ((random >> 8) as u8 % location) + 1;
        }
        group.dest_loc = location;
        let random = TFB_Random();
        group.loc.x = ((random as u16 % 10000) as i32 - 5000) as i16;
        group.loc.y = (((random >> 16) % 10000) as i32 - 5000) as i16;
        group.group_counter = 0;
        if task == IP_GROUP::EXPLORE {
            group.group_counter = (TFB_Random() as u16 % MAX_REVOLUTIONS as u16) << FACING_SHIFT;
        } else if task == IP_GROUP::ON_STATION {
            let origin = crate::planets::solar_coords::outer_location(solar, location as u16 - 1);
            let angle = (group.orbit_pos as u32 + 1) << (CIRCLE_SHIFT - FACING_SHIFT);
            group.loc.x = (origin.x as i32 + cosine(angle, STATION_RADIUS as i32)) as i16;
            group.loc.y = (origin.y as i32 + sine(angle, STATION_RADIUS as i32)) as i16;
            location = 0;
        }
        group.task = task;
        group.sys_loc = location;
    }

    unsafe fn flush(
        world: &mut World,

        header: &mut GroupHeader,
        offset: u32,
        which: u8,
        file: *mut GameStateFile,
    ) {
        if which == 0 {
            if header.group_offset[0] == 0 {
                header.group_offset[0] = length_state_file(world, file);
            }
            assert_eq!(world.npc_built_ships_ref().count(), 0);
            let mut next = world.ip_groups_ref().head();
            while let Some(id) = next {
                let groups = world.ip_groups_ref();
                let group = *groups.get(id);
                next = groups.succ(id);
                if group.in_system == 0 {
                    if world.data.game.battle_group_ref != 0 {
                        put_group_info(world, world.data.game.battle_group_ref, group.group_id);
                    } else {
                        flush(world, header, 0, group.group_id, file);
                    }
                    header.group_offset[usize::from(group.group_id)] = 0;
                    world.ip_groups().remove(id);
                    world.ip_groups().free(id);
                }
            }
        } else if which > header.num_groups {
            header.num_groups = which;
            header.group_offset[usize::from(which)] = length_state_file(world, file);
        }
        seek(world, file, offset);
        super::write_group_header(world, file, header);
        if which == 0 {
            seek(world, file, header.group_offset[0]);
            write(
                world,
                file,
                &[
                    world.last_encounter_group,
                    world.ip_groups_ref().count() as u8,
                ],
            );
            let mut next = world.ip_groups_ref().head();
            while let Some(id) = next {
                let groups = world.ip_groups_ref();
                let group = *groups.get(id);
                next = groups.succ(id);
                write(world, file, &[group.race_id]);
                super::write_ip_group(world, file, &group);
            }
        } else {
            let ships = world.npc_built_ships_ref();
            let race = ships.head().map_or(0, |id| ships.get(id).race_id);
            let count = ships.count() as u8;
            seek(world, file, header.group_offset[usize::from(which)]);
            write(world, file, &[race, count]);
            let mut next = world.npc_built_ships_ref().head();
            while let Some(id) = next {
                let ships = world.npc_built_ships_ref();
                let fragment = *ships.get(id);
                next = ships.succ(id);
                write(world, file, &[fragment.race_id]);
                super::write_ship_fragment(world, file, &fragment);
            }
        }
    }

    unsafe fn initialize_saved_groups(
        world: &mut World,
        file: *mut GameStateFile,
        offset: u32,
        header: &GroupHeader,
        solar: &crate::engine::world::generation::SolarSnapshot,
    ) -> u32 {
        world.ip_groups().reinit();
        let mut day = header.day_index as u16;
        let mut month = header.month_index as u16;
        let mut year = header.year_index;
        if offset == 0
            && (header.star_index
                != world
                    .current_star
                    .unwrap_or_else(|| panic!("group state requires a selected star"))
                    .0 as u16
                || !validate_date(world, false, &mut month, &mut day, &mut year))
        {
            close_state_file(world, file);
            let file = open_state_file(world, 1, c"wb".as_ptr());
            super::write_group_header(world, file, &empty_header());
            close_state_file(world, file);
            return 0;
        }
        for which in 1..=header.num_groups {
            if header.group_offset[which as usize] == 0 {
                continue;
            }
            seek(world, file, header.group_offset[which as usize]);
            let race = read::<1>(world, file)[0];
            let ships = read::<1>(world, file)[0];
            if ships == 0 {
                continue;
            }
            let id = build_group(world, race).unwrap_or_else(|| panic!("IP group capacity"));
            initialize_group(world.ip_groups().get_mut(id), which, solar);
        }
        if offset != 0 {
            init_group_info(world, 0);
        } else if validate_date(world, false, &mut month, &mut day, &mut year) {
            close_state_file(world, file);
            return 1;
        }
        close_state_file(world, file);
        u32::from(world.ip_groups_ref().head().is_some())
    }

    pub unsafe fn initialize_solar_groups(
        world: &mut World,
        offset: u32,
        solar: &crate::engine::world::generation::SolarSnapshot,
    ) -> u32 {
        let file = open(world, offset, u8::MAX);
        if file.is_null() {
            return 0;
        }
        seek(world, file, offset);
        let mut header = empty_header();
        super::read_group_header(world, file, &mut header);
        initialize_saved_groups(world, file, offset, &header, solar)
    }

    /// # Safety
    /// Current world and solar-system queues must be initialized and exclusively accessible.
    pub unsafe fn get_group_info(world: &mut World, offset: u32, which: u8) -> u32 {
        let file = open(world, offset, which);
        if file.is_null() {
            return 0;
        }
        seek(world, file, offset);
        let mut header: GroupHeader = mem::zeroed();
        super::read_group_header(world, file, &mut header);
        assert_ne!(
            which,
            u8::MAX,
            "solar group initialization requires supplied geometry"
        );
        if header.group_offset[which as usize] == 0 {
            close_state_file(world, file);
            return 0;
        }
        if which == 0 {
            let left = world.npc_built_ships_ref().count();
            seek(world, file, header.group_offset[0]);
            world.last_encounter_group = read::<1>(world, file)[0];
            if world.last_encounter_group != 0 {
                if world.data.game.battle_group_ref != 0 {
                    put_group_info(
                        world,
                        world.data.game.battle_group_ref,
                        world.last_encounter_group,
                    );
                } else {
                    flush(world, &mut header, offset, world.last_encounter_group, file);
                }
            }
            world.npc_built_ships().reinit();
            world.ip_groups().reinit();
            seek(world, file, header.group_offset[0] + 1);
            let mut number = read::<1>(world, file)[0];
            while number != 0 {
                let race = read::<1>(world, file)[0];
                let id =
                    build_group(world, race).unwrap_or_else(|| panic!("saved IP group capacity"));
                let mut group = *world.ip_groups_ref().get(id);
                super::read_ip_group(world, file, &mut group);
                *world.ip_groups().get_mut(id) = group;
                if group.group_id == world.last_encounter_group && left == 0 {
                    world.ip_groups().remove(id);
                    world.ip_groups().free(id);
                }
                number -= 1;
            }
            close_state_file(world, file);
            u32::from(world.ip_groups_ref().head().is_some())
        } else {
            world.last_encounter_group = which;
            if offset != 0 {
                put_group_info(world, 0, 0);
            } else {
                flush(world, &mut header, 0, 0, file);
            }
            world.ip_groups().reinit();
            world.npc_built_ships().reinit();
            seek(world, file, header.group_offset[which as usize] + 1);
            let mut number = read::<1>(world, file)[0];
            while number != 0 {
                let race = read::<1>(world, file)[0];
                let id =
                    clone_npc(world, race).unwrap_or_else(|| panic!("saved NPC fragment capacity"));
                let mut fragment = *world.npc_built_ships_ref().get(id);
                super::read_ship_fragment(world, file, &mut fragment);
                *world.npc_built_ships().get_mut(id) = fragment;
                number -= 1;
            }
            close_state_file(world, file);
            u32::from(world.npc_built_ships_ref().head().is_some())
        }
    }

    /// # Safety
    /// Current world queues and current star descriptor must be initialized.
    pub unsafe fn put_group_info(world: &mut World, mut offset: u32, mut which: u8) -> u32 {
        let file = open(world, offset, which);
        if file.is_null() {
            return offset;
        }
        let mut header: GroupHeader = mem::zeroed();
        if offset == u32::MAX {
            offset = length_state_file(world, file);
            seek(world, file, offset);
            header = empty_header();
            super::write_group_header(world, file, &header);
        }
        if which != 0 {
            seek(world, file, offset);
            if which == u8::MAX {
                world.last_encounter_group = 0;
                which = 0;
            }
        }
        super::read_group_header(world, file, &mut header);
        let mut month = 0;
        let mut day = 7;
        let mut year = 0;
        validate_date(world, true, &mut month, &mut day, &mut year);
        header.day_index = day as u8;
        header.month_index = month as u8;
        header.year_index = year;
        header.star_index = world
            .current_star
            .unwrap_or_else(|| panic!("group state requires a selected star"))
            .0 as u16;
        flush(world, &mut header, offset, which, file);
        close_state_file(world, file);
        offset
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::c_api::*;

    #[test]
    fn on_station_positions_use_the_full_rust_sine_table() {
        assert_eq!((cosine(0, 1600), sine(0, 1600)), (0, -1600));
        assert_eq!((cosine(16, 1600), sine(16, 1600)), (1600, 0));
        assert_eq!(sine(68, 1600), -1479);
    }

    #[test]
    #[serial_test::serial]
    fn group_and_fragment_records_keep_the_legacy_sixteen_byte_wire_layout() {
        let mut owner = World::new();
        let world = &mut owner;
        // SAFETY: the local World owns every state-file handle used below. These
        // zeroable ABI records and their stack pointers stay live for each call.
        unsafe {
            crate::state::ffi::rust_init_game_state(world);
            let file = open_state_file(world, 1, c"wb".as_ptr());
            let mut group: IP_GROUP = std::mem::zeroed();
            group.group_counter = 0x1234;
            group.race_id = 4;
            group.sys_loc = 2;
            group.task = 9;
            group.in_system = 7;
            group.dest_loc = 3;
            group.orbit_pos = 15;
            group.group_id = 12;
            group.loc.x = -123;
            group.loc.y = 456;
            super::write_ip_group(world, file, &group);
            assert_eq!(length_state_file(world, file), 16);
            seek_state_file(world, file, 0, 0);
            let bytes = read::<16>(world, file);
            assert_eq!(
                &bytes[..12],
                &[0, 0, 0, 0, 0x34, 0x12, 4, 0x92, 7, 0, 0xf3, 12]
            );
            seek_state_file(world, file, 0, 0);
            let mut restored: IP_GROUP = std::mem::zeroed();
            super::read_ip_group(world, file, &mut restored);
            assert_eq!(
                (
                    restored.loc.x,
                    restored.loc.y,
                    restored.task,
                    restored.orbit_pos
                ),
                (-123, 456, 9, 15)
            );
            seek_state_file(world, file, 0, 0);
            let mut ship: SHIP_FRAGMENT = std::mem::zeroed();
            ship.crew_level = 300;
            ship.max_crew = 511;
            ship.race_id = 6;
            super::write_ship_fragment(world, file, &ship);
            seek_state_file(world, file, 0, 0);
            super::read_ship_fragment(world, file, &mut ship);
            assert_eq!((ship.crew_level, ship.max_crew, ship.race_id), (44, 255, 6));
            let mut header: GroupHeader = std::mem::zeroed();
            header.num_groups = 16;
            header.star_index = 0x1234;
            header.year_index = 2160;
            header.group_offset[64] = 0xabcdef01;
            seek_state_file(world, file, 0, 0);
            super::write_group_header(world, file, &header);
            assert_eq!(length_state_file(world, file), 268);
            seek_state_file(world, file, 0, 0);
            let mut restored_header: GroupHeader = std::mem::zeroed();
            super::read_group_header(world, file, &mut restored_header);
            assert_eq!(encode_header(&restored_header), encode_header(&header));
            close_state_file(world, file);
            delete_state_file(world, 1);
        }
    }
}

#[cfg(test)]
mod record_tests {
    use super::*;

    #[test]
    fn header_bytes_keep_native_endian_fields_and_zero_padding() {
        // SAFETY: this record contains only integers, nullable pointers and optional callbacks.
        let mut header: GroupHeader = unsafe { std::mem::zeroed() };
        header.num_groups = 2;
        header.day_index = 3;
        header.month_index = 4;
        header.star_index = 0x0506;
        header.year_index = 0x0708;
        for (i, offset) in header.group_offset.iter_mut().enumerate() {
            *offset = 0x12340000 + i as u32;
        }
        let bytes = encode_header(&header);
        assert_eq!(&bytes[..4], &[2, 3, 4, 0]);
        assert_eq!(&bytes[4..6], &0x0506u16.to_ne_bytes());
        assert_eq!(&bytes[6..8], &0x0708u16.to_ne_bytes());
        for (i, chunk) in bytes[8..].as_chunks::<4>().0.iter().enumerate() {
            assert_eq!(*chunk, (0x12340000 + i as u32).to_ne_bytes());
        }
        // SAFETY: this record contains only integers, nullable pointers and optional callbacks.
        let mut restored: GroupHeader = unsafe { std::mem::zeroed() };
        decode_header(&bytes, &mut restored);
        assert_eq!(restored.group_offset, header.group_offset);
        assert_eq!(restored.star_index, header.star_index);
        assert_eq!(restored.year_index, header.year_index);
        assert_eq!(restored.num_groups, header.num_groups);
    }

    #[test]
    fn fragment_bytes_keep_truncation_and_do_not_replace_link_or_resources() {
        let mut world = crate::engine::World::new();
        let mut queue = world.built_ships();
        let first = queue.alloc().unwrap();
        let middle = queue.alloc().unwrap();
        let last = queue.alloc().unwrap();
        queue.put(first);
        queue.put(middle);
        queue.put(last);
        let fragment = queue.get_mut(middle);
        fragment.captains_name_index = 3;
        fragment.race_id = 4;
        fragment.index = 5;
        fragment.crew_level = 300;
        fragment.max_crew = 511;
        fragment.energy_level = 6;
        fragment.max_energy = 7;
        fragment.race_strings = 0x100usize as crate::graphics::native_abi::STRING;
        fragment.icons = 0x200usize as crate::graphics::native_abi::FRAME;
        let bytes = encode_fragment(fragment);
        assert_eq!(bytes, [0, 0, 3, 0, 0, 0, 4, 5, 44, 255, 6, 7, 0, 0, 0, 0]);
        decode_fragment(&bytes, fragment);
        assert_eq!(fragment.race_strings as usize, 0x100);
        assert_eq!(fragment.icons as usize, 0x200);
        assert_eq!(fragment.crew_level, 44);
        assert_eq!(fragment.max_crew, 255);
        assert_eq!(fragment.captains_name_index, 3);
        assert_eq!(queue.pred(middle), Some(first));
        assert_eq!(queue.succ(middle), Some(last));
    }

    #[test]
    fn ip_group_bytes_pack_nibbles_and_signed_coordinates() {
        // SAFETY: this record contains only integers, nullable pointers and optional callbacks.
        let mut group: IP_GROUP = unsafe { std::mem::zeroed() };
        group.group_counter = 0x1234;
        group.race_id = 2;
        group.sys_loc = 12;
        group.task = 3;
        group.in_system = 4;
        group.dest_loc = 5;
        group.orbit_pos = 15;
        group.group_id = 6;
        group.loc.x = -123;
        group.loc.y = 456;
        let bytes = encode_group(&group);
        assert_eq!(&bytes[..4], &[0; 4]);
        assert_eq!(&bytes[4..6], &0x1234u16.to_ne_bytes());
        assert_eq!(&bytes[6..12], &[2, 0x3c, 4, 0, 0xf5, 6]);
        assert_eq!(&bytes[12..14], &(-123i16).to_ne_bytes());
        assert_eq!(&bytes[14..16], &456i16.to_ne_bytes());
        // SAFETY: this record contains only integers, nullable pointers and optional callbacks.
        let mut restored: IP_GROUP = unsafe { std::mem::zeroed() };
        decode_group(&bytes, &mut restored);
        assert_eq!(restored.sys_loc, group.sys_loc);
        assert_eq!(restored.task, group.task);
        assert_eq!(restored.orbit_pos, group.orbit_pos);
        assert_eq!(restored.loc.x, group.loc.x);
        assert_eq!(restored.loc.y, group.loc.y);
    }
}
