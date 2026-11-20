//! Production checkpoint and fighter callers, including cross-process address stability.
#![forbid(unsafe_code)]
#[path = "support/battle.rs"]
mod battle;
#[path = "support/battle_context.rs"]
mod battle_context;
const CANNON_WEAPON: u32 = 11;
const GUN_WEAPON: u32 = 9;
const TRACKING_SYSTEM: u32 = 12;

use std::cell::RefMut;
use std::{collections::BTreeMap, fs, path::Path, process::Command, ptr};
use uqm_engine::battle::frames::FrameArray;
use uqm_engine::battle::record::CaptainRecord;
use uqm_engine::battle::record::CharacteristicRecord;
use uqm_engine::battle::record::RaceDescriptor;
use uqm_engine::battle::views::FrameSelection;
use uqm_engine::collections::observation;
use uqm_engine::collections::observation::Event;
use uqm_engine::graphics::geometry::Point;
use uqm_engine::species::species_id::SPECIES_ID_ANDROSYNTH_ID;
use uqm_engine::species::species_id::SPECIES_ID_EARTHLING_ID;
use uqm_engine::species::species_id::SPECIES_ID_MMRNMHRM_ID;
use uqm_engine::species::species_id::SPECIES_ID_PKUNK_ID;
use uqm_engine::species::species_id::SPECIES_ID_SIS_SHIP_ID;
use uqm_engine::species::species_id::SPECIES_ID_UMGAH_ID;
use uqm_engine::world::activity::activity_kinds::SUPER_MELEE;
use uqm_modes::battle::element::Element;
use uqm_modes::battle::element::ElementFlags;
use uqm_modes::battle::netplay::CrcState;
use uqm_rust::automation::battle_fight::BattleSnapshot;
use uqm_rust::automation::script::parse_script;
use uqm_rust::automation::script::validate_script;
use uqm_rust::automation::AutomationObserver;

#[path = "parity_state/graph.rs"]
mod graph;
use graph::{
    attach_elements, clear_queues, elements_and_fighters, event_queue_fields,
    fighters_without_journal, non_ship_parent_fields,
};

/// Lend the descriptor owned by the fixture ship until the guard drops.
fn descriptor_mut(ship: &mut battle::PoolShip) -> RefMut<'static, RaceDescriptor> {
    RefMut::map(ship.record(), |ship| {
        ship.descriptor
            .as_deref_mut()
            .unwrap_or_else(|| panic!("installed descriptor"))
    })
}

fn install_descriptor(ship: &mut battle::PoolShip, value: RaceDescriptor) {
    release_descriptor(ship);
    ship.record().descriptor = Some(Box::new(value));
}

fn release_descriptor(ship: &mut battle::PoolShip) {
    let owned = ship.record().descriptor.take();
    if let Some(owned) = owned {
        // Fixture resources are local records; retire their identities before dropping the box.
        observation::record(Event::ResourcesFree {
            descriptor: std::ptr::from_ref(&*owned) as usize,
            icons: true,
            battle: true,
        });
    }
}

/// The pool address the parity observer identifies this ship by.
fn ship_address(ship: &mut battle::PoolShip) -> usize {
    std::ptr::from_ref(&*ship.record()) as usize
}

type Fields = BTreeMap<String, String>;

#[test]
fn live_state_is_complete_stable_and_read_only() {
    battle_context::run(|_| {
        if let Some(directory) = std::env::var_os("UQM_STATE_TEST_CHILD") {
            exercise_live_callers(Path::new(&directory));
            return;
        }
        fs::create_dir_all(concat!(env!("CARGO_MANIFEST_DIR"), "/../tmp/verify"))
            .unwrap_or_else(|error| panic!("create verification directory: {error}"));
        let directory = tempfile::Builder::new()
            .prefix("parity-state-")
            .tempdir_in(concat!(env!("CARGO_MANIFEST_DIR"), "/../tmp/verify"))
            .unwrap_or_else(|error| panic!("create state test directory: {error}"));
        for name in ["first", "relocated"] {
            let output = directory.path().join(name);
            fs::create_dir(&output)
                .unwrap_or_else(|error| panic!("create child output directory: {error}"));
            let result = Command::new(
                std::env::current_exe()
                    .unwrap_or_else(|error| panic!("locate state test binary: {error}")),
            )
            .args([
                "--exact",
                "live_state_is_complete_stable_and_read_only",
                "--nocapture",
            ])
            .env("UQM_STATE_TEST_CHILD", &output)
            .output()
            .unwrap_or_else(|error| panic!("run state test child: {error}"));
            assert!(
                result.status.success(),
                "{}\n{}",
                String::from_utf8_lossy(&result.stdout),
                String::from_utf8_lossy(&result.stderr)
            );
        }
        for name in ["parity.fields", "parity.trace"] {
            assert_eq!(
                fs::read(directory.path().join("first").join(name))
                    .unwrap_or_else(|error| panic!("read first {name}: {error}")),
                fs::read(directory.path().join("relocated").join(name))
                    .unwrap_or_else(|error| panic!("read relocated {name}: {error}")),
                "{name} changed with process addresses"
            );
        }
    })
}

