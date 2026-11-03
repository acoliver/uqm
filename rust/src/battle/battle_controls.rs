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

    #[cfg(not(test))]
    pub(crate) fn frame_input(
        &self,
        cx: &mut crate::engine::battle::Cx<'_>,
        ship: Option<crate::engine::battle::ShipId>,
    ) -> u8 {
        match self.source {
            InputSource::Human => cx.human_input(self.player),
            InputSource::Computer => {
                crate::battle::context_access::ai_legacy::computer_intelligence(
                    cx,
                    self.player,
                    ship,
                )
            }
            InputSource::Network(_) => self.network().remote_input(usize::from(self.player)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
