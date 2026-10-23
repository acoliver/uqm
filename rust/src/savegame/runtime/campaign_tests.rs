use super::{groups, queues, statefiles, *};
use crate::savegame::{decode_save, encode_save};
use crate::state::ffi::{rust_init_game_state, with_state_owners};
use crate::state::game_state_keys::STATE_KEYS;
use serial_test::serial;

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
