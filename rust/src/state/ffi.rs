// FFI bindings for state management.

use std::ffi::CStr;
use std::os::raw::{c_char, c_int, c_uchar};
use std::sync::Mutex;

use super::game_state::{
    copy_state_bits_raw, get_state_32_raw, get_state_bits_raw, set_state_32_raw,
    set_state_bits_raw, GameState, NUM_GAME_STATE_BITS,
};
use super::planet_info::{PlanetInfoManager, ScanRetrieveMask, NUM_SCAN_TYPES};
use super::state_file::{FileMode, StateFileManager};

static GLOBAL_GAME_STATE: Mutex<Option<GameState>> = Mutex::new(None);
static GLOBAL_STATE_FILES: Mutex<Option<StateFileManager>> = Mutex::new(None);

#[no_mangle]
pub extern "C" fn rust_init_game_state() {
    let mut global = GLOBAL_GAME_STATE.lock().unwrap();
    if global.is_none() {
        *global = Some(GameState::new());
    }

    let mut files = GLOBAL_STATE_FILES.lock().unwrap();
    if files.is_none() {
        *files = Some(StateFileManager::new());
    }
}

#[no_mangle]
pub unsafe extern "C" fn rust_get_game_state(key: *const c_char) -> c_uchar {
    let Some((start_bit, end_bit)) = decode_state_key(key) else {
        return 0;
    };

    guard_convert_value(&GLOBAL_GAME_STATE, |state| {
        state.get_state(start_bit, end_bit)
    })
}

#[no_mangle]
pub unsafe extern "C" fn rust_set_game_state(key: *const c_char, value: c_uchar) {
    let Some((start_bit, end_bit)) = decode_state_key(key) else {
        return;
    };

    guard_convert_value_mut(&GLOBAL_GAME_STATE, |state| {
        state.set_state(start_bit, end_bit, value);
    });
}

