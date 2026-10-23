//! Save-compatible battle-group records and encounter group construction.

#![allow(non_snake_case)]

use super::abi::{GROUP_HEADER, IP_GROUP, SHIP_FRAGMENT};
use crate::state::c_api::{GameStateFile, ReadStateFile, WriteStateFile};

unsafe fn read<const N: usize>(file: *mut GameStateFile) -> [u8; N] {
    let mut bytes = [0; N];
    ReadStateFile(bytes.as_mut_ptr().cast(), 1, N as u16, file);
    bytes
}
unsafe fn write(file: *mut GameStateFile, bytes: &[u8]) {
    WriteStateFile(bytes.as_ptr().cast(), 1, bytes.len() as u16, file);
}

/// # Safety
/// `file` is an open state-file handle and `header` is writable.
#[no_mangle]
pub unsafe extern "C" fn ReadGroupHeader(file: *mut GameStateFile, header: *mut GROUP_HEADER) {
    let bytes = read::<268>(file);
    (*header).NumGroups = bytes[0];
    (*header).day_index = bytes[1];
    (*header).month_index = bytes[2];
    (*header).star_index = u16::from_ne_bytes([bytes[4], bytes[5]]);
    (*header).year_index = u16::from_ne_bytes([bytes[6], bytes[7]]);
    for (offset, bytes) in (*header)
        .GroupOffset
        .iter_mut()
        .zip(bytes[8..].as_chunks::<4>().0)
    {
        *offset = u32::from_ne_bytes(*bytes);
    }
}
/// # Safety
/// `file` is an open state-file handle and `header` is readable.
#[no_mangle]
pub unsafe extern "C" fn WriteGroupHeader(file: *mut GameStateFile, header: *const GROUP_HEADER) {
    write(
        file,
        &[
            (*header).NumGroups,
            (*header).day_index,
            (*header).month_index,
            0,
        ],
    );
    write(file, &(*header).star_index.to_ne_bytes());
    write(file, &(*header).year_index.to_ne_bytes());
    for offset in (*header).GroupOffset {
        write(file, &offset.to_ne_bytes());
    }
}
/// # Safety
/// `file` is an open state-file handle and `fragment` is writable.
#[no_mangle]
pub unsafe extern "C" fn ReadShipFragment(file: *mut GameStateFile, fragment: *mut SHIP_FRAGMENT) {
    let bytes = read::<16>(file);
    (*fragment).captains_name_index = bytes[2];
    (*fragment).race_id = bytes[6];
    (*fragment).index = bytes[7];
    (*fragment).crew_level = bytes[8] as u16;
    (*fragment).max_crew = bytes[9] as u16;
    (*fragment).energy_level = bytes[10];
    (*fragment).max_energy = bytes[11];
}
/// # Safety
/// `file` is an open state-file handle and `fragment` is readable.
#[no_mangle]
pub unsafe extern "C" fn WriteShipFragment(
    file: *mut GameStateFile,
    fragment: *const SHIP_FRAGMENT,
) {
    write(
        file,
        &[
            0,
            0,
            (*fragment).captains_name_index,
            0,
            0,
            0,
            (*fragment).race_id,
            (*fragment).index,
            (*fragment).crew_level as u8,
            (*fragment).max_crew as u8,
            (*fragment).energy_level,
            (*fragment).max_energy,
            0,
            0,
            0,
            0,
        ],
    );
}
/// # Safety
/// `file` is an open state-file handle and `group` is writable.
#[no_mangle]
pub unsafe extern "C" fn ReadIpGroup(file: *mut GameStateFile, group: *mut IP_GROUP) {
    let bytes = read::<16>(file);
    (*group).group_counter = u16::from_ne_bytes([bytes[4], bytes[5]]);
    (*group).race_id = bytes[6];
    (*group).sys_loc = bytes[7] & 15;
    (*group).task = bytes[7] >> 4;
    (*group).in_system = bytes[8];
    (*group).dest_loc = bytes[10] & 15;
    (*group).orbit_pos = bytes[10] >> 4;
    (*group).group_id = bytes[11];
    (*group).loc.x = i16::from_ne_bytes([bytes[12], bytes[13]]);
    (*group).loc.y = i16::from_ne_bytes([bytes[14], bytes[15]]);
}
/// # Safety
/// `file` is an open state-file handle and `group` is readable, with four-bit task/location fields.
#[no_mangle]
pub unsafe extern "C" fn WriteIpGroup(file: *mut GameStateFile, group: *const IP_GROUP) {
    assert!((*group).sys_loc < 16 && (*group).task < 16);
    assert!((*group).dest_loc < 16 && (*group).orbit_pos < 16);
    write(file, &[0, 0, 0, 0]);
    write(file, &(*group).group_counter.to_ne_bytes());
    write(
        file,
        &[
            (*group).race_id,
            (*group).sys_loc | ((*group).task << 4),
            (*group).in_system,
            0,
            (*group).dest_loc | ((*group).orbit_pos << 4),
            (*group).group_id,
        ],
    );
    write(file, &(*group).loc.x.to_ne_bytes());
    write(file, &(*group).loc.y.to_ne_bytes());
}