fn exercise_live_callers(directory: &Path) {
    clear_queues();
    fighters_without_journal(directory);
    uqm_platform::install(true);
    std::env::set_var("UQM_PARITY", "1");
    let document = parse_script(
        br#"{"version":2,"name":"state-test","seed":42,"fixture":"state-test","budgets":{"max_input_ticks":2,"max_presentations":1,"max_wallclock_seconds":30},"steps":[{"action":"finish"}]}"#,
        "state-test.json",
    )
    .unwrap_or_else(|error| panic!("parse state test script: {error}"));
    let _application = {
        let game = uqm_engine::Game::new();
        let input = uqm_engine::input::InputControls::default();
        let observer = Some(AutomationObserver::new(
            validate_script(document, "state-test.json")
                .unwrap_or_else(|error| panic!("validate state test script: {error}")),
            directory.into(),
            game.world.activity_control(),
            input.clone(),
            game.kernel.configuration.clone(),
        ));
        let mut application = uqm_rust::app::Application::new(game, observer);
        application.install_input(input);
        application
    };
    let observer = _application
        .observer()
        .unwrap_or_else(|| panic!("state test application has no observer"))
        .as_ref();
    crate::battle_context::with(|cx| uqm_engine::state::ffi::rust_init_game_state(cx.world));
    observation::record(Event::SpaceActive(true));
    descriptor_fields(observer, directory);
    elements_and_fighters(observer, directory);
    descriptor_assets(observer, directory);
    race_custom_fields(observer, directory);
    campaign_fields(observer, directory);
    event_queue_fields(observer, directory);
    non_ship_parent_fields(observer, directory);
    reused_owner_and_group_identities(observer, directory);
    frameset_and_media_identity(observer, directory);
    retired_space_does_not_borrow_released_frames(observer, directory);
}

fn snapshot(observer: &AutomationObserver, directory: &Path) -> (Fields, String) {
    uqm_engine::math::seed_random(123456);
    crate::battle_context::with(|cx| observer.parity_checkpoint(cx, "integration"));
    assert_eq!(
        uqm_engine::math::seed_random(123456),
        123456,
        "checkpoint changed RNG"
    );
    observer
        .write_replay_summary(&directory.join("summary.json"))
        .unwrap_or_else(|error| panic!("write state test summary: {error}"));
    let text = fs::read_to_string(directory.join("parity.fields"))
        .unwrap_or_else(|error| panic!("read checkpoint fields: {error}"));
    let (_, fields) = text
        .lines()
        .last()
        .unwrap_or_else(|| panic!("checkpoint fields file is empty"))
        .split_once(' ')
        .unwrap_or_else(|| panic!("checkpoint fields line has no boundary prefix"));
    let fields = serde_json::from_str(fields)
        .unwrap_or_else(|error| panic!("decode checkpoint fields: {error}"));
    let trace = fs::read_to_string(directory.join("parity.trace"))
        .unwrap_or_else(|error| panic!("read checkpoint trace: {error}"));
    (
        fields,
        trace
            .lines()
            .last()
            .unwrap_or_else(|| panic!("checkpoint trace file is empty"))
            .to_owned(),
    )
}

fn changed(before: &Fields, after: &Fields) -> Vec<String> {
    after
        .iter()
        .filter(|(name, value)| before.get(*name) != Some(*value))
        .map(|(name, _)| name.clone())
        .collect()
}

