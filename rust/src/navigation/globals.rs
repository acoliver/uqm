//! Ownership and lifecycle of the live navigation world, ported from globdata.c.

#![allow(non_snake_case)]

use super::abi::*;
use crate::state::ffi;
use std::{mem, ptr};

unsafe fn create_radar() {
    use crate::engine::Kernel;
    if (*Kernel::current()).radar_context.is_null() {
        #[cfg(feature = "debug-process")]
        {
            (*Kernel::current()).radar_context = CreateContextAux(c"RadarContext".as_ptr());
        }
        #[cfg(not(feature = "debug-process"))]
        {
            (*Kernel::current()).radar_context = CreateContextAux();
        }
        let old = SetContext((*Kernel::current()).radar_context);
        SetContextFGFrame((*Kernel::current()).screen);
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
    use crate::engine::{Kernel, WorldData};
    if (*Kernel::current()).flag_stat_frame.is_null() {
        (*Kernel::current()).flag_stat_frame =
            CaptureDrawable(LoadGraphicInstance(c"graphics.flagshipstatus".as_ptr()).cast());
        if (*Kernel::current()).flag_stat_frame.is_null() {
            return 0;
        }
        (*Kernel::current()).misc_data_frame =
            CaptureDrawable(LoadGraphicInstance(c"graphics.miscdata".as_ptr()).cast());
        if (*Kernel::current()).misc_data_frame.is_null() {
            return 0;
        }
        (*Kernel::current()).font_grad_frame =
            CaptureDrawable(LoadGraphicInstance(c"graphics.fontgradient".as_ptr()).cast());
    }
    create_radar();
    if (*WorldData::current()).game.current_activity as u8 == IN_HYPERSPACE as u8 {
        (*WorldData::current()).game.ship_stamp.origin.x = -1;
        (*WorldData::current()).game.ship_stamp.origin.y = -1;
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
    use crate::engine::{Kernel, World as GameWorld, WorldData};
    InitGlobData();
    (*Kernel::current()).play_frame =
        CaptureDrawable(LoadGraphicInstance(c"graphics.playmenu".as_ptr()).cast());
    let mut species = SPECIES_ID_ARILOU_ID;
    let count = SPECIES_ID_KOHR_AH_ID - species + 1 + 2;
    let queue = ptr::addr_of_mut!((*WorldData::current()).game.avail_race_q);
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
            let master = find_master_ship(ship).unwrap_or_else(|| panic!("master species {ship}"));
            copy_fleet(fleet, &master.info, &master.fleet);
        } else {
            if let Some(desc) = load_ship(ship, 0) {
                copy_fleet(fleet, desc.info(), desc.fleet());
                unload_ship(desc, 0, 0);
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
    let state = ptr::addr_of_mut!((*WorldData::current()).game);
    let sis = ptr::addr_of_mut!((*WorldData::current()).flagship);
    (*state).glob_flags = 0;
    (*state).element_worth = [1, 2, 3, 4, 5, 6, 8, 25];
    (*sis).element_amounts = [0; 8];
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
    (*state).current_activity = (IN_INTERPLANETARY | START_INTERPLANETARY) as u16;
    super::initial_state::prices(&mut *sis, &mut *state);
    utf8StringCopy(
        ((*sis).ship_name.as_mut_ptr()).cast(),
        mem::size_of_val(&(*sis).ship_name),
        (GetStringAddress(SetAbsStringTableIndex(
            (*Kernel::current()).game_strings,
            (NAMING_STRING_BASE + 2) as u16,
        )))
        .cast(),
    );
    utf8StringCopy(
        ((*sis).commander_name.as_mut_ptr()).cast(),
        mem::size_of_val(&(*sis).commander_name),
        (GetStringAddress(SetAbsStringTableIndex(
            (*Kernel::current()).game_strings,
            (NAMING_STRING_BASE + 3) as u16,
        )))
        .cast(),
    );
    SetRaceAllied(HUMAN_SHIP as u16, 1);
    CloneShipFragment(HUMAN_SHIP as u16, &mut (*state).built_ship_q, 0);
    (*sis).log_x = crate::planets::universe_units::universe_to_logx(SOL_X as i16);
    (*sis).log_y = crate::planets::universe_units::universe_to_logy(SOL_Y as i16);
    (*GameWorld::current()).select_star(None, crate::planets::starmap::current_catalog());
    (*state).autopilot = POINT { x: -1, y: -1 };
    1
}

/// # Safety
/// No renderer may be using these contexts or resources.
pub unsafe fn FreeSC2Data() {
    use crate::engine::Kernel;
    DestroyContext((*Kernel::current()).radar_context);
    (*Kernel::current()).radar_context = ptr::null_mut();
    DestroyDrawable(ReleaseDrawable((*Kernel::current()).font_grad_frame));
    (*Kernel::current()).font_grad_frame = ptr::null_mut();
    DestroyDrawable(ReleaseDrawable((*Kernel::current()).misc_data_frame));
    (*Kernel::current()).misc_data_frame = ptr::null_mut();
    DestroyDrawable(ReleaseDrawable((*Kernel::current()).flag_stat_frame));
    (*Kernel::current()).flag_stat_frame = ptr::null_mut();
}

/// # Safety
/// The initialized game structures must no longer be in use.
pub unsafe fn UninitGameStructures() {
    use crate::engine::{Kernel, WorldData};
    let state = ptr::addr_of_mut!((*WorldData::current()).game);
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
    DestroyDrawable(ReleaseDrawable((*Kernel::current()).play_frame));
    (*Kernel::current()).play_frame = ptr::null_mut();
}

/// # Safety
/// Existing world resources must have been released before resetting their owner.
pub unsafe fn InitGlobData() {
    use crate::engine::WorldData;
    let flags = (*WorldData::current()).game.glob_flags;
    *WorldData::current() = crate::engine::WorldData::new();
    (*WorldData::current()).game.glob_flags = flags;
    ffi::rust_init_game_state();
    crate::engine::battle::access::lifecycle::bind_display_array(&mut *ptr::addr_of_mut!(
        (*WorldData::current()).game
    ));
}

/// # Safety
/// Game-thread access to the live activity.
#[no_mangle]
pub unsafe extern "C" fn inHQSpace() -> BOOLEAN {
    use crate::engine::WorldData;
    u32::from((*WorldData::current()).game.current_activity as u8 as u32 == IN_HYPERSPACE)
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