fn sine(angle: u32, magnitude: i32) -> i32 {
    (crate::math::sinetab[(angle & 63) as usize] as i32 * magnitude) >> 14
}
fn cosine(angle: u32, magnitude: i32) -> i32 {
    sine(angle + 16, magnitude)
}

#[cfg(any(not(test), feature = "linked_c_archive"))]
mod live {
    use super::*;
    use crate::navigation::abi::*;
    use crate::navigation::get;
    use crate::navigation::globals::GlobData;
    use crate::planets::starmap::{star_array, CurStarDescPtr};
    use crate::state::c_api::{
        CloseStateFile, DeleteStateFile, LengthStateFile, OpenStateFile, SeekStateFile,
    };
    use std::{mem, ptr};

    static mut LAST_ENCOUNTER_GROUP: u8 = 0;
    const ENCOUNTER_PERCENT: [u8; 24] = [
        0, 0, 0, 20, 20, 0, 10, 20, 20, 20, 20, 40, 0, 20, 60, 20, 5, 20, 40, 0, 0, 40, 40, 0,
    ];
    const ENCOUNTER_MAKEUP: [u8; 23] = [
        0x51, 0, 0, 0x51, 0x51, 0, 0x51, 0x51, 0x51, 0x51, 0x51, 0x51, 0x11, 0x51, 0x51, 0x51,
        0x11, 0x51, 0x51, 0x51, 0, 0x51, 0x51,
    ];
    unsafe fn ip_queue() -> *mut QUEUE {
        ptr::addr_of_mut!(GlobData.Game_state.ip_group_q)
    }
    unsafe fn npc_queue() -> *mut QUEUE {
        ptr::addr_of_mut!(GlobData.Game_state.npc_built_ship_q)
    }
    unsafe fn fleet_queue() -> *mut QUEUE {
        ptr::addr_of_mut!(GlobData.Game_state.avail_race_q)
    }
    unsafe fn count(queue: *const QUEUE) -> u8 {
        let mut handle = (*queue).head;
        let mut count = 0u8;
        while !handle.is_null() {
            count = count.wrapping_add(1);
            handle = (*handle.cast::<LINK>()).succ;
        }
        count
    }
    unsafe fn open(offset: u32, group: u8) -> *mut GameStateFile {
        OpenStateFile(
            if offset != 0 && group != 0 { 2 } else { 1 },
            c"r+b".as_ptr(),
        )
    }
    fn empty_header() -> GROUP_HEADER {
        let mut header: GROUP_HEADER = unsafe { mem::zeroed() };
        header.star_index = u16::MAX;
        header
    }
    unsafe fn seek(file: *mut GameStateFile, offset: u32) {
        SeekStateFile(file, offset as libc::c_long, 0);
    }

    /// # Safety
    /// State storage is initialized; game-thread access only.
    #[no_mangle]
    pub unsafe extern "C" fn InitGroupInfo(first: u32) {
        let file = OpenStateFile(1, c"wb".as_ptr());
        if !file.is_null() {
            super::WriteGroupHeader(file, &empty_header());
            CloseStateFile(file);
        }
        if first != 0 {
            let file = OpenStateFile(2, c"wb".as_ptr());
            if !file.is_null() {
                write(file, &[0]);
                CloseStateFile(file);
            }
        }
    }
    /// # Safety
    /// State files must no longer be in use.
    #[no_mangle]
    pub unsafe extern "C" fn UninitGroupInfo() {
        DeleteStateFile(2);
        DeleteStateFile(1);
    }

    /// # Safety
    /// `queue` is an initialized IP_GROUP queue; race identifies an available fleet.
    #[no_mangle]
    pub unsafe extern "C" fn BuildGroup(queue: *mut QUEUE, race: u8) -> HIPGROUP {
        assert_eq!((*queue).object_size as usize, mem::size_of::<IP_GROUP>());
        let template = GetStarShipFromIndex(fleet_queue(), race as u16).cast::<FLEET_INFO>();
        if template.is_null() {
            return ptr::null_mut();
        }
        let handle = AllocLink(queue);
        if handle.is_null() {
            return handle;
        }
        let group = handle.cast::<IP_GROUP>();
        ptr::write_bytes(group.cast::<u8>(), 0, (*queue).object_size as usize);
        (*group).race_id = race;
        (*group).melee_icon = (*template).melee_icon;
        PutQueue(queue, handle);
        handle
    }

