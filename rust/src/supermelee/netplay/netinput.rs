//! Frame-delayed battle input, accessed only by the game/network event thread.
use super::abi::{BattleInputBuffer, BATTLE_INPUT_STATE};
use std::ptr;

const EMPTY: BattleInputBuffer = BattleInputBuffer {
    buf: ptr::null_mut(),
    maxSize: 0,
    first: 0,
    size: 0,
};
#[derive(Debug)]
pub(super) struct InputBuffers {
    buffers: [BattleInputBuffer; 2],
    delay: usize,
}
impl Default for InputBuffers {
    fn default() -> Self {
        Self {
            buffers: [EMPTY; 2],
            delay: 0,
        }
    }
}

impl super::session::SessionCore {
    pub fn set_battle_input_delay(&self, delay: usize) {
        self.input.borrow_mut().delay = delay;
    }
    pub fn get_battle_input_delay(&self) -> usize {
        self.input.borrow().delay
    }
    pub fn push_input(&self, player: usize, value: u8) -> bool {
        // SAFETY: the session exclusively owns its initialized input buffers.
        unsafe { BattleInputBuffer_push(&mut self.input.borrow_mut().buffers[player], value) }
    }
    pub fn pop_input(&self, player: usize) -> Option<u8> {
        let mut value = 0;
        // SAFETY: the session exclusively owns its initialized input buffers.
        unsafe {
            BattleInputBuffer_pop(&mut self.input.borrow_mut().buffers[player], &mut value)
                .then_some(value)
        }
    }
}

unsafe fn init(buffer: *mut BattleInputBuffer, capacity: usize) {
    (*buffer).buf = libc::malloc(capacity * size_of::<BATTLE_INPUT_STATE>()).cast();
    assert!(!(*buffer).buf.is_null());
    (*buffer).maxSize = capacity;
    (*buffer).first = 0;
    (*buffer).size = 0;
}

impl super::session::SessionCore {
    /// # Safety
    /// The session's buffers must not already own allocations.
    pub unsafe fn init_battle_input_buffers(&self) {
        let mut input = self.input.borrow_mut();
        let delay = input.delay;
        let capacity = delay.wrapping_mul(2).wrapping_add(2) as i32;
        for buffer in &mut input.buffers {
            init(buffer, capacity as usize);
            for _ in 0..delay {
                BattleInputBuffer_push(buffer, 0);
            }
        }
    }
    /// # Safety
    /// No caller may retain a reference to the input buffers.
    pub unsafe fn uninit_battle_input_buffers(&self) {
        for buffer in &mut self.input.borrow_mut().buffers {
            libc::free(buffer.buf.cast());
            *buffer = EMPTY;
        }
    }
}
/// # Safety
/// `buffer` is a live, exclusively owned initialized ring buffer.
#[no_mangle]
pub unsafe extern "C" fn BattleInputBuffer_push(
    buffer: *mut BattleInputBuffer,
    input: BATTLE_INPUT_STATE,
) -> bool {
    if (*buffer).size == (*buffer).maxSize {
        log::error!("NETPLAY: battleInputBuffer full.");
        super::set_errno(libc::ENOBUFS);
        return false;
    }
    let next = ((*buffer).first + (*buffer).size) % (*buffer).maxSize;
    *(*buffer).buf.add(next) = input;
    (*buffer).size += 1;
    true
}
/// # Safety
/// `buffer` is initialized and exclusively owned; `input` is writable.
#[no_mangle]
pub unsafe extern "C" fn BattleInputBuffer_pop(
    buffer: *mut BattleInputBuffer,
    input: *mut BATTLE_INPUT_STATE,
) -> bool {
    if (*buffer).size == 0 {
        super::set_errno(libc::EAGAIN);
        return false;
    }
    *input = *(*buffer).buf.add((*buffer).first);
    (*buffer).first = ((*buffer).first + 1) % (*buffer).maxSize;
    (*buffer).size -= 1;
    true
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ring_full_empty_and_wraparound_preserve_fifo() {
        unsafe {
            let mut buffer = EMPTY;
            init(&raw mut buffer, 3);
            let mut input = 99;
            assert!(!BattleInputBuffer_pop(&raw mut buffer, &mut input));
            assert_eq!(input, 99);
            for value in [1, 2, 3] {
                assert!(BattleInputBuffer_push(&raw mut buffer, value));
            }
            assert!(!BattleInputBuffer_push(&raw mut buffer, 4));
            assert!(BattleInputBuffer_pop(&raw mut buffer, &mut input));
            assert_eq!(input, 1);
            assert!(BattleInputBuffer_push(&raw mut buffer, 4));
            for expected in [2, 3, 4] {
                assert!(BattleInputBuffer_pop(&raw mut buffer, &mut input));
                assert_eq!(input, expected);
            }
            assert!(!BattleInputBuffer_pop(&raw mut buffer, &mut input));
            assert_eq!(input, 4);
            libc::free(buffer.buf.cast());
        }
    }
    #[test]
    #[serial_test::serial]
    fn delay_initializes_both_players_with_zero_input_and_releases_storage() {
        unsafe {
            let session = super::super::NetplaySession::default();
            for delay in [0, 2, 35] {
                session.set_battle_input_delay(delay);
                session.init_battle_input_buffers();
                for player in 0..2 {
                    let buffer =
                        &mut session.input.borrow_mut().buffers[player] as *mut BattleInputBuffer;
                    assert_eq!((*buffer).maxSize, delay * 2 + 2);
                    assert_eq!((*buffer).size, delay);
                    let mut input = 123;
                    for _ in 0..delay {
                        assert!(BattleInputBuffer_pop(buffer, &mut input));
                        assert_eq!(input, 0);
                    }
                    assert!(!BattleInputBuffer_pop(buffer, &mut input));
                }
                session.uninit_battle_input_buffers();
                session.uninit_battle_input_buffers();
                for player in 0..2 {
                    let buffer =
                        &mut session.input.borrow_mut().buffers[player] as *mut BattleInputBuffer;
                    assert!((*buffer).buf.is_null());
                    assert_eq!((*buffer).maxSize, 0);
                }
            }
        }
    }
}
