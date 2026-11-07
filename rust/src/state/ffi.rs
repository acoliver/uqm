#[cfg(test)]
use crate::engine::World;
// FFI bindings for state management.

use std::ffi::CStr;
use std::os::raw::{c_char, c_int, c_uchar};

use super::game_state::{
    copy_state_bits_raw, get_state_32_raw, get_state_bits_raw, set_state_32_raw,
    set_state_bits_raw, StoryState, NUM_GAME_STATE_BITS, NUM_GAME_STATE_BYTES,
};
use crate::engine::world::planet_info::{PlanetInfoManager, ScanRetrieveMask, NUM_SCAN_TYPES};
use crate::engine::world::state_file::{FileMode, StateFileManager};

pub fn rust_init_game_state(world: &mut crate::engine::World) {
    world.story = Some(StoryState::new());
    world.files.canned_nodes.clear();
}

pub fn rust_get_game_state_bits(
    world: &crate::engine::World,
    start_bit: c_int,
    end_bit: c_int,
) -> c_uchar {
    if let Some((start_bit, end_bit)) = normalize_bit_range(start_bit, end_bit) {
        read_story(world, |state| state.get_state(start_bit, end_bit))
    } else {
        0
    }
}

pub fn rust_set_game_state_bits(
    world: &mut crate::engine::World,
    start_bit: c_int,
    end_bit: c_int,
    value: c_uchar,
) {
    if let Some((start_bit, end_bit)) = normalize_bit_range(start_bit, end_bit) {
        write_story(world, |state| {
            state.set_state(start_bit, end_bit, value);
        });
    }
}

pub fn rust_get_game_state_32(world: &crate::engine::World, start_bit: c_int) -> u32 {
    let Some(start_bit) = normalize_start_bit_32(start_bit) else {
        return 0;
    };

    read_story(world, |state| state.get_state_32(start_bit))
}

pub fn rust_set_game_state_32(world: &mut crate::engine::World, start_bit: c_int, value: u32) {
    let Some(start_bit) = normalize_start_bit_32(start_bit) else {
        return;
    };

    write_story(world, |state| {
        state.set_state_32(start_bit, value);
    });
}

pub fn rust_copy_game_state(
    world: &mut crate::engine::World,
    dest_bit: c_int,
    src_start_bit: c_int,
    src_end_bit: c_int,
) {
    let Some((dest_bit, src_start_bit, src_end_bit)) =
        normalize_copy_bits(dest_bit, src_start_bit, src_end_bit)
    else {
        return;
    };

    write_story(world, |state| {
        let snapshot = StoryState::from_bytes(state.as_bytes());
        state.copy_state(dest_bit, &snapshot, src_start_bit, src_end_bit);
    });
}

pub fn rust_reset_game_state(world: &mut crate::engine::World) {
    write_story(world, |state| {
        state.reset();
    });
}

///
/// # Safety
///
/// Caller must ensure pointer arguments are valid and properly aligned.
pub unsafe fn rust_open_state_file(
    world: &mut crate::engine::World,
    file_index: c_int,
    mode: *const c_char,
) -> c_int {
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

    match files_result(world, |files| {
        files.open(file_index as usize, file_mode).map(|_| ())
    }) {
        Some(()) => 1,
        _ => 0,
    }
}

pub fn rust_close_state_file(world: &mut crate::engine::World, file_index: c_int) {
    write_files(world, |files| {
        let _ = files.close(file_index as usize);
    });
}

pub fn rust_delete_state_file(world: &mut crate::engine::World, file_index: c_int) {
    write_files(world, |files| {
        let _ = files.delete(file_index as usize);
    });
}

pub fn rust_length_state_file(world: &crate::engine::World, file_index: c_int) -> usize {
    read_files(world, |files| {
        files
            .get_file(file_index as usize)
            .map(|f| f.length())
            .unwrap_or(0)
    })
}

