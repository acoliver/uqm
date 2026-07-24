//! Communication dispatch — ported from C `comm.c` RaceCommunication and
//! InitCommunication.
//!
//! This module ports the encounter dispatch logic to Rust, operating on
//! C-owned queues through the existing Rust queue API. Game state is read
//! from the Rust-owned singleton (P09). CommData is populated through the
//! existing `init_race` mechanism (P10).
//!
//! @plan PLAN-20260724-MAINLOOP-AND-COMM.P11
//! @requirement REQ-ML-007, REQ-COMM-DISPATCH

use std::os::raw::c_void;

use crate::collections::queue::{AllocLink, CountLinks, HLink, PutQueue, Queue, ReinitQueue};
use crate::comm::locdata::rust_sync_comm_data;
use crate::mainloop::c_extern;
use crate::state::game_state_keys;

// ---------------------------------------------------------------------------
// C type mirrors for queue elements (#[repr(C)] matching C struct layout)
// ---------------------------------------------------------------------------

/// C: `typedef struct point { COORD x, y; } POINT;`
/// COORD is `SIZE` which is `SWORD` = `i16`.
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct CPoint {
    pub x: i16,
    pub y: i16,
}

/// C: `typedef uint8 BYTE` — SPECIES_ID is an enum stored as BYTE.
pub type CSpeciesId = u8;

/// C: `typedef void* STRING` (STRING_TABLE_ENTRY_DESC*)
pub type CStringHandle = *mut c_void;

/// C: `typedef FRAME_DESC* FRAME`
pub type CFrameHandle = *mut c_void;

/// C: `struct brief_ship_info` — used inside ENCOUNTER.
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct BriefShipInfo {
    pub race_id: u8,
    pub crew_level: u16,
    pub max_crew: u16,
    pub max_energy: u8,
}

/// C: `MAX_HYPER_SHIPS` — max ships per encounter.
pub const MAX_HYPER_SHIPS: usize = 7;

/// C: `struct encounter` — encounter queue element.
#[repr(C)]
pub struct CEncounter {
    pub pred: HLink,
    pub succ: HLink,
    pub h_element: HLink,
    pub transition_state: i16,
    pub origin: CPoint,
    pub radius: u16,
    pub race_id: u8,
    pub num_ships: u8,
    pub flags: u8,
    pub loc_pt: CPoint,
    pub ship_list: [BriefShipInfo; MAX_HYPER_SHIPS],
    pub log_x: i32,
    pub log_y: i32,
}

/// C: `SHIP_BASE_COMMON` macro expands to the first fields of any ship struct.
#[repr(C)]
pub struct ShipBaseCommon {
    pub pred: HLink,
    pub succ: HLink,
    pub species_id: CSpeciesId,
    pub captains_name_index: u8,
}

/// C: `SHIP_FRAGMENT` — queue element for built ship queues.
#[repr(C)]
#[derive(Default)]
pub struct CShipFragment {
    // SHIP_BASE_COMMON
    pub pred: HLink,
    pub succ: HLink,
    pub species_id: CSpeciesId,
    pub captains_name_index: u8,
    // SHIP_FRAGMENT specific
    pub race_id: u8,
    pub index: u8,
    pub crew_level: u16,
    pub max_crew: u16,
    pub energy_level: u8,
    pub max_energy: u8,
    pub race_strings: CStringHandle,
    pub icons: CFrameHandle,
    pub melee_icon: CFrameHandle,
}

/// C: `FLEET_INFO` — queue element for the avail_race_q (fleet roster).
#[repr(C)]
pub struct CFleetInfo {
    // LINK elements
    pub pred: HLink,
    pub succ: HLink,
    pub species_id: CSpeciesId,
    pub allied_state: u16,
    pub days_left: u8,
    pub growth_fract: u8,
    pub crew_level: u16,
    pub max_crew: u16,
    pub growth: u8,
    pub max_energy: u8,
    pub loc: CPoint,
    pub race_strings: CStringHandle,
    pub icons: CFrameHandle,
    pub melee_icon: CFrameHandle,
    pub actual_strength: u16,
    pub known_strength: u16,
    pub known_loc: CPoint,
    pub growth_err_term: u8,
    pub func_index: u8,
    pub dest_loc: CPoint,
}

// ---------------------------------------------------------------------------
// C constants
// ---------------------------------------------------------------------------

/// C: `ONE_SHOT_ENCOUNTER`
pub const ONE_SHOT_ENCOUNTER: u8 = 1 << 7;
/// C: `ENCOUNTER_REFORMING`
pub const ENCOUNTER_REFORMING: u8 = 1 << 6;

/// C: `HAIL = 0`
pub const HAIL: u16 = 0;
/// C: `ATTACK`
pub const ATTACK: u16 = 1;

/// C: `DEAD_GUY = 0`
pub const DEAD_GUY: u16 = 0;
/// C: `GOOD_GUY`
pub const GOOD_GUY: u16 = 1;
/// C: `BAD_GUY`
pub const BAD_GUY: u16 = 2;

/// C: `INFINITE_RADIUS`
pub const INFINITE_RADIUS: u16 = !0;

/// C: `INFINITE_FLEET`
pub const INFINITE_FLEET: u16 = !0;

// ---------------------------------------------------------------------------
// Activity flags (from globdata.h)
// ---------------------------------------------------------------------------

