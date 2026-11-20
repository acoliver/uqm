//! The battle borrows its menu's protocol session without retaining menu pointers.
use super::{
    netconnection::{Effect, Event},
    packet::Packet,
    session::{Selection, SessionCore},
};
use crate::engine::battle::net::{BattleNet, ProtocolDriver, ProtocolNotice, ProtocolNoticeKind};

impl SessionCore {
    pub(crate) fn effect(&self, event: Event, driver: &mut dyn ProtocolDriver) {
        let kind = match event.effect {
            Effect::Seed(seed) => {
                crate::math::seed_random(seed);
                return;
            }
            Effect::Connected => ProtocolNoticeKind::Connected,
            Effect::Error(error) => ProtocolNoticeKind::Error(error),
            Effect::Closed => ProtocolNoticeKind::Closed,
            Effect::Abort(reason) => ProtocolNoticeKind::Abort(reason),
            Effect::ShipSelected => ProtocolNoticeKind::ShipSelected,
            Effect::Reset { reason, remote } => {
                self.with_notices(driver, |emit| self.flush_packet_queues(emit));
                ProtocolNoticeKind::Reset { reason, remote }
            }
            other => panic!("setup event requires the borrowed melee menu: {other:?}"),
        };
        driver.notice(ProtocolNotice {
            player: event.player,
            battle_active: event.battle_active,
            kind,
        });
    }

    pub(super) fn with_notices<T>(
        &self,
        driver: &mut dyn ProtocolDriver,
        action: impl FnOnce(&mut dyn FnMut(Event)) -> T,
    ) -> T {
        action(&mut |event| self.effect(event, driver))
    }
}
impl BattleNet for SessionCore {
    fn poll(&self, driver: &mut dyn ProtocolDriver) {
        self.with_notices(driver, |emit| self.net_input_with(0, emit));
    }
    fn flush(&self, driver: &mut dyn ProtocolDriver) {
        self.with_notices(driver, |emit| self.flush_packet_queues(emit));
    }
    fn all_connected(&self) -> bool {
        self.all_connected()
    }
    fn peer_count(&self) -> usize {
        self.get_num_net_connections()
    }
    fn player_order(&self, index: u16) -> u16 {
        let discriminants = std::array::from_fn::<_, 2, _>(|side| {
            self.connection(side).map(|conn| conn.flags.discriminant)
        });
        if discriminants[0] == Some(false) || discriminants[1] == Some(true) {
            index
        } else {
            1u16.wrapping_sub(index)
        }
    }
    fn input_delay(&self) -> usize {
        self.get_battle_input_delay()
    }
    fn local_input(
        &self,
        side: usize,
        input: u8,
        activity: &crate::engine::world::activity::ActivityControl,
        driver: &mut dyn ProtocolDriver,
    ) -> u8 {
        // Replay peers retain AI evaluation (and its RNG draws), but received
        // input remains authoritative for the remote player.
        if self.replay_remote_player == Some(side) {
            return self.remote_input(side, activity, driver);
        }
        self.queue_all(Packet::BattleInput(input), false);
        self.with_notices(driver, |emit| self.flush_packet_queues(emit));
        assert!(self.push_input(side, input), "local input buffer full");
        self.pop_input(side)
            .unwrap_or_else(|| panic!("local input buffer empty after push"))
    }
    fn remote_input(
        &self,
        side: usize,
        activity: &crate::engine::world::activity::ActivityControl,
        driver: &mut dyn ProtocolDriver,
    ) -> u8 {
        self.network_battle_input(side, activity, driver)
    }
    fn set_frame(&self, frame: u32) {
        self.frame_count.set(frame);
    }
    fn frame(&self) -> u32 {
        self.frame_count.get()
    }
    fn record_checksum(&self, frame: u32, checksum: u32, driver: &mut dyn ProtocolDriver) {
        self.queue_all(Packet::Checksum { frame, checksum }, false);
        self.with_notices(driver, |emit| self.flush_packet_queues(emit));
        self.add_local_checksum(frame, checksum);
    }
    fn verify_checksum(&self, frame: u32) -> bool {
        self.verify_checksums(frame)
    }
    fn reset(&self, reason: u32, driver: &mut dyn ProtocolDriver) {
        self.with_notices(driver, |emit| self.reset_connections(reason, emit));
    }
    fn negotiate(&self, notify: bool, state: u32, driver: &mut dyn ProtocolDriver) -> bool {
        self.negotiate_ready_connections(notify, state, driver)
    }
    fn set_state(&self, state: u32) {
        self.set_state_connections(state);
    }
    fn wait_reset(&self, state: u32, driver: &mut dyn ProtocolDriver) {
        self.wait_reset_connections(state, driver);
    }
    fn init_end_frames(&self) {
        self.init_battle_state_data_connections();
    }
    fn end_ready(&self, side: usize, driver: &mut dyn ProtocolDriver) -> bool {
        self.ready_for_battle_end_player(side, driver)
    }
    fn ship_selected(&self, choice: u16) {
        if self.replay_remote_player.is_none() {
            self.queue_all(Packet::SelectShip(choice), false);
        }
    }
    fn begin(&self) {
        self.init_battle_input_buffers();
        self.init_checksum_buffers();
        self.frame_count.set(0);
        self.set_battle_active_connections(true);
    }
    fn finish(&self) {
        self.uninit_battle_input_buffers();
        self.uninit_checksum_buffers();
        self.set_battle_active_connections(false);
    }
    fn begin_selection(&self, mask: u16, available: [[bool; 14]; 2]) {
        let mut selection = self.selection.borrow_mut();
        assert_eq!(selection.mask, 0, "selection already active");
        *selection = Selection {
            mask,
            available,
            choices: [None; 2],
        };
    }
    fn selected(&self, side: usize) -> Option<u16> {
        self.selection.borrow().choices[side]
    }
    fn end_selection(&self) {
        *self.selection.borrow_mut() = Selection::default();
    }
}
