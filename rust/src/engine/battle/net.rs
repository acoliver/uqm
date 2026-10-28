//! The battle's protocol boundary, independent of the melee menu and transport.
use std::ffi::c_void;

pub trait BattleNet {
    fn poll(&self);
    fn flush(&self);
    fn all_connected(&self) -> bool;
    fn peer_count(&self) -> usize;
    fn player_order(&self, index: u16) -> u16;
    fn input_delay(&self) -> usize;
    fn local_input(&self, side: usize, input: u8) -> u8;
    fn remote_input(&self, side: usize) -> u8;
    fn set_frame(&self, frame: u32);
    fn frame(&self) -> u32;
    fn record_checksum(&self, frame: u32, checksum: u32);
    fn verify_checksum(&self, frame: u32) -> bool;
    fn reset(&self, reason: u32);
    fn negotiate(&self, notify: bool, state: u32) -> bool;
    fn set_state(&self, state: u32);
    fn wait_reset(&self, state: u32);
    fn init_end_frames(&self);
    fn end_ready(&self, side: usize) -> bool;
    fn ship_selected(&self, choice: u16);
    /// # Safety
    /// State must be the live legacy battle record until finish returns.
    unsafe fn begin(&self, state: *mut c_void);
    fn finish(&self);
    /// # Safety
    /// Picker must be the live legacy selection record, or null to release it.
    unsafe fn bind_picker(&self, mask: u16, picker: *mut c_void);
}
