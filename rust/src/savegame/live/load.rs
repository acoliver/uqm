//! Save-slot reads and installation through explicit campaign and battle owners.
use super::super::{legacy, SaveSlots};
use uqm_engine::{battle::Cx, save::summary_record::SummaryDesc};
use uqm_modes::comm::dispatch::{IN_INTERPLANETARY, START_ENCOUNTER, START_INTERPLANETARY};

pub enum LoadRequest<'a, 'game> {
    Summary(&'a mut SummaryDesc),
    Restore(&'a mut Cx<'game>),
}

fn finish_load(cx: &mut Cx<'_>, star: &super::super::StarDesc, activity: u16) {
    uqm_modes::planets::c_api::reset_encounter(cx.world);
    // A load can run inside a ship callback. Retire the roster after its leases return.
    cx.kernel.request_ship_reset();
    let point = uqm_modes::comm::dispatch::CPoint {
        x: star.star_pt.x,
        y: star.star_pt.y,
    };
    cx.world.current_star =
        uqm_modes::planets::starmap::find_star_id(cx.world, cx.world.catalog(), None, point, 0, 0);
    cx.world.next_activity = activity;
    if activity & START_ENCOUNTER == 0 && activity & 0xff == IN_INTERPLANETARY {
        cx.world.next_activity |= START_INTERPLANETARY;
    }
}

pub fn load_game(files: &uqm_engine::io::Dir, slot: u16, request: LoadRequest<'_, '_>) -> u32 {
    let loading = matches!(&request, LoadRequest::Restore(_));
    let result = (|| -> anyhow::Result<()> {
        let slots = SaveSlots::new(files);
        let cx = match request {
            LoadRequest::Summary(summary) => {
                let decoded = match slots.read_summary(usize::from(slot)) {
                    Ok(summary) => summary,
                    Err(_) => legacy::read_legacy_summary(&slots, slot)?,
                };
                *summary = SummaryDesc::from(&decoded);
                return Ok(());
            }
            LoadRequest::Restore(cx) => cx,
        };
        let (star, activity) = if slots.read_summary(usize::from(slot)).is_ok() {
            let save = slots.read(usize::from(slot))?;
            (save.star, uqm_engine::save::apply(save, cx.world)?)
        } else {
            let decoded = legacy::read_legacy_game(&slots, slot)?;
            let star = decoded.save.star;
            let activity = uqm_engine::save::apply_legacy(
                decoded.save,
                cx.world,
                decoded.state_files,
                decoded.battle_group_ref,
            )?;
            (star, activity)
        };
        finish_load(cx, &star, activity);
        Ok(())
    })();
    super::complete(result, "LoadGame", loading.then_some((slot, true)))
}

/// Enumerate the original 50-slot picker list synchronously.
pub fn load_game_descriptions(
    files: &uqm_engine::io::Dir,
    summaries: &mut [SummaryDesc; super::super::types::MAX_SAVED_GAMES],
) {
    for (i, summary) in (0..super::super::types::MAX_SAVED_GAMES as u16).zip(summaries) {
        if load_game(files, i, LoadRequest::Summary(summary)) == 0 {
            summary.year_index = 0;
        }
        log::info!(
            "RUST_PICKER: enumerated slot={i} state={}",
            if summary.year_index == 0 {
                "unavailable"
            } else {
                "occupied"
            },
        );
    }
}
