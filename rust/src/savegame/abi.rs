//! The save menu's `SUMMARY_DESC` boundary.
use super::types::Summary;
use crate::comm::locdata::CSisState;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SummaryDesc {
    pub sis: CSisState,
    pub activity: u8,
    pub flags: u8,
    pub day_index: u8,
    pub month_index: u8,
    pub year_index: u16,
    pub mcredit_lo: u8,
    pub mcredit_hi: u8,
    pub num_ships: u8,
    pub num_devices: u8,
    pub ship_list: [u8; 12],
    pub device_list: [u8; 16],
    pub save_name: [u8; 64],
}

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

impl From<&Summary> for SummaryDesc {
    fn from(s: &Summary) -> Self {
        let mut save_name = [0; 64];
        save_name[..s.save_name.as_bytes().len()].copy_from_slice(s.save_name.as_bytes());
        Self {
            sis: (&s.sis).into(),
            activity: s.activity,
            flags: s.flags,
            day_index: s.day_index,
            month_index: s.month_index,
            year_index: s.year_index,
            mcredit_lo: s.mcredit_lo,
            mcredit_hi: s.mcredit_hi,
            num_ships: s.num_ships,
            num_devices: s.num_devices,
            ship_list: s.ship_list,
            device_list: s.device_list,
            save_name,
        }
    }
}