    /// # Safety
    /// A current star, solar system, and initialized world queues are required.
    #[no_mangle]
    pub unsafe extern "C" fn BuildGroups() {
        let mut home = [
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
        let mut percent = ENCOUNTER_PERCENT;
        percent[SLYLANDRO_SHIP as usize] =
            percent[SLYLANDRO_SHIP as usize].wrapping_mul(get("SLYLANDRO_MULTIPLIER"));
        let mission = get("UTWIG_SUPOX_MISSION");
        if mission > 1 && mission < 5 {
            home[UTWIG_SHIP as usize] = 0;
            home[SUPOX_SHIP as usize] = 0;
        }
        let universe = (*CurStarDescPtr).star_pt;
        let mut best_index = 0usize;
        let mut best_percent = 0u16;
        let mut handle = (*fleet_queue()).head;
        let mut index = 0usize;
        while !handle.is_null() {
            let fleet = &*handle.cast::<FLEET_INFO>();
            let next = fleet.succ;
            let mut radius = fleet.actual_strength;
            let mut chance = percent[index] as u16;
            if radius != 0 && chance != 0 {
                if home[index] != 0 && (*CurStarDescPtr).index as u32 == home[index] {
                    best_index = index;
                    best_percent = if home[index] == SPATHI_DEFINED || home[index] == SUPOX_DEFINED
                    {
                        2
                    } else {
                        70
                    };
                    break;
                }
                if radius == u16::MAX {
                    radius = 20000;
                } else {
                    radius = ((radius as u32 * SPHERE_RADIUS_INCREMENT) >> 1) as u16;
                }
                let dx = universe.x.wrapping_sub(fleet.loc.x).wrapping_abs() as u16;
                let dy = universe.y.wrapping_sub(fleet.loc.y).wrapping_abs() as u16;
                let squared = dx as u32 * dx as u32 + dy as u32 * dy as u32;
                if dx < radius && dy < radius && squared < radius as u32 * radius as u32 {
                    if fleet.actual_strength != u16::MAX {
                        chance = 70 - (square_root(squared) as u32 * 60 / radius as u32) as u16;
                    }
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
        if best_percent != 0 {
            let mut number = ((TFB_Random() as u16 % (best_percent >> 1)) + 1) as u8;
            if number > MAX_BATTLE_GROUPS as u8 {
                number = MAX_BATTLE_GROUPS as u8;
            } else if number < 5
                && home[best_index] != 0
                && (*CurStarDescPtr).index as u32 == home[best_index]
            {
                number = 5;
            }
            let mut which = 0u8;
            while number != 0 {
                let makeup = ENCOUNTER_MAKEUP[best_index];
                for ships in (1..=makeup >> 4).rev() {
                    if ships <= makeup & 15 || (TFB_Random() as u16 % 100) < 50 {
                        CloneShipFragment(best_index as u16, npc_queue(), 0);
                    }
                }
                which += 1;
                PutGroupInfo(0, which);
                ReinitQueue(npc_queue());
                number -= 1;
            }
        }
        GetGroupInfo(0, u8::MAX);
    }

    unsafe fn flush(header: *mut GROUP_HEADER, offset: u32, which: u8, file: *mut GameStateFile) {
        if which == 0 {
            if (*header).GroupOffset[0] == 0 {
                (*header).GroupOffset[0] = LengthStateFile(file);
            }
            assert!((*npc_queue()).head.is_null());
            let mut handle = (*ip_queue()).head;
            while !handle.is_null() {
                let group = &*handle.cast::<IP_GROUP>();
                let next = group.succ;
                let id = group.group_id;
                if group.in_system == 0 {
                    if GlobData.Game_state.BattleGroupRef != 0 {
                        PutGroupInfo(GlobData.Game_state.BattleGroupRef, id);
                    } else {
                        flush(header, 0, id, file);
                    }
                    (*header).GroupOffset[id as usize] = 0;
                    RemoveQueue(ip_queue(), handle);
                    FreeLink(ip_queue(), handle);
                }
                handle = next;
            }
        } else if which > (*header).NumGroups {
            (*header).NumGroups = which;
            (*header).GroupOffset[which as usize] = LengthStateFile(file);
        }
        seek(file, offset);
        super::WriteGroupHeader(file, header);
        if which == 0 {
            let mut number = count(ip_queue());
            seek(file, (*header).GroupOffset[0]);
            write(file, &[LAST_ENCOUNTER_GROUP, number]);
            let mut handle = (*ip_queue()).head;
            while number != 0 {
                let group = handle.cast::<IP_GROUP>();
                handle = (*group).succ;
                write(file, &[(*group).race_id]);
                super::WriteIpGroup(file, group);
                number -= 1;
            }
        } else {
            let mut number = count(npc_queue());
            let mut handle = (*npc_queue()).head;
            let race = if number != 0 {
                (*handle.cast::<SHIP_FRAGMENT>()).race_id
            } else {
                0
            };
            seek(file, (*header).GroupOffset[which as usize]);
            write(file, &[race, number]);
            while number != 0 {
                let fragment = handle.cast::<SHIP_FRAGMENT>();
                handle = (*fragment).succ;
                write(file, &[(*fragment).race_id]);
                super::WriteShipFragment(file, fragment);
                number -= 1;
            }
        }
    }

    /// # Safety
    /// Current world and solar-system queues must be initialized and exclusively accessible.
    #[no_mangle]
    pub unsafe extern "C" fn GetGroupInfo(offset: u32, which: u8) -> BOOLEAN {
        let file = open(offset, which);
        if file.is_null() {
            return 0;
        }
        seek(file, offset);
        let mut header: GROUP_HEADER = mem::zeroed();
        super::ReadGroupHeader(file, &mut header);
        if which == u8::MAX {
            ReinitQueue(ip_queue());
            let mut day = header.day_index as u16;
            let mut month = header.month_index as u16;
            let mut year = header.year_index;
            if offset == 0
                && (header.star_index != CurStarDescPtr.offset_from(star_array) as u16
                    || ValidateEvent(EVENT_TYPE_ABSOLUTE_EVENT, &mut month, &mut day, &mut year)
                        == 0)
            {
                CloseStateFile(file);
                let file = OpenStateFile(1, c"wb".as_ptr());
                super::WriteGroupHeader(file, &empty_header());
                CloseStateFile(file);
                return 0;
            }
            for which in 1..=header.NumGroups {
                if header.GroupOffset[which as usize] == 0 {
                    continue;
                }
                seek(file, header.GroupOffset[which as usize]);
                let race = read::<1>(file)[0];
                let ships = read::<1>(file)[0];
                if ships == 0 {
                    continue;
                }
                let handle = BuildGroup(ip_queue(), race);
                let group = handle.cast::<IP_GROUP>();
                (*group).group_id = which;
                (*group).in_system = 1;
                let random = TFB_Random();
                let mut task = random as u8 % ON_STATION as u8;
                if task == FLEE as u8 {
                    task = ON_STATION as u8;
                }
                (*group).orbit_pos = (random >> 16) as u8 & 15;
                let mut location = (*pSolarSysState).SunDesc[0].NumPlanets;
                if location == 1 && task == EXPLORE as u8 {
                    task = IN_ORBIT as u8;
                } else {
                    location = ((random >> 8) as u8 % location) + 1;
                }
                (*group).dest_loc = location;
                let random = TFB_Random();
                (*group).loc.x = ((random as u16 % 10000) as i32 - 5000) as i16;
                (*group).loc.y = (((random >> 16) % 10000) as i32 - 5000) as i16;
                (*group).group_counter = 0;
                if task == EXPLORE as u8 {
                    (*group).group_counter =
                        ((TFB_Random() as u16 % MAX_REVOLUTIONS as u16) << FACING_SHIFT) as u16;
                } else if task == ON_STATION as u8 {
                    let origin = planetOuterLocation(location as u16 - 1);
                    let angle = ((*group).orbit_pos as u32 + 1) << (CIRCLE_SHIFT - FACING_SHIFT);
                    (*group).loc.x =
                        (origin.x as i32 + cosine(angle, STATION_RADIUS as i32)) as i16;
                    (*group).loc.y = (origin.y as i32 + sine(angle, STATION_RADIUS as i32)) as i16;
                    location = 0;
                }
                (*group).task = task;
                (*group).sys_loc = location;
            }
            if offset != 0 {
                InitGroupInfo(0);
            } else if ValidateEvent(EVENT_TYPE_ABSOLUTE_EVENT, &mut month, &mut day, &mut year) != 0
            {
                CloseStateFile(file);
                return 1;
            }
            CloseStateFile(file);
            return u32::from(!(*ip_queue()).head.is_null());
        }
        if header.GroupOffset[which as usize] == 0 {
            CloseStateFile(file);
            return 0;
        }
        if which == 0 {
            let left = count(npc_queue());
            seek(file, header.GroupOffset[0]);
            LAST_ENCOUNTER_GROUP = read::<1>(file)[0];
            if LAST_ENCOUNTER_GROUP != 0 {
                if GlobData.Game_state.BattleGroupRef != 0 {
                    PutGroupInfo(GlobData.Game_state.BattleGroupRef, LAST_ENCOUNTER_GROUP);
                } else {
                    flush(&mut header, offset, LAST_ENCOUNTER_GROUP, file);
                }
            }
            ReinitQueue(npc_queue());
            ReinitQueue(ip_queue());
            seek(file, header.GroupOffset[0] + 1);
            let mut number = read::<1>(file)[0];
            while number != 0 {
                let race = read::<1>(file)[0];
                let handle = BuildGroup(ip_queue(), race);
                let group = handle.cast::<IP_GROUP>();
                super::ReadIpGroup(file, group);
                if (*group).group_id == LAST_ENCOUNTER_GROUP && left == 0 {
                    RemoveQueue(ip_queue(), handle);
                    FreeLink(ip_queue(), handle);
                }
                number -= 1;
            }
            CloseStateFile(file);
            u32::from(!(*ip_queue()).head.is_null())
        } else {
            LAST_ENCOUNTER_GROUP = which;
            if offset != 0 {
                PutGroupInfo(0, 0);
            } else {
                flush(&mut header, 0, 0, file);
            }
            ReinitQueue(ip_queue());
            ReinitQueue(npc_queue());
            seek(file, header.GroupOffset[which as usize] + 1);
            let mut number = read::<1>(file)[0];
            while number != 0 {
                let race = read::<1>(file)[0];
                let handle = CloneShipFragment(race as u16, npc_queue(), 0);
                super::ReadShipFragment(file, handle.cast());
                number -= 1;
            }
            CloseStateFile(file);
            u32::from(!(*npc_queue()).head.is_null())
        }
    }

    /// # Safety
    /// Current world queues and current star descriptor must be initialized.
    #[no_mangle]
    pub unsafe extern "C" fn PutGroupInfo(mut offset: u32, mut which: u8) -> u32 {
        let file = open(offset, which);
        if file.is_null() {
            return offset;
        }
        let mut header: GROUP_HEADER = mem::zeroed();
        if offset == u32::MAX {
            offset = LengthStateFile(file);
            seek(file, offset);
            header = empty_header();
            super::WriteGroupHeader(file, &header);
        }
        if which != 0 {
            seek(file, offset);
            if which == u8::MAX {
                LAST_ENCOUNTER_GROUP = 0;
                which = 0;
            }
        }
        super::ReadGroupHeader(file, &mut header);
        let mut month = 0;
        let mut day = 7;
        let mut year = 0;
        ValidateEvent(EVENT_TYPE_RELATIVE_EVENT, &mut month, &mut day, &mut year);
        header.day_index = day as u8;
        header.month_index = month as u8;
        header.year_index = year;
        header.star_index = CurStarDescPtr.offset_from(star_array) as u16;
        flush(&mut header, offset, which, file);
        CloseStateFile(file);
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
        unsafe {
            crate::state::ffi::rust_init_game_state();
            let file = OpenStateFile(1, c"wb".as_ptr());
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
            super::WriteIpGroup(file, &group);
            assert_eq!(LengthStateFile(file), 16);
            SeekStateFile(file, 0, 0);
            let bytes = read::<16>(file);
            assert_eq!(
                &bytes[..12],
                &[0, 0, 0, 0, 0x34, 0x12, 4, 0x92, 7, 0, 0xf3, 12]
            );
            SeekStateFile(file, 0, 0);
            let mut restored: IP_GROUP = std::mem::zeroed();
            super::ReadIpGroup(file, &mut restored);
            assert_eq!(
                (
                    restored.loc.x,
                    restored.loc.y,
                    restored.task,
                    restored.orbit_pos
                ),
                (-123, 456, 9, 15)
            );
            SeekStateFile(file, 0, 0);
            let mut ship: SHIP_FRAGMENT = std::mem::zeroed();
            ship.crew_level = 300;
            ship.max_crew = 511;
            ship.race_id = 6;
            super::WriteShipFragment(file, &ship);
            SeekStateFile(file, 0, 0);
            super::ReadShipFragment(file, &mut ship);
            assert_eq!((ship.crew_level, ship.max_crew, ship.race_id), (44, 255, 6));
            CloseStateFile(file);
            DeleteStateFile(1);
        }
    }
}
