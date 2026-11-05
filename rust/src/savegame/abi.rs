//! The save menu's `SUMMARY_DESC` layout boundary.
pub use crate::engine::save::summary_record::SummaryDesc;

crate::c_bindings::assert_c_layout!(
    SummaryDesc,
    crate::c_bindings::global_state::SUMMARY_DESC {
        sis: SS,
        activity: Activity,
        flags: Flags,
        day_index: day_index,
        month_index: month_index,
        year_index: year_index,
        mcredit_lo: MCreditLo,
        mcredit_hi: MCreditHi,
        num_ships: NumShips,
        num_devices: NumDevices,
        ship_list: ShipList,
        device_list: DeviceList,
        save_name: SaveName,
    }
);
