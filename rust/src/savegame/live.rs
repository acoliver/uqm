//! The save menu persistence entry points, called on the game thread.
pub mod load;
pub mod save;

/// Bind the live adapter after the application has prepared its save directory.
pub fn slots(directory: uqm_engine::io::Dir) -> uqm_engine::save::slots::SaveSlots {
    uqm_engine::save::slots::SaveSlots::new(
        directory,
        load::load_game_descriptions,
        |files, cx, solar, slot, summary, name| {
            save::save_game(files, cx, solar, slot, summary, name) != 0
        },
        |files, cx, slot| load::load_game(files, slot, load::LoadRequest::Restore(cx)) != 0,
    )
}

fn complete(result: anyhow::Result<()>, operation: &str, witness: Option<(u16, bool)>) -> u32 {
    match result {
        Ok(()) => {
            if let Some((slot, loading)) = witness {
                uqm_engine::collections::observation::record_save(slot, loading);
            }
            1
        }
        Err(error) => {
            log::error!("{operation}: {error}");
            0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::savegame::slot_tests::SaveDirFixture;
    use uqm_engine::{io::Vfs, save::summary_record::SummaryDesc};

    #[test]
    #[serial_test::serial]
    fn live_descriptions_use_only_their_retained_profile() {
        let first = SaveDirFixture::new();
        let second = SaveDirFixture::new();
        let mut saved = crate::savegame::decode_save(include_bytes!(
            "../../tests/fixtures/savegame/legacy-save.00"
        ))
        .unwrap();
        saved.summary.year_index = 2156;
        first.slots().write(0, &saved).unwrap();
        saved.summary.year_index = 2158;
        second.slots().write(0, &saved).unwrap();
        let first_service = slots(Vfs::new().open_host_dir(&first.host("")));
        let second_service = slots(Vfs::new().open_host_dir(&second.host("")));
        for (service, year) in [
            (&first_service, 2156),
            (&second_service, 2158),
            (&first_service, 2156),
        ] {
            let mut descriptions = std::array::from_fn(|_| SummaryDesc::default());
            service.describe(&mut descriptions);
            assert_eq!(descriptions[0].year_index, year);
            assert!(descriptions[1..]
                .iter()
                .all(|description| description.year_index == 0));
        }
        drop(first_service);
        let mut descriptions = std::array::from_fn(|_| SummaryDesc::default());
        second_service.describe(&mut descriptions);
        assert_eq!(descriptions[0].year_index, 2158);
    }
}

#[cfg(test)]
mod profile_tests {
    use super::*;
    use crate::savegame::slot_tests::SaveDirFixture;
    use std::borrow::Cow;
    use uqm_engine::{
        battle::{Battle, Cx},
        io::Vfs,
        save::summary_record::SummaryDesc,
        world::queues::FleetInfo,
        Game,
    };

    fn game(profile: &SaveDirFixture) -> Game {
        let mut game = Game::new();
        game.world
            .set_catalog(Cow::Borrowed(&uqm_modes::planets::tables::STARMAP_ARRAY));
        for _ in 0..24 {
            let mut fleets = game.world.avail_races();
            let id = fleets.alloc().unwrap();
            *fleets.get_mut(id) = FleetInfo::default();
            fleets.put(id);
        }
        let files = Vfs::new();
        files.mount_dir("/", &profile.host(""), uqm_engine::io::Placement::Top, true);
        game.kernel
            .install_save_slots(slots(files.open_host_dir(&profile.host(""))));
        game
    }

    fn restore(game: &mut Game, battle: &mut Battle, slot: u16, year: u16, crew: u16) {
        let service = game.kernel.save_slots();
        let mut cx = Cx::new(battle, &mut game.world, &mut game.kernel, || 7);
        assert!(service.load(&mut cx, slot));
        assert_eq!(cx.world.data.game.game_clock.year_index, year);
        assert_eq!(cx.world.data.flagship.crew_enlisted, crew);
        assert_eq!(cx.world.next_activity, 3);
        cx.world.set_activity(cx.world.next_activity);
    }

    #[test]
    #[serial_test::serial]
    fn production_slot_adapter_interleaves_profile_restore_capture_and_writes() {
        let profiles = [SaveDirFixture::new(), SaveDirFixture::new()];
        let mut seed = crate::savegame::decode_save(include_bytes!(
            "../../tests/fixtures/savegame/legacy-save.00"
        ))
        .unwrap();
        for (profile, year, crew) in [(&profiles[0], 2156, 17), (&profiles[1], 2158, 29)] {
            seed.summary.year_index = year;
            seed.global.clock.year_index = year;
            seed.global.current_activity = 3;
            seed.summary.sis.crew_enlisted = crew;
            profile.slots().write(0, &seed).unwrap();
        }
        let originals = profiles
            .each_ref()
            .map(|profile| std::fs::read(profile.host("uqmsave.00")).unwrap());
        let mut games = [game(&profiles[0]), game(&profiles[1])];
        let mut battles = [Battle::new(), Battle::new()];
        let mut captures: [Option<Vec<u8>>; 2] = [None, None];
        for index in [0, 1, 0] {
            let (year, crew) = [(2156, 17), (2158, 29)][index];
            restore(&mut games[index], &mut battles[index], 0, year, crew);
            let game = &mut games[index];
            game.world.data.flagship.crew_enlisted += 1;
            let service = game.kernel.save_slots();
            let mut cx = Cx::new(&mut battles[index], &mut game.world, &mut game.kernel, || 7);
            assert!(service.save(
                &mut cx,
                None,
                1,
                &mut SummaryDesc::default(),
                c"profile capture"
            ));
            let written = profiles[index].slots().read(1).unwrap();
            assert_eq!(written.summary.year_index, year);
            assert_eq!(written.summary.sis.crew_enlisted, crew + 1);
            let bytes = std::fs::read(profiles[index].host("uqmsave.01")).unwrap();
            if let Some(expected) = &captures[index] {
                assert_eq!(&bytes, expected);
            }
            captures[index] = Some(bytes);
            let other = 1 - index;
            if let Some(expected) = &captures[other] {
                assert_eq!(
                    &std::fs::read(profiles[other].host("uqmsave.01")).unwrap(),
                    expected
                );
            } else {
                assert!(!profiles[other].host("uqmsave.01").exists());
            }
        }
        for index in [1, 0, 1] {
            games[index].world.data.flagship.crew_enlisted = 0;
            let (year, crew) = [(2156, 18), (2158, 30)][index];
            restore(&mut games[index], &mut battles[index], 1, year, crew);
        }
        for (profile, original) in profiles.iter().zip(originals) {
            assert_eq!(std::fs::read(profile.host("uqmsave.00")).unwrap(), original);
        }
    }
}
