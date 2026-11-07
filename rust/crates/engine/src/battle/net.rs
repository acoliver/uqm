//! The battle's protocol boundary, independent of the melee menu and transport.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtocolNoticeKind {
    Connected,
    Error(i32),
    Closed,
    Abort(u32),
    Reset { reason: u32, remote: bool },
    ShipSelected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProtocolNotice {
    pub player: usize,
    pub battle_active: bool,
    pub kind: ProtocolNoticeKind,
}

pub type ProtocolPoll<'a> = dyn FnMut(&mut dyn ProtocolDriver) -> bool + 'a;

pub trait ProtocolDriver {
    /// Notices complete before the next packet, without a connection borrow or legacy publication.
    fn notice(&mut self, notice: ProtocolNotice);
    /// Preserve the input loop's admissions, lending each poll that frame's driver.
    fn wait(&mut self, poll: &mut ProtocolPoll<'_>);
}

pub trait BattleNet {
    fn poll(&self, driver: &mut dyn ProtocolDriver);
    fn flush(&self, driver: &mut dyn ProtocolDriver);
    fn all_connected(&self) -> bool;
    fn peer_count(&self) -> usize;
    fn player_order(&self, index: u16) -> u16;
    fn input_delay(&self) -> usize;
    fn local_input(
        &self,
        side: usize,
        input: u8,
        activity: &crate::world::activity::ActivityControl,
        driver: &mut dyn ProtocolDriver,
    ) -> u8;
    fn remote_input(
        &self,
        side: usize,
        activity: &crate::world::activity::ActivityControl,
        driver: &mut dyn ProtocolDriver,
    ) -> u8;
    fn set_frame(&self, frame: u32);
    fn frame(&self) -> u32;
    fn record_checksum(&self, frame: u32, checksum: u32, driver: &mut dyn ProtocolDriver);
    fn verify_checksum(&self, frame: u32) -> bool;
    fn reset(&self, reason: u32, driver: &mut dyn ProtocolDriver);
    fn negotiate(&self, notify: bool, state: u32, driver: &mut dyn ProtocolDriver) -> bool;
    fn set_state(&self, state: u32);
    fn wait_reset(&self, state: u32, driver: &mut dyn ProtocolDriver);
    fn init_end_frames(&self);
    fn end_ready(&self, side: usize, driver: &mut dyn ProtocolDriver) -> bool;
    fn ship_selected(&self, choice: u16);
    fn begin(&self);
    fn finish(&self);
    fn begin_selection(&self, mask: u16, available: [[bool; 14]; 2]);
    fn selected(&self, side: usize) -> Option<u16>;
    fn end_selection(&self);
}