fn descriptor_fields(observer: &AutomationObserver, directory: &Path) {
    let descriptor: RaceDescriptor = RaceDescriptor::EMPTY;
    let mut ship = battle::PoolShip::new(0);
    ship.record().species_id = SPECIES_ID_EARTHLING_ID;
    install_descriptor(&mut ship, descriptor);
    install_ship(&mut ship);
    let (before, _) = snapshot(observer, directory);
    update_descriptor(&mut ship);
    let (after, _) = snapshot(observer, directory);
    for (name, value) in [
        ("race.max_crew", 37),
        ("race.cost", 19),
        ("race.crew", 23),
        ("race.energy", 17),
        ("race.max_energy", 29),
        ("race.flags", 123),
        ("fleet.strength", 31),
        ("fleet.x", -7),
        ("fleet.y", 9),
        ("cyborg.maneuverability", 13),
        ("cyborg.range", 53),
        ("characteristics.mass", 11),
        ("captain", 7),
        ("crew", 11),
        ("max_crew", 41),
        ("cost", 5),
        ("index", 3),
        ("weapon_counter", 2),
        ("special_counter", 4),
        ("energy_counter", 6),
        ("input", 8),
        ("status", 10),
        ("old_status", 12),
        ("facing", 14),
        ("control", 16),
    ] {
        assert_eq!(
            after[&format!("ships.0.0.{name}")],
            value.to_string(),
            "{name}"
        );
    }
    assert_eq!(changed(&before, &after).len(), 35);
    assert_characteristics(&after, "ships.0.0.characteristics");
    {
        release_descriptor(&mut ship);
        clear_queues();
    }
}

fn update_descriptor(ship: &mut battle::PoolShip) {
    let mut descriptor = descriptor_mut(ship);
    descriptor.ship_info.max_crew = 37;
    descriptor.ship_info.ship_cost = 19;
    descriptor.ship_info.crew_level = 23;
    descriptor.ship_info.energy_level = 17;
    descriptor.ship_info.max_energy = 29;
    descriptor.ship_info.ship_flags = 123;
    descriptor.fleet.strength = 31;
    descriptor.fleet.known_loc = Point { x: -7, y: 9 };
    descriptor.cyborg_control.maneuverability_index = 13;
    descriptor.cyborg_control.weapon_range = 53;
    descriptor.characteristics = sample_characteristics();
    drop(descriptor);
    let mut ship = ship.record();
    ship.captains_name_index = 7;
    ship.crew_level = 11;
    ship.max_crew = 41;
    ship.ship_cost = 5;
    ship.index = 3;
    ship.weapon_counter = 2;
    ship.special_counter = 4;
    ship.energy_counter = 6;
    ship.ship_input_state = 8;
    ship.cur_status_flags = 10;
    ship.old_status_flags = 12;
    ship.ship_facing = 14;
    ship.control = 16;
}

fn sample_characteristics() -> CharacteristicRecord {
    CharacteristicRecord {
        max_thrust: 1,
        thrust_increment: 2,
        energy_regeneration: 3,
        weapon_energy_cost: 4,
        special_energy_cost: 5,
        energy_wait: 6,
        turn_wait: 7,
        thrust_wait: 8,
        weapon_wait: 9,
        special_wait: 10,
        ship_mass: 11,
    }
}

fn assert_characteristics(fields: &Fields, prefix: &str) {
    for (index, name) in [
        "max_thrust",
        "thrust_increment",
        "energy_regeneration",
        "weapon_energy_cost",
        "special_energy_cost",
        "energy_wait",
        "turn_wait",
        "thrust_wait",
        "weapon_wait",
        "special_wait",
        "mass",
    ]
    .iter()
    .enumerate()
    {
        assert_eq!(
            fields[&format!("{prefix}.{name}")],
            (index + 1).to_string(),
            "{name}"
        );
    }
}

fn install_ship(ship: &mut battle::PoolShip) {
    let id = ship.ship_id;
    crate::battle_context::with(|cx| {
        let mut ships = cx.kernel_mut().ships(0);
        let link = ships.link(id);
        ships.put(link);
    });
    let address = ship_address(ship);
    let record = ship.record();
    observation::record(Event::Spawn {
        address,
        side: record.player_nr,
        species: record.species_id,
    });
}

