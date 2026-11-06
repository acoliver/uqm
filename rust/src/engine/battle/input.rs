//! Owned battle input and dispatch. Each installed context belongs to its player.

use crate::engine::battle::net::BattleNet;
use std::rc::Rc;

pub const NUM_PLAYERS: usize = 2;
const NETWORK_CONTROL: u8 = 8;

impl crate::engine::Kernel {
    pub fn replace_player_input(
        &mut self,
        side: usize,
        input: Option<InputContext>,
    ) -> Option<InputContext> {
        if let Some(input) = &input {
            assert_eq!(
                usize::from(input.player()),
                side,
                "input belongs to another player"
            );
        }
        std::mem::replace(&mut self.player_input[side], input)
    }

    pub fn player_order(&self, rank: u16) -> u16 {
        for side in 0..NUM_PLAYERS {
            if self.player_control[side] & NETWORK_CONTROL != 0 {
                return self.player_input[side]
                    .as_ref()
                    .unwrap_or_else(|| panic!("missing player input"))
                    .network()
                    .player_order(rank);
            }
        }
        1u16.wrapping_sub(rank)
    }
}

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
            NETWORK_CONTROL => InputSource::Network(None),
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
        fn player_order(&self, rank: u16) -> u16 {
            rank
        }
        fn input_delay(&self) -> usize {
            unreachable!()
        }
        fn local_input(
            &self,
            _: usize,
            _: u8,
            _: &crate::engine::world::activity::ActivityControl,
        ) -> u8 {
            unreachable!()
        }
        fn remote_input(
            &self,
            _: usize,
            _: &crate::engine::world::activity::ActivityControl,
        ) -> u8 {
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
        fn begin(&self) {
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
    fn input_replacement_returns_the_owned_previous_context_and_preserves_other_player() {
        let mut kernel = crate::engine::Kernel::new();
        assert!(kernel
            .replace_player_input(1, Some(InputContext::new(1, 2)))
            .is_none());
        assert!(kernel
            .replace_player_input(0, Some(InputContext::new(0, 1)))
            .is_none());
        let saved = kernel
            .replace_player_input(0, Some(InputContext::new(0, 2)))
            .unwrap();
        assert!(matches!(saved.source(), InputSource::Human));
        let computer = kernel.replace_player_input(0, Some(saved)).unwrap();
        assert!(matches!(computer.source(), InputSource::Computer));
        assert!(matches!(
            kernel.player_input[0].as_ref().unwrap().source(),
            InputSource::Human
        ));
        assert!(matches!(
            kernel.player_input[1].as_ref().unwrap().source(),
            InputSource::Computer
        ));
        assert!(kernel.replace_player_input(0, None).is_some());
        assert!(kernel.replace_player_input(0, None).is_none());
    }

    #[test]
    #[should_panic(expected = "input belongs to another player")]
    fn input_cannot_be_installed_on_another_players_slot() {
        crate::engine::Kernel::new().replace_player_input(0, Some(InputContext::new(1, 1)));
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
    fn player_order_uses_the_borrowed_kernel_and_connected_network() {
        let mut kernel = crate::engine::Kernel::new();
        kernel.player_control = [1, 6];
        assert_eq!([kernel.player_order(0), kernel.player_order(1)], [1, 0]);
        for side in 0..2 {
            kernel.player_control = [1, 6];
            kernel.player_control[side] = 8;
            let mut context = InputContext::new(side as u16, 8);
            context.connect(Rc::new(TestNet {
                ready: std::cell::Cell::new(false),
                player: std::cell::Cell::new(None),
            }));
            kernel.player_input = [None, None];
            kernel.player_input[side] = Some(context);
            assert_eq!([kernel.player_order(0), kernel.player_order(1)], [0, 1]);
        }
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

    #[test]
    #[should_panic(expected = "network session assigned to local input")]
    fn local_input_rejects_a_network_session() {
        let net = Rc::new(TestNet {
            ready: std::cell::Cell::new(false),
            player: std::cell::Cell::new(None),
        });
        InputContext::new(0, 1).connect(net);
    }

    #[test]
    #[should_panic(expected = "missing player input")]
    fn network_player_order_requires_an_installed_context() {
        let mut kernel = crate::engine::Kernel::new();
        kernel.player_control = [8, 1];
        kernel.player_input = [None, None];
        kernel.player_order(0);
    }
}
