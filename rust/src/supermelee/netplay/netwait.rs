//! Ordered ready/reset exchanges across the caller's input frames.
use super::{abi::*, netconnection::ReadyAction};
use crate::engine::battle::net::ProtocolDriver;
const NETWORK_POLL_DELAY: u32 = uqm_engine::time::counter_abi::ONE_SECOND / 24;

impl super::session::SessionCore {
    pub fn negotiate_ready_connections(
        &self,
        notify: bool,
        next: u32,
        driver: &mut dyn ProtocolDriver,
    ) -> bool {
        self.wait_connections(|side| {
            self.negotiate_ready(side, notify, next, driver);
        })
    }
    pub fn wait_reset_connections(&self, next: u32, driver: &mut dyn ProtocolDriver) -> bool {
        self.wait_connections(|side| {
            let generation = {
                let mut conn = self
                    .connection_mut(side)
                    .unwrap_or_else(|| panic!("reset connection"));
                let generation = conn.reset_generation;
                conn.reset_next = Some(next);
                conn.reset_complete();
                generation
            };
            if self
                .connection(side)
                .unwrap_or_else(|| panic!("reset connection"))
                .reset_generation
                == generation
            {
                let effects = {
                    let mut conn = self
                        .connection_mut(side)
                        .unwrap_or_else(|| panic!("reset connection"));
                    if !conn.flags.reset.local {
                        conn.local_reset(2)
                    } else {
                        Vec::new()
                    }
                };
                self.with_notices(driver, |emit| self.apply_events(effects, emit));
                self.with_notices(driver, |emit| self.flush_packet_queues(emit));
                self.wait_exchange(side, generation, true, driver);
            }
        })
    }
    fn wait_connections(&self, mut wait: impl FnMut(usize)) -> bool {
        let mut disconnected = 0;
        for side in 0..2 {
            let Some(conn) = self.connection(side) else {
                continue;
            };
            let connected = conn.flags.connected;
            drop(conn);
            if !connected {
                disconnected += 1;
                continue;
            }
            wait(side);
        }
        disconnected == 0
    }
    fn wait_exchange(
        &self,
        side: usize,
        generation: usize,
        reset: bool,
        driver: &mut dyn ProtocolDriver,
    ) {
        let mut poll = |driver: &mut dyn ProtocolDriver| {
            self.with_notices(driver, |emit| {
                self.net_input_blocking(NETWORK_POLL_DELAY, emit)
            });
            self.connection(side).is_some_and(|conn| {
                conn.flags.connected
                    && if reset {
                        conn.reset_generation == generation
                    } else {
                        conn.ready_generation == generation
                    }
            })
        };
        if !self.replay_wait(|| poll(driver)) {
            driver.wait(&mut poll);
        }
    }
    fn negotiate_ready(
        &self,
        side: usize,
        notify: bool,
        next: u32,
        driver: &mut dyn ProtocolDriver,
    ) -> bool {
        let (generation, effects) = {
            let mut conn = self
                .connection_mut(side)
                .unwrap_or_else(|| panic!("ready connection"));
            let generation = conn.ready_generation;
            let effects = conn.local_ready(
                ReadyAction::Transition(next),
                notify,
                self.frame_count.get(),
            );
            (generation, effects)
        };
        self.with_notices(driver, |emit| self.apply_events(effects, emit));
        self.with_notices(driver, |emit| self.flush_packet_queues(emit));
        if self
            .connection(side)
            .unwrap_or_else(|| panic!("ready connection"))
            .ready_generation
            == generation
        {
            self.wait_exchange(side, generation, false, driver);
        }
        self.connection(side)
            .is_some_and(|conn| conn.flags.connected)
    }
    pub(super) fn ready_for_battle_end_player(
        &self,
        side: usize,
        driver: &mut dyn ProtocolDriver,
    ) -> bool {
        let state = self
            .connection(side)
            .unwrap_or_else(|| panic!("end connection"))
            .state;
        if state == NetState_NetState_interBattle || state == NetState_NetState_inSetup {
            return true;
        }
        if state == NetState_NetState_inBattle {
            let effects = {
                let mut conn = self
                    .connection_mut(side)
                    .unwrap_or_else(|| panic!("end connection"));
                if conn.flags.ready.local {
                    return false;
                }
                conn.local_ready(ReadyAction::EndBattle, true, self.frame_count.get())
            };
            self.with_notices(driver, |emit| self.apply_events(effects, emit));
            if self
                .connection(side)
                .unwrap_or_else(|| panic!("end connection"))
                .state
                == NetState_NetState_inBattle
            {
                return false;
            }
        }
        let conn = self
            .connection(side)
            .unwrap_or_else(|| panic!("end connection"));
        assert!(
            conn.state == NetState_NetState_endingBattle
                || conn.state == NetState_NetState_endingBattle2
        );
        if self.frame_count.get() < conn.end_frame_count {
            return false;
        }
        let generation = conn.ready_generation;
        let state = conn.state;
        drop(conn);
        if state == NetState_NetState_endingBattle {
            self.wait_exchange(side, generation, false, driver);
            let conn = self
                .connection(side)
                .unwrap_or_else(|| panic!("end connection"));
            assert_eq!(conn.state, NetState_NetState_endingBattle2);
            if self.frame_count.get() < conn.end_frame_count {
                return false;
            }
        }
        self.negotiate_ready(side, true, NetState_NetState_interBattle, driver);
        true
    }
}
