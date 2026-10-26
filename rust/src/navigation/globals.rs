//! Ownership and lifecycle of the live navigation world, ported from globdata.c.

#![allow(non_snake_case)]

use super::abi::*;
use crate::state::ffi;
use std::{mem, ptr};

#[no_mangle]
pub static mut GlobData: crate::ships::native::abi::GLOBDATA = unsafe { mem::zeroed() };
#[no_mangle]
pub static mut RadarContext: CONTEXT = ptr::null_mut();
#[no_mangle]
pub static mut PlayFrame: FRAME = ptr::null_mut();

/// # Safety
/// Game-thread access to the initialized state singleton.
#[no_mangle]
pub unsafe extern "C" fn getGameState(start: i32, end: i32) -> u8 {
    ffi::rust_get_game_state_bits(start, end)
}
/// # Safety
/// Game-thread access to the initialized state singleton.
#[no_mangle]
pub unsafe extern "C" fn setGameState(start: i32, end: i32, value: u8) {
    ffi::rust_set_game_state_bits(start, end, value);
}

unsafe fn create_radar() {
    if RadarContext.is_null() {
        #[cfg(feature = "debug-process")]
        {
            RadarContext = CreateContextAux(c"RadarContext".as_ptr());
        }
        #[cfg(not(feature = "debug-process"))]
        {
            RadarContext = CreateContextAux();
        }
        let old = SetContext(RadarContext);
        SetContextFGFrame(Screen);
        let mut rect = RECT {
            corner: POINT {
                x: (4 + ScreenWidth - STATUS_WIDTH as i32 - SAFE_X as i32) as i16,
                y: (10 + ScreenHeight - 13 - RADAR_HEIGHT as i32) as i16,
            },
            extent: EXTENT {
                width: RADAR_WIDTH as i16,
                height: RADAR_HEIGHT as i16,
            },
        };
        SetContextClipRect(&mut rect);
        SetContext(old);
    }
}

/// # Safety
/// Resource system and screen contexts must be initialized on the game thread.
#[no_mangle]
pub unsafe extern "C" fn LoadSC2Data() -> BOOLEAN {
    if FlagStatFrame.is_null() {
        FlagStatFrame =
            CaptureDrawable(LoadGraphicInstance(c"graphics.flagshipstatus".as_ptr()).cast());
        if FlagStatFrame.is_null() {
            return 0;
        }
        MiscDataFrame = CaptureDrawable(LoadGraphicInstance(c"graphics.miscdata".as_ptr()).cast());
        if MiscDataFrame.is_null() {
            return 0;
        }
        FontGradFrame =
            CaptureDrawable(LoadGraphicInstance(c"graphics.fontgradient".as_ptr()).cast());
    }
    create_radar();
    if GlobData.Game_state.CurrentActivity as u8 == IN_HYPERSPACE as u8 {
        GlobData.Game_state.ShipStamp.origin.x = -1;
        GlobData.Game_state.ShipStamp.origin.y = -1;
    }
    1
}

unsafe fn copy_fleet(dst: *mut FLEET_INFO, src: *const SHIP_INFO, fleet: *const FLEET_STUFF) {
    (*dst).crew_level = (*src).crew_level;
    (*dst).max_crew = (*src).max_crew;
    (*dst).max_energy = (*src).max_energy;
    (*dst).race_strings = (*src).race_strings;
    (*dst).icons = (*src).icons;
    (*dst).melee_icon = (*src).melee_icon;
    (*dst).actual_strength = (*fleet).strength;
    (*dst).known_loc = (*fleet).known_loc;
}

