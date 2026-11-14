//! Fixed save-menu summary records, shared with the live snapshot boundary.
use super::super::world::SisState;
use uqm_save::Summary;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SummaryDesc {
    pub sis: SisState,
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

impl Default for SummaryDesc {
    fn default() -> Self {
        Self {
            sis: SisState::default(),
            activity: 0,
            flags: 0,
            day_index: 0,
            month_index: 0,
            year_index: 0,
            mcredit_lo: 0,
            mcredit_hi: 0,
            num_ships: 0,
            num_devices: 0,
            ship_list: [0; 12],
            device_list: [0; 16],
            save_name: [0; 64],
        }
    }
}

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
