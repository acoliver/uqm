//! Ownership and lifecycle of the live navigation world, ported from globdata.c.

use super::abi::*;
use crate::engine::StoryKey;
use std::{mem, ptr};

unsafe fn create_radar(kernel: &mut crate::engine::Kernel) {
    if kernel.radar_context.is_null() {
        #[cfg(feature = "debug-process")]
        {
            kernel.radar_context = CreateContextAux(c"RadarContext".as_ptr());
        }
        #[cfg(not(feature = "debug-process"))]
        {
            (*kernel).radar_context = CreateContextAux();
        }
        let old = SetContext(kernel.radar_context);
        SetContextFGFrame(kernel.screen);
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
pub unsafe fn load_sc2_data(cx: &mut crate::engine::battle::Cx<'_>) -> BOOLEAN {
    let kernel = cx.kernel();
    let world = &raw mut cx.world.data;
    if (*kernel).flag_stat_frame.is_null() {
        (*kernel).flag_stat_frame =
            CaptureDrawable(LoadGraphicInstance(c"graphics.flagshipstatus".as_ptr()).cast());
        if (*kernel).flag_stat_frame.is_null() {
            return 0;
        }
        (*kernel).misc_data_frame =
            CaptureDrawable(LoadGraphicInstance(c"graphics.miscdata".as_ptr()).cast());
        if (*kernel).misc_data_frame.is_null() {
            return 0;
        }
        (*kernel).font_grad_frame =
            CaptureDrawable(LoadGraphicInstance(c"graphics.fontgradient".as_ptr()).cast());
    }
    create_radar(cx.kernel);
    if (*world).game.current_activity as u8 == IN_HYPERSPACE as u8 {
        (*world).game.ship_stamp.origin.x = -1;
        (*world).game.ship_stamp.origin.y = -1;
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

unsafe fn initialize_races(cx: &mut crate::engine::battle::Cx<'_>) {
    let mut species = SPECIES_ID_ARILOU_ID;
    let count = SPECIES_ID_KOHR_AH_ID - species + 1 + 2;
    cx.world.avail_races().reinit();
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
        let handle = cx.world.avail_races().alloc().expect("fleet capacity");
        let mut value = FLEET_INFO::default();
        let fleet = &raw mut value;
        (*fleet).species_id = ship;
        if i < count - 1 {
            let master = find_master_ship(cx.kernel, ship)
                .unwrap_or_else(|| panic!("master species {ship}"));
            copy_fleet(fleet, &master.info, &master.fleet);
        } else {
            if let Some(desc) = load_ship(cx.world, ship, 0) {
                copy_fleet(fleet, desc.info(), desc.fleet());
                unload_ship(cx.world, desc, 0, 0);
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
        *cx.world.avail_races().get_mut(handle) = value;
        cx.world.avail_races().put(handle);
    }
}

/// # Safety
/// Game-thread initialization, before any world queues have been allocated.
#[no_mangle]
pub unsafe extern "C" fn InitGameStructures() -> BOOLEAN {
    // SAFETY: the lifecycle borrows owners from Cx; callbacks republish it synchronously.
    crate::app::bridge::enter(|cx| {
        init_glob_data(cx.world);
        cx.kernel.play_frame =
            CaptureDrawable(LoadGraphicInstance(c"graphics.playmenu".as_ptr()).cast());
        // SAFETY: initialized master templates and the owned World remain live through construction.
        unsafe {
            initialize_races(cx);
        }
        init_siscontexts(cx.kernel);
        load_sc2_data(cx);
        let catalog = crate::planets::starmap::current_catalog()
            .unwrap_or_else(|| panic!("planet state initialization requires the star catalog"));
        crate::state::ffi::rust_init_planet_info(
            cx.world,
            crate::planets::starmap::catalog_star_count(catalog) as i32,
        );
        init_group_info(cx.world, 1);
        let strings = cx.kernel.game_strings;
        let state = &raw mut cx.world.data.game;
        let sis = &raw mut cx.world.data.flagship;
        (*state).glob_flags = 0;
        (*state).element_worth = [1, 2, 3, 4, 5, 6, 8, 25];
        (*sis).element_amounts = [0; 8];
        super::initial_state::equipment(&mut *sis);
        cx.world.built_ships().reinit();
        cx.world.npc_built_ships().reinit();
        cx.world.ip_groups().reinit();
        cx.world.encounters().reinit();
        (*state).current_activity = (IN_INTERPLANETARY | START_INTERPLANETARY) as u16;
        super::initial_state::prices(&mut *sis, &mut *state);
        utf8StringCopy(
            ((*sis).ship_name.as_mut_ptr()).cast(),
            mem::size_of_val(&(*sis).ship_name),
            (GetStringAddress(SetAbsStringTableIndex(
                strings,
                (NAMING_STRING_BASE + 2) as u16,
            )))
            .cast(),
        );
        utf8StringCopy(
            ((*sis).commander_name.as_mut_ptr()).cast(),
            mem::size_of_val(&(*sis).commander_name),
            (GetStringAddress(SetAbsStringTableIndex(
                strings,
                (NAMING_STRING_BASE + 3) as u16,
            )))
            .cast(),
        );
        let sis = &raw mut cx.world.data.flagship;
        let human = cx
            .world
            .avail_races_ref()
            .at(HUMAN_SHIP as usize)
            .expect("human fleet");
        cx.world.avail_races().get_mut(human).allied_state = GOOD_GUY as u16;
        let template = *cx.world.avail_races_ref().get(human);
        cx.world.built_ships().clone_fragment(
            &template,
            HUMAN_SHIP as u8,
            0,
            crate::comm::dispatch::production_captain_name,
        );
        (*sis).log_x = crate::planets::universe_units::universe_to_logx(SOL_X as i16);
        (*sis).log_y = crate::planets::universe_units::universe_to_logy(SOL_Y as i16);
        cx.world
            .select_star(None, crate::planets::starmap::current_catalog());
        cx.world.data.game.autopilot = POINT { x: -1, y: -1 };
        1
    })
}

/// # Safety
/// No renderer may be using these contexts or resources.
pub unsafe fn free_sc2_data(kernel: &mut crate::engine::Kernel) {
    DestroyContext(kernel.radar_context);
    kernel.radar_context = ptr::null_mut();
    DestroyDrawable(ReleaseDrawable(kernel.font_grad_frame));
    kernel.font_grad_frame = ptr::null_mut();
    DestroyDrawable(ReleaseDrawable(kernel.misc_data_frame));
    kernel.misc_data_frame = ptr::null_mut();
    DestroyDrawable(ReleaseDrawable(kernel.flag_stat_frame));
    kernel.flag_stat_frame = ptr::null_mut();
}

/// # Safety
/// The initialized game structures must no longer be in use.
pub unsafe fn uninit_game_structures() {
    // SAFETY: the lifecycle borrows owners from Cx; callbacks republish it synchronously.
    crate::app::bridge::enter(|cx| {
        let kernel = cx.kernel();
        cx.world.encounters().reinit();
        cx.world.ip_groups().reinit();
        cx.world.npc_built_ships().reinit();
        cx.world.built_ships().reinit();
        uninit_group_info(cx.world);
        uninit_planet_info(cx.world);
        if let Some(probe) = cx.world.avail_races_ref().tail() {
            let probe = *cx.world.avail_races_ref().get(probe);
            DestroyDrawable(ReleaseDrawable(probe.melee_icon));
            DestroyDrawable(ReleaseDrawable(probe.icons));
            DestroyStringTable(ReleaseStringTable(probe.race_strings));
        }
        cx.world.avail_races().reinit();
        DestroyDrawable(ReleaseDrawable((*kernel).play_frame));
        (*kernel).play_frame = ptr::null_mut();
    })
}

/// Reset campaign values, retaining the caller’s global flags. Queue/resource
/// teardown is a separate lifecycle step; this does not release those handles.
pub fn init_glob_data(world: &mut crate::engine::World) {
    let flags = world.data.game.glob_flags;
    world.data = crate::engine::WorldData::new();
    world.data.game.glob_flags = flags;
    world.story = Some(crate::engine::world::story::GameState::new());
    world.files.canned_nodes.clear();
    crate::engine::battle::access::lifecycle::bind_display_array(&mut world.data.game);
}

pub fn in_hqspace(world: &crate::engine::World) -> BOOLEAN {
    u32::from(world.activity() as u8 as u32 == IN_HYPERSPACE)
}

pub fn in_hyper_space(world: &crate::engine::World) -> BOOLEAN {
    u32::from(in_hqspace(world) != 0 && world.story_state(StoryKey::ARILOU_SPACE_SIDE) <= 1)
}

pub fn in_quasi_space(world: &crate::engine::World) -> BOOLEAN {
    u32::from(in_hqspace(world) != 0 && world.story_state(StoryKey::ARILOU_SPACE_SIDE) > 1)
}