/// # Safety
/// Game-thread initialization, before any world queues have been allocated.
#[no_mangle]
pub unsafe extern "C" fn InitGameStructures() -> BOOLEAN {
    InitGlobData();
    PlayFrame = CaptureDrawable(LoadGraphicInstance(c"graphics.playmenu".as_ptr()).cast());
    let mut species = SPECIES_ID_ARILOU_ID;
    let count = SPECIES_ID_KOHR_AH_ID - species + 1 + 2;
    let queue = ptr::addr_of_mut!(GlobData.Game_state.avail_race_q);
    InitQueue(queue, count as u16, mem::size_of::<FLEET_INFO>() as u16);
    for i in 0..count {
        let ship = if i < count - 2 {
            let value = species;
            species += 1;
            value
        } else if i == count - 2 {
            SPECIES_ID_YEHAT_ID
        } else {
            SPECIES_ID_UR_QUAN_PROBE_ID
        };
        let handle = AllocLink(queue);
        if handle.is_null() {
            continue;
        }
        let fleet = handle.cast::<FLEET_INFO>();
        (*fleet).SpeciesID = ship;
        if i < count - 1 {
            let master = FindMasterShip(ship).cast::<MASTER_SHIP_INFO>();
            copy_fleet(fleet, &(*master).ShipInfo, &(*master).Fleet);
        } else {
            let desc = c_load_ship(ship, 0);
            if !desc.is_null() {
                copy_fleet(fleet, &(*desc).ship_info, &(*desc).fleet);
                c_free_ship(desc, 0, 0);
            }
        }
        (*fleet).allied_state = BAD_GUY as u16;
        (*fleet).known_strength = 0;
        (*fleet).loc = (*fleet).known_loc;
        if i == YEHAT_REBEL_SHIP {
            (*fleet).actual_strength = 0;
        }
        (*fleet).growth = 0;
        (*fleet).growth_fract = 0;
        (*fleet).growth_err_term = 127;
        (*fleet).days_left = 0;
        (*fleet).func_index = u8::MAX;
        PutQueue(queue, handle);
    }
    InitSISContexts();
    LoadSC2Data();
    InitPlanetInfo();
    InitGroupInfo(1);
    let state = ptr::addr_of_mut!(GlobData.Game_state);
    let sis = ptr::addr_of_mut!(GlobData.SIS_state);
    (*state).glob_flags = 0;
    (*state).ElementWorth = [1, 2, 3, 4, 5, 6, 8, 25];
    (*sis).ElementAmounts = [0; 8];
    super::initial_state::equipment(&mut *sis);
    InitQueue(
        &mut (*state).built_ship_q,
        MAX_BUILT_SHIPS as u16,
        mem::size_of::<SHIP_FRAGMENT>() as u16,
    );
    InitQueue(
        &mut (*state).npc_built_ship_q,
        MAX_SHIPS_PER_SIDE as u16,
        mem::size_of::<SHIP_FRAGMENT>() as u16,
    );
    InitQueue(
        &mut (*state).ip_group_q,
        MAX_BATTLE_GROUPS as u16,
        mem::size_of::<IP_GROUP>() as u16,
    );
    InitQueue(
        &mut (*state).encounter_q,
        MAX_ENCOUNTERS as u16,
        mem::size_of::<ENCOUNTER>() as u16,
    );
    (*state).CurrentActivity = (IN_INTERPLANETARY | START_INTERPLANETARY) as u16;
    super::initial_state::prices(&mut *sis, &mut *state);
    utf8StringCopy(
        ((*sis).ShipName.as_mut_ptr()).cast(),
        mem::size_of_val(&(*sis).ShipName),
        (GetStringAddress(SetAbsStringTableIndex(
            GameStrings,
            (NAMING_STRING_BASE + 2) as u16,
        )))
        .cast(),
    );
    utf8StringCopy(
        ((*sis).CommanderName.as_mut_ptr()).cast(),
        mem::size_of_val(&(*sis).CommanderName),
        (GetStringAddress(SetAbsStringTableIndex(
            GameStrings,
            (NAMING_STRING_BASE + 3) as u16,
        )))
        .cast(),
    );
    SetRaceAllied(HUMAN_SHIP as u16, 1);
    CloneShipFragment(HUMAN_SHIP as u16, &mut (*state).built_ship_q, 0);
    (*sis).log_x = crate::planets::universe_units::universe_to_logx(SOL_X as i16);
    (*sis).log_y = crate::planets::universe_units::universe_to_logy(SOL_Y as i16);
    crate::planets::starmap::CurStarDescPtr = ptr::null_mut();
    (*state).autopilot = POINT { x: -1, y: -1 };
    1
}

/// # Safety
/// No renderer may be using these contexts or resources.
pub unsafe fn FreeSC2Data() {
    DestroyContext(RadarContext);
    RadarContext = ptr::null_mut();
    DestroyDrawable(ReleaseDrawable(FontGradFrame));
    FontGradFrame = ptr::null_mut();
    DestroyDrawable(ReleaseDrawable(MiscDataFrame));
    MiscDataFrame = ptr::null_mut();
    DestroyDrawable(ReleaseDrawable(FlagStatFrame));
    FlagStatFrame = ptr::null_mut();
}

/// # Safety
/// The initialized game structures must no longer be in use.
pub unsafe fn UninitGameStructures() {
    let state = ptr::addr_of_mut!(GlobData.Game_state);
    UninitQueue(ptr::addr_of_mut!((*state).encounter_q));
    UninitQueue(ptr::addr_of_mut!((*state).ip_group_q));
    UninitQueue(ptr::addr_of_mut!((*state).npc_built_ship_q));
    UninitQueue(ptr::addr_of_mut!((*state).built_ship_q));
    UninitGroupInfo();
    UninitPlanetInfo();
    let probe = (*state).avail_race_q.tail.cast::<FLEET_INFO>();
    if !probe.is_null() {
        DestroyDrawable(ReleaseDrawable((*probe).melee_icon));
        DestroyDrawable(ReleaseDrawable((*probe).icons));
        DestroyStringTable(ReleaseStringTable((*probe).race_strings));
    }
    UninitQueue(ptr::addr_of_mut!((*state).avail_race_q));
    DestroyDrawable(ReleaseDrawable(PlayFrame));
    PlayFrame = ptr::null_mut();
}

/// # Safety
/// Existing world resources must have been released before resetting their owner.
pub unsafe fn InitGlobData() {
    let flags = GlobData.Game_state.glob_flags;
    ptr::write_bytes(ptr::addr_of_mut!(GlobData), 0, 1);
    GlobData.Game_state.glob_flags = flags;
    ffi::rust_init_game_state();
    GlobData.Game_state.DisplayArray = ptr::addr_of_mut!(DisplayArray).cast();
}

/// # Safety
/// Game-thread access to the live activity.
#[no_mangle]
pub unsafe extern "C" fn inHQSpace() -> BOOLEAN {
    u32::from(GlobData.Game_state.CurrentActivity as u8 as u32 == IN_HYPERSPACE)
}
/// # Safety
/// Game-thread access to the live activity and game-state bits.
#[no_mangle]
pub unsafe extern "C" fn inHyperSpace() -> BOOLEAN {
    u32::from(inHQSpace() != 0 && ffi::rust_get_game_state_bits(42, 43) <= 1)
}
/// # Safety
/// Game-thread access to the live activity and game-state bits.
#[no_mangle]
pub unsafe extern "C" fn inQuasiSpace() -> BOOLEAN {
    u32::from(inHQSpace() != 0 && ffi::rust_get_game_state_bits(42, 43) > 1)
}
