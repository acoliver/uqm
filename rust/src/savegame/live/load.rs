//! The LoadGame ABI and installation of a decoded save.
use super::super::abi::SummaryDesc;
use super::super::runtime;
use super::super::{SaveFile, SaveSlots};
use crate::comm::dispatch::{IN_INTERPLANETARY, START_ENCOUNTER, START_INTERPLANETARY};
use crate::engine::{Kernel, World};
use crate::mainloop::dir_prep::saveDir;

use crate::planets::starmap::FindStar;

pub(in crate::savegame) unsafe fn finish_load(star: &super::super::StarDesc, activity: u16) {
    // Loading can run inside a ship callback's menu. Unlink the old roster now,
    // but keep its records alive through callbacks and terminal-outcome capture.
    (**Kernel::current()).request_ship_reset();
    let mut point = crate::comm::dispatch::CPoint {
        x: star.star_pt.x,
        y: star.star_pt.y,
    };
    let selected =
        FindStar(std::ptr::null_mut(), &mut point, 0, 0).cast::<crate::engine::world::CStarDesc>();
    World::current().write(|world| {
        crate::planets::c_api::reset_encounter(world);
        runtime::live::select_live_star(world, selected.as_ref());
        world.next_activity = activity;
        if activity & START_ENCOUNTER == 0 && activity & 0xff == IN_INTERPLANETARY {
            world.next_activity |= START_INTERPLANETARY;
        }
    });
}

unsafe fn install(save: SaveFile) -> anyhow::Result<()> {
    let star = save.star;
    let next_activity = World::current().write(|world| crate::engine::save::apply(save, world))?;
    finish_load(&star, next_activity);
    Ok(())
}

/// # Safety
/// Game thread; summary is null for loading, otherwise a writable SUMMARY_DESC.
#[no_mangle]
pub unsafe extern "C" fn LoadGame(slot: u16, summary: *mut SummaryDesc) -> u32 {
    let result = (|| -> anyhow::Result<()> {
        let slots = SaveSlots::new(saveDir.cast())?;
        // C tries the old filename when opening or decoding the summary fails.
        let Ok(saved_summary) = slots.read_summary(usize::from(slot)) else {
            let loaded = World::current().write(|world| {
                // SAFETY: this game-thread callback owns World; summary is null or caller-writable.
                unsafe { super::super::legacy::load_legacy_game(world, slot, summary) }
            })?;
            if let Some((star, activity)) = loaded {
                finish_load(&star, activity);
            }
            return Ok(());
        };
        if !summary.is_null() {
            summary.write(SummaryDesc::from(&saved_summary));
            return Ok(());
        }
        install(slots.read(usize::from(slot))?)
    })();
    super::complete(
        result,
        "LoadGame",
        summary.is_null().then_some((slot, true)),
    )
}
