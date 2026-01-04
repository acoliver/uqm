// Game State Management
// Provides safe Rust wrappers around 300+ game state bits
// Preserves exact C bitfield layout for save/load compatibility

use std::fmt;

pub const NUM_GAME_STATE_BITS: usize = 2048; // Estimate based on globdata.h
pub const NUM_GAME_STATE_BYTES: usize = (NUM_GAME_STATE_BITS + 7) >> 3;

/// Game state container with C-compatible bitfield layout
#[repr(C)]
pub struct GameState {
    /// Raw game state bytes - must maintain exact C layout
    bytes: [u8; NUM_GAME_STATE_BYTES],
}

impl GameState {
    /// Create a new, zero-initialized game state
    pub fn new() -> Self {
        GameState {
            bytes: [0u8; NUM_GAME_STATE_BYTES],
        }
    }

    /// Get game state value from bit range [start_bit, end_bit] inclusive
    /// Returns a byte value containing the extracted bits
    ///
    /// # Safety
    /// Must maintain exact bit extraction logic from C implementation
    pub fn get_state(&self, start_bit: usize, end_bit: usize) -> u8 {
        assert!(start_bit < NUM_GAME_STATE_BITS);
        assert!(end_bit < NUM_GAME_STATE_BITS);
        assert!(end_bit >= start_bit);
        assert!((end_bit - start_bit + 1) <= 8);

        let start_byte = start_bit >> 3;
        let end_byte = end_bit >> 3;
        let start_offset = start_bit & 7;

        // Calculate mask without overflow - use u16 for intermediate calculations
        let bit_width = end_bit - start_bit + 1;
        let mask = if bit_width == 8 {
            0xFFu8
        } else {
            (1u8 << bit_width) - 1
        };

        if start_byte == end_byte {
            // Single byte case
            (self.bytes[start_byte] >> start_offset) & mask
        } else {
            // Multi-byte case
            let low = self.bytes[start_byte] >> start_offset;
            let high_bit_count = (end_bit & 7) + 1;
            let high_mask = if high_bit_count == 8 {
                0xFFu8
            } else {
                (1u8 << high_bit_count) - 1
            };
            let high = self.bytes[end_byte] & high_mask;

            (low | (high << (end_bit - start_bit - (end_bit & 7)))) & mask
        }
    }

    /// Set game state value for bit range [start_bit, end_bit] inclusive
    ///
    /// # Safety
    /// Must maintain exact bit setting logic from C implementation
    pub fn set_state(&mut self, start_bit: usize, end_bit: usize, value: u8) {
        assert!(start_bit < NUM_GAME_STATE_BITS);
        assert!(end_bit < NUM_GAME_STATE_BITS);
        assert!(end_bit >= start_bit);
        assert!((end_bit - start_bit + 1) <= 8);
        // Check value fits in bitfield - saturate subtraction to avoid overflow
        let bit_width = end_bit - start_bit + 1;
        // For 8-bit fields, all u8 values are valid
        if bit_width < 8 {
            let max_value = (1u8 << bit_width) - 1;
            assert!(
                value <= max_value,
                "Value {} exceeds bitfield width {} bits",
                value,
                bit_width
            );
        }
        // bit_width == 8 means all values 0-255 are valid, no check needed

        let start_byte = start_bit >> 3;
        let end_byte = end_bit >> 3;
        let start_offset = start_bit & 7;
        // Use u16 to avoid overflow when bit_width is 8
        let mask_u16 = ((1u16 << (end_bit - start_bit + 1)) - 1) << start_offset;
        let mask = (mask_u16 & 0xFF) as u8;

        // Clear the target bits
        self.bytes[start_byte] &= !mask;

        // Set the target bits
        self.bytes[start_byte] |= (value << start_offset) & mask;

        // Handle multi-byte case
        if start_byte < end_byte {
            let high_bit_count = (end_bit & 7) + 1;
            let high_mask = (1u8 << high_bit_count) - 1;
            let high_shift = end_bit - start_bit - (end_bit & 7);

            self.bytes[end_byte] &= !high_mask;
            self.bytes[end_byte] |= (value >> high_shift) & high_mask;
        }
    }

    /// Get a 32-bit game state value starting at start_bit
    pub fn get_state_32(&self, start_bit: usize) -> u32 {
        assert!(start_bit + 31 < NUM_GAME_STATE_BITS);

        let mut result: u32 = 0;
        for i in 0..4 {
            let byte_val = self.get_state(start_bit + (i * 8), start_bit + (i * 8) + 7);
            result |= (byte_val as u32) << (i * 8);
        }
        result
    }