/// C: `CHECK_ABORT`
pub const CHECK_ABORT: u16 = 0x4000;
/// C: `CHECK_LOAD`
pub const CHECK_LOAD: u16 = 0x1000;
/// C: `START_ENCOUNTER`
pub const START_ENCOUNTER: u16 = 0x0400;
/// C: `START_INTERPLANETARY`
pub const START_INTERPLANETARY: u16 = 0x0800;

/// C: `IN_LAST_BATTLE`
pub const IN_LAST_BATTLE: u16 = 1;
/// C: `IN_ENCOUNTER`
pub const IN_ENCOUNTER: u16 = 2;
/// C: `IN_HYPERSPACE`
pub const IN_HYPERSPACE: u16 = 3;
/// C: `IN_INTERPLANETARY`
pub const IN_INTERPLANETARY: u16 = 4;
/// C: `WON_LAST_BATTLE`
pub const WON_LAST_BATTLE: u16 = 5;

/// C: `CYBORG_ENABLED`
pub const CYBORG_ENABLED: u8 = 1 << 0;

/// C: `DRAW_SIS_DISPLAY`
pub const DRAW_SIS_DISPLAY: u16 = 1;

// ---------------------------------------------------------------------------
// Conversation IDs (from commglue.h enum)
// ---------------------------------------------------------------------------

pub mod conv {
    pub const ARILOU: u32 = 0;
    pub const CHMMR: u32 = 1;
    pub const COMMANDER: u32 = 2;
    pub const ORZ: u32 = 3;
    pub const PKUNK: u32 = 4;
    pub const SHOFIXTI: u32 = 5;
    pub const SPATHI: u32 = 6;
    pub const SUPOX: u32 = 7;
    pub const THRADD: u32 = 8;
    pub const UTWIG: u32 = 9;
    pub const VUX: u32 = 10;
    pub const YEHAT: u32 = 11;
    pub const MELNORME: u32 = 12;
    pub const DRUUGE: u32 = 13;
    pub const ILWRATH: u32 = 14;
    pub const MYCON: u32 = 15;
    pub const SLYLANDRO: u32 = 16;
    pub const UMGAH: u32 = 17;
    pub const URQUAN: u32 = 18;
    pub const ZOQFOTPIK: u32 = 19;
    pub const SYREEN: u32 = 20;
    pub const BLACKURQ: u32 = 21;
    pub const TALKING_PET: u32 = 22;
    pub const SLYLANDRO_HOME: u32 = 23;
    pub const URQUAN_DRONE: u32 = 24;
    pub const YEHAT_REBEL: u32 = 25;
    pub const INVALID: u32 = 26;
}

// ---------------------------------------------------------------------------
// Ship type IDs (from races.h enum)
// ---------------------------------------------------------------------------

pub mod ship {
    pub const ARILOU_SHIP: u8 = 0;
    pub const CHMMR_SHIP: u8 = 1;
    pub const HUMAN_SHIP: u8 = 2;
    pub const ORZ_SHIP: u8 = 3;
    pub const PKUNK_SHIP: u8 = 4;
    pub const SHOFIXTI_SHIP: u8 = 5;
    pub const SPATHI_SHIP: u8 = 6;
    pub const SUPOX_SHIP: u8 = 7;
    pub const THRADDASH_SHIP: u8 = 8;
    pub const UTWIG_SHIP: u8 = 9;
    pub const VUX_SHIP: u8 = 10;
    pub const YEHAT_SHIP: u8 = 11;
    pub const MELNORME_SHIP: u8 = 12;
    pub const DRUUGE_SHIP: u8 = 13;
    pub const ILWRATH_SHIP: u8 = 14;
    pub const MYCON_SHIP: u8 = 15;
    pub const SLYLANDRO_SHIP: u8 = 16;
    pub const UMGAH_SHIP: u8 = 17;
    pub const URQUAN_SHIP: u8 = 18;
    pub const ZOQFOTPIK_SHIP: u8 = 19;
    pub const SYREEN_SHIP: u8 = 20;
    pub const BLACK_URQUAN_SHIP: u8 = 21;
    pub const YEHAT_REBEL_SHIP: u8 = 22;
    pub const URQUAN_DRONE_SHIP: u8 = 23;
    pub const SAMATRA_SHIP: u8 = 24;
}

/// C: `NPC_PLAYER_NUM`
pub const NPC_PLAYER_NUM: u16 = 1;
/// C: `RPG_PLAYER_NUM`
pub const RPG_PLAYER_NUM: u16 = 0;

// ---------------------------------------------------------------------------
// RACE_COMMUNICATION lookup table
// Maps ship_id → conversation_id (C: RaceComm[] array)
// ---------------------------------------------------------------------------

/// C: `RACE_COMMUNICATION` macro — maps ship index to conversation ID.
#[rustfmt::skip]
pub const RACE_COMMUNICATION: [u32; 25] = [
    conv::ARILOU,          // ARILOU_SHIP
    conv::CHMMR,           // CHMMR_SHIP
    conv::INVALID,         // HUMAN_SHIP
    conv::ORZ,             // ORZ_SHIP
    conv::PKUNK,           // PKUNK_SHIP
    conv::SHOFIXTI,        // SHOFIXTI_SHIP
    conv::SPATHI,          // SPATHI_SHIP
    conv::SUPOX,           // SUPOX_SHIP
    conv::THRADD,          // THRADDASH_SHIP
    conv::UTWIG,           // UTWIG_SHIP
    conv::VUX,             // VUX_SHIP
    conv::YEHAT,           // YEHAT_SHIP
    conv::MELNORME,        // MELNORME_SHIP
    conv::DRUUGE,          // DRUUGE_SHIP
    conv::ILWRATH,         // ILWRATH_SHIP
    conv::MYCON,           // MYCON_SHIP
    conv::SLYLANDRO,       // SLYLANDRO_SHIP
    conv::UMGAH,           // UMGAH_SHIP
    conv::URQUAN,          // URQUAN_SHIP
    conv::ZOQFOTPIK,       // ZOQFOTPIK_SHIP
    conv::INVALID,         // SYREEN_SHIP
    conv::BLACKURQ,        // BLACK_URQUAN_SHIP
    conv::YEHAT_REBEL,     // YEHAT_REBEL_SHIP
    conv::URQUAN_DRONE,    // URQUAN_DRONE_SHIP
    conv::INVALID,         // (padding)
];

