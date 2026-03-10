// Game State Management
// Provides safe Rust wrappers around the serialized game-state bitfield.

use std::fmt;

include!(concat!(env!("OUT_DIR"), "/state_generated.rs"));

fn assert_raw_bit_range(state_len: usize, start_bit: usize, end_bit: usize) {
    assert!(end_bit >= start_bit);
    assert!((end_bit - start_bit + 1) <= 8);
    assert!(end_bit < state_len * 8);
}

pub fn get_state_bits_raw(state: &[u8], start_bit: usize, end_bit: usize) -> u8 {
    assert_raw_bit_range(state.len(), start_bit, end_bit);

    let start_byte = start_bit >> 3;
    let end_byte = end_bit >> 3;
    let start_offset = start_bit & 7;
    let bit_width = end_bit - start_bit + 1;
    let mask = if bit_width == 8 {
        0xFFu8
    } else {
        (1u8 << bit_width) - 1
    };

    if start_byte == end_byte {
        (state[start_byte] >> start_offset) & mask
    } else {
        let low = state[start_byte] >> start_offset;
        let high_bit_count = (end_bit & 7) + 1;
        let high_mask = if high_bit_count == 8 {
            0xFFu8
        } else {
            (1u8 << high_bit_count) - 1
        };
        let high = state[end_byte] & high_mask;

        (low | (high << (end_bit - start_bit - (end_bit & 7)))) & mask
    }
}

pub fn set_state_bits_raw(state: &mut [u8], start_bit: usize, end_bit: usize, value: u8) {
    assert_raw_bit_range(state.len(), start_bit, end_bit);

    let bit_width = end_bit - start_bit + 1;
    if bit_width < 8 {
        let max_value = (1u8 << bit_width) - 1;
        assert!(
            value <= max_value,
            "Value {} exceeds bitfield width {} bits",
            value,
            bit_width
        );
    }

    let start_byte = start_bit >> 3;
    let end_byte = end_bit >> 3;
    let start_offset = start_bit & 7;
    let mask_u16 = ((1u16 << bit_width) - 1) << start_offset;
    let mask = (mask_u16 & 0xFF) as u8;

    state[start_byte] &= !mask;
    state[start_byte] |= (value << start_offset) & mask;

    if start_byte < end_byte {
        let high_bit_count = (end_bit & 7) + 1;
        let high_mask = if high_bit_count == 8 {
            0xFFu8
        } else {
            (1u8 << high_bit_count) - 1
        };
        let high_shift = end_bit - start_bit - (end_bit & 7);

        state[end_byte] &= !high_mask;
        state[end_byte] |= (value >> high_shift) & high_mask;
    }
}

pub fn get_state_32_raw(state: &[u8], start_bit: usize) -> u32 {
    assert!(start_bit + 31 < state.len() * 8);

    let mut result: u32 = 0;
    for i in 0..4 {
        let byte_val = get_state_bits_raw(state, start_bit + (i * 8), start_bit + (i * 8) + 7);
        result |= (byte_val as u32) << (i * 8);
    }
    result
}

pub fn set_state_32_raw(state: &mut [u8], start_bit: usize, value: u32) {
    assert!(start_bit + 31 < state.len() * 8);

    let mut v = value;
    for i in 0..4 {
        set_state_bits_raw(
            state,
            start_bit + (i * 8),
            start_bit + (i * 8) + 7,
            (v & 0xFF) as u8,
        );
        v >>= 8;
    }
}

pub fn copy_state_bits_raw(dest: &mut [u8], target: usize, src: &[u8], begin: usize, end: usize) {
    assert!(begin <= end);
    if begin == end {
        return;
    }

    assert!(end < src.len() * 8);
    assert!(target + (end - begin) < dest.len() * 8);

    let mut begin = begin;
    let mut target = target;

    while begin < end {
        let mut delta = 7;
        if begin + delta > end {
            delta = end - begin;
        }
        let value = get_state_bits_raw(src, begin, begin + delta);
        set_state_bits_raw(dest, target, target + delta, value);
        begin += 8;
        target += 8;
    }
}

/// Game state container with C-compatible bitfield layout.
#[repr(C)]
pub struct GameState {
    bytes: [u8; NUM_GAME_STATE_BYTES],
}

