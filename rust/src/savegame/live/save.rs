//! Save slots and live session preparation for the owned engine snapshot.
use crate::engine::battle::Cx;
use std::ffi::CStr;

use super::super::abi::SummaryDesc;
use super::super::runtime::{self, groups, live};
use super::super::{SaveFile, SaveSlots};
use crate::engine::world::generation::SolarSnapshot;
use crate::mainloop::dir_prep::saveDir;
use crate::navigation::globals::in_hqspace;
use crate::navigation::groups::live::put_group_info;

/// # Safety
/// The game thread retains the active solar session and initialized star catalog.
pub(crate) unsafe fn capture(
    cx: &mut Cx<'_>,
    solar: Option<SolarSnapshot>,
    name: &[u8],
) -> anyhow::Result<SaveFile> {
    let star = live::capture_live_star(cx.world)?;
    let point = cx.world.data.game.ip_location;
    if in_hqspace(cx.world) != 0 {
        crate::engine::save::hyper_save::prepare_hyper_state(cx.world, cx.kernel)?;
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
        // SAFETY: the game-thread save directory remains open throughout the synchronous operation.
        let slots = unsafe { SaveSlots::new(saveDir.cast())? };
        // SAFETY: the supplied context retains campaign and native solar resources.
        let mut save = unsafe { capture(cx, solar, name.to_bytes())? };
        *summary = SummaryDesc::from(&save.summary);
        runtime::prepare_summary_for_write(&mut save.summary);
        slots.write(usize::from(slot), &save)?;
        Ok(())
    })();
    super::complete(result, "SaveGame", Some((slot, false)))
}
