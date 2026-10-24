use crate::battle::netplay::CrcState;

pub fn crc_init(state: &mut CrcState) {
    state.init();
}

pub fn crc_process_bytes(state: &mut CrcState, bytes: &[u8]) {
    for &byte in bytes {
        state.update(byte);
    }
}

pub fn crc_process_u8(state: &mut CrcState, value: u8) {
    crc_process_bytes(state, &[value]);
}

pub fn crc_process_u16(state: &mut CrcState, value: u16) {
    state.process_u16(value);
}

pub fn crc_process_u32(state: &mut CrcState, value: u32) {
    state.process_u32(value);
}

pub fn crc_finish(state: &CrcState) -> u32 {
    state.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc_uses_borrowed_rust_state_and_slices() {
        let _: fn(&mut CrcState) = crc_init;
        let _: fn(&CrcState) -> u32 = crc_finish;
        let _: fn(&mut CrcState, &[u8]) = crc_process_bytes;
    }

    #[test]
    fn c_integer_vector_and_standard_byte_vector() {
        let mut state = CrcState::new();
        crc_init(&mut state);
        crc_process_u8(&mut state, 0x12);
        crc_process_u16(&mut state, 0x3456);
        crc_process_u32(&mut state, 0x789abcde);
        assert_eq!(crc_finish(&state), 0x3ce4f575);
        crc_init(&mut state);
        crc_process_bytes(&mut state, b"123456789");
        assert_eq!(crc_finish(&state), 0xcbf43926);
        crc_init(&mut state);
        crc_process_bytes(&mut state, &[]);
        assert_eq!(crc_finish(&state), 0);
    }

    #[test]
    fn bytes_match_incremental_little_endian_integer_updates() {
        let mut bytes = CrcState::new();
        let mut integers = CrcState::new();
        for value in [0, 1, u32::MAX, 0x89abcdef] {
            crc_process_bytes(&mut bytes, &value.to_le_bytes());
            crc_process_u32(&mut integers, value);
            assert_eq!(crc_finish(&bytes), crc_finish(&integers));
        }
        for part in b"123456789".chunks(2) {
            crc_process_bytes(&mut bytes, part);
            for &byte in part {
                crc_process_u8(&mut integers, byte);
            }
        }
        assert_eq!(crc_finish(&bytes), crc_finish(&integers));
    }
}
