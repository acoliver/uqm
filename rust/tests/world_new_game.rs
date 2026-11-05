use std::cell::RefCell;
use uqm_rust::engine::{
    world::{
        new_game::{FleetRoster, FleetTemplate, NewGameInputs},
        story::GameState,
    },
    StoryKey, World,
};

fn roster() -> FleetRoster {
    FleetRoster {
        first_species: 0,
        last_species: 2,
        rebel_species: 1,
        probe_species: 5,
        rebel_slot: 3,
    }
}

#[test]
fn new_game_prepares_resources_after_reset_and_installs_the_owned_campaign() {
    let mut world = World::new();
    world.data.game.glob_flags = 0xa4;
    world.data.flagship.crew_enlisted = 999;
    world.story = Some(GameState::new());
    world.set_story_state(StoryKey::STARBASE_AVAILABLE, 1);
    let old_ship = world.built_ships().alloc().unwrap();
    world.built_ships().put(old_ship);
    let stages = RefCell::new(Vec::new());
    let mut loaded = Vec::new();

    world.new_game(
        |world| {
            stages.borrow_mut().push("prepare");
            assert_eq!(world.data.game.glob_flags, 0xa4);
            assert_eq!(world.data.flagship.crew_enlisted, 0);
            assert_eq!(world.story_value(StoryKey::STARBASE_AVAILABLE), Some(0));
            assert_eq!(world.built_ships_ref().count(), 1);
            world.initialize_fleets(roster(), |world, species| {
                assert_eq!(world.avail_races_ref().count(), loaded.len());
                loaded.push(species);
                FleetTemplate {
                    crew_level: 18,
                    max_crew: 20,
                    max_energy: 12,
                    strength: 100,
                    ..FleetTemplate::default()
                }
            });
            NewGameInputs {
                activity: 0x0804,
                human_fleet: 1,
                log_position: (123, 456),
                ship_name: *b"Flagship\0\0\0\0\0\0\0\0",
                commander_name: *b"Captain\0\0\0\0\0\0\0\0\0",
            }
        },
        || {
            stages.borrow_mut().push("captain");
            7
        },
    );

    assert_eq!(loaded, [0, 1, 2, 1, 5]);
    assert_eq!(stages.into_inner(), ["prepare", "captain"]);
    assert_eq!(world.data.game.glob_flags, 0);
    assert_eq!(world.activity(), 0x0804);
    assert_eq!(world.data.game.element_worth, [1, 2, 3, 4, 5, 6, 8, 25]);
    assert_eq!(world.data.game.crew_cost, 3);
    assert_eq!(world.data.game.fuel_cost, 20);
    assert_eq!(world.data.flagship.crew_enlisted, 50);
    assert_eq!(world.data.flagship.fuel_on_board, 1000);
    assert_eq!(world.data.flagship.num_landers, 1);
    assert_eq!(world.data.flagship.log_x, 123);
    assert_eq!(world.data.flagship.log_y, 456);
    assert_eq!(&world.data.flagship.ship_name[..8], b"Flagship");
    assert_eq!(&world.data.flagship.commander_name[..7], b"Captain");
    assert_eq!(world.data.game.autopilot.x, -1);
    assert_eq!(world.data.game.autopilot.y, -1);
    assert!(world.current_star.is_none());
    let ships = world.built_ships_ref();
    assert_eq!(ships.count(), 1);
    let escort = ships.get(ships.head().unwrap());
    assert_eq!((escort.species_id, escort.race_id), (1, 1));
    assert_eq!((escort.crew_level, escort.max_crew), (18, 20));
    assert_eq!(escort.captains_name_index, 7);
    let fleets = world.avail_races_ref();
    assert_eq!(fleets.get(fleets.at(1).unwrap()).allied_state, 1);
    assert_eq!(fleets.get(fleets.at(3).unwrap()).actual_strength, 0);
    assert_eq!(world.npc_built_ships_ref().count(), 0);
    assert_eq!(world.ip_groups_ref().count(), 0);
    assert_eq!(world.encounters_ref().count(), 0);
}

#[test]
fn fleet_initialization_replaces_the_old_roster_and_resets_growth_state() {
    let mut world = World::new();
    for _ in 0..2 {
        world.initialize_fleets(roster(), |_, species| FleetTemplate {
            crew_level: species as u16,
            strength: 40,
            ..FleetTemplate::default()
        });
        let fleets = world.avail_races_ref();
        assert_eq!(fleets.count(), 5);
        for index in 0..5 {
            let fleet = fleets.get(fleets.at(index).unwrap());
            assert_eq!(fleet.allied_state, 2);
            assert_eq!(fleet.known_strength, 0);
            assert_eq!(fleet.growth, 0);
            assert_eq!(fleet.growth_fract, 0);
            assert_eq!(fleet.growth_err_term, 127);
            assert_eq!(fleet.days_left, 0);
            assert_eq!(fleet.func_index, u8::MAX);
            assert_eq!(fleet.actual_strength, if index == 3 { 0 } else { 40 });
        }
    }
}