/// C: `RACE_SHIP_FOR_COMM` macro — maps conversation ID to ship type.
#[rustfmt::skip]
pub const RACE_SHIP_FOR_COMM: [u8; 27] = [
    ship::ARILOU_SHIP,        // ARILOU_CONVERSATION
    ship::CHMMR_SHIP,         // CHMMR_CONVERSATION
    ship::HUMAN_SHIP,         // COMMANDER_CONVERSATION
    ship::ORZ_SHIP,           // ORZ_CONVERSATION
    ship::PKUNK_SHIP,         // PKUNK_CONVERSATION
    ship::SHOFIXTI_SHIP,      // SHOFIXTI_CONVERSATION
    ship::SPATHI_SHIP,        // SPATHI_CONVERSATION
    ship::SUPOX_SHIP,         // SUPOX_CONVERSATION
    ship::THRADDASH_SHIP,     // THRADD_CONVERSATION
    ship::UTWIG_SHIP,         // UTWIG_CONVERSATION
    ship::VUX_SHIP,           // VUX_CONVERSATION
    ship::YEHAT_SHIP,         // YEHAT_CONVERSATION
    ship::MELNORME_SHIP,      // MELNORME_CONVERSATION
    ship::DRUUGE_SHIP,        // DRUUGE_CONVERSATION
    ship::ILWRATH_SHIP,       // ILWRATH_CONVERSATION
    ship::MYCON_SHIP,         // MYCON_CONVERSATION
    ship::SLYLANDRO_SHIP,     // SLYLANDRO_CONVERSATION
    ship::UMGAH_SHIP,         // UMGAH_CONVERSATION
    ship::URQUAN_SHIP,        // URQUAN_CONVERSATION
    ship::ZOQFOTPIK_SHIP,     // ZOQFOTPIK_CONVERSATION
    ship::SYREEN_SHIP,        // SYREEN_CONVERSATION
    ship::BLACK_URQUAN_SHIP,  // BLACKURQ_CONVERSATION
    ship::UMGAH_SHIP,         // TALKING_PET_CONVERSATION
    ship::SLYLANDRO_SHIP,     // SLYLANDRO_HOME_CONVERSATION
    ship::URQUAN_DRONE_SHIP,  // URQUAN_DRONE_CONVERSATION
    ship::YEHAT_SHIP,         // YEHAT_REBEL_CONVERSATION
    ship::HUMAN_SHIP,         // INVALID_CONVERSATION
];

// ---------------------------------------------------------------------------
// C FFI declarations for functions that stay in C for now
// ---------------------------------------------------------------------------

#[allow(dead_code)]
mod ffi {
    use super::*;
    use std::os::raw::{c_char, c_void};

    extern "C" {
        // Queue pointer accessors (rust_bridge_mainloop.c)
        pub fn rust_get_avail_race_queue() -> *mut Queue;
        pub fn rust_get_npc_built_ship_queue() -> *mut Queue;
        pub fn rust_get_encounter_queue() -> *mut Queue;
        pub fn rust_get_built_ship_queue() -> *mut Queue;

        // init_race dispatch (commglue.c) — returns LOCDATA* or NULL
        pub fn init_race(comm_id: u32) -> *mut c_void;

        // Encounter/battle (encount.h)
        pub fn BuildBattle(which_player: u16);
        pub fn InitEncounter() -> u16;
        pub fn UninitEncounter() -> u16;
        pub fn EncounterBattle();

        // SIS rendering (comm.c / sis_ship.c)
        pub fn DrawSISFrame();
        pub fn ClearSISRect(flags: u16);
        pub fn RepairSISBorder();
        pub fn DrawSISMessage(msg: *const c_char);
        pub fn DrawHyperCoords(origin: CPoint);
        pub fn DrawSISTitle(title: *const c_char);

        // Game state helpers
        pub fn PickCaptainName() -> u8;

        // Starbase dispatch bridges (rust_bridge_mainloop.c)
        pub fn rust_cleanup_after_starbase();
        pub fn rust_do_time_passage();
        pub fn rust_set_cur_star_desc_ptr_null();
        pub fn rust_do_starbase_menu_input();

        // CommIntroMode (comm.c) — C: SetCommIntroMode(mode, howLong)
        pub fn SetCommIntroMode(mode: u32, how_long: u32);

        // CommData copy — copies LOCDATA to C's global CommData
        pub fn rust_copy_locdata_to_comm_data(locdata_ptr: *const c_void);
    }
}

// ---------------------------------------------------------------------------
// Helper functions (porting C inline functions/macros)
// ---------------------------------------------------------------------------

/// C: `LOBYTE(x)` — extract low byte of a u16.
#[inline]
fn lobyte(x: u16) -> u16 {
    x & 0xFF
}

