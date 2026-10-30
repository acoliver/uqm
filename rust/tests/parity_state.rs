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
        Coordinator,
    },
    battle::{
        element::{Element, ElementFlags},
        netplay::CrcState,
    },
    collections::observation::{self, Event},
    graphics::native_abi::{drawable_desc, frame_desc},
    ships::native::abi::*,
};

#[path = "parity_state/graph.rs"]
mod graph;
use graph::{
    attach_elements, clear_queues, elements_and_fighters, event_queue_fields,
    fighters_without_journal, non_ship_parent_fields,
};

type Fields = BTreeMap<String, String>;

#[test]
fn live_state_is_complete_stable_and_read_only() {
    battle_context::run(|| {
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
    fighters_without_journal(directory);
    uqm_rust::time::source::install(true);
    std::env::set_var("UQM_PARITY", "1");
    let document = parse_script(
        br#"{"version":2,"name":"state-test","seed":42,"fixture":"state-test","budgets":{"max_input_ticks":2,"max_presentations":1,"max_wallclock_seconds":30},"steps":[{"action":"finish"}]}"#,
        "state-test.json",
    )
    .unwrap_or_else(|error| panic!("parse state test script: {error}"));
    Coordinator::init(
        validate_script(document, "state-test.json")
            .unwrap_or_else(|error| panic!("validate state test script: {error}")),
        directory.into(),
    );
    // SAFETY: this process has one game thread and no running game or renderer.
    unsafe {
        uqm_rust::state::ffi::rust_init_game_state();
    }
    observation::record(Event::SpaceActive(true));
    descriptor_fields(directory);
    elements_and_fighters(directory);
    descriptor_assets(directory);
    race_custom_fields(directory);
    campaign_fields(directory);
    event_queue_fields(directory);
    non_ship_parent_fields(directory);
    reused_owner_and_group_identities(directory);
    frameset_and_media_identity(directory);
    retired_space_does_not_borrow_released_frames(directory);
}

fn snapshot(directory: &Path) -> (Fields, String) {
    uqm_rust::math::seed_random(123456);
    Coordinator::parity_checkpoint("integration");
    assert_eq!(
        uqm_rust::math::seed_random(123456),
        123456,
        "checkpoint changed RNG"
    );
    Coordinator::write_replay_summary(&directory.join("summary.json"))
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

fn descriptor_fields(directory: &Path) {
    // SAFETY: all native records contain only scalar fields, optional callbacks and pointers.
    let mut descriptor: RACE_DESC = unsafe { std::mem::zeroed() };
    // SAFETY: STARSHIP contains integers, pointers and optional callbacks; zero is valid.
    let mut ship: STARSHIP = unsafe { std::mem::zeroed() };
    ship.SpeciesID = SPECIES_ID_EARTHLING_ID;
    ship.RaceDescPtr = &mut descriptor;
    // SAFETY: stack records outlive the synchronous snapshots, and the queues are idle.
    unsafe {
        install_ship(&mut ship);
    }
    let (before, _) = snapshot(directory);
    update_descriptor(&mut descriptor, &mut ship);
    let (after, _) = snapshot(directory);
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
        clear_queues();
    }
}

fn update_descriptor(descriptor: &mut RACE_DESC, ship: &mut STARSHIP) {
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
    ship.ShipFacing = 14;
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
unsafe fn install_ship(ship: &mut STARSHIP) {
    // SAFETY: the caller owns the idle queues and keeps ship at this address
    // until the synchronous snapshot finishes and the queue is detached.
    unsafe {
        race_q[0].pq_tab = ptr::from_mut(ship).cast();
        race_q[0].object_size = std::mem::size_of_val(ship) as u16;
        race_q[0].num_objects = 1;
        race_q[0].head = ptr::from_mut(ship).cast();
        race_q[0].tail = race_q[0].head;
    }
    observation::record(Event::Spawn {
        address: ptr::from_mut(ship) as usize,
        side: ship.playerNr,
        species: ship.SpeciesID,
    });
}

fn race_custom_fields(directory: &Path) {
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
        let mut descriptor: RACE_DESC = std::mem::zeroed();
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
            race_q[0].head = ship.cast();
            race_q[0].tail = race_q[0].head;
            observation::record(Event::Spawn {
                address: ship as usize,
                side: 0,
                species: SPECIES_ID_PKUNK_ID,
            });
            disp_q.head = phoenix.cast();
            disp_q.tail = disp_q.head;
            let (fields, _) = snapshot(directory);
            assert_eq!(
                fields["ships.0.0.custom.phoenix"],
                fields["elements.0.identity"]
            );
            assert_eq!(fields["race.pkunk.last_sound"], "0");
            disp_q.head = ptr::null_mut();
            disp_q.tail = ptr::null_mut();
            uqm_rust::app::bridge::enter(|cx| cx.dispose(phoenix_object.id));
            let (retired, _) = snapshot(directory);
            assert_eq!(
                retired["ships.0.0.custom.phoenix"],
                fields["ships.0.0.custom.phoenix"]
            );
            let mut reused = battle::Object::new(&pool);
            assert_eq!(reused.id.slot, phoenix_object.id.slot);
            assert_ne!(reused.id.gen, phoenix_object.id.gen);
            assert!(!uqm_rust::app::bridge::enter(|cx| cx
                .elements()
                .contains(phoenix_object.id)));
            disp_q.head = reused.ptr().cast();
            disp_q.tail = disp_q.head;
            let (reallocated, _) = snapshot(directory);
            assert_eq!(
                reallocated["ships.0.0.custom.phoenix"],
                reallocated["elements.0.identity"]
            );
            assert_ne!(
                reallocated["ships.0.0.custom.phoenix"],
                fields["ships.0.0.custom.phoenix"]
            );
            disp_q.head = ptr::null_mut();
            disp_q.tail = ptr::null_mut();
            race_q[0].head = ptr::null_mut();
            race_q[0].tail = ptr::null_mut();
            for name in ["saved_preprocess", "saved_postprocess", "saved_death"] {
                assert!(fields[&format!("ships.0.0.custom.{name}")]
                    .starts_with("ships::races::human_native::"));
            }
        }
        let mut ship: STARSHIP = std::mem::zeroed();
        ship.RaceDescPtr = &mut descriptor;
        ship.SpeciesID = SPECIES_ID_UMGAH_ID;
        install_ship(&mut ship);
        ship.SpeciesID = SPECIES_ID_UMGAH_ID;
        {
            let descriptor = &mut descriptor;
            descriptor.data = ptr::null_mut();
            descriptor.race_state = uqm_rust::engine::battle::RaceState::Umgah {
                previous_facing: Some(27),
            };
        }
        let (fields, _) = snapshot(directory);
        assert_eq!(fields["ships.0.0.custom.prev_facing"], "27");
        ship.SpeciesID = SPECIES_ID_ANDROSYNTH_ID;
        (*ship.RaceDescPtr).data = ptr::null_mut();
        (*ship.RaceDescPtr).race_state = uqm_rust::engine::battle::RaceState::Androsynth {
            collision: Some(uqm_rust::engine::battle::CollisionHook::Legacy(
                known_collision(),
            )),
        };
        let (fields, _) = snapshot(directory);
        assert!(fields["ships.0.0.custom.collision"].ends_with("::collision"));
        ship.SpeciesID = SPECIES_ID_MMRNMHRM_ID;
        let mut mmrnmhrm = Box::new(uqm_rust::ships::races::legacy_descriptor(
            SPECIES_ID_MMRNMHRM_ID,
        ));
        mmrnmhrm.race_state = uqm_rust::engine::battle::RaceState::Mmrnmhrm {
            alternate: sample_characteristics(),
        };
        ship.RaceDescPtr = &raw mut *mmrnmhrm;
        let (fields, _) = snapshot(directory);
        assert_characteristics(&fields, "ships.0.0.custom.characteristics");
        sis_fields(directory, &mut ship);
        clear_queues();
    }
}

fn known_collision() -> unsafe extern "C" fn(*mut ELEMENT, *mut POINT, *mut ELEMENT, *mut POINT) {
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

unsafe fn sis_fields(directory: &Path, ship: &mut STARSHIP) {
    // SAFETY: this child process has no running game; no other references to SIS
    // state exist while this short exclusive borrow initializes its module slots.
    unsafe {
        let sis = &raw mut GlobData.SIS_state;
        let sis = &mut *sis;
        sis.ModuleSlots
            .fill(uqm_rust::starbase::sis_modules::EMPTY_SLOT);
        sis.ModuleSlots[15] = GUN_WEAPON as u8;
        sis.ModuleSlots[14] = CANNON_WEAPON as u8;
        sis.ModuleSlots[4] = TRACKING_SYSTEM as u8;
        sis.DriveSlots
            .fill(uqm_rust::starbase::sis_modules::EMPTY_SLOT);
        sis.JetSlots
            .fill(uqm_rust::starbase::sis_modules::EMPTY_SLOT);
        GlobData.Game_state.CurrentActivity = SUPER_MELEE as u16;
    }
    ship.SpeciesID = SPECIES_ID_SIS_SHIP_ID;
    // SAFETY: the caller runs native initialization on the sole game thread.
    let mut descriptor =
        unsafe { uqm_rust::ships::races::legacy_descriptor(SPECIES_ID_SIS_SHIP_ID) };
    ship.RaceDescPtr = &raw mut descriptor;
    let (fields, _) = snapshot(directory);
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
    ship.RaceDescPtr = ptr::null_mut();
}

fn campaign_fields(directory: &Path) {
    let (before, _) = snapshot(directory);
    // SAFETY: the game is idle and this is its sole thread.
    unsafe {
        GlobData.Game_state.GameClock.day_index = 7;
        GlobData.Game_state.GameClock.month_index = 9;
        GlobData.Game_state.GameClock.year_index = 2161;
        GlobData.Game_state.GameClock.tick_count = -17;
        GlobData.Game_state.BattleGroupRef = 123;
        uqm_rust::state::ffi::rust_set_game_state(c"BOMB_CARRIER".as_ptr(), 1);
    }
    let (after, _) = snapshot(directory);
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

fn descriptor_assets(directory: &Path) {
    // SAFETY: native records contain no invalid zero scalar representations.
    let mut descriptor: RACE_DESC = unsafe { std::mem::zeroed() };
    // SAFETY: STARSHIP contains integers, pointers and optional callbacks; zero is valid.
    let mut ship: STARSHIP = unsafe { std::mem::zeroed() };
    // SAFETY: drawable_desc contains integers, pointers and optional callbacks.
    let mut drawable: drawable_desc = unsafe { std::mem::zeroed() };
    // SAFETY: frame_desc contains integers, pointers and optional callbacks.
    let mut frame: frame_desc = unsafe { std::mem::zeroed() };
    let mut strings = uqm_rust::strings::string_table::StringTableEntry {
        data: ptr::null_mut(),
        length: 0,
        index: 3,
        parent: ptr::null_mut(),
    };
    let mut music = ptr::null_mut();
    let mut code = 0u8;
    frame.parent = &mut drawable;
    frame.Index = 9;
    let frame_pointer = ptr::from_mut(&mut frame).cast();
    descriptor.ship_info.race_strings_rsc = c"test.strings".as_ptr();
    descriptor.ship_info.icons_rsc = c"test.icons".as_ptr();
    descriptor.ship_info.melee_icon_rsc = c"test.melee".as_ptr();
    descriptor.ship_info.race_strings = &mut strings;
    descriptor.ship_info.icons = frame_pointer;
    descriptor.ship_info.melee_icon = frame_pointer;
    descriptor.ship_data.ship_rsc = [c"test.ship".as_ptr(); 3];
    descriptor.ship_data.weapon_rsc = [c"test.weapon".as_ptr(); 3];
    descriptor.ship_data.special_rsc = [c"test.special".as_ptr(); 3];
    descriptor.ship_data.ship = [frame_pointer; 3];
    descriptor.ship_data.weapon = [frame_pointer; 3];
    descriptor.ship_data.special = [frame_pointer; 3];
    descriptor.ship_data.captain_control = CAPTAIN_STUFF {
        captain_rsc: c"test.captain".as_ptr(),
        background: frame_pointer,
        turn: frame_pointer,
        thrust: frame_pointer,
        weapon: frame_pointer,
        special: frame_pointer,
    };
    descriptor.ship_data.victory_ditty_rsc = c"test.victory".as_ptr();
    descriptor.ship_data.ship_sounds_rsc = c"test.sounds".as_ptr();
    descriptor.ship_data.ship_sounds = &mut strings;
    descriptor.ship_data.victory_ditty = &mut music;
    descriptor.code_ref = ptr::from_mut(&mut code).cast();
    ship.SpeciesID = SPECIES_ID_EARTHLING_ID;
    ship.RaceDescPtr = &mut descriptor;
    ship.icons = frame_pointer;
    ship.race_strings = &mut strings;
    // SAFETY: the full local resource graph stays live until the snapshot returns.
    unsafe {
        install_ship(&mut ship);
    }
    observation::record(Event::Drawable(ptr::from_mut(&mut drawable) as usize));
    observation::record(Event::Frame {
        address: ptr::from_mut(&mut frame) as usize,
        owner: ptr::from_mut(&mut drawable) as usize,
        index: frame.Index,
    });
    let (fields, _) = snapshot(directory);
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

fn reused_owner_and_group_identities(directory: &Path) {
    // SAFETY: STARSHIP contains integers, pointers and optional callbacks; zero is valid.
    let mut ship: STARSHIP = unsafe { std::mem::zeroed() };
    ship.SpeciesID = SPECIES_ID_EARTHLING_ID;
    // SAFETY: the idle queue borrows this stack ship only until clear_queues below.
    unsafe {
        install_ship(&mut ship);
    }
    let _ = snapshot(directory);
    let callback = known_collision() as usize;
    observation::record(Event::Hook {
        parent: ptr::from_mut(&mut ship) as usize,
        callback,
    });
    ship.SpeciesID = SPECIES_ID_UMGAH_ID;
    observation::record(Event::Spawn {
        address: ptr::from_mut(&mut ship) as usize,
        side: 0,
        species: SPECIES_ID_UMGAH_ID,
    });
    observation::record(Event::Hook {
        parent: ptr::from_mut(&mut ship) as usize,
        callback,
    });
    let (_, line) = snapshot(directory);
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

fn observe_test_frames(views: &[FRAME], role: &str) {
    observation::record(Event::FrameSet {
        address: views.as_ptr() as usize,
        frames: views.iter().map(|frame| *frame as usize).collect(),
        role: role.into(),
    });
}

fn frameset_and_media_identity(directory: &Path) {
    // SAFETY: RACE_DESC contains integers, pointers and optional callbacks.
    let mut descriptor: RACE_DESC = unsafe { std::mem::zeroed() };
    // SAFETY: STARSHIP contains integers, pointers and optional callbacks; zero is valid.
    let mut ship: STARSHIP = unsafe { std::mem::zeroed() };
    // SAFETY: drawable_desc contains integers, pointers and optional callbacks.
    let mut drawable: drawable_desc = unsafe { std::mem::zeroed() };
    // SAFETY: frame_desc contains integers, pointers and optional callbacks.
    let mut first: frame_desc = unsafe { std::mem::zeroed() };
    // SAFETY: frame_desc contains integers, pointers and optional callbacks.
    let mut second: frame_desc = unsafe { std::mem::zeroed() };
    first.parent = &mut drawable;
    second.parent = &mut drawable;
    first.Index = 1;
    second.Index = 2;
    descriptor.ship_data.ship = [ptr::from_mut(&mut first).cast(); 3];
    let mut element = Element::new();
    ship.SpeciesID = SPECIES_ID_EARTHLING_ID;
    ship.RaceDescPtr = &mut descriptor;
    element.p_parent = ptr::from_mut(&mut ship).cast();
    // SAFETY: the stack graph stays live through all snapshots and is detached below.
    unsafe {
        attach_elements(std::slice::from_mut(&mut element));
        install_ship(&mut ship);
    }
    observation::record(Event::Allocate(ptr::from_mut(&mut element) as usize));
    observation::record(Event::Drawable(ptr::from_mut(&mut drawable) as usize));
    for frame in [&mut first, &mut second] {
        observation::record(Event::Frame {
            address: ptr::from_mut(frame) as usize,
            owner: ptr::from_mut(&mut drawable) as usize,
            index: frame.Index,
        });
    }
    observe_test_frames(&descriptor.ship_data.ship, "test-ship");
    observation::assign_set(
        &mut element.current.farray,
        descriptor.ship_data.ship.as_mut_ptr().cast(),
        None,
    );
    let (before, _) = snapshot(directory);
    observation::assign_view(
        &mut descriptor.ship_data.ship[1],
        ptr::from_mut(&mut second).cast(),
    );
    let (after, _) = snapshot(directory);
    assert_eq!(before["elements.0.current.farray.0.index"], "1");
    assert_eq!(after["elements.0.current.farray.0.index"], "1");
    assert_eq!(before["elements.0.current.farray.1.index"], "1");
    assert_eq!(after["elements.0.current.farray.1.index"], "2");
    let mut replacement = descriptor;
    let ship = &mut ship;
    ship.RaceDescPtr = &mut replacement;
    let element = &mut element;
    observe_test_frames(&replacement.ship_data.ship, "replacement-ship");
    observation::assign_set(
        &mut element.next.farray,
        replacement.ship_data.ship.as_mut_ptr().cast(),
        None,
    );
    let (outgoing, _) = snapshot(directory);
    assert_eq!(
        outgoing["elements.0.current.farray.role"],
        after["elements.0.current.farray.role"]
    );
    assert_ne!(
        outgoing["elements.0.current.farray.role"],
        outgoing["elements.0.next.farray.role"]
    );
    media_identity(directory, &mut replacement);
    // SAFETY: all snapshots finished; detach stack nodes before returning.
    unsafe {
        clear_queues();
    }
}

fn media_identity(directory: &Path, descriptor: &mut RACE_DESC) {
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
    let (before, _) = snapshot(directory);
    let entry = &mut entry;
    entry.parent = &mut table_b;
    descriptor.ship_data.victory_ditty = &mut music_b;
    let (after, _) = snapshot(directory);
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

fn retired_space_does_not_borrow_released_frames(directory: &Path) {
    let mut element = Element::new();
    element.current.frame = ptr::dangling_mut();
    element.next.frame = ptr::dangling_mut();
    element.current.farray = ptr::null_mut();
    // SAFETY: only the element pool is retained after space teardown; the frame
    // graph is deliberately invalid and must not be traversed by the observer.
    unsafe {
        clear_queues();
        attach_elements(std::slice::from_mut(&mut element));
        disp_q.head = ptr::from_mut(&mut element).cast();
        disp_q.tail = disp_q.head;
    }
    observation::record(Event::Allocate(ptr::from_mut(&mut element) as usize));
    observation::record(Event::Drawable(17));
    observation::record(Event::Frame {
        address: element.current.frame as usize,
        owner: 17,
        index: 23,
    });
    observation::record(Event::DrawableFree(17));
    observation::record(Event::SpaceActive(false));
    let (fields, _) = snapshot(directory);
    assert_eq!(fields["mode.space_active"], "false");
    assert_eq!(fields["elements.0.current.frame.index"], "23");
    assert_eq!(fields["elements.0.next.frame.index"], "23");
    assert_eq!(fields["elements.0.current.frame.retired"], "true");
    assert_eq!(
        fields["elements.0.flags"],
        element.state_flags.bits().to_string()
    );
    // SAFETY: detach the stack element before it is dropped.
    unsafe {
        clear_queues();
    }
}
