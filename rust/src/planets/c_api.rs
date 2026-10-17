//! The C functions and globals the default system generator still needs, in
//! one place. Each goes away when its owner is ported.

use std::ffi::c_void;

use crate::c_bindings::global_state::IP_GROUP;
use crate::collections::queue::{Queue, ReinitQueue};
use crate::comm::locdata::CGlobData;
use crate::planets::generate::default::{Npcs, PlanetNames};

extern "C" {
    #[link_name = "GlobData"]
    static mut GLOB_DATA: CGlobData;
    #[link_name = "EncounterRace"]
    static ENCOUNTER_RACE: i16;
    fn GetGroupInfo(offset: u32, which_group: u8) -> bool;
    fn PutGroupInfo(offset: u32, which_group: u8) -> u32;
    fn BuildGroups();
    fn LoadPlanet(surf_def_frame: *mut c_void);
    fn utf8StringCopy(dst: *mut u8, size: usize, src: *const std::ffi::c_char);
    fn rust_get_game_state_bits(start: i32, end: i32) -> u8;
    fn rust_set_game_state_bits(start: i32, end: i32, value: u8);
}

/// `GROUP_INIT_IP`, `GROUP_SAVE_IP` (`state.h`): `(BYTE)~0`.
const GROUP_INIT_IP: u8 = !0;
const GROUP_SAVE_IP: u8 = !0;
/// `GROUP_LOAD_IP` is `GROUP_LIST` (`state.h`).
const GROUP_LOAD_IP: u8 = 0;
/// `GROUPS_RANDOM`.
const GROUPS_RANDOM: u32 = 0;

/// `LoadPlanet (NULL)`.
///
/// # Safety
/// Game thread, inside a planet orbit.
pub unsafe fn load_planet_default_surface() {
    LoadPlanet(std::ptr::null_mut());
}

/// The live game, through `GlobData` and the C group functions.
pub struct LiveGame;

fn glob() -> *mut CGlobData {
    std::ptr::addr_of_mut!(GLOB_DATA)
}

impl Npcs for LiveGame {
    fn init_ip_groups(&mut self) -> bool {
        // SAFETY: game thread; `GlobData` is initialised.
        unsafe { GetGroupInfo((*glob()).game_state.battle_group_ref, GROUP_INIT_IP) }
    }
    fn clear_battle_group_ref(&mut self) {
        // SAFETY: as above.
        unsafe { (*glob()).game_state.battle_group_ref = 0 };
    }
    fn build_groups(&mut self) {
        // SAFETY: game thread.
        unsafe { BuildGroups() };
    }
    fn load_ip_groups(&mut self) {
        // SAFETY: game thread.
        unsafe { GetGroupInfo(GROUPS_RANDOM, GROUP_LOAD_IP) };
    }
    fn save_ip_groups(&mut self) {
        // SAFETY: game thread.
        unsafe { PutGroupInfo(GROUPS_RANDOM, GROUP_SAVE_IP) };
    }
    fn reinit_npc_built_ship_q(&mut self) {
        // SAFETY: the queue is the live `npc_built_ship_q`.
        unsafe {
            ReinitQueue(
                std::ptr::addr_of_mut!((*glob()).game_state.npc_built_ship_q).cast::<Queue>(),
            )
        };
    }
    fn reinit_ip_group_q(&mut self) {
        // SAFETY: the queue is the live `ip_group_q`.
        unsafe {
            ReinitQueue(std::ptr::addr_of_mut!((*glob()).game_state.ip_group_q).cast::<Queue>())
        };
    }
    fn yehat_civil_war(&self) -> bool {
        // SAFETY: reads the Rust-owned game state.
        unsafe { rust_get_game_state_bits(YEHAT_CIVIL_WAR.0, YEHAT_CIVIL_WAR.1) != 0 }
    }
    fn encounter_race(&self) -> i16 {
        // SAFETY: plain global read.
        unsafe { std::ptr::addr_of!(ENCOUNTER_RACE).read() }
    }
    fn for_each_ip_group(&mut self, visit: &mut dyn FnMut(&mut IP_GROUP)) {
        // SAFETY: walks the live `ip_group_q`; the successor is read before
        // the visit, as C does. `LockLink` on a table queue is a cast: the
        // links are the elements themselves.
        unsafe {
            let queue = std::ptr::addr_of!((*glob()).game_state.ip_group_q);
            let mut handle = (*queue).head;
            while !handle.is_null() {
                let group = handle.cast::<IP_GROUP>();
                let next = (*group).succ;
                visit(&mut *group);
                handle = next;
            }
        }
    }
}

/// `YEHAT_CIVIL_WAR` start and end bit (`globdata.h`).
const YEHAT_CIVIL_WAR: (i32, i32) = (285, 285);
/// `BATTLE_PLANET` start and end bit.
const BATTLE_PLANET: (i32, i32) = (751, 758);

impl PlanetNames for LiveGame {
    fn set_planet_name(&mut self, string_index: u16) {
        // SAFETY: `PlanetName` is the 16-byte name field of the SIS state;
        // `GAME_STRING` text is NUL terminated.
        unsafe {
            let text = crate::comm::hail::game_string(i32::from(string_index));
            let name = &mut (*glob()).sis_state.planet_name;
            utf8StringCopy(name.as_mut_ptr(), name.len(), text);
        }
    }
    fn set_battle_planet(&mut self, value: u8) {
        // SAFETY: writes the Rust-owned game state.
        unsafe { rust_set_game_state_bits(BATTLE_PLANET.0, BATTLE_PLANET.1, value) };
    }
}