///
/// # Safety
///
/// No safety requirements; marked unsafe for C ABI compatibility.
pub unsafe fn rust_read_state_file(
    world: &mut crate::engine::World,
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

    write_files(world, |files| {
        files
            .get_file_mut(file_index as usize)
            .map(|f| f.read(slice).unwrap_or(0) / size)
            .unwrap_or(0)
    })
}

///
/// # Safety
///
/// No safety requirements; marked unsafe for C ABI compatibility.
pub unsafe fn rust_write_state_file(
    world: &mut crate::engine::World,
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

    write_files(world, |files| {
        match files.get_file_mut(file_index as usize) {
            Some(file) => match file.write(slice) {
                Ok(()) => count,
                Err(_) => 0,
            },
            None => 0,
        }
    })
}

pub fn rust_seek_state_file(
    world: &mut crate::engine::World,
    file_index: c_int,
    offset: i64,
    whence: c_int,
) -> c_int {
    use crate::engine::world::state_file::SeekWhence;

    let seek_whence = match whence {
        0 => SeekWhence::Set,
        1 => SeekWhence::Current,
        2 => SeekWhence::End,
        _ => return 0,
    };

    write_files(world, |state_files| {
        match state_files.get_file_mut(file_index as usize) {
            Some(file) => match file.seek(offset, seek_whence) {
                Ok(()) => 1,
                Err(_) => 0,
            },
            None => 0,
        }
    })
}

pub fn rust_get_game_state_bytes(world: &crate::engine::World) -> *const u8 {
    read_story(world, |state| state.as_bytes().as_ptr())
}

///
/// # Safety
///
/// No safety requirements; marked unsafe for C ABI compatibility.
pub fn rust_get_game_state_size() -> usize {
    super::game_state::NUM_GAME_STATE_BYTES
}

///
/// # Safety
///
/// Caller must ensure pointer arguments are valid and properly aligned.
pub unsafe fn rust_restore_game_state_from_bytes(
    world: &mut crate::engine::World,
    bytes: *const u8,
    size: usize,
) {
    if bytes.is_null() || size < NUM_GAME_STATE_BYTES {
        return;
    }

    let mut arr = [0u8; NUM_GAME_STATE_BYTES];
    std::ptr::copy_nonoverlapping(bytes, arr.as_mut_ptr(), NUM_GAME_STATE_BYTES);

    replace_game_state(world, &arr).expect("game-state singleton poisoned");
}

pub use crate::engine::world::story::GameStateOwnerError;

/// Copy of the singleton's bytes: the game state a save writes as "GmSt".
pub fn snapshot_game_state(
    world: &crate::engine::World,
) -> Result<[u8; NUM_GAME_STATE_BYTES], GameStateOwnerError> {
    world
        .story
        .as_ref()
        .map(|state| *state.as_bytes())
        .ok_or(GameStateOwnerError::NotInitialized)
}

/// Borrow the story and file fields together for a synchronous game-thread operation.
pub fn with_state_owners<R>(
    world: &mut crate::engine::World,
    f: impl FnOnce(&mut StoryState, &mut StateFileManager) -> R,
) -> Result<R, GameStateOwnerError> {
    let game = world
        .story
        .as_mut()
        .ok_or(GameStateOwnerError::NotInitialized)?;
    Ok(f(game, &mut world.files))
}

/// Borrow the owned state files for a synchronous game-thread operation.
pub fn with_state_files<R>(
    world: &mut crate::engine::World,
    f: impl FnOnce(&mut StateFileManager) -> R,
) -> Result<R, GameStateOwnerError> {
    Ok(write_files(world, f))
}

/// Replace the whole singleton with `bytes`, creating it when absent: a load
/// defines the game state whether or not a New Game ran first.
pub fn replace_game_state(
    world: &mut crate::engine::World,
    bytes: &[u8; NUM_GAME_STATE_BYTES],
) -> Result<(), GameStateOwnerError> {
    world.story = Some(StoryState::from_bytes(bytes));
    Ok(())
}