impl GameState {
    /// Create a new, zero-initialized game state.
    pub fn new() -> Self {
        GameState {
            bytes: [0u8; NUM_GAME_STATE_BYTES],
        }
    }

    /// Look up the bit range for a named state entry.
    pub fn lookup_bits(name: &str) -> Option<(usize, usize)> {
        lookup_game_state_bits(name)
    }

    /// Get a game state value from bit range [start_bit, end_bit] inclusive.
    pub fn get_state(&self, start_bit: usize, end_bit: usize) -> u8 {
        get_state_bits_raw(&self.bytes, start_bit, end_bit)
    }

    /// Set a game state value for bit range [start_bit, end_bit] inclusive.
    pub fn set_state(&mut self, start_bit: usize, end_bit: usize, value: u8) {
        set_state_bits_raw(&mut self.bytes, start_bit, end_bit, value)
    }

    /// Get a 32-bit game state value starting at start_bit.
    pub fn get_state_32(&self, start_bit: usize) -> u32 {
        get_state_32_raw(&self.bytes, start_bit)
    }

    /// Set a 32-bit game state value starting at start_bit.
    pub fn set_state_32(&mut self, start_bit: usize, value: u32) {
        set_state_32_raw(&mut self.bytes, start_bit, value)
    }

    /// Copy game state bits from source to destination.
    pub fn copy_state(
        &mut self,
        dest_bit: usize,
        src: &GameState,
        src_start_bit: usize,
        src_end_bit: usize,
    ) {
        copy_state_bits_raw(&mut self.bytes, dest_bit, &src.bytes, src_start_bit, src_end_bit)
    }

    /// Get raw bytes for serialization.
    pub fn as_bytes(&self) -> &[u8; NUM_GAME_STATE_BYTES] {
        &self.bytes
    }

    /// Restore state bytes from serialized storage.
    pub fn from_bytes(bytes: &[u8; NUM_GAME_STATE_BYTES]) -> Self {
        GameState { bytes: *bytes }
    }

    /// Reset all game state to zero.
    pub fn reset(&mut self) {
        self.bytes = [0u8; NUM_GAME_STATE_BYTES];
    }
}

impl Default for GameState {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for GameState {
    fn clone(&self) -> Self {
        GameState { bytes: self.bytes }
    }
}

impl fmt::Debug for GameState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GameState")
            .field("num_bits", &NUM_GAME_STATE_BITS)
            .field("num_bytes", &NUM_GAME_STATE_BYTES)
            .field(
                "non_zero_bytes",
                &self.bytes.iter().filter(|&&b| b != 0).count(),
            )
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generated_layout_matches_c_header() {
        assert_eq!(NUM_GAME_STATE_BITS, 1238);
        assert_eq!(NUM_GAME_STATE_BYTES, 155);
        assert_eq!(GameState::lookup_bits("SHOFIXTI_VISITS"), Some((0, 2)));
        assert_eq!(GameState::lookup_bits("SHOFIXTI_RECRUITED"), Some((12, 12)));
        assert_eq!(GameState::lookup_bits("SPATHI_VISITS"), Some((18, 20)));
    }

    #[test]
    fn test_raw_helpers_operate_on_plain_byte_buffers() {
        let mut bytes = [0u8; NUM_GAME_STATE_BYTES];

        set_state_bits_raw(&mut bytes, 0, 2, 0b101);
        set_state_bits_raw(&mut bytes, 12, 12, 1);
        set_state_32_raw(&mut bytes, 32, 0xDEADBEEF);

        assert_eq!(get_state_bits_raw(&bytes, 0, 2), 0b101);
        assert_eq!(get_state_bits_raw(&bytes, 12, 12), 1);
        assert_eq!(get_state_32_raw(&bytes, 32), 0xDEADBEEF);
    }

    #[test]
    fn test_copy_state_bits_raw_matches_c_helper_semantics() {
        let mut src = [0u8; NUM_GAME_STATE_BYTES];
        let mut dest = [0u8; NUM_GAME_STATE_BYTES];

        set_state_bits_raw(&mut src, 0, 7, 0xAB);
        set_state_bits_raw(&mut src, 8, 15, 0xCD);

        copy_state_bits_raw(&mut dest, 32, &src, 0, 15);
        copy_state_bits_raw(&mut dest, 80, &src, 0, 0);

        assert_eq!(get_state_bits_raw(&dest, 32, 39), 0xAB);
        assert_eq!(get_state_bits_raw(&dest, 40, 47), 0xCD);
        assert_eq!(get_state_bits_raw(&dest, 80, 80), 0);
    }