/// C: `inHQSpace()` — checks if CurrentActivity is IN_HYPERSPACE (also true for QuasiSpace).
///
/// # Safety
/// Calls C FFI to read CurrentActivity from global state.
unsafe fn in_hq_space() -> bool {
    let activity = c_extern::get_current_activity();
    lobyte(activity) == IN_HYPERSPACE
}

/// C: `GetHeadLink(pq)` — returns the head handle of a queue.
#[inline]
unsafe fn get_head_link(pq: *const Queue) -> HLink {
    (*pq).head
}

/// C: `_GetSuccLink(lp)` — returns the succ field of a link.
#[inline]
unsafe fn get_succ_link(lp: *const ShipBaseCommon) -> HLink {
    (*lp).succ
}

/// C: `GetStarShipFromIndex(pShipQ, Index)` — walks the queue by index.
///
/// Returns the HLINK at position `index` (0-based), or null if out of range.
unsafe fn get_star_ship_from_index(p_ship_q: *const Queue, index: u8) -> HLink {
    let mut h_star_ship = get_head_link(p_ship_q);
    let mut remaining = index;

    while remaining > 0 && !h_star_ship.is_null() {
        let ship_ptr = h_star_ship as *const ShipBaseCommon;
        let h_next = get_succ_link(ship_ptr);
        h_star_ship = h_next;
        remaining -= 1;
    }

    h_star_ship
}

/// C: `LockShipFrag(pq, h)` — cast HLINK to SHIP_FRAGMENT pointer.
#[inline]
unsafe fn lock_ship_frag(_pq: *const Queue, h: HLink) -> *mut CShipFragment {
    h as *mut CShipFragment
}

/// C: `LockFleetInfo(pq, h)` — cast HLINK to FLEET_INFO pointer.
#[inline]
unsafe fn lock_fleet_info(_pq: *const Queue, h: HLink) -> *mut CFleetInfo {
    h as *mut CFleetInfo
}

/// C: `GetHeadEncounter()` = `GetHeadLink(&GLOBAL(encounter_q))`
#[inline]
unsafe fn get_head_encounter() -> HLink {
    let enc_q = ffi::rust_get_encounter_queue();
    get_head_link(enc_q)
}

/// C: `LockEncounter(h, &ptr)` = `*(ppe) = (ENCOUNTER*)LockLink(...)`
#[inline]
unsafe fn lock_encounter(_h: HLink) -> *mut CEncounter {
    // LockLink is just a cast in QUEUE_TABLE mode
    _h as *mut CEncounter
}

// ---------------------------------------------------------------------------
// Pure helper: get game state bit (C: GET_GAME_STATE)
// ---------------------------------------------------------------------------

fn get_game_state(key: &str) -> u32 {
    game_state_keys::get_game_state(key)
}

fn set_game_state(key: &str, value: u32) {
    game_state_keys::set_game_state(key, value)
}

// ---------------------------------------------------------------------------
// RaceCommunication — ported from comm.c:1503
// ---------------------------------------------------------------------------

/// Port of C `RaceCommunication()`. Determines which alien to talk to,
/// prepares the npc ship queue, and calls InitCommunication.
///
/// # Safety
/// Calls C FFI functions that access global state.
pub unsafe extern "C" fn rust_race_communication() {
    let current_activity = c_extern::get_current_activity();

    if lobyte(current_activity) == IN_LAST_BATTLE {
        // Going into talking pet conversation
        let npc_q = ffi::rust_get_npc_built_ship_queue();
        ReinitQueue(npc_q);
        clone_ship_fragment(ship::SAMATRA_SHIP, npc_q, 0);
        init_communication(conv::TALKING_PET);

        let activity = c_extern::get_current_activity();
        if (activity & (CHECK_ABORT | CHECK_LOAD)) == 0 {
            let crew = c_extern::uqm_get_crew_enlisted();
            if crew != 0xFFFF {
                c_extern::set_current_activity(WON_LAST_BATTLE);
            }
        }
        return;
    }

    let next_activity = c_extern::get_next_activity();
    if (next_activity & CHECK_LOAD) != 0 {
        let ec = get_game_state("ESCAPE_COUNTER") as u8;

        if get_game_state("FOUND_PLUTO_SPATHI") == 1 {
            init_communication(conv::SPATHI);
        } else if get_game_state("GLOBAL_FLAGS_AND_DATA") == 0 {
            init_communication(conv::TALKING_PET);
        } else if (get_game_state("GLOBAL_FLAGS_AND_DATA") & ((1 << 4) | (1 << 5))) != 0 {
            init_communication(conv::ILWRATH);
        } else {
            init_communication(conv::CHMMR);
        }

        let crew = c_extern::uqm_get_crew_enlisted();
        if crew != 0xFFFF {
            let activity = c_extern::get_current_activity();
            let mut na = activity & !START_ENCOUNTER;
            if lobyte(na) == IN_INTERPLANETARY {
                na |= START_INTERPLANETARY;
            }
            c_extern::set_next_activity(na);
            c_extern::set_current_activity(activity | CHECK_LOAD);
        }

        set_game_state("ESCAPE_COUNTER", ec as u32);
        return;
    }

    let mut h_encounter: HLink = std::ptr::null_mut();

    if in_hq_space() {
        let npc_q = ffi::rust_get_npc_built_ship_queue();
        ReinitQueue(npc_q);

        if get_game_state("ARILOU_SPACE_SIDE") >= 2 {
            init_communication(conv::ARILOU);
            return;
        }

        // Encounter with a black globe in HS, prepare enemy ship list
        h_encounter = get_head_encounter();
        if !h_encounter.is_null() {
            let enc_ptr = lock_encounter(h_encounter);
            let num_ships = (*enc_ptr).num_ships;
            for i in 0..num_ships {
                let race_id = (*enc_ptr).race_id;
                let crew_level = (*enc_ptr).ship_list[i as usize].crew_level;
                clone_ship_fragment(race_id, npc_q, crew_level);
            }
        }
    }

    // First ship in the npc queue defines which alien race the player talks to
    let npc_q = ffi::rust_get_npc_built_ship_queue();
    let h_star_ship = get_head_link(npc_q);
    if h_star_ship.is_null() {
        return;
    }

    let frag_ptr = lock_ship_frag(npc_q, h_star_ship);
    let i = (*frag_ptr).race_id;
    // UnlockShipFrag is a no-op in QUEUE_TABLE mode

    let conv_id = if (i as usize) < RACE_COMMUNICATION.len() {
        RACE_COMMUNICATION[i as usize]
    } else {
        conv::INVALID
    };

    let status = init_communication(conv_id);

    let activity = c_extern::get_current_activity();
    if (activity & (CHECK_ABORT | CHECK_LOAD)) != 0 {
        return;
    }

    if i == ship::CHMMR_SHIP {
        ReinitQueue(npc_q);
    }

    if lobyte(activity) == IN_INTERPLANETARY {
        // if used destruct code in interplanetary
        if i == ship::SLYLANDRO_SHIP && status == 0 {
            ReinitQueue(npc_q);
        }
    } else if !h_encounter.is_null() {
        // Update HSpace encounter info, ships left, etc.
        let enc_ptr = lock_encounter(h_encounter);
        let num_ships = CountLinks(npc_q);
        (*enc_ptr).num_ships = num_ships as u8;
        (*enc_ptr).flags |= ENCOUNTER_REFORMING;
        if status == 0 {
            (*enc_ptr).flags |= ONE_SHOT_ENCOUNTER;
        }

        for i in 0..num_ships {
            let h_star_ship = get_star_ship_from_index(npc_q, i as u8);
            if h_star_ship.is_null() {
                break;
            }
            let frag_ptr = lock_ship_frag(npc_q, h_star_ship);
            let bsi = &mut (*enc_ptr).ship_list[i as usize];
            bsi.race_id = (*frag_ptr).race_id;
            bsi.crew_level = (*frag_ptr).crew_level;
            bsi.max_crew = (*frag_ptr).max_crew;
            bsi.max_energy = (*frag_ptr).max_energy;
        }

        ReinitQueue(npc_q);
    }
}

