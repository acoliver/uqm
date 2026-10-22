use super::abi::crc_State;

const fn table() -> [u32; 256] {
    let mut result = [0; 256];
    let mut i = 0;
    while i < 256 {
        let mut crc = i as u32;
        let mut bit = 0;
        while bit < 8 {
            crc = (crc >> 1) ^ if crc & 1 != 0 { 0xedb88320 } else { 0 };
            bit += 1;
        }
        result[i] = crc;
        i += 1;
    }
    result
}

#[no_mangle]
pub static crcTable: [u32; 256] = table();

fn process(state: &mut crc_State, bytes: &[u8]) {
    for &byte in bytes {
        state.crc = (state.crc >> 8) ^ crcTable[((state.crc ^ u32::from(byte)) & 0xff) as usize];
    }
}

/// # Safety
/// `state` must point to a writable CRC state.
#[no_mangle]
pub unsafe extern "C" fn crc_init(state: *mut crc_State) {
    (*state).crc = u32::MAX;
}

/// # Safety
/// `state` must be writable and `buf` must contain `len` readable bytes.
#[no_mangle]
pub unsafe extern "C" fn crc_processBytes(state: *mut crc_State, buf: *mut u8, len: usize) {
    // The old, unused C byte loop omitted its pointer increment. Integer CRC
    // updates below already consumed each byte once, in little-endian order.
    for index in 0..len {
        process(&mut *state, &[*buf.add(index)]);
    }
}

macro_rules! integer {
    ($name:ident, $ty:ty) => {
        /// # Safety
        /// `state` must point to a writable CRC state.
        #[no_mangle]
        pub unsafe extern "C" fn $name(state: *mut crc_State, value: $ty) {
            process(&mut *state, &value.to_le_bytes());
        }
    };
}
integer!(crc_processUint8, u8);
integer!(crc_processUint16, u16);
integer!(crc_processUint32, u32);

/// # Safety
/// `state` must point to an initialized CRC state.
#[no_mangle]
pub unsafe extern "C" fn crc_finish(state: *const crc_State) -> u32 {
    !(*state).crc
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn c_integer_vector_and_standard_byte_vector() {
        unsafe {
            let mut state = crc_State { crc: 0 };
            crc_init(&mut state);
            crc_processUint8(&mut state, 0x12);
            crc_processUint16(&mut state, 0x3456);
            crc_processUint32(&mut state, 0x789abcde);
            assert_eq!(crc_finish(&state), 0x3ce4f575);
            crc_init(&mut state);
            crc_processBytes(&mut state, b"123456789".as_ptr().cast_mut(), 9);
            assert_eq!(crc_finish(&state), 0xcbf43926);
            crc_init(&mut state);
            crc_processBytes(&mut state, std::ptr::null_mut(), 0);
            assert_eq!(crc_finish(&state), 0);
        }
    }
}
