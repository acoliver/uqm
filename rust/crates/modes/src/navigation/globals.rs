//! Ownership and lifecycle of the live navigation world, ported from globdata.c.

use super::groups::{init_group_info, uninit_group_info};
use crate::navigation::hyper_units::{SOL_X, SOL_Y};
use crate::starbase::ids::NAMING_STRING_BASE;
use crate::starbase::ids::{RADAR_HEIGHT, RADAR_WIDTH, SAFE_X, STATUS_WIDTH};
use crate::ui::border::init_siscontexts;
use uqm_content::ships::DescriptorLoader;
use uqm_content::ships::{master_native::find_master_ship, types::SpeciesId};
use uqm_engine::battle::record::{FleetRecord, ShipInfoRecord};
use uqm_engine::graphics::{geometry::Extent, geometry::Point, geometry::Rect};
use uqm_engine::world::activity::activity_flags::START_INTERPLANETARY;
use uqm_engine::world::activity::ActivityKind;
use uqm_engine::world::new_game::{FleetRoster, FleetTemplate, NewGameInputs};

fn load_sc2_data(cx: &mut uqm_engine::battle::Cx<'_>) -> u32 {
    let (width, height) = cx.screen_dimensions();
    let radar_clip = Rect {
        corner: Point {
            x: (4 + width - STATUS_WIDTH - i32::from(SAFE_X)) as i16,
            y: (10 + height - 13 - RADAR_HEIGHT) as i16,
        },
        extent: Extent {
            width: RADAR_WIDTH as i16,
            height: RADAR_HEIGHT as i16,
        },
    };
    if !cx.kernel.load_navigation_graphics(radar_clip) {
        return 0;
    }
    if cx.world.activity() as u8 == ActivityKind::InHyperspace as u8 {
        cx.world.data.game.ship_stamp.origin.x = -1;
        cx.world.data.game.ship_stamp.origin.y = -1;
    }
    1
}