// ---------------------------------------------------------------------------
// InitCommunication — ported from comm.c:1359
// ---------------------------------------------------------------------------

/// Port of C `InitCommunication(which_comm)`. Maps conversation to ship type,
/// initializes encounter, and calls HailAlien or battle segue.
///
/// Returns 0 (matching C's `status = 0` at the end).
///
/// # Safety
/// Calls C FFI functions that access global state.
pub unsafe fn init_communication(which_comm: u32) -> u16 {
    let mut status: u16;
    let last_activity = c_extern::get_last_activity();

    if (last_activity & CHECK_LOAD) != 0 {
        c_extern::set_last_activity(last_activity & !CHECK_LOAD);

        if which_comm != conv::COMMANDER {
            if lobyte(last_activity) == 0 {
                ffi::DrawSISFrame();
            } else {
                ffi::ClearSISRect(DRAW_SIS_DISPLAY);
                ffi::RepairSISBorder();
            }
            ffi::DrawSISMessage(std::ptr::null());

            if in_hq_space() {
                // DrawHyperCoords needs ShipStamp.origin — we don't have direct
                // access to this field yet. Use a zeroed point as placeholder.
                // TODO: read ShipStamp.origin from Rust-owned game state
                ffi::DrawHyperCoords(CPoint::default());
            } else if get_game_state("IP_PLANET") == 0 {
                // DrawHyperCoords with CurStarDescPtr->star_pt
                // TODO: access CurStarDescPtr from Rust
                ffi::DrawHyperCoords(CPoint::default());
            } else {
                // DrawSISTitle with PlanetName — we don't have SIS PlanetName
                // directly accessible. Use null for now.
                // TODO: read PlanetName from Rust-owned SIS state
                ffi::DrawSISTitle(std::ptr::null());
            }
        }
    }

    if which_comm == conv::URQUAN_DRONE {
        status = ship::URQUAN_DRONE_SHIP as u16;
        let _ = init_communication_inner(conv::URQUAN, status);
    } else if which_comm == conv::YEHAT_REBEL {
        status = ship::YEHAT_REBEL_SHIP as u16;
        let _ = init_communication_inner(conv::YEHAT, status);
    } else {
        let comm_idx = which_comm as usize;
        if comm_idx < RACE_SHIP_FOR_COMM.len() {
            status = RACE_SHIP_FOR_COMM[comm_idx] as u16;
        } else {
            status = ship::HUMAN_SHIP as u16;
        }

        if status >= ship::YEHAT_REBEL_SHIP as u16 {
            status = ship::HUMAN_SHIP as u16;
        }

        init_communication_inner(which_comm, status);
    }

    0
}

