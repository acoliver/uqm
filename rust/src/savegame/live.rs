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
