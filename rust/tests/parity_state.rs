//! Production checkpoint and fighter callers, including cross-process address stability.
#[path = "support/battle.rs"]
mod battle;
#[path = "support/battle_context.rs"]
mod battle_context;

use std::{collections::BTreeMap, fs, path::Path, process::Command, ptr};
use uqm_rust::{
    automation::{
        battle_fight::BattleSnapshot,
        script::{parse_script, validate_script},
        AutomationObserver,
    },
    battle::{
        element::{Element, ElementFlags},
        netplay::CrcState,
    },
    collections::observation::{self, Event},
    engine::battle::{frames::FrameArray, views::FrameSelection},
    graphics::native_abi::frame_desc,
    ships::native::abi::*,
};

#[path = "parity_state/graph.rs"]
mod graph;
use graph::{
    attach_elements, clear_queues, elements_and_fighters, event_queue_fields,
    fighters_without_journal, non_ship_parent_fields,
};

// A test handle projects the descriptor owned by its ship, without retaining a borrow across snapshots.
struct DescriptorRecord(*mut RaceDescriptor);
impl std::ops::Deref for DescriptorRecord {
    type Target = RaceDescriptor;
    fn deref(&self) -> &RaceDescriptor {
        // SAFETY: each test retains its owning ship (or detached box) until this handle's last use.
        unsafe { &*self.0 }
    }
}
impl std::ops::DerefMut for DescriptorRecord {
    fn deref_mut(&mut self) -> &mut RaceDescriptor {
        // SAFETY: test edits and synchronous snapshots do not overlap.
        unsafe { &mut *self.0 }
    }
}
fn install_descriptor(ship: &mut ShipRecord, value: RaceDescriptor) -> DescriptorRecord {
    release_descriptor(ship);
    ship.descriptor = Some(Box::new(value));
    DescriptorRecord(ship.descriptor_ptr())
}