    #[test]
    fn test_new_game_state() {
        let state = GameState::new();
        assert_eq!(state.bytes.len(), NUM_GAME_STATE_BYTES);
        assert_eq!(state.bytes.iter().filter(|&&b| b != 0).count(), 0);
    }

    #[test]
    fn test_get_set_state_single_bit() {
        let mut state = GameState::new();

        state.set_state(0, 0, 1);
        assert_eq!(state.get_state(0, 0), 1);
        assert_eq!(state.bytes[0], 0b00000001);

        state.set_state(7, 7, 1);
        assert_eq!(state.get_state(7, 7), 1);
        assert_eq!(state.bytes[0], 0b10000001);

        state.set_state(0, 0, 0);
        assert_eq!(state.get_state(0, 0), 0);
        assert_eq!(state.bytes[0], 0b10000000);
    }

    #[test]
    fn test_get_set_state_multi_bits() {
        let mut state = GameState::new();

        state.set_state(0, 3, 0b1101);
        assert_eq!(state.get_state(0, 3), 0b1101);
        assert_eq!(state.bytes[0], 0b1101);

        state.set_state(5, 7, 0b101);
        assert_eq!(state.get_state(5, 7), 0b101);
        assert_eq!(state.bytes[0], 0b10101101);
    }

    #[test]
    fn test_get_set_state_cross_byte() {
        let mut state = GameState::new();

        let value = 0b1101;
        state.set_state(6, 9, value);

        assert_eq!(state.get_state(6, 9), value);
        assert_eq!(state.bytes[0] & 0b11000000, 0b01000000);
        assert_eq!(state.bytes[1] & 0b00000011, 0b00000011);
    }

    #[test]
    fn test_get_state_32() {
        let mut state = GameState::new();

        state.set_state_32(0, 0xDEADBEEF);
        assert_eq!(state.get_state_32(0), 0xDEADBEEF);

        state.set_state_32(64, 0xCAFEBABE);
        assert_eq!(state.get_state_32(64), 0xCAFEBABE);
    }

    #[test]
    fn test_copy_state() {
        let mut src = GameState::new();
        let mut dest = GameState::new();

        src.set_state(0, 7, 0xAB);
        src.set_state(8, 15, 0xCD);

        dest.copy_state(32, &src, 0, 15);

        assert_eq!(dest.get_state(32, 39), 0xAB);
        assert_eq!(dest.get_state(40, 47), 0xCD);
    }

    #[test]
    fn test_reset() {
        let mut state = GameState::new();
        state.set_state(0, 7, 0xFF);
        state.set_state(100, 107, 0xFF);

        state.reset();

        assert_eq!(state.bytes.iter().filter(|&&b| b != 0).count(), 0);
    }

    #[test]
    fn test_default() {
        let state: GameState = Default::default();
        assert_eq!(state.bytes.len(), NUM_GAME_STATE_BYTES);
    }

    #[test]
    fn test_clone() {
        let mut state1 = GameState::new();
        state1.set_state(0, 7, 0xAB);
        state1.set_state(100, 103, 0x0D);

        let state2 = state1.clone();

        assert_eq!(state2.get_state(0, 7), 0xAB);
        assert_eq!(state2.get_state(100, 103), 0x0D);
    }

    #[test]
    #[should_panic(expected = "Value 15 exceeds bitfield width 3 bits")]
    fn test_set_state_value_too_large() {
        let mut state = GameState::new();
        state.set_state(0, 2, 0b1111);
    }

    #[test]
    fn test_multiple_non_overlapping_sets() {
        let mut state = GameState::new();

        state.set_state(0, 2, 0b101);
        state.set_state(3, 5, 0b110);
        state.set_state(6, 7, 0b11);

        assert_eq!(state.get_state(0, 2), 0b101);
        assert_eq!(state.get_state(3, 5), 0b110);
        assert_eq!(state.get_state(6, 7), 0b11);
        assert_eq!(state.bytes[0], 0b11110101);
    }
}
