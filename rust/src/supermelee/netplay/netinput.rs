//! Frame-delayed battle input, accessed only by the game/network event thread.
use super::abi::{BattleInputBuffer, BATTLE_INPUT_STATE};
use std::ptr;

const EMPTY: BattleInputBuffer = BattleInputBuffer {
    buf: ptr::null_mut(),
    maxSize: 0,
    first: 0,
    size: 0,
};
static mut BUFFERS: [BattleInputBuffer; 2] = [EMPTY; 2];
static mut INPUT_DELAY: usize = 0;

/// # Safety
/// Must be called on the game thread before initializing the buffers.
#[no_mangle]
pub unsafe extern "C" fn setBattleInputDelay(delay: usize) {
    INPUT_DELAY = delay;
}
/// # Safety
/// Must be called on the game thread.
#[no_mangle]
pub unsafe extern "C" fn getBattleInputDelay() -> usize {
    INPUT_DELAY
}

unsafe fn init(buffer: *mut BattleInputBuffer, capacity: usize) {
    (*buffer).buf = libc::malloc(capacity * size_of::<BATTLE_INPUT_STATE>()).cast();
    assert!(!(*buffer).buf.is_null());
    (*buffer).maxSize = capacity;
    (*buffer).first = 0;
    (*buffer).size = 0;
}

/// # Safety
/// Buffers must not already own allocations. Called on the game thread.
#[no_mangle]
pub unsafe extern "C" fn initBattleInputBuffers() {
    // Preserve the C int conversion before allocating.
    let capacity = INPUT_DELAY.wrapping_mul(2).wrapping_add(2) as i32;
    for player in 0..2 {
        let buffer = getBattleInputBuffer(player);
        init(buffer, capacity as usize);
        for _ in 0..INPUT_DELAY {
            BattleInputBuffer_push(buffer, 0);
        }
    }
}
/// # Safety
/// No callers may retain buffer references. Called on the game thread.
#[no_mangle]
pub unsafe extern "C" fn uninitBattleInputBuffers() {
    for player in 0..2 {
        let buffer = getBattleInputBuffer(player);
        libc::free((*buffer).buf.cast());
        buffer.write(EMPTY);
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
/// # Safety
/// `player` is 0 or 1. Only the game thread may access the returned buffer.
#[no_mangle]
pub unsafe extern "C" fn getBattleInputBuffer(player: usize) -> *mut BattleInputBuffer {
    assert!(player < 2);
    ptr::addr_of_mut!(BUFFERS)
        .cast::<BattleInputBuffer>()
        .add(player)
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
            let previous = getBattleInputDelay();
            for delay in [0, 2, 35] {
                setBattleInputDelay(delay);
                initBattleInputBuffers();
                for player in 0..2 {
                    let buffer = getBattleInputBuffer(player);
                    assert_eq!((*buffer).maxSize, delay * 2 + 2);
                    assert_eq!((*buffer).size, delay);
                    let mut input = 123;
                    for _ in 0..delay {
                        assert!(BattleInputBuffer_pop(buffer, &mut input));
                        assert_eq!(input, 0);
                    }
                    assert!(!BattleInputBuffer_pop(buffer, &mut input));
                }
                uninitBattleInputBuffers();
                uninitBattleInputBuffers();
                for player in 0..2 {
                    let buffer = getBattleInputBuffer(player);
                    assert!((*buffer).buf.is_null());
                    assert_eq!((*buffer).maxSize, 0);
                }
            }
            setBattleInputDelay(previous);
        }
    }
}