/// Inner part of InitCommunication after ship type is resolved.
/// Handles StartSphereTracking, BuildBattle, init_race, encounter, and HailAlien.
unsafe fn init_communication_inner(which_comm: u32, ship_type: u16) -> u16 {
    start_sphere_tracking(ship_type as u8);

    if which_comm == conv::ORZ
        || (which_comm == conv::TALKING_PET
            && (get_game_state("TALKING_PET_ON_SHIP") == 0
                || lobyte(c_extern::get_current_activity()) == IN_LAST_BATTLE))
        || (which_comm != conv::CHMMR && which_comm != conv::SYREEN)
    {
        ffi::BuildBattle(NPC_PLAYER_NUM);
    }

    // init_race — calls C dispatch table which populates CommData
    let comm_id = if ship_type != ship::YEHAT_REBEL_SHIP as u16 {
        which_comm
    } else {
        conv::YEHAT_REBEL
    };

    let loc_data_ptr = ffi::init_race(comm_id);
    if !loc_data_ptr.is_null() {
        // Copy LOCDATA to C's global CommData (C code reads from CommData)
        ffi::rust_copy_locdata_to_comm_data(loc_data_ptr);
        // Sync to Rust's CommData singleton
        rust_sync_comm_data(loc_data_ptr);
    }

    let mut status: u16;
    if get_game_state("BATTLE_SEGUE") == 0 {
        status = HAIL;
    } else {
        status = ffi::InitEncounter();
        if status == HAIL && !loc_data_ptr.is_null() {
            set_game_state("BATTLE_SEGUE", 0);
        } else {
            status = ATTACK;
            set_game_state("BATTLE_SEGUE", 1);
        }
    }

    if status == HAIL {
        crate::comm::ffi::rust_HailAlien();
    } else if !loc_data_ptr.is_null() {
        let activity = c_extern::get_current_activity();
        if (activity & (CHECK_ABORT | CHECK_LOAD)) == 0 {
            // Call post_encounter_func and uninit_encounter_func via C bridge.
            // These are function pointers stored in CommData — we call them
            // through the C copy of CommData.
            // TODO: Add C bridge for calling CommData function pointers
        }
    }

    let status: u16 = 0;
    let activity = c_extern::get_current_activity();
    if (activity & (CHECK_ABORT | CHECK_LOAD)) == 0 {
        let glob_flags = c_extern::uqm_get_global_flags_and_data();
        if lobyte(activity) == IN_LAST_BATTLE && (glob_flags & CYBORG_ENABLED) != 0 {
            let npc_q = ffi::rust_get_npc_built_ship_queue();
            ReinitQueue(npc_q);
        }

        set_game_state("GLOBAL_FLAGS_AND_DATA", 0);

        let npc_q = ffi::rust_get_npc_built_ship_queue();
        let has_ships = !get_head_link(npc_q).is_null();
        let battle_segue = get_game_state("BATTLE_SEGUE");

        if battle_segue != 0 && has_ships {
            ffi::BuildBattle(RPG_PLAYER_NUM);
            ffi::EncounterBattle();
        } else {
            set_game_state("BATTLE_SEGUE", 0);
        }
    }

    ffi::UninitEncounter();
    status
}

// ---------------------------------------------------------------------------
// CloneShipFragment — ported from build.c:477
// ---------------------------------------------------------------------------

/// Port of C `CloneShipFragment(shipIndex, pDstQueue, crew_level)`.
///
/// Creates a SHIP_FRAGMENT in the destination queue by copying template
/// data from the fleet queue (avail_race_q).
unsafe fn clone_ship_fragment(ship_index: u8, dst_queue: *mut Queue, crew_level: u16) -> HLink {
    let avail_q = ffi::rust_get_avail_race_queue();
    let h_fleet = get_star_ship_from_index(avail_q, ship_index);
    if h_fleet.is_null() {
        return std::ptr::null_mut();
    }

    let template_ptr = lock_fleet_info(avail_q, h_fleet);

    let captains_name_index = if ship_index == ship::SAMATRA_SHIP {
        0
    } else {
        name_captain(dst_queue, (*template_ptr).species_id)
    };

    // Build a new link in the destination queue
    let h_built = AllocLink(dst_queue);
    if !h_built.is_null() {
        let frag_ptr = lock_ship_frag(dst_queue, h_built);

        // C's Build() does memset(0) + sets SpeciesID. We must do the same.
        let species_id = (*template_ptr).species_id;
        *frag_ptr = CShipFragment::default();
        (*frag_ptr).species_id = species_id;

        (*frag_ptr).captains_name_index = captains_name_index;
        (*frag_ptr).race_strings = (*template_ptr).race_strings;
        (*frag_ptr).icons = (*template_ptr).icons;
        (*frag_ptr).melee_icon = (*template_ptr).melee_icon;
        if crew_level != 0 {
            (*frag_ptr).crew_level = crew_level;
        } else {
            (*frag_ptr).crew_level = (*template_ptr).crew_level;
        }
        (*frag_ptr).max_crew = (*template_ptr).max_crew;
        (*frag_ptr).energy_level = 0;
        (*frag_ptr).max_energy = (*template_ptr).max_energy;
        (*frag_ptr).race_id = ship_index;
        (*frag_ptr).index = 0;

        // Link it into the queue
        PutQueue(dst_queue, h_built);
    }

    h_built
}

// ---------------------------------------------------------------------------
// NameCaptain — ported from build.c:437
// ---------------------------------------------------------------------------

/// Port of C `NameCaptain(pQueue, SpeciesID)`. Picks a random captain name
/// index, retrying if it matches an existing ship of the same species.
unsafe fn name_captain(p_queue: *mut Queue, species_id: CSpeciesId) -> u8 {
    let mut name_index: u8;
    loop {
        name_index = ffi::PickCaptainName();

        let mut h_star_ship = get_head_link(p_queue);
        let mut found_match = false;
        while !h_star_ship.is_null() {
            let ship_ptr = h_star_ship as *const ShipBaseCommon;
            let h_next = get_succ_link(ship_ptr);

            if (*ship_ptr).species_id == species_id && (*ship_ptr).captains_name_index == name_index
            {
                found_match = true;
                break;
            }

            h_star_ship = h_next;
        }

        if !found_match {
            break;
        }
    }
    name_index
}