fn fleet_template(info: &ShipInfoRecord, fleet: &FleetRecord) -> FleetTemplate {
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

fn initialize_races(cx: &mut uqm_engine::battle::Cx<'_>) {
    let kernel = &mut *cx.kernel;
    cx.world.initialize_fleets(
        FleetRoster {
            first_species: SpeciesId::Arilou as u32,
            last_species: SpeciesId::KohrAh as u32,
            rebel_species: SpeciesId::Yehat as u32,
            probe_species: SpeciesId::UrQuanProbe as u32,
            rebel_slot: usize::from(uqm_engine::species::ship::YEHAT_REBEL_SHIP),
        },
        |world, species| {
            if species == SpeciesId::UrQuanProbe as u32 {
                if let Some(desc) = uqm_engine::battle::RaceDesc::load(world, species, false) {
                    let template = fleet_template(desc.info(), desc.fleet());
                    world.retain_fleet_resources(species, desc.picker_resources());
                    let assets = desc.into_campaign_probe(world);
                    world.retain_campaign_probe(assets);
                    template
                } else {
                    FleetTemplate::default()
                }
            } else {
                let master = find_master_ship(kernel, species)
                    .unwrap_or_else(|| panic!("master species {species}"));
                world.retain_fleet_resources(
                    species,
                    uqm_content::ships::master_native::engine::picker_resources(kernel, species),
                );
                fleet_template(&master.info, &master.fleet)
            }
        },
    );
}

fn campaign_names(kernel: &uqm_engine::Kernel) -> ([u8; 16], [u8; 16]) {
    let copy = |offset| {
        uqm_engine::save::summary::utf8_string_copy(
            kernel
                .game_text(NAMING_STRING_BASE as usize + offset)
                .to_bytes(),
        )
    };
    (copy(2), copy(3))
}

/// Initialize campaign owners before allocating the new world queues.
pub fn init_game_structures(cx: &mut uqm_engine::battle::Cx<'_>) -> u32 {
    let battle = &mut *cx.b;
    let kernel = &mut *cx.kernel;
    let rng = cx.rng;
    cx.world.new_game(
        |world| {
            let mut cx = uqm_engine::battle::Cx::new(battle, world, kernel, rng);
            cx.kernel.load_navigation_play();
            initialize_races(&mut cx);
            let dimensions = cx.screen_dimensions();
            init_siscontexts(cx.kernel, dimensions);
            load_sc2_data(&mut cx);
            let catalog = cx.world.catalog();
            uqm_engine::state::ffi::rust_init_planet_info(
                cx.world,
                crate::planets::starmap::catalog_star_count(catalog) as i32,
            );
            init_group_info(cx.world, 1);
            let (ship_name, commander_name) = campaign_names(cx.kernel);
            NewGameInputs {
                activity: ActivityKind::InInterplanetary as u16 | START_INTERPLANETARY,
                human_fleet: usize::from(uqm_engine::species::ship::HUMAN_SHIP),
                log_position: (
                    crate::planets::universe_units::universe_to_logx(SOL_X),
                    crate::planets::universe_units::universe_to_logy(SOL_Y),
                ),
                ship_name,
                commander_name,
            }
        },
        crate::comm::dispatch::production_captain_name,
    );
    1
}

/// Release campaign resources after the active game has stopped using them.
pub fn uninit_game_structures(cx: &mut uqm_engine::battle::Cx<'_>) {
    cx.world.encounters().reinit();
    cx.world.ip_groups().reinit();
    cx.world.npc_built_ships().reinit();
    cx.world.built_ships().reinit();
    uninit_group_info(cx.world);
    uqm_engine::state::ffi::rust_uninit_planet_info(cx.world);
    cx.world.release_campaign_probe();
    cx.world.avail_races().reinit();
    cx.world.clear_fleet_resources();
    cx.kernel.release_navigation_play();
}

#[cfg(test)]
mod tests {
    use super::*;
    use uqm_engine::{world::state_file::RANDGRPINFO_FILE, StoryKey, World};
    const IN_HYPERSPACE: u16 = ActivityKind::InHyperspace as u16;

    #[test]
    fn campaign_names_read_retained_text_and_keep_byte_truncation_and_padding() {
        use std::ffi::CString;
        use uqm_engine::strings::owned_table::LoadedTextTable;
        let mut kernel = uqm_engine::Kernel::new();
        let mut strings = vec![CString::default(); NAMING_STRING_BASE as usize + 4];
        strings[NAMING_STRING_BASE as usize + 2] = CString::new("12345678901234é").unwrap();
        strings[NAMING_STRING_BASE as usize + 3] = CString::new("Zelnick").unwrap();
        kernel.install_game_text(LoadedTextTable::new(strings));
        let (ship, commander) = super::campaign_names(&kernel);
        assert_eq!(&ship[..14], b"12345678901234");
        assert_eq!(&ship[14..], &[0xc3, 0]);
        assert_eq!(&commander[..7], b"Zelnick");
        assert_eq!(&commander[7..], &[0; 9]);
        drop(kernel);
        assert_eq!(ship[14], 0xc3);
    }

    #[test]
    fn reset_retains_flags_queues_and_files_in_the_supplied_world() {
        let reset: fn(&mut World) = World::reset_game_data;
        let mut world = World::new();
        let mut other = World::new();
        world.data.game.glob_flags = 0xa5;
        world.data.flagship.fuel_on_board = 1234;
        world.set_activity(IN_HYPERSPACE);
        world.story = Some(uqm_engine::world::story::StoryState::new());
        world.set_story_state(StoryKey::ARILOU_SPACE_SIDE, 2);
        other.set_activity(IN_HYPERSPACE);
        let mut queue = world.ip_groups();
        let group = queue.alloc().unwrap();
        queue.get_mut(group).group_id = 7;
        queue.put(group);
        world
            .files
            .replace_contents(RANDGRPINFO_FILE, b"retained groups")
            .unwrap();

        assert_eq!(world.story_state(StoryKey::ARILOU_SPACE_SIDE), 2);
        reset(&mut world);
        assert_eq!(world.data.game.glob_flags, 0xa5);
        assert_eq!(world.data.flagship.fuel_on_board, 0);
        assert_eq!(world.activity(), 0);
        assert_eq!(world.story_state(StoryKey::ARILOU_SPACE_SIDE), 0);
        assert_eq!(world.ip_groups_ref().head(), Some(group));
        assert_eq!(world.ip_groups_ref().get(group).group_id, 7);
        assert_eq!(
            world.files.get_file(RANDGRPINFO_FILE).unwrap().contents(),
            b"retained groups"
        );
        assert_eq!(other.activity(), IN_HYPERSPACE);
    }
}
