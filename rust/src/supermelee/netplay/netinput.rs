//! Frame-delayed battle input owned by the session on the game thread.
use std::collections::VecDeque;

#[derive(Debug, Default)]
pub(super) struct InputBuffers {
    buffers: [VecDeque<u8>; 2],
    delay: usize,
    capacity: usize,
}

impl super::session::SessionCore {
    pub fn set_battle_input_delay(&self, delay: usize) {
        self.input.borrow_mut().delay = delay;
    }
    pub fn get_battle_input_delay(&self) -> usize {
        self.input.borrow().delay
    }
    pub fn push_input(&self, player: usize, value: u8) -> bool {
        let mut input = self.input.borrow_mut();
        if input.buffers[player].len() == input.capacity {
            super::set_errno(libc::ENOBUFS);
            return false;
        }
        input.buffers[player].push_back(value);
        true
    }
    pub fn pop_input(&self, player: usize) -> Option<u8> {
        self.input.borrow_mut().buffers[player].pop_front()
    }
    pub fn init_battle_input_buffers(&self) {
        let mut input = self.input.borrow_mut();
        let delay = input.delay;
        input.capacity = delay * 2 + 2;
        for buffer in &mut input.buffers {
            assert!(buffer.is_empty(), "battle input already initialized");
            buffer.resize(delay, 0);
        }
    }
    pub fn uninit_battle_input_buffers(&self) {
        let mut input = self.input.borrow_mut();
        input.buffers = Default::default();
        input.capacity = 0;
    }
}
#[cfg(test)]
mod tests {
    use super::super::NetplaySession;
    #[test]
    fn ring_full_empty_and_wraparound_preserve_fifo() {
        let session = NetplaySession::default();
        session.init_battle_input_buffers();
        assert_eq!(session.pop_input(0), None);
        assert!(session.push_input(0, 1));
        assert!(session.push_input(0, 2));
        assert!(!session.push_input(0, 3));
        assert_eq!(session.pop_input(0), Some(1));
        assert!(session.push_input(0, 3));
        assert_eq!(session.pop_input(0), Some(2));
        assert_eq!(session.pop_input(0), Some(3));
        assert_eq!(session.pop_input(0), None);
    }
    #[test]
    fn delay_initializes_both_players_with_zero_input_and_releases_storage() {
        let session = NetplaySession::default();
        for delay in [0, 2, 35] {
            session.set_battle_input_delay(delay);
            session.init_battle_input_buffers();
            assert_eq!(session.input.borrow().capacity, delay * 2 + 2);
            for player in 0..2 {
                for _ in 0..delay {
                    assert_eq!(session.pop_input(player), Some(0));
                }
                assert_eq!(session.pop_input(player), None);
            }
            session.uninit_battle_input_buffers();
            session.uninit_battle_input_buffers();
            assert_eq!(session.input.borrow().capacity, 0);
            assert!(session.input.borrow().buffers.iter().all(|b| b.is_empty()));
        }
    }
}
