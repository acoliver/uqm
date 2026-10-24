use super::{groups, queues, statefiles, *};
use crate::savegame::{decode_save, encode_save};
use crate::state::ffi::{rust_init_game_state, with_state_owners};
use crate::state::game_state_keys::STATE_KEYS;
use serial_test::serial;

#[test]
fn final_fleet_has_eleven_crewed_avatars_and_the_restored_jugger() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/saves");
    let saved =
        decode_save(&std::fs::read(root.join("connected-final-fleet.sav")).unwrap()).unwrap();
    assert_eq!(saved.ship_queue.len(), 12);
    for ship in &saved.ship_queue {
        let expected = if ship.index == 8 {
            (9, 20, 20)
        } else {
            (1, 42, 42)
        };
        assert_eq!((ship.race_index, ship.crew_level, ship.max_crew), expected);
    }
    assert_eq!(
        saved
            .race_queue
            .iter()
            .find(|race| race.index == 1)
            .unwrap()
            .allied_state,
        1
    );
}

#[test]
fn utwig_handover_checkpoint_preserves_both_alliances_and_the_mission() {
    use crate::state::game_state::get_state_bits_raw;
    use crate::state::game_state_keys::bit_range;

    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/saves/utwig-alliance.sav");
    let saved = decode_save(&std::fs::read(path).unwrap()).unwrap();
    for name in ["UTWIG_HAVE_ULTRON", "UTWIG_SUPOX_MISSION"] {
        let (start, end) = bit_range(name).unwrap();
        assert_eq!(
            get_state_bits_raw(&saved.game_state, start as usize, end as usize),
            1,
            "{name}"
        );
    }
    for race in [7, 9] {
        let fleet = saved
            .race_queue
            .iter()
            .find(|fleet| fleet.index == race)
            .unwrap();
        assert_eq!(fleet.allied_state, 1, "fleet {race} must be allied");
        assert!(fleet.days_left > 0, "fleet {race} must be travelling");
        let destination = if race == 9 {
            (7208, 7000)
        } else {
            (6479, 7541)
        };
        assert_eq!((fleet.dest_loc.x, fleet.dest_loc.y), destination);
        if race == 9 {
            assert_eq!(fleet.func_index, 9);
        }
    }
}

#[test]
fn jugger_trade_crews_three_allied_utwig_ships() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/saves");
    let save =
        decode_save(&std::fs::read(root.join("connected-jugger-prep.sav")).unwrap()).unwrap();
    assert_eq!(save.ship_queue.len(), 3);
    for ship in &save.ship_queue {
        assert_eq!(
            (ship.race_index, ship.crew_level, ship.max_crew),
            (9, 20, 20)
        );
    }
    assert_eq!(
        save.race_queue
            .iter()
            .find(|race| race.index == 9)
            .unwrap()
            .allied_state,
        1
    );
}

#[test]
fn paid_combat_preparation_crews_three_cruisers_and_keeps_the_survivor() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/saves");
    let before =
        decode_save(&std::fs::read(root.join("connected-lure-prep.sav")).unwrap()).unwrap();
    let after =
        decode_save(&std::fs::read(root.join("connected-combat-prep.sav")).unwrap()).unwrap();
    assert_eq!(after.ship_queue.len(), 4);
    let survivor = &before.ship_queue[0];
    assert_eq!(
        after
            .ship_queue
            .iter()
            .find(|ship| ship.index == survivor.index),
        Some(survivor)
    );
    let purchased: Vec<_> = after
        .ship_queue
        .iter()
        .filter(|ship| ship.index != survivor.index)
        .collect();
    assert_eq!(purchased.len(), 3);
    for ship in purchased {
        assert_eq!(
            (ship.race_index, ship.crew_level, ship.max_crew),
            (2, 18, 18)
        );
    }
}

#[test]
fn paid_lure_preparation_preserves_the_surviving_cruiser() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/saves");
    let ambush = decode_save(&std::fs::read(root.join("connected-ambush.sav")).unwrap()).unwrap();
    let prepared =
        decode_save(&std::fs::read(root.join("connected-lure-prep.sav")).unwrap()).unwrap();
    assert_eq!(prepared.ship_queue, ambush.ship_queue);
}