fn race_custom_fields(observer: &AutomationObserver, directory: &Path) {
    // match the races' repr(C) saved-data records and outlive the snapshots.
    {
        let human = Some(uqm_engine::battle::ElementHook::Native(
            uqm_content::ships::races::human_native::engine::nuke_preprocess,
        ));
        let descriptor: RaceDescriptor = RaceDescriptor::EMPTY;
        {
            let pool = battle::Battle::new(8);
            let mut phoenix_object = battle::Object::new(&pool);
            let phoenix = std::ptr::from_ref(&*phoenix_object.element_mut()) as usize;
            observation::record(Event::Allocate(phoenix));
            let mut owner = battle::Ship::bare(&pool, SPECIES_ID_PKUNK_ID, 0);
            owner.descriptor_mut().race_state = uqm_engine::battle::RaceState::Pkunk {
                phoenix: Some(phoenix_object.id),
                saved_pre: human,
                saved_post: human,
                saved_death: human,
            };
            let ship = std::ptr::from_ref(&*owner.ship_mut()) as usize;
            crate::battle_context::with(|cx| {
                let mut ships = cx.kernel_mut().ships(0);
                let link = ships.link(owner.ship_id);
                ships.put(link);
            });
            observation::record(Event::Spawn {
                address: ship,
                side: 0,
                species: SPECIES_ID_PKUNK_ID,
            });
            battle::put(phoenix_object.id);
            let (fields, _) = snapshot(observer, directory);
            assert_eq!(
                fields["ships.0.0.custom.phoenix"],
                fields["elements.0.identity"]
            );
            assert_eq!(fields["race.pkunk.last_sound"], "0");
            crate::battle_context::with(|cx| {
                uqm_modes::battle::sounds::engine::unlink(cx, phoenix_object.id);
                cx.dispose(phoenix_object.id);
            });
            let (retired, _) = snapshot(observer, directory);
            assert_eq!(
                retired["ships.0.0.custom.phoenix"],
                fields["ships.0.0.custom.phoenix"]
            );
            let reused = battle::Object::new(&pool);
            assert_eq!(reused.id.slot(), phoenix_object.id.slot());
            assert_ne!(reused.id.generation(), phoenix_object.id.generation());
            assert!(!crate::battle_context::with(|cx| cx
                .elements()
                .contains(phoenix_object.id)));
            battle::put(reused.id);
            let (reallocated, _) = snapshot(observer, directory);
            assert_eq!(
                reallocated["ships.0.0.custom.phoenix"],
                reallocated["elements.0.identity"]
            );
            assert_ne!(
                reallocated["ships.0.0.custom.phoenix"],
                fields["ships.0.0.custom.phoenix"]
            );
            crate::battle_context::with(|cx| {
                uqm_modes::battle::sounds::engine::unlink(cx, reused.id)
            });
            crate::battle_context::with(|cx| {
                let mut ships = cx.kernel_mut().ships(0);
                let link = ships.link(owner.ship_id);
                ships.remove(link);
            });
            for name in ["saved_preprocess", "saved_postprocess", "saved_death"] {
                assert!(fields[&format!("ships.0.0.custom.{name}")]
                    .starts_with("ships::races::human_native::"));
            }
            // The fixture drops its descriptor directly rather than through the loader.
            // Retire its observation before a later descriptor can reuse this allocation.
            observation::record(Event::ResourcesFree {
                descriptor: std::ptr::from_ref(&*owner.descriptor_mut()) as usize,
                icons: true,
                battle: true,
            });
        }
        clear_queues();
        let mut ship = battle::PoolShip::new(0);
        install_descriptor(&mut ship, descriptor);
        ship.record().species_id = SPECIES_ID_UMGAH_ID;
        install_ship(&mut ship);
        ship.record().species_id = SPECIES_ID_UMGAH_ID;
        descriptor_mut(&mut ship).race_state = uqm_engine::battle::RaceState::Umgah {
            previous_facing: Some(27),
        };
        let (fields, _) = snapshot(observer, directory);
        assert_eq!(fields["ships.0.0.custom.prev_facing"], "27");
        ship.record().species_id = SPECIES_ID_ANDROSYNTH_ID;
        descriptor_mut(&mut ship).race_state = uqm_engine::battle::RaceState::Androsynth {
            collision: Some(uqm_engine::battle::CollisionHook::Native(known_collision())),
        };
        let (fields, _) = snapshot(observer, directory);
        assert!(fields["ships.0.0.custom.collision"].ends_with("::collision"));
        ship.record().species_id = SPECIES_ID_MMRNMHRM_ID;
        let mut mmrnmhrm = Box::new(
            crate::battle_context::with(|cx| {
                uqm_content::ships::races::descriptor(cx.world, SPECIES_ID_MMRNMHRM_ID)
            })
            .into_legacy(),
        );
        mmrnmhrm.race_state = uqm_engine::battle::RaceState::Mmrnmhrm {
            alternate: sample_characteristics(),
        };
        release_descriptor(&mut ship);
        ship.record().descriptor = Some(mmrnmhrm);
        let (fields, _) = snapshot(observer, directory);
        assert_characteristics(&fields, "ships.0.0.custom.characteristics");
        sis_fields(observer, directory, &mut ship);
        release_descriptor(&mut ship);
        clear_queues();
    }
}