#[no_mangle]
pub extern "C" fn rust_get_game_state_bits(start_bit: c_int, end_bit: c_int) -> c_uchar {
    if let Some((start_bit, end_bit)) = normalize_bit_range(start_bit, end_bit) {
        guard_convert_value(&GLOBAL_GAME_STATE, |state| {
            state.get_state(start_bit, end_bit)
        })
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn rust_set_game_state_bits(start_bit: c_int, end_bit: c_int, value: c_uchar) {
    if let Some((start_bit, end_bit)) = normalize_bit_range(start_bit, end_bit) {
        guard_convert_value_mut(&GLOBAL_GAME_STATE, |state| {
            state.set_state(start_bit, end_bit, value);
        });
    }
}

#[no_mangle]
pub extern "C" fn rust_get_game_state_32(start_bit: c_int) -> u32 {
    let Some(start_bit) = normalize_start_bit_32(start_bit) else {
        return 0;
    };

    guard_convert_value(&GLOBAL_GAME_STATE, |state| state.get_state_32(start_bit))
}

#[no_mangle]
pub extern "C" fn rust_set_game_state_32(start_bit: c_int, value: u32) {
    let Some(start_bit) = normalize_start_bit_32(start_bit) else {
        return;
    };

    guard_convert_value_mut(&GLOBAL_GAME_STATE, |state| {
        state.set_state_32(start_bit, value);
    });
}

#[no_mangle]
pub extern "C" fn rust_copy_game_state(dest_bit: c_int, src_start_bit: c_int, src_end_bit: c_int) {
    let Some((dest_bit, src_start_bit, src_end_bit)) =
        normalize_copy_bits(dest_bit, src_start_bit, src_end_bit)
    else {
        return;
    };

    let mut guard = match GLOBAL_GAME_STATE.lock() {
        Ok(g) => g,
        Err(_) => return,
    };

    if let Some(state) = guard.as_mut() {
        let snapshot = GameState::from_bytes(state.as_bytes());
        state.copy_state(dest_bit, &snapshot, src_start_bit, src_end_bit);
    }
}

#[no_mangle]
pub extern "C" fn rust_reset_game_state() {
    guard_convert_value_mut(&GLOBAL_GAME_STATE, |state| {
        state.reset();
    });
}

#[no_mangle]
pub unsafe extern "C" fn rust_open_state_file(file_index: c_int, mode: *const c_char) -> c_int {
    if mode.is_null() {
        return 0;
    }

    let c_str = CStr::from_ptr(mode);
    let mode_str = match c_str.to_str() {
        Ok(s) => s,
        Err(_) => return 0,
    };

    let file_mode = match mode_str {
        "rb" => FileMode::Read,
        "wb" => FileMode::Write,
        "r+b" => FileMode::ReadWrite,
        _ => return 0,
    };

    match guard_convert_state_result_mut(&GLOBAL_STATE_FILES, |files| {
        files.open(file_index as usize, file_mode).map(|_| ())
    }) {
        Some(()) => 1,
        _ => 0,
    }
}

#[no_mangle]
pub extern "C" fn rust_close_state_file(file_index: c_int) {
    guard_convert_state_value_mut(&GLOBAL_STATE_FILES, |files| {
        let _ = files.close(file_index as usize);
    });
}

#[no_mangle]
pub extern "C" fn rust_delete_state_file(file_index: c_int) {
    guard_convert_state_value_mut(&GLOBAL_STATE_FILES, |files| {
        let _ = files.delete(file_index as usize);
    });
}

#[no_mangle]
pub extern "C" fn rust_length_state_file(file_index: c_int) -> usize {
    guard_convert_state_value(&GLOBAL_STATE_FILES, |files| {
        files
            .get_file(file_index as usize)
            .map(|f| f.length())
            .unwrap_or(0)
    })
}

#[no_mangle]
pub unsafe extern "C" fn rust_read_state_file(
    file_index: c_int,
    buf: *mut u8,
    size: usize,
    count: usize,
) -> usize {
    if buf.is_null() || size == 0 {
        return 0;
    }

    let bytes = size * count;
    let slice = std::slice::from_raw_parts_mut(buf, bytes);

    guard_convert_state_value_mut(&GLOBAL_STATE_FILES, |files| {
        files
            .get_file_mut(file_index as usize)
            .map(|f| f.read(slice).unwrap_or(0) / size)
            .unwrap_or(0)
    })
}

#[no_mangle]
pub unsafe extern "C" fn rust_write_state_file(
    file_index: c_int,
    buf: *const u8,
    size: usize,
    count: usize,
) -> usize {
    if buf.is_null() || size == 0 {
        return 0;
    }

    let bytes = size * count;
    let slice = std::slice::from_raw_parts(buf, bytes);

    guard_convert_state_value_mut(&GLOBAL_STATE_FILES, |files| {
        match files.get_file_mut(file_index as usize) {
            Some(file) => match file.write(slice) {
                Ok(()) => count,
                Err(_) => 0,
            },
            None => 0,
        }
    })
}

#[no_mangle]
pub extern "C" fn rust_seek_state_file(file_index: c_int, offset: i64, whence: c_int) -> c_int {
    use super::state_file::SeekWhence;

    let seek_whence = match whence {
        0 => SeekWhence::Set,
        1 => SeekWhence::Current,
        2 => SeekWhence::End,
        _ => return 0,
    };

    guard_convert_state_value_mut(&GLOBAL_STATE_FILES, |state_files| {
        match state_files.get_file_mut(file_index as usize) {
            Some(file) => match file.seek(offset, seek_whence) {
                Ok(()) => 1,
                Err(_) => 0,
            },
            None => 0,
        }
    })
}

#[no_mangle]
pub extern "C" fn rust_get_game_state_bytes() -> *const u8 {
    guard_convert_value(&GLOBAL_GAME_STATE, |state| state.as_bytes().as_ptr())
}

#[no_mangle]
pub extern "C" fn rust_get_game_state_size() -> usize {
    super::game_state::NUM_GAME_STATE_BYTES
}

#[no_mangle]
pub unsafe extern "C" fn rust_restore_game_state_from_bytes(bytes: *const u8, size: usize) {
    use super::game_state::NUM_GAME_STATE_BYTES;

    if bytes.is_null() || size < NUM_GAME_STATE_BYTES {
        return;
    }

    let mut arr = [0u8; NUM_GAME_STATE_BYTES];
    std::ptr::copy_nonoverlapping(bytes, arr.as_mut_ptr(), NUM_GAME_STATE_BYTES);

    let mut global = GLOBAL_GAME_STATE.lock().unwrap();
    if global.is_none() {
        *global = Some(GameState::new());
    }
    if let Some(state) = global.as_mut() {
        *state = GameState::from_bytes(&arr);
    }
}

#[no_mangle]
pub unsafe extern "C" fn rust_get_game_state_bits_from_bytes(
    bytes: *const u8,
    start_bit: c_int,
    end_bit: c_int,
) -> c_uchar {
    let Some((start_bit, end_bit)) = normalize_bit_range(start_bit, end_bit) else {
        return 0;
    };
    let Some(state) = normalize_state_bytes(bytes, NUM_GAME_STATE_BITS) else {
        return 0;
    };

    get_state_bits_raw(state, start_bit, end_bit)
}

#[no_mangle]
pub unsafe extern "C" fn rust_set_game_state_bits_in_bytes(
    bytes: *mut u8,
    start_bit: c_int,
    end_bit: c_int,
    value: c_uchar,
) {
    let Some((start_bit, end_bit)) = normalize_bit_range(start_bit, end_bit) else {
        return;
    };
    let Some(state) = normalize_state_bytes_mut(bytes, NUM_GAME_STATE_BITS) else {
        return;
    };

    set_state_bits_raw(state, start_bit, end_bit, value);
}

#[no_mangle]
pub unsafe extern "C" fn rust_get_game_state32_from_bytes(
    bytes: *const u8,
    start_bit: c_int,
) -> u32 {
    let Some(start_bit) = normalize_start_bit_32(start_bit) else {
        return 0;
    };
    let Some(state) = normalize_state_bytes(bytes, NUM_GAME_STATE_BITS) else {
        return 0;
    };

    get_state_32_raw(state, start_bit)
}

#[no_mangle]
pub unsafe extern "C" fn rust_set_game_state32_in_bytes(
    bytes: *mut u8,
    start_bit: c_int,
    value: u32,
) {
    let Some(start_bit) = normalize_start_bit_32(start_bit) else {
        return;
    };
    let Some(state) = normalize_state_bytes_mut(bytes, NUM_GAME_STATE_BITS) else {
        return;
    };

    set_state_32_raw(state, start_bit, value);
}

#[no_mangle]
pub unsafe extern "C" fn rust_copy_game_state_bits_between_bytes(
    dest: *mut u8,
    target: c_int,
    src: *const u8,
    begin: c_int,
    end: c_int,
) {
    let Some((target, begin, end)) = normalize_copy_bits(target, begin, end) else {
        return;
    };
    let Some(dest_state) = normalize_state_bytes_mut(dest, NUM_GAME_STATE_BITS) else {
        return;
    };
    let Some(src_state) = normalize_state_bytes(src, NUM_GAME_STATE_BITS) else {
        return;
    };

    copy_state_bits_raw(dest_state, target, src_state, begin, end);
}

#[no_mangle]
pub unsafe extern "C" fn rust_init_planet_info(num_stars: c_int) -> c_int {
    if num_stars < 0 {
        return 0;
    }

    match guard_convert_state_result_mut(&GLOBAL_STATE_FILES, |files| {
        let mut manager = PlanetInfoManager::new(files);
        manager.init_planet_info(num_stars as usize)
    }) {
        Some(()) => 1,
        None => 0,
    }
}

#[no_mangle]
pub extern "C" fn rust_uninit_planet_info() {
    guard_convert_state_value_mut(&GLOBAL_STATE_FILES, |files| {
        let mut manager = PlanetInfoManager::new(files);
        let _ = manager.uninit_planet_info();
    });
}

#[no_mangle]
pub unsafe extern "C" fn rust_get_planet_info(
    star_index: c_int,
    planet_index: c_int,
    moon_index: c_int,
    planet_num_moons: *const u8,
    num_planets: c_int,
    out_mask: *mut u32,
) -> c_int {
    if out_mask.is_null() || planet_num_moons.is_null() || num_planets < 0 {
        return 0;
    }
    if star_index < 0 || planet_index < 0 || moon_index < 0 {
        return 0;
    }

    let num_planets = num_planets as usize;
    let planet_num_moons = std::slice::from_raw_parts(planet_num_moons, num_planets);
    let out_mask = std::slice::from_raw_parts_mut(out_mask, NUM_SCAN_TYPES);

    match guard_convert_state_result_mut(&GLOBAL_STATE_FILES, |files| {
        let mut manager = PlanetInfoManager::new(files);
        manager.get_planet_info(
            star_index as usize,
            planet_index as usize,
            moon_index as usize,
            planet_num_moons,
        )
    }) {
        Some(mask) => {
            out_mask.copy_from_slice(&mask.to_array());
            1
        }
        None => {
            out_mask.fill(0);
            0
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn rust_put_planet_info(
    star_index: c_int,
    planet_index: c_int,
    moon_index: c_int,
    planet_num_moons: *const u8,
    num_planets: c_int,
    mask: *const u32,
) -> c_int {
    if planet_num_moons.is_null() || mask.is_null() || num_planets < 0 {
        return 0;
    }
    if star_index < 0 || planet_index < 0 || moon_index < 0 {
        return 0;
    }

    let num_planets = num_planets as usize;
    let planet_num_moons = std::slice::from_raw_parts(planet_num_moons, num_planets);
    let mask_values = std::slice::from_raw_parts(mask, NUM_SCAN_TYPES);
    let mask = ScanRetrieveMask::from_array(&[mask_values[0], mask_values[1], mask_values[2]]);

    match guard_convert_state_result_mut(&GLOBAL_STATE_FILES, |files| {
        let mut manager = PlanetInfoManager::new(files);
        manager.put_planet_info(
            star_index as usize,
            planet_index as usize,
            moon_index as usize,
            &mask,
            planet_num_moons,
        )
    }) {
        Some(()) => 1,
        None => 0,
    }
}

unsafe fn decode_state_key(key: *const c_char) -> Option<(usize, usize)> {
    if key.is_null() {
        return None;
    }

    let c_str = CStr::from_ptr(key);
    let key_str = c_str.to_str().ok()?;
    GameState::lookup_bits(key_str)
}

fn normalize_bit_range(start_bit: c_int, end_bit: c_int) -> Option<(usize, usize)> {
    if start_bit < 0 || end_bit < 0 || end_bit < start_bit {
        return None;
    }

    let start_bit = start_bit as usize;
    let end_bit = end_bit as usize;
    if end_bit >= super::game_state::NUM_GAME_STATE_BITS || end_bit - start_bit >= 8 {
        return None;
    }

    Some((start_bit, end_bit))
}

fn normalize_start_bit_32(start_bit: c_int) -> Option<usize> {
    if start_bit < 0 {
        return None;
    }

    let start_bit = start_bit as usize;
    if start_bit + 31 >= super::game_state::NUM_GAME_STATE_BITS {
        return None;
    }

    Some(start_bit)
}

fn normalize_copy_bits(
    dest_bit: c_int,
    src_start_bit: c_int,
    src_end_bit: c_int,
) -> Option<(usize, usize, usize)> {
    if dest_bit < 0 || src_start_bit < 0 || src_end_bit < 0 || src_end_bit < src_start_bit {
        return None;
    }

    let dest_bit = dest_bit as usize;
    let src_start_bit = src_start_bit as usize;
    let src_end_bit = src_end_bit as usize;
    let width = src_end_bit - src_start_bit;

    if src_end_bit >= super::game_state::NUM_GAME_STATE_BITS {
        return None;
    }
    if dest_bit + width >= super::game_state::NUM_GAME_STATE_BITS {
        return None;
    }

    Some((dest_bit, src_start_bit, src_end_bit))
}

unsafe fn normalize_state_bytes<'a>(bytes: *const u8, bit_count: usize) -> Option<&'a [u8]> {
    if bytes.is_null() {
        return None;
    }

    let byte_count = (bit_count + 7) >> 3;
    Some(std::slice::from_raw_parts(bytes, byte_count))
}

unsafe fn normalize_state_bytes_mut<'a>(bytes: *mut u8, bit_count: usize) -> Option<&'a mut [u8]> {
    if bytes.is_null() {
        return None;
    }

    let byte_count = (bit_count + 7) >> 3;
    Some(std::slice::from_raw_parts_mut(bytes, byte_count))
}

fn guard_convert_value<R, F>(mutex: &Mutex<Option<GameState>>, f: F) -> R
where
    F: FnOnce(&GameState) -> R,
    R: Default,
{
    match mutex.lock() {
        Ok(guard) => match guard.as_ref() {
            Some(state) => f(state),
            None => R::default(),
        },
        Err(_) => R::default(),
    }
}

fn guard_convert_value_mut<R, F>(mutex: &Mutex<Option<GameState>>, f: F) -> R
where
    F: FnOnce(&mut GameState) -> R,
    R: Default,
{
    match mutex.lock() {
        Ok(mut guard) => match guard.as_mut() {
            Some(state) => f(state),
            None => R::default(),
        },
        Err(_) => R::default(),
    }
}

fn ensure_state_files_init(guard: &mut Option<StateFileManager>) {
    if guard.is_none() {
        *guard = Some(StateFileManager::new());
    }
}

fn guard_convert_state_value<R, F>(mutex: &Mutex<Option<StateFileManager>>, f: F) -> R
where
    F: FnOnce(&StateFileManager) -> R,
    R: Default,
{
    match mutex.lock() {
        Ok(mut guard) => {
            ensure_state_files_init(&mut guard);
            match guard.as_ref() {
                Some(state) => f(state),
                None => R::default(),
            }
        }
        Err(_) => R::default(),
    }
}

fn guard_convert_state_value_mut<R, F>(mutex: &Mutex<Option<StateFileManager>>, f: F) -> R
where
    F: FnOnce(&mut StateFileManager) -> R,
    R: Default,
{
    match mutex.lock() {
        Ok(mut guard) => {
            ensure_state_files_init(&mut guard);
            match guard.as_mut() {
                Some(state) => f(state),
                None => R::default(),
            }
        }
        Err(_) => R::default(),
    }
}

fn guard_convert_state_result_mut<R, E, F>(
    mutex: &Mutex<Option<StateFileManager>>,
    f: F,
) -> Option<R>
where
    F: FnOnce(&mut StateFileManager) -> Result<R, E>,
{
    match mutex.lock() {
        Ok(mut guard) => {
            ensure_state_files_init(&mut guard);
            match guard.as_mut() {
                Some(state) => f(state).ok(),
                None => None,
            }
        }
        Err(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    #[serial]
    fn test_rust_init_game_state() {
        rust_init_game_state();

        let guard = GLOBAL_GAME_STATE.lock().unwrap();
        assert!(guard.is_some());
    }

    #[test]
    #[serial]
    fn test_named_state_lookup_uses_generated_ranges() {
        rust_init_game_state();

        unsafe {
            rust_set_game_state(b"SHOFIXTI_VISITS\0".as_ptr() as *const c_char, 5);
            rust_set_game_state(b"SHOFIXTI_RECRUITED\0".as_ptr() as *const c_char, 1);
            rust_set_game_state(b"SPATHI_VISITS\0".as_ptr() as *const c_char, 3);

            assert_eq!(
                rust_get_game_state(b"SHOFIXTI_VISITS\0".as_ptr() as *const c_char),
                5
            );
            assert_eq!(
                rust_get_game_state(b"SHOFIXTI_RECRUITED\0".as_ptr() as *const c_char),
                1
            );
            assert_eq!(
                rust_get_game_state(b"SPATHI_VISITS\0".as_ptr() as *const c_char),
                3
            );
            assert_eq!(
                rust_get_game_state(b"NOT_A_REAL_STATE\0".as_ptr() as *const c_char),
                0
            );
        }
    }

    #[test]
    #[serial]
    fn test_rust_get_set_game_state_bits() {
        rust_init_game_state();
        rust_set_game_state_bits(0, 2, 5);
        let result = rust_get_game_state_bits(0, 2);
        assert_eq!(result, 5);
    }

    #[test]
    #[serial]
    fn test_rust_get_set_game_state_32() {
        rust_init_game_state();

        let test_value = 0xDEADBEEF;
        rust_set_game_state_32(0, test_value);

        let result = rust_get_game_state_32(0);
        assert_eq!(result, test_value);
    }

    #[test]
    #[serial]
    fn test_raw_byte_buffer_ffi_access() {
        let mut bytes = [0u8; super::super::game_state::NUM_GAME_STATE_BYTES];

        unsafe {
            rust_set_game_state_bits_in_bytes(bytes.as_mut_ptr(), 0, 2, 5);
            rust_set_game_state_bits_in_bytes(bytes.as_mut_ptr(), 12, 12, 1);
            rust_set_game_state32_in_bytes(bytes.as_mut_ptr(), 32, 0xCAFEBABE);

            assert_eq!(rust_get_game_state_bits_from_bytes(bytes.as_ptr(), 0, 2), 5);
            assert_eq!(
                rust_get_game_state_bits_from_bytes(bytes.as_ptr(), 12, 12),
                1
            );
            assert_eq!(
                rust_get_game_state32_from_bytes(bytes.as_ptr(), 32),
                0xCAFEBABE
            );
        }
    }

    #[test]
    #[serial]
    fn test_raw_byte_buffer_copy_matches_c_semantics() {
        let mut src = [0u8; super::super::game_state::NUM_GAME_STATE_BYTES];
        let mut dest = [0u8; super::super::game_state::NUM_GAME_STATE_BYTES];

        unsafe {
            rust_set_game_state_bits_in_bytes(src.as_mut_ptr(), 0, 7, 0xAB);
            rust_set_game_state_bits_in_bytes(src.as_mut_ptr(), 8, 15, 0xCD);

            rust_copy_game_state_bits_between_bytes(dest.as_mut_ptr(), 32, src.as_ptr(), 0, 15);
            rust_copy_game_state_bits_between_bytes(dest.as_mut_ptr(), 80, src.as_ptr(), 0, 0);

            assert_eq!(
                rust_get_game_state_bits_from_bytes(dest.as_ptr(), 32, 39),
                0xAB
            );
            assert_eq!(
                rust_get_game_state_bits_from_bytes(dest.as_ptr(), 40, 47),
                0xCD
            );
            assert_eq!(
                rust_get_game_state_bits_from_bytes(dest.as_ptr(), 80, 80),
                0
            );
        }
    }

    #[test]
    #[serial]
    fn test_rust_reset_game_state() {
        rust_init_game_state();

        rust_set_game_state_bits(0, 7, 0xFF);
        rust_reset_game_state();

        let result = rust_get_game_state_bits(0, 7);
        assert_eq!(result, 0);
    }

    #[test]
    #[serial]
    fn test_rust_planet_info_round_trip() {
        rust_init_game_state();

        let moon_counts = [2u8, 0u8];
        let input_mask = [0x11u32, 0x22u32, 0x33u32];
        let mut output_mask = [0u32; NUM_SCAN_TYPES];

        unsafe {
            assert_eq!(rust_init_planet_info(8), 1);
            assert_eq!(
                rust_put_planet_info(
                    1,
                    0,
                    1,
                    moon_counts.as_ptr(),
                    moon_counts.len() as c_int,
                    input_mask.as_ptr()
                ),
                1
            );
            assert_eq!(
                rust_get_planet_info(
                    1,
                    0,
                    1,
                    moon_counts.as_ptr(),
                    moon_counts.len() as c_int,
                    output_mask.as_mut_ptr(),
                ),
                1
            );
        }

        assert_eq!(output_mask, input_mask);
    }

    #[test]
    #[serial]
    fn test_rust_open_state_file() {
        rust_init_game_state();

        unsafe {
            let result = rust_open_state_file(0, b"wb\0".as_ptr() as *const c_char);
            assert_eq!(result, 1);
        }
    }

    #[test]
    #[serial]
    fn test_rust_write_read_state_file() {
        rust_init_game_state();

        let test_data = b"Hello, World!";

        unsafe {
            rust_open_state_file(0, b"wb\0".as_ptr() as *const c_char);
            let written = rust_write_state_file(0, test_data.as_ptr(), 1, test_data.len());
            assert_eq!(written, test_data.len());

            rust_seek_state_file(0, 0, 0);

            let mut buf = vec![0u8; test_data.len()];
            let read = rust_read_state_file(0, buf.as_mut_ptr(), 1, test_data.len());
            assert_eq!(read, test_data.len());
            assert_eq!(&buf, test_data);
        }
    }

    #[test]
    #[serial]
    fn test_rust_length_state_file() {
        rust_init_game_state();

        let test_data = b"Test";

        unsafe {
            rust_open_state_file(1, b"wb\0".as_ptr() as *const c_char);
            rust_write_state_file(1, test_data.as_ptr(), 1, test_data.len());
        }

        let length = rust_length_state_file(1);
        assert_eq!(length, test_data.len());
    }

    #[test]
    #[serial]
    fn test_rust_delete_state_file() {
        rust_init_game_state();

        let test_data = b"Test";

        unsafe {
            rust_open_state_file(2, b"wb\0".as_ptr() as *const c_char);
            rust_write_state_file(2, test_data.as_ptr(), 1, test_data.len());
        }

        rust_delete_state_file(2);
        let length = rust_length_state_file(2);
        assert_eq!(length, 0);
    }

    #[test]
    #[serial]
    fn test_rust_seek_state_file() {
        rust_init_game_state();

        let test_data = b"HelloWorld";

        unsafe {
            rust_open_state_file(0, b"wb\0".as_ptr() as *const c_char);
            rust_write_state_file(0, test_data.as_ptr(), 1, test_data.len());
        }

        let result = rust_seek_state_file(0, 5, 0);
        assert_eq!(result, 1);

        let mut buf = vec![0u8; 5];
        unsafe {
            let read = rust_read_state_file(0, buf.as_mut_ptr(), 1, 5);
            assert_eq!(read, 5);
        }
        assert_eq!(&buf, b"World");
    }

    #[test]
    #[serial]
    fn test_rust_get_game_state_bytes() {
        rust_init_game_state();

        let test_value: u8 = 0xAB;
        rust_set_game_state_bits(0, 7, test_value);

        let ptr = rust_get_game_state_bytes();
        assert!(!ptr.is_null());

        unsafe {
            assert_eq!(*ptr, test_value);
        }
    }

    #[test]
    #[serial]
    fn test_rust_get_game_state_size() {
        let size = rust_get_game_state_size();
        assert_eq!(size, super::super::game_state::NUM_GAME_STATE_BYTES);
    }

    #[test]
    #[serial]
    fn test_rust_restore_game_state_from_bytes() {
        rust_init_game_state();

        rust_set_game_state_bits(0, 7, 0xAB);
        let ptr = rust_get_game_state_bytes();
        let size = rust_get_game_state_size();

        let mut buffer = vec![0u8; size];
        unsafe {
            std::ptr::copy_nonoverlapping(ptr, buffer.as_mut_ptr(), size);
        }

        rust_reset_game_state();
        rust_init_game_state();

        assert_eq!(rust_get_game_state_bits(0, 7), 0);

        unsafe {
            rust_restore_game_state_from_bytes(buffer.as_ptr(), size);
        }

        assert_eq!(rust_get_game_state_bits(0, 7), 0xAB);
    }
}