fn release_descriptor(ship: &mut ShipRecord) {
    if let Some(owned) = ship.descriptor.take() {
        // Fixture resources are stack records; retire their identities before dropping the box.
        observation::record(Event::ResourcesFree {
            descriptor: std::ptr::from_ref(&*owned) as usize,
            icons: true,
            battle: true,
        });
    }
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
    // SAFETY: this process exclusively owns the published fixture pools.
    unsafe { clear_queues() };
    fighters_without_journal(directory);
    uqm_rust::time::source::install(true);
    std::env::set_var("UQM_PARITY", "1");
    let document = parse_script(
        br#"{"version":2,"name":"state-test","seed":42,"fixture":"state-test","budgets":{"max_input_ticks":2,"max_presentations":1,"max_wallclock_seconds":30},"steps":[{"action":"finish"}]}"#,
        "state-test.json",
    )
    .unwrap_or_else(|error| panic!("parse state test script: {error}"));
    let _application = {
        let game = uqm_rust::engine::Game::new();
        let input = uqm_rust::engine::input::InputControls::default();
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
    uqm_rust::engine::World::current().write(uqm_rust::state::ffi::rust_init_game_state);
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
    uqm_rust::math::seed_random(123456);
    observer.parity_checkpoint("integration");
    assert_eq!(
        uqm_rust::math::seed_random(123456),
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
    // SAFETY: all native records contain only scalar fields, optional callbacks and pointers.
    let descriptor: RaceDescriptor = unsafe { std::mem::zeroed() };
    // SAFETY: STARSHIP contains integers, pointers and optional callbacks; zero is valid.
    let mut ship = unsafe { battle::PoolShip::new(0) };
    ship.SpeciesID = SPECIES_ID_EARTHLING_ID;
    let mut descriptor = install_descriptor(&mut ship, descriptor);
    // SAFETY: stack records outlive the synchronous snapshots, and the queues are idle.
    unsafe {
        install_ship(&mut ship);
    }
    let (before, _) = snapshot(observer, directory);
    update_descriptor(&mut descriptor, &mut ship);
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
    // SAFETY: detach stack records before they are dropped.
    unsafe {
        release_descriptor(&mut ship);
        clear_queues();
    }
}

fn update_descriptor(descriptor: &mut RaceDescriptor, ship: &mut ShipRecord) {
    descriptor.ship_info.max_crew = 37;
    descriptor.ship_info.ship_cost = 19;
    descriptor.ship_info.crew_level = 23;
    descriptor.ship_info.energy_level = 17;
    descriptor.ship_info.max_energy = 29;
    descriptor.ship_info.ship_flags = 123;
    descriptor.fleet.strength = 31;
    descriptor.fleet.known_loc = POINT { x: -7, y: 9 };
    descriptor.cyborg_control.ManeuverabilityIndex = 13;
    descriptor.cyborg_control.WeaponRange = 53;
    descriptor.characteristics = sample_characteristics();
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

fn sample_characteristics() -> CHARACTERISTIC_STUFF {
    CHARACTERISTIC_STUFF {
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

/// # Safety
/// A synchronous snapshot must finish before the ship is moved or freed.
unsafe fn install_ship(ship: &mut ShipRecord) {
    // SAFETY: the caller owns the idle queues and keeps ship at this address
    // until the synchronous snapshot finishes and the queue is detached.
    unsafe {
        uqm_rust::app::bridge::enter(|cx| {
            let mut ships = cx.kernel_mut().ships(0);
            let link = ships
                .id(ptr::from_mut(ship) as usize)
                .unwrap_or_else(|| panic!("fixture ship is missing"));
            ships.put(link);
        });
    }
    observation::record(Event::Spawn {
        address: ptr::from_mut(ship) as usize,
        side: ship.playerNr,
        species: ship.SpeciesID,
    });
}

fn race_custom_fields(observer: &AutomationObserver, directory: &Path) {
    // SAFETY: native initializers run on this sole game thread; all test allocations
    // match the races' repr(C) saved-data records and outlive the snapshots.
    unsafe {
        let human = uqm_rust::ships::hook_registry::hooks()
            .into_iter()
            .find_map(|(address, name)| {
                (name == "ships::races::human_native::nuke_preprocess").then_some(address)
            })
            .unwrap_or_else(|| panic!("registered native Cruiser element hook"));
        // SAFETY: nuke_preprocess has the Native element signature.
        let human: uqm_rust::engine::battle::ElementFn = std::mem::transmute(human);
        let human = Some(uqm_rust::engine::battle::ElementHook::Native(human));
        let descriptor: RaceDescriptor = std::mem::zeroed();
        {
            let pool = battle::Battle::new(8);
            let mut phoenix_object = battle::Object::new(&pool);
            let phoenix = phoenix_object.ptr();
            observation::record(Event::Allocate(phoenix as usize));
            let mut owner = battle::Ship::bare(&pool, SPECIES_ID_PKUNK_ID, 0);
            owner.descriptor_mut().race_state = uqm_rust::engine::battle::RaceState::Pkunk {
                phoenix: Some(battle::id(phoenix)),
                saved_pre: human,
                saved_post: human,
                saved_death: human,
            };
            let ship = ptr::from_mut(owner.ship_mut());
            uqm_rust::app::bridge::enter(|cx| {
                let mut ships = cx.kernel_mut().ships(0);
                let link = ships.link(owner.ship_id);
                ships.put(link);
            });
            observation::record(Event::Spawn {
                address: ship as usize,
                side: 0,
                species: SPECIES_ID_PKUNK_ID,
            });
            battle::put(phoenix.cast());
            let (fields, _) = snapshot(observer, directory);
            assert_eq!(
                fields["ships.0.0.custom.phoenix"],
                fields["elements.0.identity"]
            );
            assert_eq!(fields["race.pkunk.last_sound"], "0");
            uqm_rust::app::bridge::enter(|cx| {
                cx.unlink(phoenix_object.id);
                cx.dispose(phoenix_object.id);
            });
            let (retired, _) = snapshot(observer, directory);
            assert_eq!(
                retired["ships.0.0.custom.phoenix"],
                fields["ships.0.0.custom.phoenix"]
            );
            let mut reused = battle::Object::new(&pool);
            assert_eq!(reused.id.slot(), phoenix_object.id.slot());
            assert_ne!(reused.id.generation(), phoenix_object.id.generation());
            assert!(!uqm_rust::app::bridge::enter(|cx| cx
                .elements()
                .contains(phoenix_object.id)));
            battle::put(reused.ptr().cast());
            let (reallocated, _) = snapshot(observer, directory);
            assert_eq!(
                reallocated["ships.0.0.custom.phoenix"],
                reallocated["elements.0.identity"]
            );
            assert_ne!(
                reallocated["ships.0.0.custom.phoenix"],
                fields["ships.0.0.custom.phoenix"]
            );
            uqm_rust::app::bridge::enter(|cx| cx.unlink(reused.id));
            uqm_rust::app::bridge::enter(|cx| {
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
                descriptor: ptr::from_mut(owner.descriptor_mut()) as usize,
                icons: true,
                battle: true,
            });
        }
        clear_queues();
        let mut ship = battle::PoolShip::new(0);
        let mut descriptor = install_descriptor(&mut ship, descriptor);
        ship.SpeciesID = SPECIES_ID_UMGAH_ID;
        install_ship(&mut ship);
        ship.SpeciesID = SPECIES_ID_UMGAH_ID;
        {
            let descriptor = &mut descriptor;
            descriptor.race_state = uqm_rust::engine::battle::RaceState::Umgah {
                previous_facing: Some(27),
            };
        }
        let (fields, _) = snapshot(observer, directory);
        assert_eq!(fields["ships.0.0.custom.prev_facing"], "27");
        ship.SpeciesID = SPECIES_ID_ANDROSYNTH_ID;
        ship.descriptor
            .as_deref_mut()
            .unwrap_or_else(|| panic!("installed descriptor"))
            .race_state = uqm_rust::engine::battle::RaceState::Androsynth {
            collision: Some(uqm_rust::engine::battle::CollisionHook::Native(
                known_collision(),
            )),
        };
        let (fields, _) = snapshot(observer, directory);
        assert!(fields["ships.0.0.custom.collision"].ends_with("::collision"));
        ship.SpeciesID = SPECIES_ID_MMRNMHRM_ID;
        let mut mmrnmhrm = Box::new(
            uqm_rust::app::bridge::enter(|cx| {
                uqm_rust::ships::races::descriptor(cx.world, SPECIES_ID_MMRNMHRM_ID)
            })
            .into_legacy(),
        );
        mmrnmhrm.race_state = uqm_rust::engine::battle::RaceState::Mmrnmhrm {
            alternate: sample_characteristics(),
        };
        release_descriptor(&mut ship);
        ship.descriptor = Some(mmrnmhrm);
        let (fields, _) = snapshot(observer, directory);
        assert_characteristics(&fields, "ships.0.0.custom.characteristics");
        sis_fields(observer, directory, &mut ship);
        release_descriptor(&mut ship);
        clear_queues();
    }
}

fn known_collision() -> uqm_rust::engine::battle::CollisionFn {
    let registry = uqm_rust::ships::hook_registry::hooks();
    let address = registry
        .iter()
        .find(|(_, name)| *name == "battle::ship_native::collision")
        .unwrap_or_else(|| panic!("collision callback is absent from the registry"))
        .0;
    // SAFETY: this exact registered callback is collision, whose native function
    // signature matches the returned ELEMENT/POINT collision function pointer.
    unsafe { std::mem::transmute(address) }
}

unsafe fn sis_fields(observer: &AutomationObserver, directory: &Path, ship: &mut ShipRecord) {
    uqm_rust::engine::World::current().write(|world| {
        let sis = &mut world.data.flagship;
        sis.module_slots
            .fill(uqm_rust::starbase::sis_modules::EMPTY_SLOT);
        sis.module_slots[15] = GUN_WEAPON as u8;
        sis.module_slots[14] = CANNON_WEAPON as u8;
        sis.module_slots[4] = TRACKING_SYSTEM as u8;
        sis.drive_slots
            .fill(uqm_rust::starbase::sis_modules::EMPTY_SLOT);
        sis.jet_slots
            .fill(uqm_rust::starbase::sis_modules::EMPTY_SLOT);
        world.set_activity(SUPER_MELEE as u16);
    });
    ship.SpeciesID = SPECIES_ID_SIS_SHIP_ID;
    // SAFETY: the caller runs native initialization on the sole game thread.
    let descriptor = unsafe {
        uqm_rust::app::bridge::enter(|cx| {
            uqm_rust::ships::races::descriptor(cx.world, SPECIES_ID_SIS_SHIP_ID)
        })
        .into_legacy()
    };
    let descriptor = install_descriptor(ship, descriptor);
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
    // SAFETY: the descriptor and published world outlive this synchronous teardown.
    unsafe {
        uqm_rust::app::bridge::enter(|cx| {
            let race = descriptor
                .race
                .unwrap_or_else(|| panic!("Native SIS descriptor"));
            let mut stats = uqm_rust::engine::battle::views::RaceStats {
                ship_info: uqm_rust::engine::battle::views::Info {
                    ship_flags: descriptor.ship_info.ship_flags,
                    ship_cost: descriptor.ship_info.ship_cost,
                    crew_level: descriptor.ship_info.crew_level,
                    max_crew: descriptor.ship_info.max_crew,
                    energy_level: descriptor.ship_info.energy_level,
                    max_energy: descriptor.ship_info.max_energy,
                },
                characteristics: descriptor.characteristics,
                intelligence: uqm_rust::engine::battle::views::Intelligence::default(),
                state: descriptor.race_state,
            };
            race.uninit.unwrap_or_else(|| panic!("SIS teardown"))(cx.world, &mut stats);
        });
    }
    release_descriptor(ship);
}

fn campaign_fields(observer: &AutomationObserver, directory: &Path) {
    let (before, _) = snapshot(observer, directory);
    // SAFETY: the game is idle and this is its sole thread.
    unsafe {
        (**uqm_rust::engine::WorldData::current())
            .game
            .game_clock
            .day_index = 7;
        (**uqm_rust::engine::WorldData::current())
            .game
            .game_clock
            .month_index = 9;
        (**uqm_rust::engine::WorldData::current())
            .game
            .game_clock
            .year_index = 2161;
        (**uqm_rust::engine::WorldData::current())
            .game
            .game_clock
            .tick_count = -17;
        (**uqm_rust::engine::WorldData::current())
            .game
            .battle_group_ref = 123;
        uqm_rust::engine::World::current().write(|world| {
            uqm_rust::state::ffi::rust_set_game_state(world, c"BOMB_CARRIER".as_ptr(), 1)
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
    // SAFETY: native records contain no invalid zero scalar representations.
    let mut descriptor: RaceDescriptor = unsafe { std::mem::zeroed() };
    // SAFETY: STARSHIP contains integers, pointers and optional callbacks; zero is valid.
    let mut ship = unsafe { battle::PoolShip::new(0) };
    // SAFETY: drawable_desc contains integers, pointers and optional callbacks.
    let native = battle::NativeFrameSet::new(16);
    // SAFETY: the fixture retains this registered allocation throughout every snapshot.
    let (drawable, frame) = unsafe {
        let frame = &mut *native.frame(9).cast::<frame_desc>();
        (&mut *frame.parent, frame)
    };
    let mut strings = uqm_rust::strings::string_table::StringTableEntry {
        data: ptr::null_mut(),
        length: 0,
        index: 3,
        parent: ptr::null_mut(),
    };
    let mut music = ptr::null_mut();
    frame.parent = ptr::from_mut(drawable);
    frame.Index = 9;
    let frame_pointer = ptr::from_mut(frame).cast();
    descriptor.ship_info.race_strings_rsc = c"test.strings".as_ptr();
    descriptor.ship_info.icons_rsc = c"test.icons".as_ptr();
    descriptor.ship_info.melee_icon_rsc = c"test.melee".as_ptr();
    descriptor.ship_info.race_strings = &mut strings;
    descriptor.ship_info.icons = frame_pointer;
    descriptor.ship_info.melee_icon = frame_pointer;
    descriptor.ship_data.ship_rsc = [c"test.ship".as_ptr(); 3];
    descriptor.ship_data.weapon_rsc = [c"test.weapon".as_ptr(); 3];
    descriptor.ship_data.special_rsc = [c"test.special".as_ptr(); 3];
    // SAFETY: native retains this registered drawable through every snapshot.
    let view = FrameSelection::retain(frame_pointer);
    descriptor.ship_data.ship = FrameArray::retain([view.view(); 3]);
    descriptor.ship_data.weapon = FrameArray::retain([view.view(); 3]);
    descriptor.ship_data.special = FrameArray::retain([view.view(); 3]);
    descriptor.ship_data.captain_control = CAPTAIN_STUFF {
        captain_rsc: c"test.captain".as_ptr(),
        background: frame_pointer,
        turn: frame_pointer,
        thrust: frame_pointer,
        weapon: frame_pointer,
        special: view.clone(),
    };
    descriptor.ship_data.victory_ditty_rsc = c"test.victory".as_ptr();
    descriptor.ship_data.ship_sounds_rsc = c"test.sounds".as_ptr();
    descriptor.ship_data.ship_sounds = &mut strings;
    descriptor.ship_data.victory_ditty = &mut music;
    descriptor.race = uqm_rust::ships::races::race(SPECIES_ID_EARTHLING_ID);
    ship.SpeciesID = SPECIES_ID_EARTHLING_ID;
    install_descriptor(&mut ship, descriptor);
    ship.icons = frame_pointer;
    ship.race_strings = &mut strings;
    // SAFETY: the full local resource graph stays live until the snapshot returns.
    unsafe {
        install_ship(&mut ship);
    }
    observation::record(Event::Drawable(ptr::from_mut(drawable) as usize));
    observation::record(Event::Frame {
        address: ptr::from_mut(frame) as usize,
        owner: ptr::from_mut(drawable) as usize,
        index: frame.Index,
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
    // SAFETY: detach all stack resource owners before returning.
    unsafe {
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
    // SAFETY: STARSHIP contains integers, pointers and optional callbacks; zero is valid.
    let mut ship = unsafe { battle::PoolShip::new(0) };
    ship.SpeciesID = SPECIES_ID_EARTHLING_ID;
    // SAFETY: the idle queue borrows this stack ship only until clear_queues below.
    unsafe {
        install_ship(&mut ship);
    }
    let _ = snapshot(observer, directory);
    let callback = known_collision() as usize;
    observation::record(Event::Hook {
        parent: ptr::from_mut(&mut *ship) as usize,
        callback,
    });
    ship.SpeciesID = SPECIES_ID_UMGAH_ID;
    observation::record(Event::Spawn {
        address: ptr::from_mut(&mut *ship) as usize,
        side: 0,
        species: SPECIES_ID_UMGAH_ID,
    });
    observation::record(Event::Hook {
        parent: ptr::from_mut(&mut *ship) as usize,
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
    // SAFETY: the last snapshot finished; detach the ship before it is dropped.
    unsafe {
        clear_queues();
    }
}

fn observe_test_frames(views: &FrameArray, role: &str) {
    observation::record(Event::FrameSet {
        address: views.as_ptr() as usize,
        frames: views
            .views()
            .iter()
            .map(|frame| frame.raw() as usize)
            .collect(),
        role: role.into(),
    });
}

fn frameset_and_media_identity(observer: &AutomationObserver, directory: &Path) {
    // SAFETY: RACE_DESC contains integers, pointers and optional callbacks.
    let mut descriptor: RaceDescriptor = unsafe { std::mem::zeroed() };
    // SAFETY: STARSHIP contains integers, pointers and optional callbacks; zero is valid.
    let mut ship = unsafe { battle::PoolShip::new(0) };
    let native = battle::NativeFrameSet::new(3);
    // SAFETY: the registered drawable owns these distinct frame slots throughout all snapshots.
    let (drawable, first, second) = unsafe {
        let first = &mut *native.frame(1).cast::<frame_desc>();
        let second = &mut *native.frame(2).cast::<frame_desc>();
        (&mut *first.parent, first, second)
    };
    // SAFETY: native owns the registered first frame throughout this fixture.
    let view = FrameSelection::retain(ptr::from_mut(first).cast());
    descriptor.ship_data.ship = FrameArray::retain([view.view(); 3]);
    let mut element = graph::ElementRecord::new();
    ship.SpeciesID = SPECIES_ID_EARTHLING_ID;
    let descriptor = install_descriptor(&mut ship, descriptor);
    element.parent = uqm_rust::engine::battle::Parent::Ship(ship.ship_id);
    // SAFETY: the stack graph stays live through all snapshots and is detached below.
    unsafe {
        attach_elements(std::slice::from_mut(&mut element));
        install_ship(&mut ship);
    }
    observation::record(Event::Allocate(element.pointer() as usize));
    observation::record(Event::Drawable(ptr::from_mut(drawable) as usize));
    for frame in [&mut *first, &mut *second] {
        observation::record(Event::Frame {
            address: ptr::from_mut(frame) as usize,
            owner: ptr::from_mut(drawable) as usize,
            index: frame.Index,
        });
    }
    observe_test_frames(&descriptor.ship_data.ship, "test-ship");
    observation::assign_set(
        &mut element.current.image.frames,
        uqm_rust::engine::battle::frames::Frames::shared(
            uqm_rust::engine::battle::frames::SharedFrames::Fixture(0),
            descriptor.ship_data.ship.clone(),
        ),
        None,
    );
    let (before, _) = snapshot(observer, directory);
    // SAFETY: the fixture retains the same drawable and its array through the next snapshot.
    unsafe {
        observation::assign_view(
            &mut *descriptor.ship_data.ship.as_ptr().add(1),
            ptr::from_mut(second).cast(),
        );
    }
    let (after, _) = snapshot(observer, directory);
    assert_eq!(before["elements.0.current.farray.0.index"], "1");
    assert_eq!(after["elements.0.current.farray.0.index"], "1");
    assert_eq!(before["elements.0.current.farray.1.index"], "1");
    assert_eq!(after["elements.0.current.farray.1.index"], "2");
    let mut replacement = RaceDescriptor::EMPTY;
    replacement.ship_data.ship = FrameArray::retain(descriptor.ship_data.ship.views());
    let ship = &mut ship;
    let _outgoing_descriptor = ship
        .descriptor
        .take()
        .unwrap_or_else(|| panic!("outgoing frame owner"));
    let mut replacement = install_descriptor(ship, replacement);
    let element = &mut element;
    observe_test_frames(&replacement.ship_data.ship, "replacement-ship");
    observation::assign_set(
        &mut element.next.image.frames,
        uqm_rust::engine::battle::frames::Frames::shared(
            uqm_rust::engine::battle::frames::SharedFrames::Fixture(0),
            replacement.ship_data.ship.clone(),
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
    media_identity(observer, directory, &mut replacement);
    // SAFETY: all snapshots finished; detach stack nodes before returning.
    unsafe {
        release_descriptor(ship);
        observation::record(Event::ResourcesFree {
            descriptor: std::ptr::from_ref(&*_outgoing_descriptor) as usize,
            icons: true,
            battle: true,
        });
        clear_queues();
    }
}

fn media_identity(
    observer: &AutomationObserver,
    directory: &Path,
    descriptor: &mut RaceDescriptor,
) {
    use uqm_rust::strings::string_table::{StringTable, StringTableEntry};
    // SAFETY: StringTable contains only integers and pointers; zero is valid.
    let mut table_a: StringTable = unsafe { std::mem::zeroed() };
    // SAFETY: StringTable contains only integers and pointers; zero is valid.
    let mut table_b: StringTable = unsafe { std::mem::zeroed() };
    let mut entry = StringTableEntry {
        data: ptr::null_mut(),
        length: 0,
        index: 3,
        parent: &mut table_a,
    };
    let mut music_a = ptr::null_mut();
    let mut music_b = ptr::null_mut();
    descriptor.ship_info.race_strings = &mut entry;
    descriptor.ship_data.victory_ditty = &mut music_a;
    let (before, _) = snapshot(observer, directory);
    let entry = &mut entry;
    entry.parent = &mut table_b;
    descriptor.ship_data.victory_ditty = &mut music_b;
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
    descriptor.ship_info.race_strings = ptr::null_mut();
    descriptor.ship_data.victory_ditty = ptr::null_mut();
}

fn retired_space_does_not_borrow_released_frames(observer: &AutomationObserver, directory: &Path) {
    // SAFETY: clear the prior fixture graph before allocating this owned operand.
    unsafe {
        clear_queues();
    }
    let mut element = graph::ElementRecord::new();
    let native = battle::NativeFrameSet::new(24);
    // SAFETY: the registered fixture retains all 24 frame slots.
    let frame = FrameSelection::retain(native.frame(23).cast());
    element.current.image.frame = frame.clone();
    element.next.image.frame = frame;
    drop(native);
    element.current.image.frames = uqm_rust::engine::battle::frames::Frames::ABSENT;
    // SAFETY: the element pool and its selections remain owned after space teardown.
    // The retired journal graph must not be traversed by the observer.
    unsafe {
        attach_elements(std::slice::from_mut(&mut element));
    }
    observation::record(Event::Allocate(element.pointer() as usize));
    observation::record(Event::Drawable(17));
    observation::record(Event::Frame {
        address: element.current.image.frame.raw() as usize,
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
    assert_eq!(fields["elements.0.flags"], element.state_flags.to_string());
    // SAFETY: detach the stack element before it is dropped.
    unsafe {
        clear_queues();
    }
}