fn known_collision() -> uqm_engine::battle::CollisionFn {
    uqm_engine::battle::motion::collision
}

fn sis_fields(observer: &AutomationObserver, directory: &Path, ship: &mut battle::PoolShip) {
    crate::battle_context::with(|cx| {
        let sis = &mut cx.world.data.flagship;
        sis.module_slots
            .fill(uqm_modes::starbase::sis_modules::EMPTY_SLOT);
        sis.module_slots[15] = GUN_WEAPON as u8;
        sis.module_slots[14] = CANNON_WEAPON as u8;
        sis.module_slots[4] = TRACKING_SYSTEM as u8;
        sis.drive_slots
            .fill(uqm_modes::starbase::sis_modules::EMPTY_SLOT);
        sis.jet_slots
            .fill(uqm_modes::starbase::sis_modules::EMPTY_SLOT);
        cx.world.set_activity(SUPER_MELEE);
    });
    ship.record().species_id = SPECIES_ID_SIS_SHIP_ID;
    let descriptor = {
        crate::battle_context::with(|cx| {
            uqm_content::ships::races::descriptor(cx.world, SPECIES_ID_SIS_SHIP_ID)
        })
        .into_legacy()
    };
    install_descriptor(ship, descriptor);
    let (fields, _) = snapshot(observer, directory);
    assert_eq!(fields["ships.0.0.custom.num_trackers"], "1");
    assert_eq!(fields["ships.0.0.custom.num_blasters"], "3");
    for index in 0..6 {
        for name in [
            "cx",
            "cy",
            "flags",
            "sender",
            "pixoffs",
            "speed",
            "hit_points",
            "damage",
            "face",
            "index",
            "life",
            "blast_offs",
            "preprocess",
            "farray.owner",
            "farray.index",
        ] {
            assert!(
                fields.contains_key(&format!("ships.0.0.custom.missiles.{index}.{name}")),
                "missile {index} {name}"
            );
        }
    }
    assert_eq!(fields["ships.0.0.custom.missiles.0.hit_points"], "2");
    assert_eq!(fields["ships.0.0.custom.missiles.1.hit_points"], "6");
    assert_eq!(fields["ships.0.0.custom.missiles.0.blast_offs"], "8");
    assert!(fields["ships.0.0.custom.missiles.0.preprocess"].ends_with("::blaster_preprocess"));
    let (race, mut stats) = {
        let descriptor = descriptor_mut(ship);
        let race = descriptor
            .race
            .unwrap_or_else(|| panic!("Native SIS descriptor"));
        let stats = uqm_engine::battle::views::RaceStats {
            ship_info: uqm_engine::battle::views::Info {
                ship_flags: descriptor.ship_info.ship_flags,
                ship_cost: descriptor.ship_info.ship_cost,
                crew_level: descriptor.ship_info.crew_level,
                max_crew: descriptor.ship_info.max_crew,
                energy_level: descriptor.ship_info.energy_level,
                max_energy: descriptor.ship_info.max_energy,
            },
            characteristics: descriptor.characteristics,
            intelligence: uqm_engine::battle::views::Intelligence::default(),
            state: descriptor.race_state,
        };
        (race, stats)
    };
    crate::battle_context::with(|cx| {
        race.uninit.unwrap_or_else(|| panic!("SIS teardown"))(cx.world, &mut stats);
    });
    release_descriptor(ship);
}

