//! Descriptor for the noncombat Ur-Quan drone.

#[cfg(not(test))]
pub(crate) mod engine {
    use crate::ships::native::abi::RACE_DESC;
    use std::mem;

    const fn descriptor() -> RACE_DESC {
        let mut d: RACE_DESC = unsafe { mem::zeroed() };
        d.ship_info.crew_level = 1;
        d.ship_info.max_crew = 1;
        d.ship_info.energy_level = 1;
        d.ship_info.max_energy = 1;
        d.ship_info.melee_icon_rsc = c"ship.drone.meleeicons".as_ptr();
        d
    }

    static mut PROBE_DESC: RACE_DESC = descriptor();

    #[no_mangle]
    pub extern "C" fn init_probe() -> *mut RACE_DESC {
        &raw mut PROBE_DESC
    }
}
