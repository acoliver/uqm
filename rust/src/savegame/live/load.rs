//! The LoadGame ABI and installation of a decoded save.
use super::super::abi::SummaryDesc;
use super::super::runtime::{self, groups, queues, statefiles};
use super::super::{SaveFile, SaveSlots};
use crate::collections::queue::{Queue, ReinitQueue};
use crate::comm::dispatch::{IN_INTERPLANETARY, START_ENCOUNTER, START_INTERPLANETARY};
use crate::mainloop::dir_prep::saveDir;
use crate::planets::c_api::GLOB_DATA;

#[no_mangle]
pub static mut NextActivity: u16 = 0;

use crate::planets::starmap::FindStar;

extern "C" {
    static mut CurStarDescPtr: *mut runtime::star::CStarDesc;
}

pub(in crate::savegame) unsafe fn finish_load(star: &super::super::StarDesc, activity: u16) {
    crate::planets::c_api::reset_encounter();
    for player in 0..2 {
        ReinitQueue(
            crate::c_bindings::live_queues::player_ships(player)
                .cast_mut()
                .cast::<Queue>(),
        );
    }
    let mut point = crate::comm::dispatch::CPoint {
        x: star.star_pt.x,
        y: star.star_pt.y,
    };
    CurStarDescPtr = FindStar(std::ptr::null_mut(), &mut point, 0, 0).cast();
    NextActivity = activity;
    if activity & START_ENCOUNTER == 0 && activity & 0xff == IN_INTERPLANETARY {
        NextActivity |= START_INTERPLANETARY;
    }
}

unsafe fn install(save: SaveFile) -> anyhow::Result<()> {
    let game = &mut (*std::ptr::addr_of_mut!(GLOB_DATA)).game_state;
    let restored = runtime::restore_runtime(game, &save.global, &save.game_state, &save.events)?;
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
        &statefiles::StateFileSections {
            scan: save.scan,
            canned_nodes: save.canned_nodes,
            group_list: save.group_list,
            battle_groups: save.battle_groups,
        },
    )?;
    GLOB_DATA.sis_state = (&save.summary.sis).into();
    finish_load(&save.star, restored.next_activity);
    Ok(())
}

/// # Safety
/// Game thread; summary is null for loading, otherwise a writable SUMMARY_DESC.
#[no_mangle]
pub unsafe extern "C" fn LoadGame(slot: u16, summary: *mut SummaryDesc) -> u32 {
    let result = (|| -> anyhow::Result<()> {
        let slots = SaveSlots::new(saveDir)?;
        // C tries the old filename when opening or decoding the summary fails.
        let Ok(saved_summary) = slots.read_summary(usize::from(slot)) else {
            return if super::super::legacy::LoadLegacyGame(slot, summary) != 0 {
                Ok(())
            } else {
                anyhow::bail!("no readable save in slot {slot}")
            };
        };
        if !summary.is_null() {
            summary.write(SummaryDesc::from(&saved_summary));
            return Ok(());
        }
        install(slots.read(usize::from(slot))?)
    })();
    match result {
        Ok(()) => 1,
        Err(error) => {
            log::error!("LoadGame: {error}");
            0
        }
    }
}
