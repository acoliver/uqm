//! The LoadGame ABI and installation of a decoded save.
use super::super::abi::SummaryDesc;
use super::super::runtime::{self, groups, queues, statefiles};
use super::super::{SaveFile, SaveSlots};
use crate::comm::dispatch::{IN_INTERPLANETARY, START_ENCOUNTER, START_INTERPLANETARY};
use crate::engine::{Kernel, World};
use crate::mainloop::dir_prep::saveDir;

#[no_mangle]
pub static mut NextActivity: u16 = 0;

use crate::planets::starmap::FindStar;

pub(in crate::savegame) unsafe fn finish_load(star: &super::super::StarDesc, activity: u16) {
    crate::planets::c_api::reset_encounter();
    // Loading can run inside a ship callback's menu. Unlink the old roster now,
    // but keep its records alive through callbacks and terminal-outcome capture.
    (**Kernel::current()).request_ship_reset();
    let mut point = crate::comm::dispatch::CPoint {
        x: star.star_pt.x,
        y: star.star_pt.y,
    };
    let selected =
        FindStar(std::ptr::null_mut(), &mut point, 0, 0).cast::<crate::engine::world::CStarDesc>();
    (**World::current()).select_star(selected.as_ref(), Some(runtime::live::live_catalog()));
    NextActivity = activity;
    if activity & START_ENCOUNTER == 0 && activity & 0xff == IN_INTERPLANETARY {
        NextActivity |= START_INTERPLANETARY;
    }
}

unsafe fn install(save: SaveFile) -> anyhow::Result<()> {
    let next_activity = World::current().write(|world| -> anyhow::Result<u16> {
        // SAFETY: CGameState's size, alignment and every field offset are asserted against GAME_STATE.
        let game = &mut *std::ptr::addr_of_mut!(world.data.game)
            .cast::<crate::comm::locdata::CGameState>();
        let restored = runtime::restore_runtime(
            game,
            &mut world.story,
            &save.global,
            &save.game_state,
            &save.events,
        )?;
        queues::restore_queues(
            game,
            &queues::QueueSections {
                race_queue: save.race_queue,
                npc_ship_queue: save.npc_ship_queue,
                ship_queue: save.ship_queue,
            },
        )?;
        groups::restore_loaded_groups(
            game,
            &groups::GroupSections {
                ip_group_queue: save.ip_group_queue,
                encounters: save.encounters,
            },
        )?;
        statefiles::restore_state_files(
            game,
            &mut world.story,
            &mut world.files,
            &statefiles::StateFileSections {
                scan: save.scan,
                canned_nodes: save.canned_nodes,
                group_list: save.group_list,
                battle_groups: save.battle_groups,
            },
        )?;
        std::ptr::addr_of_mut!(world.data.flagship)
            .cast::<crate::comm::locdata::CSisState>()
            .write((&save.summary.sis).into());
        Ok(restored.next_activity)
    })?;
    finish_load(&save.star, next_activity);
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