#[test]
#[serial]
fn every_committed_campaign_save_restores_and_round_trips_its_state() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/saves");
    let mut saves: Vec<_> = std::fs::read_dir(&root)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|ext| ext == "sav"))
        .collect();
    saves.sort();
    assert!(!saves.is_empty());
    for path in saves {
        let saved = decode_save(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(
            decode_save(&encode_save(&saved).unwrap()).unwrap(),
            saved,
            "{}",
            path.display()
        );
        unsafe {
            rust_init_game_state();
        }
        let mut game = super::queue_tests::LiveGame::new();
        assert!(crate::collections::queue::InitQueue(
            std::ptr::addr_of_mut!(game.game_clock.event_q).cast(),
            crate::time::clock_bridge::NUM_EVENTS as u16,
            std::mem::size_of::<crate::time::clock_bridge::CEvent>() as u16,
        ));

        let restored =
            restore_runtime(&mut game, &saved.global, &saved.game_state, &saved.events).unwrap();
        assert_eq!(restored.next_activity, saved.global.current_activity);
        game.current_activity = restored.next_activity;
        let q = queues::QueueSections {
            race_queue: saved.race_queue.clone(),
            npc_ship_queue: saved.npc_ship_queue.clone(),
            ship_queue: saved.ship_queue.clone(),
        };
        let mut name = 0;
        queues::restore_queues_with(&mut game, &q, &mut || {
            name += 1;
            name
        })
        .unwrap();
        assert_eq!(queues::capture_queues(&game).unwrap(), q);
        let g = groups::GroupSections {
            ip_group_queue: saved.ip_group_queue.clone(),
            encounters: saved.encounters.clone(),
        };
        groups::restore_groups(&mut game, &g).unwrap();
        assert_eq!(groups::capture_groups(&game).unwrap(), g);
        statefiles::restore_state_files(
            &mut game,
            &statefiles::StateFileSections {
                scan: saved.scan.clone(),
                group_list: saved.group_list.clone(),
                battle_groups: saved.battle_groups.clone(),
            },
        )
        .unwrap();
        let actual = capture_runtime(&game).unwrap();
        assert_eq!(actual.global, saved.global);
        assert_eq!(actual.events, saved.events);
        // Defined-group offsets are rebuilt while all story bits remain equal.
        assert_eq!(&actual.game_state[..98], &saved.game_state[..98]);
        let provenance: serde_json::Value =
            serde_json::from_slice(&std::fs::read(path.with_extension("provenance.json")).unwrap())
                .unwrap();
        let state = &provenance["campaign_state"];
        // Values read independently from the committed wire records.
        let expected = match path.file_stem().unwrap().to_str().unwrap() {
            "alliance-after-luna" => (23, 2, 2155, 50, 896, 0, 1),
            "alliance-after-mercury" => (21, 2, 2155, 50, 926, 0, 1),
            "allied-starbase" => (10, 3, 2155, 46, 896, 2500, 1),
            "at-alpha-centauri" => (18, 3, 2155, 46, 401, 2500, 1),
            "burvixese-caster" => (1, 11, 2155, 32, 13382, 115, 4),
            "broken-ultron" => (7, 7, 2155, 46, 10434, 115, 4),
            "funded-sol" => (14, 3, 2155, 46, 800, 2304, 1),
            "ilwrath-approach" => (8, 1, 2156, 32, 3722, 115, 4),
            "caster-diversion" => (8, 1, 2156, 32, 3722, 115, 4),
            "south-refueled" => (4, 2, 2156, 32, 21000, 115, 4),
            "dev-shattered-knowledge" => (6, 4, 2156, 32, 12880, 115, 4),
            "dev-syreen-shuttle" => (22, 5, 2156, 31, 7036, 115, 4),
            "dev-pkunk-resupply" => (15, 4, 2156, 100, 3358, 461, 6),
            "dev-fueled-crew" => (27, 4, 2156, 150, 21000, 534, 3),
            "dev-funded-knowledge" => (11, 6, 2156, 150, 13162, 534, 3),
            "dev-portal-approach" => (28, 5, 2156, 150, 16002, 534, 3),
            "dev-funded-shuttle" => (16, 7, 2156, 145, 6810, 534, 3),
            "dev-funded-vault" => (28, 7, 2156, 145, 4915, 534, 3),
            "dev-arilou-home" => (12, 7, 2156, 150, 14251, 534, 3),
            "dev-syreen-ambush" => (10, 8, 2156, 135, 2086, 534, 3),
            "dev-rainbow-refueled" => (13, 8, 2156, 70, 21000, 534, 3),
            "dev-ambush-drives" => (29, 8, 2156, 150, 18800, 315, 2),
            "dev-driven-ambush" => (7, 10, 2156, 144, 12021, 315, 2),
            "dev-mycon-lure" => (26, 10, 2156, 70, 0, 315, 2),
            "dev-lure-refueled" => (17, 10, 2156, 116, 21000, 315, 2),
            "dev-fueled-mycon-lure" => (30, 10, 2156, 92, 16835, 315, 2),
            "dev-armed-flagship" => (22, 11, 2156, 92, 12200, 315, 2),
            "dev-sun-fleet" => (23, 11, 2156, 50, 5200, 737, 6),
            "dev-sun-device" => (17, 12, 2156, 37, 581, 1887, 4),
            "dev-ambush-crew" => (28, 8, 2156, 150, 18779, 294, 3),
            "dev-refueled-ambush" => (11, 9, 2156, 64, 15935, 534, 3),
            "dev-talking-pet" => (14, 7, 2156, 150, 14251, 1404, 0),
            "dev-crew-ready" => (4, 4, 2156, 100, 4900, 461, 2),
            "dev-refitted" => (28, 3, 2156, 32, 15400, 665, 2),
            "dev-vault-open" => (7, 6, 2156, 29, 5141, 115, 4),
            "dev-taalo-protected" => (2, 3, 2156, 32, 18187, 115, 4),
            "west-bomb-refueled" => (14, 8, 2156, 20, 21000, 1175, 0),
            "war-cleared" => (1, 1, 2157, 20, 21000, 1175, 0),
            "connected-final-fleet" => (12, 5, 2158, 50, 11000, 960491, 12),
            "connected-chmmr-bomb" => (11, 5, 2158, 50, 11000, 994850, 1),
            "connected-sun-device" => (28, 3, 2158, 32, 5770, 3729, 2),
            "connected-lure" => (25, 3, 2158, 35, 6110, 2419, 3),
            "connected-jugger-prep" => (23, 2, 2158, 50, 1300, 323, 3),
            "connected-combat-prep" => (23, 2, 2158, 50, 1300, 496, 4),
            "connected-lure-prep" => (22, 2, 2158, 150, 5300, 849, 1),
            "connected-ambush" => (3, 2, 2158, 50, 9164, 3194, 1),
            "connected-vault" => (15, 12, 2157, 12, 6283, 2348, 4),
            "connected-vault-resupply" => (9, 1, 2158, 50, 1399, 2288, 4),
            "connected-syreen" => (16, 11, 2157, 34, 3124, 2348, 4),
            "connected-shattered" => (19, 9, 2157, 36, 11000, 1798, 6),
            "connected-talking-pet" => (10, 9, 2157, 36, 2852, 1798, 2),
            "connected-umgah-fleet" => (16, 7, 2157, 50, 11000, 928, 5),
            "connected-rainbow-funded" => (16, 6, 2157, 8, 11000, 7, 2),
            "connected-arilou" => (9, 7, 2157, 32, 3350, 7, 2),
            "connected-full-drives" => (15, 5, 2157, 41, 11000, 7, 2),
            "connected-funded-fleet" => (22, 4, 2157, 41, 11000, 307, 2),
            "connected-taalo" => (25, 3, 2157, 50, 7494, 158, 1),
            "starbase-armed" => (1, 3, 2157, 50, 9900, 158, 1),
            "portal-sol" => (27, 2, 2157, 8, 16230, 1175, 0),
            "portal-staging" => (26, 1, 2157, 8, 16877, 1175, 0),
            "southeast-staging" => (4, 1, 2157, 14, 20256, 1175, 0),
            "bomb-refueled" => (26, 6, 2156, 32, 21000, 1175, 0),
            "utwig-bomb" => (5, 6, 2156, 32, 6192, 1175, 0),
            "bomb-preguard" => (3, 6, 2156, 32, 6244, 115, 4),
            "utwig-alliance" => (26, 5, 2156, 32, 6934, 115, 4),
            "repaired-ultron" => (4, 4, 2156, 32, 12880, 115, 4),
            "north-refueled" => (6, 9, 2155, 43, 21000, 115, 4),
            "north-ready" => (20, 4, 2155, 46, 20000, 115, 4),
            "north-artifacts" => (19, 8, 2155, 43, 4896, 115, 4),
            "spindle-pkunk" => (2, 4, 2155, 46, 3457, 1464, 5),
            "thraddash-diverted" => (13, 8, 2155, 46, 5494, 115, 4),
            other => panic!("missing expected state for {other}"),
        };
        assert_eq!(
            (
                saved.global.clock.day_index,
                saved.global.clock.month_index,
                saved.global.clock.year_index,
                saved.summary.sis.crew_enlisted,
                saved.summary.sis.fuel_on_board,
                saved.summary.sis.res_units,
                saved.ship_queue.len()
            ),
            expected
        );
        with_state_owners(|game, _| {
            for (name, value) in state.as_object().into_iter().flatten() {
                if let Some(name) = name.strip_prefix("story:") {
                    let (_, start, width) =
                        STATE_KEYS.iter().find(|(key, _, _)| *key == name).unwrap();
                    assert_eq!(
                        u64::from(game.get_state(*start as usize, (*start + *width - 1) as usize)),
                        value.as_u64().unwrap(),
                        "{} {name}",
                        path.display()
                    );
                }
            }
        })
        .unwrap();
        crate::collections::queue::UninitQueue(
            std::ptr::addr_of_mut!(game.game_clock.event_q).cast(),
        );
    }
}