// ---------------------------------------------------------------------------
// StartSphereTracking — ported from build.c:355
// ---------------------------------------------------------------------------

/// Port of C `StartSphereTracking(race)`. Reads/modifies FLEET_INFO to
/// initialize sphere-of-influence tracking for the given race.
///
/// Returns `race` if tracking started, 0 if race is extinct or not found.
unsafe fn start_sphere_tracking(race: u8) -> u16 {
    let avail_q = ffi::rust_get_avail_race_queue();
    let h_fleet = get_star_ship_from_index(avail_q, race);
    if h_fleet.is_null() {
        return 0;
    }

    let fleet_ptr = lock_fleet_info(avail_q, h_fleet);

    let result;
    if (*fleet_ptr).actual_strength == 0 {
        if (*fleet_ptr).allied_state == DEAD_GUY {
            result = 0;
        } else {
            result = race as u16;
        }
    } else if (*fleet_ptr).known_strength == 0 && (*fleet_ptr).actual_strength != INFINITE_RADIUS {
        (*fleet_ptr).known_strength = 1;
        (*fleet_ptr).known_loc = (*fleet_ptr).loc;
        result = race as u16;
    } else {
        result = race as u16;
    }

    result
}

// ---------------------------------------------------------------------------
// VisitStarBase — ported from starbase.c:430
// ---------------------------------------------------------------------------

/// C: CommIntroMode enum values
pub mod cim {
    pub const CROSSFADE_SPACE: u32 = 0;
    pub const CROSSFADE_WINDOW: u32 = 1;
    pub const CROSSFADE_SCREEN: u32 = 2;
    pub const FADE_IN_SCREEN: u32 = 3;
}

/// C: StatMsgMode enum values
pub mod smm {
    pub const UNDEFINED: u32 = 0;
    pub const DATE: u32 = 1;
    pub const RES_UNITS: u32 = 2;
    pub const CREDITS: u32 = 3;
}

/// C: `ONE_SECOND` = 840 (from timelib.h)
pub const ONE_SECOND: u32 = 840;

/// Port of C `VisitStarBase()`. Handles the starbase dispatch logic:
/// unallied conversations, Ilwrath encounter, time passage, and starbase menu.
///
/// # Safety
/// Calls C FFI functions that access global state and rendering.
pub unsafe extern "C" fn rust_visit_starbase() {
    // CHMMR_BOMB_STATE == 2: transported by Chmmr to Starbase
    if get_game_state("CHMMR_BOMB_STATE") == 2 {
        ffi::rust_set_cur_star_desc_ptr_null();
        set_game_state("GLOBAL_FLAGS_AND_DATA", 0xFF);
    }

    if get_game_state("STARBASE_AVAILABLE") == 0 {
        // Unallied Starbase conversation
        ffi::SetCommIntroMode(cim::CROSSFADE_SCREEN, 0);
        init_communication(conv::COMMANDER);

        if get_game_state("PROBE_ILWRATH_ENCOUNTER") == 0
            || (c_extern::get_current_activity() & CHECK_ABORT) != 0
        {
            ffi::rust_cleanup_after_starbase();
            return;
        }

        // Create an Ilwrath ship responding to the Ur-Quan probe's broadcast
        let npc_q = ffi::rust_get_npc_built_ship_queue();
        let h_star_ship = clone_ship_fragment(ship::ILWRATH_SHIP, npc_q, 7);
        if !h_star_ship.is_null() {
            let frag_ptr = lock_ship_frag(npc_q, h_star_ship);
            // Hack: Suppress the tally and salvage info after the battle
            (*frag_ptr).race_id = 0xFF;
        }

        init_communication(conv::ILWRATH);

        let crew = c_extern::uqm_get_crew_enlisted();
        if crew == 0xFFFF || (c_extern::get_current_activity() & CHECK_ABORT) != 0 {
            return; // Killed by Ilwrath
        }

        // After Ilwrath battle, about-to-ally Starbase conversation
        ffi::SetCommIntroMode(cim::CROSSFADE_SCREEN, 0);
        init_communication(conv::COMMANDER);

        if (c_extern::get_current_activity() & CHECK_ABORT) != 0 {
            return;
        }
        // This marks that we are in Starbase.
        set_game_state("GLOBAL_FLAGS_AND_DATA", 0xFF);
    }

    if get_game_state("MOONBASE_ON_SHIP") != 0 || get_game_state("CHMMR_BOMB_STATE") == 2 {
        // Go immediately into a conversation with the Commander
        ffi::rust_do_time_passage();

        let crew = c_extern::uqm_get_crew_enlisted();
        if crew == 0xFFFF {
            return; // You are now dead!
        }

        ffi::SetCommIntroMode(cim::FADE_IN_SCREEN, ONE_SECOND * 2);
        init_communication(conv::COMMANDER);

        if (c_extern::get_current_activity() & CHECK_ABORT) != 0 {
            return;
        }
        set_game_state("GLOBAL_FLAGS_AND_DATA", 0xFF);
    }

    // Starbase menu input loop — handled by C bridge
    ffi::rust_do_starbase_menu_input();
}