fn campaign_fields(observer: &AutomationObserver, directory: &Path) {
    let (before, _) = snapshot(observer, directory);
    {
        crate::battle_context::with(|cx| cx.world.data.game.game_clock.day_index = 7);
        crate::battle_context::with(|cx| cx.world.data.game.game_clock.month_index = 9);
        crate::battle_context::with(|cx| cx.world.data.game.game_clock.year_index = 2161);
        crate::battle_context::with(|cx| cx.world.data.game.game_clock.tick_count = -17);
        crate::battle_context::with(|cx| cx.world.data.game.battle_group_ref = 123);
        crate::battle_context::with(|cx| {
            cx.world
                .set_story_state(uqm_engine::StoryKey::BOMB_CARRIER, 1)
        });
    }
    let (after, _) = snapshot(observer, directory);
    assert_eq!(after["mode.clock.day"], "7");
    assert_eq!(after["mode.clock.month"], "9");
    assert_eq!(after["mode.clock.year"], "2161");
    assert_eq!(after["mode.clock.tick"], "-17");
    assert_eq!(after["mode.battle_group_ref"], "123");
    assert!(changed(&before, &after)
        .iter()
        .any(|name| name.starts_with("mode.story.")));
    assert_eq!(after["mode.events.initialized"], "false");
}

fn descriptor_assets(observer: &AutomationObserver, directory: &Path) {
    let mut descriptor: RaceDescriptor = RaceDescriptor::EMPTY;
    let mut ship = battle::PoolShip::new(0);
    let native = battle::NativeFrameSet::new(16);
    let strings = Some(uqm_engine::strings::owned_table::EntryRef::new(0x5000, 3));
    let mut music = ptr::null_mut();
    let frame_pointer = native.frame(9);
    descriptor.ship_info.race_strings_rsc = Some(c"test.strings");
    descriptor.ship_info.icons_rsc = Some(c"test.icons");
    descriptor.ship_info.melee_icon_rsc = Some(c"test.melee");
    descriptor.ship_info.race_strings = strings;
    descriptor.ship_info.icons = frame_pointer;
    descriptor.ship_info.melee_icon = frame_pointer;
    descriptor.ship_data.ship_rsc = [Some(c"test.ship"); 3];
    descriptor.ship_data.weapon_rsc = [Some(c"test.weapon"); 3];
    descriptor.ship_data.special_rsc = [Some(c"test.special"); 3];
    let view = FrameSelection::retain(frame_pointer);
    descriptor.ship_data.ship = FrameArray::retain([view.view(); 3]);
    descriptor.ship_data.weapon = FrameArray::retain([view.view(); 3]);
    descriptor.ship_data.special = FrameArray::retain([view.view(); 3]);
    descriptor.ship_data.captain_control = CaptainRecord {
        captain_rsc: Some(c"test.captain"),
        background: frame_pointer,
        turn: frame_pointer,
        thrust: frame_pointer,
        weapon: frame_pointer,
        special: view.clone(),
    };
    descriptor.ship_data.victory_ditty_rsc = Some(c"test.victory");
    descriptor.ship_data.ship_sounds_rsc = Some(c"test.sounds");
    descriptor.ship_data.ship_sounds = strings;
    descriptor.ship_data.victory_ditty = &mut music;
    descriptor.race = uqm_content::ships::races::race(SPECIES_ID_EARTHLING_ID);
    ship.record().species_id = SPECIES_ID_EARTHLING_ID;
    install_descriptor(&mut ship, descriptor);
    ship.set_picker_resources(uqm_engine::battle::link_queue::ShipPickerResources::new(
        uqm_engine::strings::owned_table::LoadedTextTable::fixture(vec![Vec::new(); 4]).entry(3),
        view.clone(),
    ));
    {
        install_ship(&mut ship);
    }
    let drawable = frame_pointer
        .drawable()
        .map_or(0, |drawable| drawable.bits());
    observation::record(Event::Drawable(drawable));
    observation::record(Event::Frame {
        address: frame_pointer.bits(),
        owner: drawable,
        index: frame_pointer.index(),
    });
    let (fields, _) = snapshot(observer, directory);
    assert_eq!(fields["ships.0.0.race.icons.resource"], "test.icons");
    assert_eq!(fields["ships.0.0.race.strings.resource"], "test.strings");
    assert_eq!(fields["ships.0.0.race.melee_icon.resource"], "test.melee");
    assert_eq!(fields["ships.0.0.race.code.loaded"], "true");
    assert_eq!(fields["ships.0.0.strings.index"], "3");
    assert_eq!(fields["ships.0.0.data.sounds.index"], "3");
    assert_eq!(fields["ships.0.0.data.victory.loaded"], "true");
    asset_expectations(&fields);
    {
        release_descriptor(&mut ship);
        clear_queues();
    }
}

