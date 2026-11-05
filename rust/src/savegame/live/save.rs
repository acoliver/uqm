//! The SaveGame ABI and live-state capture.
use crate::engine::World;
use crate::engine::WorldData;
use std::ffi::{c_char, CStr};

use super::super::abi::SummaryDesc;
use super::super::runtime::{self, groups, live};
use super::super::{SaveFile, SaveSlots};
use crate::mainloop::dir_prep::saveDir;

use crate::navigation::globals::in_hqspace;

use crate::planets::solarsys::playerInSolarSystem;

use crate::navigation::groups::live::put_group_info;

pub(crate) unsafe fn capture(name: &[u8]) -> anyhow::Result<SaveFile> {
    let star = live::capture_live_star()?;
    let point = WorldData::current().read(|world| world.game.ip_location);
    if World::current().read(in_hqspace) != 0 {
        live::prepare_live_hyper_state()?;
    } else if playerInSolarSystem() {
        live::prepare_live_solar_location()?;
    }
    World::current().write(|world| {
        if groups::saves_ip_group_queue(world.activity()) {
            put_group_info(world, 0, u8::MAX);
        }
    });
    let lease = World::current();
    let world = &mut **lease;
    live::capture_prepared(world, star, point, name)
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
        let slots = SaveSlots::new(saveDir.cast())?;
        let mut save = capture(CStr::from_ptr(name).to_bytes())?;
        summary.write(SummaryDesc::from(&save.summary));
        runtime::prepare_summary_for_write(&mut save.summary);
        slots.write(usize::from(slot), &save)?;
        Ok(())
    })();
    super::complete(result, "SaveGame", Some((slot, false)))
}
