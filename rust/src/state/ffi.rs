// FFI bindings for State Management module
// Provides C-compatible interface for game state access

use std::ffi::CStr;
use std::os::raw::{c_char, c_int, c_uchar};
use std::sync::Mutex;

use super::game_state::GameState;
use super::state_file::{FileMode, StateFileManager};

/// Static game state instance (thread-safe)
static GLOBAL_GAME_STATE: Mutex<Option<GameState>> = Mutex::new(None);
static GLOBAL_STATE_FILES: Mutex<Option<StateFileManager>> = Mutex::new(None);

/// Initialize the global game state
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

/// Get a game state value by key name
///
/// # Safety
///
/// - `key` must be either null or a valid null-terminated C string
/// - The memory referenced by `key` must not be modified for the duration of this call
#[no_mangle]
pub unsafe extern "C" fn rust_get_game_state(key: *const c_char) -> c_uchar {
    if key.is_null() {
        return 0;
    }

    let c_str = CStr::from_ptr(key);
    let key_str = match c_str.to_str() {
        Ok(s) => s,
        Err(_) => return 0,
    };

    let guard = GLOBAL_GAME_STATE.lock().unwrap();
    if let Some(state) = guard.as_ref() {
        // Parse key to get bit range (format: "NAME_start_end")
        // For now, we'll use a simple mapping based on known keys
        // In a full implementation, this would parse the key name
        // and look up the bit range from a table
        match key_str {
            "SHOFIXTI_VISITS" => state.get_state(0, 2),
            "SHOFIXTI_RECRUITED" => state.get_state(12, 12),
            "PATHI_VISITS" => state.get_state(15, 17),
            _ => {
                // For unknown keys, return 0
                // In production, we'd have a comprehensive mapping
                0
            }
        }
    } else {
        0
    }
}

/// Set a game state value by key name
///
/// # Safety
///
/// - `key` must be either null or a valid null-terminated C string
/// - The memory referenced by `key` must not be modified for the duration of this call
#[no_mangle]
pub unsafe extern "C" fn rust_set_game_state(key: *const c_char, value: c_uchar) {
    if key.is_null() {
        return;
    }

    let c_str = CStr::from_ptr(key);
    let key_str = match c_str.to_str() {
        Ok(s) => s,
        Err(_) => return,
    };

    let mut guard = GLOBAL_GAME_STATE.lock().unwrap();
    if let Some(state) = guard.as_mut() {
        // Parse key to get bit range
        match key_str {
            "SHOFIXTI_VISITS" => state.set_state(0, 2, value),
            "SHOFIXTI_RECRUITED" => state.set_state(12, 12, value),
            "SPATHI_VISITS" => state.set_state(15, 17, value),
            _ => {
                // For unknown keys, do nothing
                // In production, we'd have a comprehensive mapping
            }
        }
    }
}

/// Get game state value for a direct bit range
#[no_mangle]
pub extern "C" fn rust_get_game_state_bits(start_bit: c_int, end_bit: c_int) -> c_uchar {
    guard_convert_value(&GLOBAL_GAME_STATE, |state| {
        state.get_state(start_bit as usize, end_bit as usize)
    })
}

/// Set game state value for a direct bit range
#[no_mangle]
pub extern "C" fn rust_set_game_state_bits(start_bit: c_int, end_bit: c_int, value: c_uchar) {
    guard_convert_value_mut(&GLOBAL_GAME_STATE, |state| {
        state.set_state(start_bit as usize, end_bit as usize, value);
    });
}

/// Get a 32-bit game state value starting at the given bit
#[no_mangle]
pub extern "C" fn rust_get_game_state_32(start_bit: c_int) -> u32 {
    guard_convert_value(&GLOBAL_GAME_STATE, |state| {
        state.get_state_32(start_bit as usize)
    })
}

/// Set a 32-bit game state value starting at the given bit
#[no_mangle]
pub extern "C" fn rust_set_game_state_32(start_bit: c_int, value: u32) {
    guard_convert_value_mut(&GLOBAL_GAME_STATE, |state| {
        state.set_state_32(start_bit as usize, value);
    });
}

/// Copy game state bits from source to destination
///
/// Acquires the mutex once, snapshots source bytes, then copies.
/// Previous implementation deadlocked by acquiring the same mutex twice.
#[no_mangle]
pub extern "C" fn rust_copy_game_state(dest_bit: c_int, src_start_bit: c_int, src_end_bit: c_int) {
    let mut guard = match GLOBAL_GAME_STATE.lock() {
        Ok(g) => g,
        Err(_) => return,
    };
    if let Some(state) = guard.as_mut() {
        let snapshot = GameState::from_bytes(state.as_bytes());
        state.copy_state(
            dest_bit as usize,
            &snapshot,
            src_start_bit as usize,
            src_end_bit as usize,
        );
    }
}

/// Reset all game state to zero
#[no_mangle]
pub extern "C" fn rust_reset_game_state() {
    guard_convert_value_mut(&GLOBAL_GAME_STATE, |state| {
        state.reset();
    });
}

/// Open a state file
///
/// # Safety
///
/// - `mode` must be either null or a valid null-terminated C string
/// - The memory referenced by `mode` must not be modified for the duration of this call
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

