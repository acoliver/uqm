//! The SaveGame ABI and live-state capture.
use std::ffi::{c_char, CStr};

use super::super::abi::SummaryDesc;
use super::super::runtime::{self, groups, live, queues, statefiles};
use super::super::{SaveFile, SaveSlots};
use crate::mainloop::dir_prep::saveDir;
use crate::planets::c_api::GLOB_DATA;

use crate::navigation::globals::inHQSpace;

use crate::planets::solarsys::playerInSolarSystem;

use crate::navigation::groups::live::PutGroupInfo;

unsafe fn capture(name: &[u8]) -> anyhow::Result<SaveFile> {
    let star = live::capture_live_star()?;
    let point = GLOB_DATA.game_state.ip_location;
    if inHQSpace() != 0 {
        live::prepare_live_hyper_state()?;
    } else if playerInSolarSystem() {
        live::prepare_live_solar_location()?;
    }
    if groups::saves_ip_group_queue(GLOB_DATA.game_state.current_activity) {
        PutGroupInfo(0, u8::MAX);
    }
    let summary = live::capture_live_summary(name)?;
    let game = &mut (*std::ptr::addr_of_mut!(GLOB_DATA)).game_state;
    let runtime = runtime::capture_runtime(game)?;
    game.ip_location = point;
    game.in_orbit = 0;
    let queues = queues::capture_queues_for_activity(game)?;
    let groups = groups::capture_groups(game)?;
    let files = statefiles::capture_state_files(game)?;
    Ok(SaveFile {
        summary,
        global: runtime.global,
        game_state: runtime.game_state,
        race_queue: queues.race_queue,
        ip_group_queue: groups.ip_group_queue,
        npc_ship_queue: queues.npc_ship_queue,
        ship_queue: queues.ship_queue,
        events: runtime.events,
        encounters: groups.encounters,
        scan: files.scan,
        group_list: files.group_list,
        battle_groups: files.battle_groups,
        star,
    })
}

/// # Safety
/// Game thread; summary and name must point to live writable/terminated objects.
#[no_mangle]
pub unsafe extern "C" fn SaveGame(
    slot: u16,
    summary: *mut SummaryDesc,
    name: *const c_char,
) -> u32 {
    let result = (|| -> anyhow::Result<()> {
        let slots = SaveSlots::new(saveDir)?;
        let mut save = capture(CStr::from_ptr(name).to_bytes())?;
        summary.write(SummaryDesc::from(&save.summary));
        // save.c write_a16 never advanced its input pointer. Preserve the
        // bytes written by that API without changing the general Rust codec.
        let first_amount = save.summary.sis.element_amounts[0];
        save.summary.sis.element_amounts.fill(first_amount);
        slots.write(usize::from(slot), &save)?;
        Ok(())
    })();
    match result {
        Ok(()) => 1,
        Err(error) => {
            log::error!("SaveGame: {error}");
            0
        }
    }
}