fn asset_expectations(fields: &Fields) {
    for kind in ["ship", "weapon", "special"] {
        for view in 0..3 {
            let prefix = format!("ships.0.0.data.{kind}.{view}");
            assert_eq!(
                fields[&format!("{prefix}.resource")],
                format!("test.{kind}")
            );
            assert_eq!(
                fields[&format!("{prefix}.owner")],
                fields["ships.0.0.icons.owner"]
            );
            assert_eq!(fields[&format!("{prefix}.index")], "9");
        }
    }
    for part in ["background", "turn", "thrust", "weapon", "special"] {
        assert_eq!(
            fields[&format!("ships.0.0.data.captain.{part}.owner")],
            fields["ships.0.0.icons.owner"]
        );
        assert_eq!(fields[&format!("ships.0.0.data.captain.{part}.index")], "9");
    }
    for name in ["icons", "race.icons", "race.melee_icon"] {
        assert_eq!(
            fields[&format!("ships.0.0.{name}.owner")],
            fields["ships.0.0.icons.owner"]
        );
    }
    for key in ["captain", "victory", "sounds"] {
        assert_eq!(
            fields[&format!("ships.0.0.data.{key}.resource")],
            format!("test.{key}")
        );
    }
}

fn reused_owner_and_group_identities(observer: &AutomationObserver, directory: &Path) {
    let mut ship = battle::PoolShip::new(0);
    ship.record().species_id = SPECIES_ID_EARTHLING_ID;
    {
        install_ship(&mut ship);
    }
    let _ = snapshot(observer, directory);
    let callback = known_collision() as usize;
    observation::record(Event::Hook {
        parent: ship_address(&mut ship),
        callback,
    });
    ship.record().species_id = SPECIES_ID_UMGAH_ID;
    observation::record(Event::Spawn {
        address: ship_address(&mut ship),
        side: 0,
        species: SPECIES_ID_UMGAH_ID,
    });
    observation::record(Event::Hook {
        parent: ship_address(&mut ship),
        callback,
    });
    let (_, line) = snapshot(observer, directory);
    let old = line
        .find("[hook 3 battle::ship_native::collision]")
        .unwrap_or_else(|| panic!("old owner hook is absent from checkpoint"));
    let new = line
        .find(&format!(
            "[hook {} battle::ship_native::collision]",
            SPECIES_ID_UMGAH_ID
        ))
        .unwrap_or_else(|| panic!("new owner hook is absent from checkpoint"));
    assert!(old < new);
    {
        clear_queues();
    }
}

fn observe_test_frames(views: &FrameArray, role: &str) {
    observation::record(Event::FrameSet {
        address: views.as_ptr() as usize,
        frames: views
            .views()
            .iter()
            .map(|frame| (frame.raw()).bits())
            .collect(),
        role: role.into(),
    });
}

