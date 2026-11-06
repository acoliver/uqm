//! The battle's protocol boundary, independent of the melee menu and transport.
pub type ProtocolWait<'a> = dyn FnMut(&mut dyn FnMut() -> bool) + 'a;

pub trait BattleNet {
    fn poll(&self);
    fn flush(&self);
    fn all_connected(&self) -> bool;
    fn peer_count(&self) -> usize;
    fn player_order(&self, index: u16) -> u16;
    fn input_delay(&self) -> usize;
    fn local_input(
        &self,
        side: usize,
        input: u8,
        activity: &crate::engine::world::activity::ActivityControl,
    ) -> u8;
    fn remote_input(
        &self,
        side: usize,
        activity: &crate::engine::world::activity::ActivityControl,
    ) -> u8;
    fn set_frame(&self, frame: u32);
    fn frame(&self) -> u32;
    fn record_checksum(&self, frame: u32, checksum: u32);
    fn verify_checksum(&self, frame: u32) -> bool;
    fn reset(&self, reason: u32);
    fn negotiate(&self, notify: bool, state: u32, wait: &mut ProtocolWait<'_>) -> bool;
    fn set_state(&self, state: u32);
    fn wait_reset(&self, state: u32, wait: &mut ProtocolWait<'_>);
    fn init_end_frames(&self);
    fn end_ready(&self, side: usize, wait: &mut ProtocolWait<'_>) -> bool;
    fn ship_selected(&self, choice: u16);
    fn begin(&self);
    fn finish(&self);
    fn begin_selection(&self, mask: u16, available: [[bool; 14]; 2]);
    fn selected(&self, side: usize) -> Option<u16>;
    fn end_selection(&self);
}
