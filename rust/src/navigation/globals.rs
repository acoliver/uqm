//! Ownership and lifecycle of the live navigation world, ported from globdata.c.

use super::abi::*;
use crate::engine::{
    world::new_game::{FleetRoster, FleetTemplate, NewGameInputs},
    StoryKey,
};
#[cfg(not(test))]
use crate::ships::DescriptorLoader;
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
    let kernel = &mut *cx.kernel;
    if kernel.flag_stat_frame.is_null() {
        kernel.flag_stat_frame =
            CaptureDrawable(LoadGraphicInstance(c"graphics.flagshipstatus".as_ptr()).cast());
        if kernel.flag_stat_frame.is_null() {
            return 0;
        }
        kernel.misc_data_frame =
            CaptureDrawable(LoadGraphicInstance(c"graphics.miscdata".as_ptr()).cast());
        if kernel.misc_data_frame.is_null() {
            return 0;
        }
        kernel.font_grad_frame =
            CaptureDrawable(LoadGraphicInstance(c"graphics.fontgradient".as_ptr()).cast());
    }
    create_radar(kernel);
    if cx.world.activity() as u8 == IN_HYPERSPACE as u8 {
        cx.world.data.game.ship_stamp.origin.x = -1;
        cx.world.data.game.ship_stamp.origin.y = -1;
    }
    1
}

fn fleet_template(info: &SHIP_INFO, fleet: &FLEET_STUFF) -> FleetTemplate {
    FleetTemplate {
        crew_level: info.crew_level,
        max_crew: info.max_crew,
        max_energy: info.max_energy,
        race_strings: info.race_strings,
        icons: info.icons,
        melee_icon: info.melee_icon,
        strength: fleet.strength,
        known_location: fleet.known_loc,
    }
}

fn initialize_races(cx: &mut crate::engine::battle::Cx<'_>) {
    let kernel = &mut *cx.kernel;
    cx.world.initialize_fleets(
        FleetRoster {
            first_species: SPECIES_ID_ARILOU_ID,
            last_species: SPECIES_ID_KOHR_AH_ID,
            rebel_species: SPECIES_ID_YEHAT_ID,
            probe_species: SPECIES_ID_UR_QUAN_PROBE_ID,
            rebel_slot: YEHAT_REBEL_SHIP as usize,
        },
        |world, species| {
            if species == SPECIES_ID_UR_QUAN_PROBE_ID {
                if let Some(desc) = crate::engine::battle::RaceDesc::load(world, species, false) {
                    let template = fleet_template(desc.info(), desc.fleet());
                    world.retain_fleet_resources(species, desc.picker_resources());
                    desc.unload(world, false, false);
                    template
                } else {
                    FleetTemplate::default()
                }
            } else {
                let master = find_master_ship(kernel, species)
                    .unwrap_or_else(|| panic!("master species {species}"));
                world.retain_fleet_resources(
                    species,
                    crate::ships::master_native::engine::picker_resources(kernel, species),
                );
                fleet_template(&master.info, &master.fleet)
            }
        },
    );
}

/// # Safety
/// Game-thread initialization, before any world queues have been allocated.
pub unsafe fn init_game_structures(cx: &mut crate::engine::battle::Cx<'_>) -> BOOLEAN {
    let battle = &mut *cx.b;
    let kernel = &mut *cx.kernel;
    let rng = cx.rng;
    cx.world.new_game(
        |world| {
            let mut cx = crate::engine::battle::Cx::new(battle, world, kernel, rng);
            cx.kernel.play_frame =
                CaptureDrawable(LoadGraphicInstance(c"graphics.playmenu".as_ptr()).cast());
            initialize_races(&mut cx);
            let dimensions = cx.screen_dimensions();
            init_siscontexts(cx.kernel, dimensions);
            load_sc2_data(&mut cx);
            let catalog = crate::planets::starmap::current_catalog()
                .unwrap_or_else(|| panic!("planet state initialization requires the star catalog"));
            crate::state::ffi::rust_init_planet_info(
                cx.world,
                crate::planets::starmap::catalog_star_count(catalog) as i32,
            );
            init_group_info(cx.world, 1);
            let strings = cx.kernel.game_strings;
            let mut ship_name = [0; 16];
            let mut commander_name = [0; 16];
            utf8StringCopy(
                ship_name.as_mut_ptr().cast(),
                mem::size_of_val(&ship_name),
                GetStringAddress(SetAbsStringTableIndex(
                    strings,
                    (NAMING_STRING_BASE + 2) as u16,
                ))
                .cast(),
            );
            utf8StringCopy(
                commander_name.as_mut_ptr().cast(),
                mem::size_of_val(&commander_name),
                GetStringAddress(SetAbsStringTableIndex(
                    strings,
                    (NAMING_STRING_BASE + 3) as u16,
                ))
                .cast(),
            );
            NewGameInputs {
                activity: (IN_INTERPLANETARY | START_INTERPLANETARY) as u16,
                human_fleet: HUMAN_SHIP as usize,
                log_position: (
                    crate::planets::universe_units::universe_to_logx(SOL_X as i16),
                    crate::planets::universe_units::universe_to_logy(SOL_Y as i16),
                ),
                ship_name,
                commander_name,
            }
        },
        crate::comm::dispatch::production_captain_name,
    );
    1
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
pub unsafe fn uninit_game_structures(cx: &mut crate::engine::battle::Cx<'_>) {
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
    cx.world.clear_fleet_resources();
    DestroyDrawable(ReleaseDrawable(cx.kernel.play_frame));
    cx.kernel.play_frame = ptr::null_mut();
}

/// Reset campaign values, retaining the caller’s global flags. Queue/resource
/// teardown is a separate lifecycle step; this does not release those handles.
pub fn init_glob_data(world: &mut crate::engine::World) {
    world.reset_game_data();
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