    /// Set a 32-bit game state value starting at start_bit
    pub fn set_state_32(&mut self, start_bit: usize, value: u32) {
        assert!(start_bit + 31 < NUM_GAME_STATE_BITS);

        let mut v = value;
        for i in 0..4 {
            self.set_state(
                start_bit + (i * 8),
                start_bit + (i * 8) + 7,
                (v & 0xFF) as u8,
            );
            v >>= 8;
        }
    }

    /// Copy game state bits from source to destination
    pub fn copy_state(
        &mut self,
        dest_bit: usize,
        src: &GameState,
        src_start_bit: usize,
        src_end_bit: usize,
    ) {
        let mut begin = src_start_bit;
        let mut target = dest_bit;

        while begin <= src_end_bit {
            let delta = if begin + 7 > src_end_bit {
                src_end_bit - begin
            } else {
                7
            };
            let b = src.get_state(begin, begin + delta);
            self.set_state(target, target + delta, b);
            begin += delta + 1;
            target += delta + 1;
        }
    }

    /// Get raw bytes for serialization
    pub fn as_bytes(&self) -> &[u8; NUM_GAME_STATE_BYTES] {
        &self.bytes
    }

    /// Set raw bytes from deserialization
    pub fn from_bytes(bytes: &[u8; NUM_GAME_STATE_BYTES]) -> Self {
        GameState { bytes: *bytes }
    }

    /// Reset all game state to zero
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
    fn test_new_game_state() {
        let state = GameState::new();
        assert_eq!(state.bytes.len(), NUM_GAME_STATE_BYTES);
        assert_eq!(state.bytes.iter().filter(|&&b| b != 0).count(), 0);
    }

    #[test]
    fn test_get_set_state_single_bit() {
        let mut state = GameState::new();

        // Set bit 0
        state.set_state(0, 0, 1);
        assert_eq!(state.get_state(0, 0), 1);
        assert_eq!(state.bytes[0], 0b00000001);

        // Set bit 7
        state.set_state(7, 7, 1);
        assert_eq!(state.get_state(7, 7), 1);
        assert_eq!(state.bytes[0], 0b10000001);

        // Clear bit 0
        state.set_state(0, 0, 0);
        assert_eq!(state.get_state(0, 0), 0);
        assert_eq!(state.bytes[0], 0b10000000);
    }

    #[test]
    fn test_get_set_state_multi_bits() {
        let mut state = GameState::new();

        // Set bits 0-3 (value 0b1101 = 13)
        state.set_state(0, 3, 0b1101);
        assert_eq!(state.get_state(0, 3), 0b1101);
        assert_eq!(state.bytes[0], 0b1101);

        // Set bits 5-7 (value 0b101 = 5)
        state.set_state(5, 7, 0b101);
        assert_eq!(state.get_state(5, 7), 0b101);
        // Byte 0 should be 0b10101101
        assert_eq!(state.bytes[0], 0b10101101);
    }

    #[test]
    fn test_get_set_state_cross_byte() {
        let mut state = GameState::new();

        // Set bits 6-9 (crosses byte boundary)
        let value = 0b1101; // 4 bits
        state.set_state(6, 9, value);

        // Verify
        assert_eq!(state.get_state(6, 9), value);
        // Bits 6-7 should be in byte 0, bits 8-9 in byte 1
        assert_eq!(state.bytes[0] & 0b11000000, 0b01000000);
        assert_eq!(state.bytes[1] & 0b00000011, 0b00000011);
    }

    #[test]
    fn test_get_state_32() {
        let mut state = GameState::new();

        // Set a 32-bit value
        state.set_state_32(0, 0xDEADBEEF);
        assert_eq!(state.get_state_32(0), 0xDEADBEEF);

        // Set at a different offset
        state.set_state_32(64, 0xCAFEBABE);
        assert_eq!(state.get_state_32(64), 0xCAFEBABE);
    }

    #[test]
    fn test_copy_state() {
        let mut src = GameState::new();
        let mut dest = GameState::new();

        // Set some bits in source
        src.set_state(0, 7, 0xAB);
        src.set_state(8, 15, 0xCD);

        // Copy to destination (copy bits 0-15 to bits 32-47)
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
        state.set_state(0, 2, 0b1111); // 4 bits (value 15) in 3-bit field
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
        assert_eq!(state.bytes[0], 0b11110101); // 0b11, 0b110, 0b101 = 11 110 101
    }
}