/// Close a state file
#[no_mangle]
pub extern "C" fn rust_close_state_file(file_index: c_int) {
    guard_convert_state_value_mut(&GLOBAL_STATE_FILES, |files| {
        let _ = files.close(file_index as usize);
    });
}

/// Delete a state file
#[no_mangle]
pub extern "C" fn rust_delete_state_file(file_index: c_int) {
    guard_convert_state_value_mut(&GLOBAL_STATE_FILES, |files| {
        let _ = files.delete(file_index as usize);
    });
}

/// Get the length of a state file
#[no_mangle]
pub extern "C" fn rust_length_state_file(file_index: c_int) -> usize {
    guard_convert_state_value(&GLOBAL_STATE_FILES, |files| {
        files
            .get_file(file_index as usize)
            .map(|f| f.length())
            .unwrap_or(0)
    })
}

/// Read from a state file
///
/// # Safety
///
/// - `buf` must be either null or a valid pointer to writable memory
/// - `buf` must point to at least `size * count` bytes if not null
/// - The memory referenced by `buf` must not be modified by other threads during this call
///
/// # Returns
/// Number of items read
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

/// Write to a state file
///
/// # Safety
///
/// - `buf` must be either null or a valid pointer to readable memory
/// - `buf` must point to at least `size * count` bytes if not null
/// - The memory referenced by `buf` must not be modified during this call
///
/// # Returns
/// Number of items written
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

/// Seek in a state file
///
/// # Returns
/// 1 on success, 0 on failure
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

/// Get game state bytes pointer (for serialization)
#[no_mangle]
pub extern "C" fn rust_get_game_state_bytes() -> *const u8 {
    guard_convert_value(&GLOBAL_GAME_STATE, |state| state.as_bytes().as_ptr())
}

/// Get game state bytes size
#[no_mangle]
pub extern "C" fn rust_get_game_state_size() -> usize {
    use super::game_state::NUM_GAME_STATE_BYTES;
    NUM_GAME_STATE_BYTES
}

/// Restore game state from bytes
///
/// # Safety
///
/// - `bytes` must be either null or a valid pointer to readable memory
/// - If not null, `bytes` must point to at least NUM_GAME_STATE_BYTES bytes
/// - The memory referenced by `bytes` must not be modified during this call
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

// Helper functions for safe FFI operations

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

/// Ensure the state file manager is initialized (auto-init on first use).
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

    #[test]
    fn test_rust_init_game_state() {
        rust_init_game_state();

        let guard = GLOBAL_GAME_STATE.lock().unwrap();
        assert!(guard.is_some());
    }

    #[test]
    fn test_rust_get_set_game_state_bits() {
        rust_init_game_state();

        // Set bits 0-2
        rust_set_game_state_bits(0, 2, 5);

        // Get them back
        let result = rust_get_game_state_bits(0, 2);
        assert_eq!(result, 5);
    }

    #[test]
    fn test_rust_get_set_game_state_32() {
        rust_init_game_state();

        let test_value = 0xDEADBEEF;
        rust_set_game_state_32(0, test_value);

        let result = rust_get_game_state_32(0);
        assert_eq!(result, test_value);
    }

    #[test]
    fn test_rust_reset_game_state() {
        rust_init_game_state();

        rust_set_game_state_bits(0, 7, 0xFF);
        rust_reset_game_state();

        let result = rust_get_game_state_bits(0, 7);
        assert_eq!(result, 0);
    }

    #[test]
    fn test_rust_open_state_file() {
        rust_init_game_state();

        unsafe {
            let result = rust_open_state_file(0, b"wb\0".as_ptr() as *const c_char);
            assert_eq!(result, 1);
        }
    }

    #[test]
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
    fn test_rust_seek_state_file() {
        rust_init_game_state();

        let test_data = b"HelloWorld";

        unsafe {
            rust_open_state_file(0, b"wb\0".as_ptr() as *const c_char);
            rust_write_state_file(0, test_data.as_ptr(), 1, test_data.len());
        }

        // Seek to byte 5
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
    fn test_rust_get_game_state_bytes() {
        rust_init_game_state();

        // Set value using a u8 literal to match the expected type
        let test_value: u8 = 0xAB;
        rust_set_game_state_bits(0, 7, test_value);

        let ptr = rust_get_game_state_bytes();
        assert!(!ptr.is_null());

        unsafe {
            assert_eq!(*ptr, test_value);
        }
    }

    #[test]
    fn test_rust_get_game_state_size() {
        let size = rust_get_game_state_size();
        assert!(size > 0);
    }

    #[test]
    fn test_rust_restore_game_state_from_bytes() {
        rust_init_game_state();

        rust_set_game_state_bits(0, 7, 0xAB);
        let ptr = rust_get_game_state_bytes();
        let size = rust_get_game_state_size();

        // Copy the data before resetting
        let mut buffer = vec![0u8; size];
        unsafe {
            std::ptr::copy_nonoverlapping(ptr, buffer.as_mut_ptr(), size);
        }

        // Reset
        rust_reset_game_state();

        // Re-initialize to ensure state exists
        rust_init_game_state();

        assert_eq!(rust_get_game_state_bits(0, 7), 0);

        // Restore from copied data
        unsafe {
            rust_restore_game_state_from_bytes(buffer.as_ptr(), size);
        }

        // Verify restored
        assert_eq!(rust_get_game_state_bits(0, 7), 0xAB);
    }
}
