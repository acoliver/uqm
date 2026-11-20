//! Remaining state.c entry points. File storage stays in `ffi`'s sole manager.

use super::ffi::*;
use std::ffi::{c_char, c_int, c_long, c_void};
use std::ptr;

/// Opaque to C. Its address identifies a file, not an independently owned buffer.
#[repr(C)]
pub struct GameStateFile {
    index: c_int,
}

static FILES: [GameStateFile; 3] = [
    GameStateFile { index: 0 },
    GameStateFile { index: 1 },
    GameStateFile { index: 2 },
];

/// # Safety
/// `mode` must be a NUL-terminated C string.
pub unsafe fn open_state_file(
    world: &mut crate::World,
    index: c_int,
    mode: *const c_char,
) -> *mut GameStateFile {
    if !(0..3).contains(&index) || rust_open_state_file(world, index, mode) == 0 {
        return ptr::null_mut();
    }
    ptr::addr_of!(FILES)
        .cast_mut()
        .cast::<GameStateFile>()
        .add(index as usize)
}

/// # Safety
/// `file` must be a handle returned by `OpenStateFile`.
pub unsafe fn close_state_file(world: &mut crate::World, file: *mut GameStateFile) {
    rust_close_state_file(world, (*file).index);
}

/// # Safety
/// Called on the game thread, without concurrent state-file access.
pub fn delete_state_file(world: &mut crate::World, index: c_int) {
    rust_delete_state_file(world, index);
}

/// # Safety
/// `file` must be a handle returned by `OpenStateFile`.
pub unsafe fn length_state_file(world: &crate::World, file: *mut GameStateFile) -> u32 {
    rust_length_state_file(world, (*file).index) as u32
}

/// # Safety
/// `buffer` must hold `size * count` writable bytes; `file` must be a live handle.
pub unsafe fn read_state_file(
    world: &mut crate::World,
    buffer: *mut c_void,
    size: u16,
    count: u16,
    file: *mut GameStateFile,
) -> c_int {
    ({
        rust_read_state_file(
            world,
            (*file).index,
            buffer.cast(),
            size as usize,
            count as usize,
        )
    }) as c_int
}

/// # Safety
/// `buffer` must hold `size * count` readable bytes; `file` must be a live handle.
pub unsafe fn write_state_file(
    world: &mut crate::World,
    buffer: *const c_void,
    size: u16,
    count: u16,
    file: *mut GameStateFile,
) -> c_int {
    ({
        rust_write_state_file(
            world,
            (*file).index,
            buffer.cast(),
            size as usize,
            count as usize,
        )
    }) as c_int
}

/// # Safety
/// `file` must be a handle returned by `OpenStateFile`.
pub unsafe fn seek_state_file(
    world: &mut crate::World,
    file: *mut GameStateFile,
    offset: c_long,
    whence: c_int,
) -> c_int {
    rust_seek_state_file(world, (*file).index, offset as _, whence)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[serial_test::serial]
    fn handles_share_the_existing_file_manager_and_preserve_io() {
        let mut game = crate::Game::new();
        // SAFETY: serial tests own the published lifecycle until this call returns.
        unsafe {
            {
                rust_init_game_state(&mut game.world);
                assert!(open_state_file(&mut game.world, -1, c"wb".as_ptr()).is_null());
                assert!(open_state_file(&mut game.world, 3, c"wb".as_ptr()).is_null());
                let file = open_state_file(&mut game.world, 1, c"wb".as_ptr());
                assert!(!file.is_null());
                let data = [11u8, 22, 33, 44];
                assert_eq!(
                    write_state_file(&mut game.world, data.as_ptr().cast(), 2, 2, file),
                    2
                );
                assert_eq!(length_state_file(&game.world, file), 4);
                assert_eq!(rust_length_state_file(&game.world, 1), 4);
                assert_eq!(seek_state_file(&mut game.world, file, -2, 2), 1);
                let mut output = [0u8; 2];
                assert_eq!(
                    read_state_file(&mut game.world, output.as_mut_ptr().cast(), 1, 2, file),
                    2
                );
                assert_eq!(output, [33, 44]);
                close_state_file(&mut game.world, file);
                assert_eq!(open_state_file(&mut game.world, 1, c"rb".as_ptr()), file);
                close_state_file(&mut game.world, file);
                delete_state_file(&mut game.world, 1);
            }
        }
    }
}
