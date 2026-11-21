//! Save slots and live session preparation for the owned engine snapshot.
use std::ffi::CStr;
use uqm_engine::battle::Cx;

use super::super::runtime::{self, groups, live};
use super::super::{SaveFile, SaveSlots};
use uqm_engine::save::summary_record::SummaryDesc;
use uqm_engine::world::generation::SolarSnapshot;
use uqm_modes::navigation::groups::put_group_info;
use uqm_save::SlotError;

/// Capture the running game for a save; the game thread retains the active
/// solar session and initialized star catalog.
pub(crate) fn capture(
    cx: &mut Cx<'_>,
    solar: Option<SolarSnapshot>,
    name: &[u8],
) -> anyhow::Result<SaveFile> {
    let star = live::capture_live_star(cx.world)?;
    let point = cx.world.data.game.ip_location;
    if cx.world.in_hqspace() {
        uqm_engine::save::hyper_save::prepare_hyper_state(cx.world, cx.kernel)?;
    } else if let Some(solar) = solar {
        live::prepare_live_solar_location(cx.world, &solar)?;
    }
    if groups::saves_ip_group_queue(cx.world.activity()) {
        put_group_info(cx.world, 0, u8::MAX);
    }
    live::capture_prepared(cx.world, cx.kernel, star, point, name, solar)
}

pub fn save_game(
    cx: &mut Cx<'_>,
    solar: Option<SolarSnapshot>,
    slot: u16,
    summary: &mut SummaryDesc,
    name: &CStr,
) -> u32 {
    let result = (|| -> anyhow::Result<()> {
        let files = uqm_engine::io::save_dir().ok_or(SlotError::NoSaveDir)?;
        let slots = SaveSlots::new(&files);
        let mut save = capture(cx, solar, name.to_bytes())?;
        *summary = SummaryDesc::from(&save.summary);
        runtime::prepare_summary_for_write(&mut save.summary);
        slots.write(usize::from(slot), &save)?;
        Ok(())
    })();
    super::complete(result, "SaveGame", Some((slot, false)))
}