///
/// # Safety
///
/// No safety requirements; marked unsafe for C ABI compatibility.
pub unsafe fn rust_get_game_state_bits_from_bytes(
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

///
/// # Safety
///
/// No safety requirements; marked unsafe for C ABI compatibility.
pub unsafe fn rust_set_game_state_bits_in_bytes(
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

///
/// # Safety
///
/// No safety requirements; marked unsafe for C ABI compatibility.
pub unsafe fn rust_get_game_state32_from_bytes(bytes: *const u8, start_bit: c_int) -> u32 {
    let Some(start_bit) = normalize_start_bit_32(start_bit) else {
        return 0;
    };
    let Some(state) = normalize_state_bytes(bytes, NUM_GAME_STATE_BITS) else {
        return 0;
    };

    get_state_32_raw(state, start_bit)
}

///
/// # Safety
///
/// No safety requirements; marked unsafe for C ABI compatibility.
pub unsafe fn rust_set_game_state32_in_bytes(bytes: *mut u8, start_bit: c_int, value: u32) {
    let Some(start_bit) = normalize_start_bit_32(start_bit) else {
        return;
    };
    let Some(state) = normalize_state_bytes_mut(bytes, NUM_GAME_STATE_BITS) else {
        return;
    };

    set_state_32_raw(state, start_bit, value);
}

///
/// # Safety
///
/// No safety requirements; marked unsafe for C ABI compatibility.
pub unsafe fn rust_copy_game_state_bits_between_bytes(
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

pub fn rust_init_planet_info(world: &mut crate::engine::World, num_stars: c_int) -> c_int {
    if num_stars < 0 {
        return 0;
    }

    match files_result(world, |files| {
        let mut manager = PlanetInfoManager::new(files);
        manager.init_planet_info(num_stars as usize)
    }) {
        Some(()) => 1,
        None => 0,
    }
}

pub fn rust_uninit_planet_info(world: &mut crate::engine::World) {
    write_files(world, |files| {
        let mut manager = PlanetInfoManager::new(files);
        let _ = manager.uninit_planet_info();
    });
}

///
/// # Safety
///
/// No safety requirements; marked unsafe for C ABI compatibility.
pub unsafe fn rust_get_planet_info(
    world: &mut crate::engine::World,
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

    match files_result(world, |files| {
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

///
/// # Safety
///
/// No safety requirements; marked unsafe for C ABI compatibility.
pub unsafe fn rust_put_planet_info(
    world: &mut crate::engine::World,
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

    match files_result(world, |files| {
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

fn read_story<R: Default>(world: &crate::engine::World, f: impl FnOnce(&StoryState) -> R) -> R {
    world.story.as_ref().map_or_else(R::default, f)
}
fn write_story<R: Default>(
    world: &mut crate::engine::World,
    f: impl FnOnce(&mut StoryState) -> R,
) -> R {
    world.story.as_mut().map_or_else(R::default, f)
}
fn read_files<R>(world: &crate::engine::World, f: impl FnOnce(&StateFileManager) -> R) -> R {
    f(&world.files)
}
fn write_files<R>(
    world: &mut crate::engine::World,
    f: impl FnOnce(&mut StateFileManager) -> R,
) -> R {
    f(&mut world.files)
}
fn files_result<R, E>(
    world: &mut crate::engine::World,
    f: impl FnOnce(&mut StateFileManager) -> Result<R, E>,
) -> Option<R> {
    write_files(world, f).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::StoryKey;
    use serial_test::serial;

    #[test]
    #[serial]
    fn test_rust_init_game_state() {
        let mut game = crate::engine::Game::new();
        // SAFETY: serial tests own the published lifecycle until this call returns.
        unsafe {
            crate::app::bridge::with_game(&mut game, || {
                World::current().write(rust_init_game_state);

                assert!(World::current().read(|world| read_story(world, |_| true)));
            })
        }
    }

    #[test]
    #[serial]
    fn starting_a_second_game_does_not_inherit_the_first_games_state() {
        let mut game = crate::engine::Game::new();
        // SAFETY: serial tests own the published lifecycle until this call returns.
        unsafe {
            crate::app::bridge::with_game(&mut game, || {
                // InitGlobData memsets the C shadow and calls this to match, so a New
                // Game after finishing one must not remember what the player already
                // did. Leaving the bits set makes the Ur-Quan probe never appear and
                // the moon base never generate, because the game believes both are
                // already done.
                {
                    World::current().write(rust_init_game_state);
                    World::current()
                        .write(|world| world.set_story_state(StoryKey::MOONBASE_DESTROYED, 1));
                    World::current()
                        .write(|world| world.set_story_state(StoryKey::MOONBASE_ON_SHIP, 1));
                    assert_eq!(
                        World::current()
                            .read(|world| world.story_state(StoryKey::MOONBASE_DESTROYED)),
                        1
                    );

                    World::current().write(rust_init_game_state);

                    assert_eq!(
                        World::current()
                            .read(|world| world.story_state(StoryKey::MOONBASE_DESTROYED)),
                        0,
                        "a new game must start with the moon base intact"
                    );
                    assert_eq!(
                        World::current()
                            .read(|world| world.story_state(StoryKey::MOONBASE_ON_SHIP)),
                        0
                    );
                }
            })
        }
    }

    #[test]
    #[serial]
    fn a_new_game_clears_every_state_bit() {
        let mut game = crate::engine::Game::new();
        // SAFETY: serial tests own the published lifecycle until this call returns.
        unsafe {
            crate::app::bridge::with_game(&mut game, || {
                use super::super::game_state::NUM_GAME_STATE_BITS;

                // Every bit, including the partial trailing byte. The last six bits
                // belong to SAMATRA_GRPOFFS3, so stopping at the last whole byte would
                // let a reset regression confined to them pass unnoticed.
                let ranges: Vec<(c_int, c_int, c_uchar)> = (0..NUM_GAME_STATE_BITS)
                    .step_by(8)
                    .map(|start| {
                        let end = (start + 7).min(NUM_GAME_STATE_BITS - 1);
                        let width = end - start + 1;
                        let full = if width == 8 {
                            0xFF
                        } else {
                            // set_state_bits_raw rejects a value wider than the field.
                            ((1u16 << width) - 1) as u8
                        };
                        (start as c_int, end as c_int, full)
                    })
                    .collect();

                {
                    World::current().write(rust_init_game_state);
                    for (start, end, value) in &ranges {
                        World::current()
                            .write(|world| rust_set_game_state_bits(world, *start, *end, *value));
                        assert_eq!(
                            World::current()
                                .read(|world| rust_get_game_state_bits(world, *start, *end)),
                            *value,
                            "bit range {start}..={end} did not accept its value"
                        );
                    }

                    World::current().write(rust_init_game_state);

                    for (start, end, _) in &ranges {
                        assert_eq!(
                            World::current()
                                .read(|world| rust_get_game_state_bits(world, *start, *end)),
                            0,
                            "bit range {start}..={end} survived a new game"
                        );
                    }
                }
                assert_eq!(
                    ranges.last().map(|(_, end, _)| *end),
                    Some(NUM_GAME_STATE_BITS as c_int - 1),
                    "the final state bit must be covered"
                );
            })
        }
    }

    #[test]
    #[serial]
    fn test_typed_state_keys_use_generated_ranges() {
        let mut game = crate::engine::Game::new();
        // SAFETY: serial tests own the published lifecycle until this call returns.
        unsafe {
            crate::app::bridge::with_game(&mut game, || {
                World::current().write(rust_init_game_state);

                {
                    World::current()
                        .write(|world| world.set_story_state(StoryKey::SHOFIXTI_VISITS, 5));
                    World::current()
                        .write(|world| world.set_story_state(StoryKey::SHOFIXTI_RECRUITED, 1));
                    World::current()
                        .write(|world| world.set_story_state(StoryKey::SPATHI_VISITS, 3));

                    assert_eq!(
                        World::current().read(|world| world.story_state(StoryKey::SHOFIXTI_VISITS)),
                        5
                    );
                    assert_eq!(
                        World::current()
                            .read(|world| world.story_state(StoryKey::SHOFIXTI_RECRUITED)),
                        1
                    );
                    assert_eq!(
                        World::current().read(|world| world.story_state(StoryKey::SPATHI_VISITS)),
                        3
                    );
                    assert!(StoryKey::from_name("NOT_A_REAL_STATE").is_none());
                }
            })
        }
    }

    #[test]
    #[serial]
    fn test_rust_get_set_game_state_bits() {
        let mut game = crate::engine::Game::new();
        // SAFETY: serial tests own the published lifecycle until this call returns.
        unsafe {
            crate::app::bridge::with_game(&mut game, || {
                World::current().write(rust_init_game_state);
                World::current().write(|world| rust_set_game_state_bits(world, 0, 2, 5));
                let result = World::current().read(|world| rust_get_game_state_bits(world, 0, 2));
                assert_eq!(result, 5);
            })
        }
    }

    #[test]
    #[serial]
    fn test_rust_get_set_game_state_32() {
        let mut game = crate::engine::Game::new();
        // SAFETY: serial tests own the published lifecycle until this call returns.
        unsafe {
            crate::app::bridge::with_game(&mut game, || {
                World::current().write(rust_init_game_state);

                let test_value = 0xDEADBEEF;
                World::current().write(|world| rust_set_game_state_32(world, 0, test_value));

                let result = World::current().read(|world| rust_get_game_state_32(world, 0));
                assert_eq!(result, test_value);
            })
        }
    }

    #[test]
    #[serial]
    fn test_raw_byte_buffer_ffi_access() {
        let mut game = crate::engine::Game::new();
        // SAFETY: serial tests own the published lifecycle until this call returns.
        unsafe {
            crate::app::bridge::with_game(&mut game, || {
                let mut bytes = [0u8; super::super::game_state::NUM_GAME_STATE_BYTES];

                {
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
            })
        }
    }

    #[test]
    #[serial]
    fn test_raw_byte_buffer_copy_matches_c_semantics() {
        let mut game = crate::engine::Game::new();
        // SAFETY: serial tests own the published lifecycle until this call returns.
        unsafe {
            crate::app::bridge::with_game(&mut game, || {
                let mut src = [0u8; super::super::game_state::NUM_GAME_STATE_BYTES];
                let mut dest = [0u8; super::super::game_state::NUM_GAME_STATE_BYTES];

                {
                    rust_set_game_state_bits_in_bytes(src.as_mut_ptr(), 0, 7, 0xAB);
                    rust_set_game_state_bits_in_bytes(src.as_mut_ptr(), 8, 15, 0xCD);

                    rust_copy_game_state_bits_between_bytes(
                        dest.as_mut_ptr(),
                        32,
                        src.as_ptr(),
                        0,
                        15,
                    );
                    rust_copy_game_state_bits_between_bytes(
                        dest.as_mut_ptr(),
                        80,
                        src.as_ptr(),
                        0,
                        0,
                    );

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
            })
        }
    }

    #[test]
    #[serial]
    fn test_rust_reset_game_state() {
        let mut game = crate::engine::Game::new();
        // SAFETY: serial tests own the published lifecycle until this call returns.
        unsafe {
            crate::app::bridge::with_game(&mut game, || {
                World::current().write(rust_init_game_state);

                World::current().write(|world| rust_set_game_state_bits(world, 0, 7, 0xFF));
                World::current().write(rust_reset_game_state);

                let result = World::current().read(|world| rust_get_game_state_bits(world, 0, 7));
                assert_eq!(result, 0);
            })
        }
    }

    #[test]
    #[serial]
    fn test_rust_planet_info_round_trip() {
        let mut game = crate::engine::Game::new();
        // SAFETY: serial tests own the published lifecycle until this call returns.
        unsafe {
            crate::app::bridge::with_game(&mut game, || {
                World::current().write(rust_init_game_state);

                let moon_counts = [2u8, 0u8];
                let input_mask = [0x11u32, 0x22u32, 0x33u32];
                let mut output_mask = [0u32; NUM_SCAN_TYPES];

                {
                    assert_eq!(
                        World::current().write(|world| rust_init_planet_info(world, 8)),
                        1
                    );
                    assert_eq!(
                        World::current().write(|world| rust_put_planet_info(
                            world,
                            1,
                            0,
                            1,
                            moon_counts.as_ptr(),
                            moon_counts.len() as c_int,
                            input_mask.as_ptr()
                        )),
                        1
                    );
                    assert_eq!(
                        World::current().write(|world| rust_get_planet_info(
                            world,
                            1,
                            0,
                            1,
                            moon_counts.as_ptr(),
                            moon_counts.len() as c_int,
                            output_mask.as_mut_ptr(),
                        )),
                        1
                    );
                }

                assert_eq!(output_mask, input_mask);
            })
        }
    }

    #[test]
    #[serial]
    fn test_rust_open_state_file() {
        let mut game = crate::engine::Game::new();
        // SAFETY: serial tests own the published lifecycle until this call returns.
        unsafe {
            crate::app::bridge::with_game(&mut game, || {
                World::current().write(rust_init_game_state);

                {
                    let result = World::current().write(|world| {
                        rust_open_state_file(world, 0, c"wb".as_ptr() as *const c_char)
                    });
                    assert_eq!(result, 1);
                }
            })
        }
    }

    #[test]
    #[serial]
    fn test_rust_write_read_state_file() {
        let mut game = crate::engine::Game::new();
        // SAFETY: serial tests own the published lifecycle until this call returns.
        unsafe {
            crate::app::bridge::with_game(&mut game, || {
                World::current().write(rust_init_game_state);

                let test_data = b"Hello, World!";

                {
                    World::current().write(|world| {
                        rust_open_state_file(world, 0, c"wb".as_ptr() as *const c_char)
                    });
                    let written = World::current().write(|world| {
                        rust_write_state_file(world, 0, test_data.as_ptr(), 1, test_data.len())
                    });
                    assert_eq!(written, test_data.len());

                    World::current().write(|world| rust_seek_state_file(world, 0, 0, 0));

                    let mut buf = vec![0u8; test_data.len()];
                    let read = World::current().write(|world| {
                        rust_read_state_file(world, 0, buf.as_mut_ptr(), 1, test_data.len())
                    });
                    assert_eq!(read, test_data.len());
                    assert_eq!(&buf, test_data);
                }
            })
        }
    }

    #[test]
    #[serial]
    fn test_rust_length_state_file() {
        let mut game = crate::engine::Game::new();
        // SAFETY: serial tests own the published lifecycle until this call returns.
        unsafe {
            crate::app::bridge::with_game(&mut game, || {
                World::current().write(rust_init_game_state);

                let test_data = b"Test";

                {
                    World::current().write(|world| {
                        rust_open_state_file(world, 1, c"wb".as_ptr() as *const c_char)
                    });
                    World::current().write(|world| {
                        rust_write_state_file(world, 1, test_data.as_ptr(), 1, test_data.len())
                    });
                }

                let length = World::current().read(|world| rust_length_state_file(world, 1));
                assert_eq!(length, test_data.len());
            })
        }
    }

    #[test]
    #[serial]
    fn test_rust_delete_state_file() {
        let mut game = crate::engine::Game::new();
        // SAFETY: serial tests own the published lifecycle until this call returns.
        unsafe {
            crate::app::bridge::with_game(&mut game, || {
                World::current().write(rust_init_game_state);

                let test_data = b"Test";

                {
                    World::current().write(|world| {
                        rust_open_state_file(world, 2, c"wb".as_ptr() as *const c_char)
                    });
                    World::current().write(|world| {
                        rust_write_state_file(world, 2, test_data.as_ptr(), 1, test_data.len())
                    });
                }

                World::current().write(|world| rust_delete_state_file(world, 2));
                let length = World::current().read(|world| rust_length_state_file(world, 2));
                assert_eq!(length, 0);
            })
        }
    }

    #[test]
    #[serial]
    fn test_rust_seek_state_file() {
        let mut game = crate::engine::Game::new();
        // SAFETY: serial tests own the published lifecycle until this call returns.
        unsafe {
            crate::app::bridge::with_game(&mut game, || {
                World::current().write(rust_init_game_state);

                let test_data = b"HelloWorld";

                {
                    World::current().write(|world| {
                        rust_open_state_file(world, 0, c"wb".as_ptr() as *const c_char)
                    });
                    World::current().write(|world| {
                        rust_write_state_file(world, 0, test_data.as_ptr(), 1, test_data.len())
                    });
                }

                let result = World::current().write(|world| rust_seek_state_file(world, 0, 5, 0));
                assert_eq!(result, 1);

                let mut buf = vec![0u8; 5];
                {
                    let read = World::current()
                        .write(|world| rust_read_state_file(world, 0, buf.as_mut_ptr(), 1, 5));
                    assert_eq!(read, 5);
                }
                assert_eq!(&buf, b"World");
            })
        }
    }

    #[test]
    #[serial]
    fn test_rust_get_game_state_bytes() {
        let mut game = crate::engine::Game::new();
        // SAFETY: serial tests own the published lifecycle until this call returns.
        unsafe {
            crate::app::bridge::with_game(&mut game, || {
                World::current().write(rust_init_game_state);

                let test_value: u8 = 0xAB;
                World::current().write(|world| rust_set_game_state_bits(world, 0, 7, test_value));

                let ptr = World::current().read(rust_get_game_state_bytes);
                assert!(!ptr.is_null());

                {
                    assert_eq!(*ptr, test_value);
                }
            })
        }
    }

    #[test]
    #[serial]
    fn test_rust_get_game_state_size() {
        let mut game = crate::engine::Game::new();
        // SAFETY: serial tests own the published lifecycle until this call returns.
        unsafe {
            crate::app::bridge::with_game(&mut game, || {
                let size = rust_get_game_state_size();
                assert_eq!(size, super::super::game_state::NUM_GAME_STATE_BYTES);
            })
        }
    }

    #[test]
    #[serial]
    fn test_rust_restore_game_state_from_bytes() {
        let mut game = crate::engine::Game::new();
        // SAFETY: serial tests own the published lifecycle until this call returns.
        unsafe {
            crate::app::bridge::with_game(&mut game, || {
                World::current().write(rust_init_game_state);

                World::current().write(|world| rust_set_game_state_bits(world, 0, 7, 0xAB));
                let ptr = World::current().read(rust_get_game_state_bytes);
                let size = rust_get_game_state_size();

                let mut buffer = vec![0u8; size];
                {
                    std::ptr::copy_nonoverlapping(ptr, buffer.as_mut_ptr(), size);
                }

                World::current().write(rust_reset_game_state);
                World::current().write(rust_init_game_state);

                assert_eq!(
                    World::current().read(|world| rust_get_game_state_bits(world, 0, 7)),
                    0
                );

                {
                    World::current().write(|world| {
                        rust_restore_game_state_from_bytes(world, buffer.as_ptr(), size)
                    });
                }

                assert_eq!(
                    World::current().read(|world| rust_get_game_state_bits(world, 0, 7)),
                    0xAB
                );
            })
        }
    }
}