fn frameset_and_media_identity(observer: &AutomationObserver, directory: &Path) {
    let mut descriptor: RaceDescriptor = RaceDescriptor::EMPTY;
    let mut ship = battle::PoolShip::new(0);
    let native = battle::NativeFrameSet::new(3);
    let (first, second) = (native.frame(1), native.frame(2));
    let drawable = first.drawable().map_or(0, |drawable| drawable.bits());
    let view = FrameSelection::retain(first);
    descriptor.ship_data.ship = FrameArray::retain([view.view(); 3]);
    let mut element = graph::ElementRecord::new();
    ship.record().species_id = SPECIES_ID_EARTHLING_ID;
    install_descriptor(&mut ship, descriptor);
    element.record().parent = uqm_engine::battle::Parent::Ship(ship.ship_id);
    {
        attach_elements(std::slice::from_mut(&mut element));
        install_ship(&mut ship);
    }
    observation::record(Event::Allocate(element.address()));
    observation::record(Event::Drawable(drawable));
    for frame in [first, second] {
        observation::record(Event::Frame {
            address: frame.bits(),
            owner: drawable,
            index: frame.index(),
        });
    }
    let ship_frames = descriptor_mut(&mut ship).ship_data.ship.clone();
    observe_test_frames(&ship_frames, "test-ship");
    observation::assign_set(
        &mut element.record().current.image.frames,
        uqm_engine::battle::frames::Frames::shared(
            uqm_engine::battle::frames::SharedFrames::Fixture(0),
            ship_frames.clone(),
        ),
        None,
    );
    let (before, _) = snapshot(observer, directory);
    let second_view = FrameSelection::retain(second);
    ship_frames.replace_view(1, second_view.view());
    let (after, _) = snapshot(observer, directory);
    assert_eq!(before["elements.0.current.farray.0.index"], "1");
    assert_eq!(after["elements.0.current.farray.0.index"], "1");
    assert_eq!(before["elements.0.current.farray.1.index"], "1");
    assert_eq!(after["elements.0.current.farray.1.index"], "2");
    let mut replacement = RaceDescriptor::EMPTY;
    replacement.ship_data.ship = FrameArray::retain(ship_frames.views());
    let _outgoing_descriptor = ship
        .record()
        .descriptor
        .take()
        .unwrap_or_else(|| panic!("outgoing frame owner"));
    install_descriptor(&mut ship, replacement);
    let replacement_frames = descriptor_mut(&mut ship).ship_data.ship.clone();
    observe_test_frames(&replacement_frames, "replacement-ship");
    observation::assign_set(
        &mut element.record().next.image.frames,
        uqm_engine::battle::frames::Frames::shared(
            uqm_engine::battle::frames::SharedFrames::Fixture(0),
            replacement_frames,
        ),
        None,
    );
    let (outgoing, _) = snapshot(observer, directory);
    assert_eq!(
        outgoing["elements.0.current.farray.role"],
        after["elements.0.current.farray.role"]
    );
    assert_ne!(
        outgoing["elements.0.current.farray.role"],
        outgoing["elements.0.next.farray.role"]
    );
    media_identity(observer, directory, &mut ship);
    {
        release_descriptor(&mut ship);
        observation::record(Event::ResourcesFree {
            descriptor: std::ptr::from_ref(&*_outgoing_descriptor) as usize,
            icons: true,
            battle: true,
        });
        clear_queues();
    }
}

fn media_identity(observer: &AutomationObserver, directory: &Path, ship: &mut battle::PoolShip) {
    use uqm_engine::strings::owned_table::EntryRef;
    let mut music_a = ptr::null_mut();
    let mut music_b = ptr::null_mut();
    descriptor_mut(ship).ship_info.race_strings = Some(EntryRef::new(0xa000, 3));
    descriptor_mut(ship).ship_data.victory_ditty = &mut music_a;
    let (before, _) = snapshot(observer, directory);
    descriptor_mut(ship).ship_info.race_strings = Some(EntryRef::new(0xb000, 3));
    descriptor_mut(ship).ship_data.victory_ditty = &mut music_b;
    let (after, _) = snapshot(observer, directory);
    assert_eq!(
        before["ships.0.0.race.strings.index"],
        after["ships.0.0.race.strings.index"]
    );
    assert_ne!(
        before["ships.0.0.race.strings.table"],
        after["ships.0.0.race.strings.table"]
    );
    assert_ne!(
        before["ships.0.0.data.victory.resource_owner"],
        after["ships.0.0.data.victory.resource_owner"]
    );
    descriptor_mut(ship).ship_info.race_strings = None;
    descriptor_mut(ship).ship_data.victory_ditty = ptr::null_mut();
}

fn retired_space_does_not_borrow_released_frames(observer: &AutomationObserver, directory: &Path) {
    {
        clear_queues();
    }
    let mut element = graph::ElementRecord::new();
    let native = battle::NativeFrameSet::new(24);
    let frame = FrameSelection::retain(native.frame(23));
    element.record().current.image.frame = frame.clone();
    element.record().next.image.frame = frame;
    drop(native);
    element.record().current.image.frames = uqm_engine::battle::frames::Frames::ABSENT;
    // The retired journal graph must not be traversed by the observer.
    {
        attach_elements(std::slice::from_mut(&mut element));
    }
    observation::record(Event::Allocate(element.address()));
    observation::record(Event::Drawable(17));
    observation::record(Event::Frame {
        address: (element.record().current.image.frame.raw()).bits(),
        owner: 17,
        index: 23,
    });
    observation::record(Event::DrawableFree(17));
    observation::record(Event::SpaceActive(false));
    let (fields, _) = snapshot(observer, directory);
    assert_eq!(fields["mode.space_active"], "false");
    assert_eq!(fields["elements.0.current.frame.index"], "23");
    assert_eq!(fields["elements.0.next.frame.index"], "23");
    assert_eq!(fields["elements.0.current.frame.retired"], "true");
    assert_eq!(
        fields["elements.0.flags"],
        element.record().state_flags.to_string()
    );
    {
        clear_queues();
    }
}