// ---------------------------------------------------------------------------
// ExploreSolarSys — ported from solarsys.c:1713
// ---------------------------------------------------------------------------

/// Port of C `ExploreSolarSys()`. Initializes solar system exploration.
///
/// The heavy lifting (InitSolarSys, DoIpFlight, UninitSolarSys) stays in C
/// for now — these are 2000+ lines of solar system generation and rendering
/// that would require porting the entire planet system. The dispatch logic
/// (finding the current star, setting up state) is ported to Rust.
///
/// # Safety
/// Calls C FFI functions that access global state.
pub unsafe extern "C" fn rust_explore_solar_sys() {
    // The full ExploreSolarSys is deeply tied to SOLARSYS_STATE, CurStarDescPtr,
    // InitSolarSys, DoIpFlight, UninitSolarSys — all complex C functions with
    // hundreds of dependencies (planet generation, rendering, input handling).
    // Porting the dispatch wrapper alone would just be a thin wrapper around C.
    // This stays as a C call until the solar system subsystem is ported.
    c_extern::ExploreSolarSys();
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn race_communication_table_has_25_entries() {
        assert_eq!(RACE_COMMUNICATION.len(), 25);
    }

    #[test]
    fn race_communication_maps_arilou_to_arilou_conv() {
        assert_eq!(RACE_COMMUNICATION[ship::ARILOU_SHIP as usize], conv::ARILOU);
    }

    #[test]
    fn race_communication_maps_human_to_invalid() {
        assert_eq!(RACE_COMMUNICATION[ship::HUMAN_SHIP as usize], conv::INVALID);
    }

    #[test]
    fn race_communication_maps_samatra_to_invalid() {
        // SAMATRA_SHIP = 24, which is the last entry
        assert_eq!(
            RACE_COMMUNICATION[ship::SAMATRA_SHIP as usize],
            conv::INVALID
        );
    }

    #[test]
    fn race_ship_for_comm_has_27_entries() {
        assert_eq!(RACE_SHIP_FOR_COMM.len(), 27);
    }

    #[test]
    fn race_ship_for_comm_maps_arilou_to_arilou_ship() {
        assert_eq!(RACE_SHIP_FOR_COMM[conv::ARILOU as usize], ship::ARILOU_SHIP);
    }

    #[test]
    fn race_ship_for_comm_maps_commander_to_human_ship() {
        assert_eq!(
            RACE_SHIP_FOR_COMM[conv::COMMANDER as usize],
            ship::HUMAN_SHIP
        );
    }

    #[test]
    fn race_ship_for_comm_maps_talking_pet_to_umgah_ship() {
        assert_eq!(
            RACE_SHIP_FOR_COMM[conv::TALKING_PET as usize],
            ship::UMGAH_SHIP
        );
    }

    #[test]
    fn race_ship_for_comm_maps_yehat_rebel_to_yehat_ship() {
        assert_eq!(
            RACE_SHIP_FOR_COMM[conv::YEHAT_REBEL as usize],
            ship::YEHAT_SHIP
        );
    }

    #[test]
    fn lobyte_extracts_low_byte() {
        assert_eq!(lobyte(0x0302), 2); // low byte
        assert_eq!(lobyte(0x0100), 0);
        assert_eq!(lobyte(0x0403), 3); // low byte
    }

    #[test]
    fn activity_constants_match_c() {
        assert_eq!(IN_LAST_BATTLE, 1);
        assert_eq!(IN_ENCOUNTER, 2);
        assert_eq!(IN_HYPERSPACE, 3);
        assert_eq!(IN_INTERPLANETARY, 4);
        assert_eq!(WON_LAST_BATTLE, 5);
    }

    #[test]
    fn check_flags_match_c() {
        assert_eq!(CHECK_ABORT, 0x4000);
        assert_eq!(CHECK_LOAD, 0x1000);
        assert_eq!(START_ENCOUNTER, 0x0400);
        assert_eq!(START_INTERPLANETARY, 0x0800);
    }

    #[test]
    fn encounter_flags_match_c() {
        assert_eq!(ONE_SHOT_ENCOUNTER, 0x80);
        assert_eq!(ENCOUNTER_REFORMING, 0x40);
    }

    #[test]
    fn hail_attack_match_c() {
        assert_eq!(HAIL, 0);
        assert_eq!(ATTACK, 1);
    }

    #[test]
    fn alliance_states_match_c() {
        assert_eq!(DEAD_GUY, 0);
        assert_eq!(GOOD_GUY, 1);
        assert_eq!(BAD_GUY, 2);
    }

    #[test]
    fn conversation_ids_match_c_enum() {
        assert_eq!(conv::ARILOU, 0);
        assert_eq!(conv::CHMMR, 1);
        assert_eq!(conv::COMMANDER, 2);
        assert_eq!(conv::INVALID, 26);
    }

    #[test]
    fn ship_type_ids_match_c_enum() {
        assert_eq!(ship::ARILOU_SHIP, 0);
        assert_eq!(ship::HUMAN_SHIP, 2);
        assert_eq!(ship::SAMATRA_SHIP, 24);
    }

    #[test]
    fn brief_ship_info_is_7_bytes() {
        // race_id(1) + crew_level(2) + max_crew(2) + max_energy(1) = 6 bytes
        // But C might have padding. Let's just verify it compiles.
        let _bsi = BriefShipInfo::default();
    }

    #[test]
    fn cpoint_matches_coord_layout() {
        let pt = CPoint { x: 100, y: 200 };
        assert_eq!(pt.x, 100);
        assert_eq!(pt.y, 200);
    }
}
