//! Owned battle input and dispatch. Each installed context belongs to its player.

use crate::engine::battle::net::BattleNet;
use std::rc::Rc;

pub const NUM_PLAYERS: usize = 2;

#[derive(Clone)]
pub enum InputSource {
    Human,
    Computer,
    Network(Option<Rc<dyn BattleNet>>),
}

#[derive(Clone)]
pub struct InputContext {
    player: u16,
    source: InputSource,
}

impl InputContext {
    pub fn new(player: u16, control: u8) -> Self {
        assert!(usize::from(player) < NUM_PLAYERS, "invalid battle player");
        let source = match control & 15 {
            1 => InputSource::Human,
            2 | 6 => InputSource::Computer,
            8 => InputSource::Network(None),
            _ => panic!("invalid battle control method {control}"),
        };
        Self { player, source }
    }

    pub fn player(&self) -> u16 {
        self.player
    }

    pub fn source(&self) -> &InputSource {
        &self.source
    }

    pub(crate) fn connect(&mut self, net: Rc<dyn BattleNet>) {
        match &mut self.source {
            InputSource::Network(session) => *session = Some(net),
            _ => panic!("network session assigned to local input"),
        }
    }

    pub(crate) fn network(&self) -> &dyn BattleNet {
        match &self.source {
            InputSource::Network(Some(net)) => &**net,
            _ => panic!("network input has no session"),
        }
    }

    pub fn battle_end_ready(&self) -> bool {
        match self.source {
            InputSource::Human | InputSource::Computer => true,
            InputSource::Network(_) => self.network().end_ready(usize::from(self.player)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestNet {
        ready: std::cell::Cell<bool>,
        player: std::cell::Cell<Option<usize>>,
    }
    impl BattleNet for TestNet {
        fn end_ready(&self, side: usize) -> bool {
            self.player.set(Some(side));
            self.ready.get()
        }
        fn poll(&self) {
            unreachable!()
        }
        fn flush(&self) {
            unreachable!()
        }
        fn all_connected(&self) -> bool {
            unreachable!()
        }
        fn peer_count(&self) -> usize {
            unreachable!()
        }
        fn player_order(&self, _: u16) -> u16 {
            unreachable!()
        }
        fn input_delay(&self) -> usize {
            unreachable!()
        }
        fn local_input(&self, _: usize, _: u8) -> u8 {
            unreachable!()
        }
        fn remote_input(&self, _: usize) -> u8 {
            unreachable!()
        }
        fn set_frame(&self, _: u32) {
            unreachable!()
        }
        fn frame(&self) -> u32 {
            unreachable!()
        }
        fn record_checksum(&self, _: u32, _: u32) {
            unreachable!()
        }
        fn verify_checksum(&self, _: u32) -> bool {
            unreachable!()
        }
        fn reset(&self, _: u32) {
            unreachable!()
        }
        fn negotiate(&self, _: bool, _: u32) -> bool {
            unreachable!()
        }
        fn set_state(&self, _: u32) {
            unreachable!()
        }
        fn wait_reset(&self, _: u32) {
            unreachable!()
        }
        fn init_end_frames(&self) {
            unreachable!()
        }
        fn ship_selected(&self, _: u16) {
            unreachable!()
        }
        unsafe fn begin(&self, _: *mut std::ffi::c_void) {
            unreachable!()
        }
        fn finish(&self) {
            unreachable!()
        }
        unsafe fn bind_picker(&self, _: u16, _: *mut std::ffi::c_void) {
            unreachable!()
        }
    }

    #[test]
    fn network_readiness_follows_the_connected_session_and_its_player() {
        let net = Rc::new(TestNet {
            ready: std::cell::Cell::new(false),
            player: std::cell::Cell::new(None),
        });
        let mut context = InputContext::new(1, 8);
        context.connect(net.clone());
        assert!(!context.battle_end_ready());
        assert_eq!(net.player.get(), Some(1));
        net.ready.set(true);
        assert!(context.battle_end_ready());
        let copy = context.clone();
        net.ready.set(false);
        assert!(!copy.battle_end_ready());
    }

    #[test]
    fn local_control_kinds_keep_player_identity_and_are_ready() {
        for player in 0..NUM_PLAYERS as u16 {
            for control in [1, 2, 6, 1 | 16, 2 | 64] {
                let context = InputContext::new(player, control);
                assert_eq!(context.player(), player);
                assert!(context.battle_end_ready());
                assert_eq!(
                    matches!(context.source(), InputSource::Human),
                    control & 15 == 1
                );
            }
        }
    }

    #[test]
    #[should_panic(expected = "invalid battle player")]
    fn invalid_player_is_rejected() {
        InputContext::new(2, 1);
    }

    #[test]
    #[should_panic(expected = "invalid battle control method")]
    fn invalid_control_is_rejected() {
        InputContext::new(0, 0);
    }

    #[test]
    #[should_panic(expected = "network input has no session")]
    fn network_dispatch_requires_its_session() {
        InputContext::new(1, 8).battle_end_ready();
    }
}
