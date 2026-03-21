// C Resource Bridge - FFI bindings for resource loading from C
// @plan PLAN-20260314-SHIPS.P05

use super::types::ShipsError;

#[cfg(not(test))]
use std::ffi::c_void;

// ---------------------------------------------------------------------------
// Opaque handle types (matching C library conventions)
// ---------------------------------------------------------------------------

/// Opaque handle to a C DRAWABLE (graphic/frame).
pub type DrawableHandle = usize;

/// Opaque handle to a C MUSIC object.
pub type MusicHandle = usize;

/// Opaque handle to a C SOUND object.
pub type SoundHandle = usize;

/// Opaque handle to a C STRING_TABLE.
pub type StringTableHandle = usize;

/// Sentinel value for NULL_RESOURCE (C: NULL_RESOURCE == 0).
pub const NULL_RESOURCE: u32 = 0;

// ---------------------------------------------------------------------------
// FFI declarations for C resource loading functions
// ---------------------------------------------------------------------------

#[cfg(not(test))]
extern "C" {
    fn LoadGraphic(res_id: *const c_void) -> *mut c_void;
    fn CaptureDrawable(drawable: *mut c_void) -> *mut c_void;
    fn ReleaseDrawable(drawable: *mut c_void) -> *mut c_void;
    fn DestroyDrawable(drawable: *mut c_void) -> i32;

    fn LoadMusic(res_id: *const c_void) -> *mut c_void;
    fn DestroyMusic(music: *mut c_void) -> i32;

    fn LoadSound(res_id: *const c_void) -> *mut c_void;
    fn CaptureSound(sound: *mut c_void) -> *mut c_void;
    fn ReleaseSound(sound: *mut c_void) -> *mut c_void;
    fn DestroySound(sound: *mut c_void) -> i32;

    fn LoadStringTable(res_id: *const c_void) -> *mut c_void;
    fn CaptureStringTable(table: *mut c_void) -> *mut c_void;
    fn ReleaseStringTable(table: *mut c_void) -> *mut c_void;
    fn DestroyStringTable(table: *mut c_void) -> i32;
}

// ---------------------------------------------------------------------------
// Safe Rust wrappers for resource loading
// ---------------------------------------------------------------------------

/// Loads a graphic resource from C and returns a captured drawable handle.
///
/// Wraps C `LoadGraphic()` + `CaptureDrawable()`.
///
/// # Errors
/// Returns `ShipsError::LoadFailed` if the resource cannot be loaded.
#[cfg(not(test))]
pub fn load_graphic(res_id: u32) -> Result<DrawableHandle, ShipsError> {
    if res_id == NULL_RESOURCE {
        return Ok(0);
    }

    unsafe {
        let res_ptr = res_id as *const c_void;
        let drawable = LoadGraphic(res_ptr);
        if drawable.is_null() {
            return Err(ShipsError::LoadFailed(format!(
                "LoadGraphic failed for resource ID {}",
                res_id
            )));
        }
        let captured = CaptureDrawable(drawable);
        if captured.is_null() {
            return Err(ShipsError::LoadFailed(format!(
                "CaptureDrawable failed for resource ID {}",
                res_id
            )));
        }
        Ok(captured as usize)
    }
}

/// Loads a music resource from C and returns a music handle.
///
/// Wraps C `LoadMusic()`.
///
/// # Errors
/// Returns `ShipsError::LoadFailed` if the resource cannot be loaded.
#[cfg(not(test))]
pub fn load_music(res_id: u32) -> Result<MusicHandle, ShipsError> {
    if res_id == NULL_RESOURCE {
        return Ok(0);
    }

    unsafe {
        let res_ptr = res_id as *const c_void;
        let music = LoadMusic(res_ptr);
        if music.is_null() {
            return Err(ShipsError::LoadFailed(format!(
                "LoadMusic failed for resource ID {}",
                res_id
            )));
        }
        Ok(music as usize)
    }
}

/// Loads a sound resource from C and returns a captured sound handle.
///
/// Wraps C `LoadSound()` + `CaptureSound()`.
///
/// # Errors
/// Returns `ShipsError::LoadFailed` if the resource cannot be loaded.
#[cfg(not(test))]
pub fn load_sound(res_id: u32) -> Result<SoundHandle, ShipsError> {
    if res_id == NULL_RESOURCE {
        return Ok(0);
    }

    unsafe {
        let res_ptr = res_id as *const c_void;
        let sound = LoadSound(res_ptr);
        if sound.is_null() {
            return Err(ShipsError::LoadFailed(format!(
                "LoadSound failed for resource ID {}",
                res_id
            )));
        }
        let captured = CaptureSound(sound);
        if captured.is_null() {
            return Err(ShipsError::LoadFailed(format!(
                "CaptureSound failed for resource ID {}",
                res_id
            )));
        }
        Ok(captured as usize)
    }
}

/// Loads a string table resource from C and returns a captured table handle.
///
/// Wraps C `LoadStringTable()` + `CaptureStringTable()`.
///
/// # Errors
/// Returns `ShipsError::LoadFailed` if the resource cannot be loaded.
#[cfg(not(test))]
pub fn load_string_table(res_id: u32) -> Result<StringTableHandle, ShipsError> {
    if res_id == NULL_RESOURCE {
        return Ok(0);
    }

    unsafe {
        let res_ptr = res_id as *const c_void;
        let table = LoadStringTable(res_ptr);
        if table.is_null() {
            return Err(ShipsError::LoadFailed(format!(
                "LoadStringTable failed for resource ID {}",
                res_id
            )));
        }
        let captured = CaptureStringTable(table);
        if captured.is_null() {
            return Err(ShipsError::LoadFailed(format!(
                "CaptureStringTable failed for resource ID {}",
                res_id
            )));
        }
        Ok(captured as usize)
    }
}

/// Frees a graphic drawable handle.
///
/// Wraps C `ReleaseDrawable()` + `DestroyDrawable()`.
#[cfg(not(test))]
pub fn free_graphic(handle: DrawableHandle) {
    if handle == 0 {
        return;
    }

    unsafe {
        let drawable = handle as *mut c_void;
        let released = ReleaseDrawable(drawable);
        DestroyDrawable(released);
    }
}

/// Frees a music handle.
///
/// Wraps C `DestroyMusic()`.
#[cfg(not(test))]
pub fn free_music(handle: MusicHandle) {
    if handle == 0 {
        return;
    }

    unsafe {
        let music = handle as *mut c_void;
        DestroyMusic(music);
    }
}

/// Frees a sound handle.
///
/// Wraps C `ReleaseSound()` + `DestroySound()`.
#[cfg(not(test))]
pub fn free_sound(handle: SoundHandle) {
    if handle == 0 {
        return;
    }

    unsafe {
        let sound = handle as *mut c_void;
        let released = ReleaseSound(sound);
        DestroySound(released);
    }
}

/// Frees a string table handle.
///
/// Wraps C `ReleaseStringTable()` + `DestroyStringTable()`.
#[cfg(not(test))]
pub fn free_string_table(handle: StringTableHandle) {
    if handle == 0 {
        return;
    }

    unsafe {
        let table = handle as *mut c_void;
        let released = ReleaseStringTable(table);
        DestroyStringTable(released);
    }
}

// ---------------------------------------------------------------------------
// Test mocks (only compiled for tests)
// ---------------------------------------------------------------------------

#[cfg(test)]
use std::sync::atomic::{AtomicU64, Ordering};

#[cfg(test)]
static MOCK_HANDLE_COUNTER: AtomicU64 = AtomicU64::new(1);

#[cfg(test)]
thread_local! {
    static ALLOCATED_HANDLES: std::cell::RefCell<std::collections::HashSet<usize>> =
        std::cell::RefCell::new(std::collections::HashSet::new());
    static FREED_HANDLES: std::cell::RefCell<Vec<usize>> =
        std::cell::RefCell::new(Vec::new());
    /// Resource ID that should trigger a load failure (0 = no failure).
    static FAIL_ON_RES_ID: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
fn allocate_mock_handle() -> usize {
    let handle = MOCK_HANDLE_COUNTER.fetch_add(1, Ordering::Relaxed) as usize;
    ALLOCATED_HANDLES.with(|handles| {
        handles.borrow_mut().insert(handle);
    });
    handle
}

#[cfg(test)]
fn free_mock_handle(handle: usize) {
    FREED_HANDLES.with(|freed| {
        freed.borrow_mut().push(handle);
    });
    ALLOCATED_HANDLES.with(|handles| {
        handles.borrow_mut().remove(&handle);
    });
}

/// Returns the number of currently allocated (not yet freed) mock handles.
#[cfg(test)]
pub fn mock_allocated_count() -> usize {
    ALLOCATED_HANDLES.with(|handles| handles.borrow().len())
}

/// Returns true if the given handle is currently allocated.
#[cfg(test)]
pub fn mock_is_allocated(handle: usize) -> bool {
    ALLOCATED_HANDLES.with(|handles| handles.borrow().contains(&handle))
}

/// Returns how many times a handle was freed.
#[cfg(test)]
pub fn mock_free_count(handle: usize) -> usize {
    FREED_HANDLES.with(|freed| freed.borrow().iter().filter(|&&h| h == handle).count())
}

/// Resets all mock tracking state (allocated handles, freed log, failure injection).
#[cfg(test)]
pub fn mock_reset() {
    ALLOCATED_HANDLES.with(|handles| handles.borrow_mut().clear());
    FREED_HANDLES.with(|freed| freed.borrow_mut().clear());
    FAIL_ON_RES_ID.with(|f| f.set(0));
}

/// Sets a resource ID that will cause the next load to fail.
/// Set to 0 to disable failure injection.
#[cfg(test)]
pub fn mock_set_fail_on_res_id(res_id: u32) {
    FAIL_ON_RES_ID.with(|f| f.set(res_id));
}

#[cfg(test)]
fn should_fail(res_id: u32) -> bool {
    FAIL_ON_RES_ID.with(|f| {
        let fail_id = f.get();
        fail_id != 0 && fail_id == res_id
    })
}

#[cfg(test)]
pub fn load_graphic(res_id: u32) -> Result<DrawableHandle, ShipsError> {
    if res_id == NULL_RESOURCE {
        return Ok(0);
    }
    if should_fail(res_id) {
        return Err(ShipsError::LoadFailed(format!(
            "Mock failure: load_graphic for resource {}",
            res_id
        )));
    }
    Ok(allocate_mock_handle())
}

#[cfg(test)]
pub fn load_music(res_id: u32) -> Result<MusicHandle, ShipsError> {
    if res_id == NULL_RESOURCE {
        return Ok(0);
    }
    if should_fail(res_id) {
        return Err(ShipsError::LoadFailed(format!(
            "Mock failure: load_music for resource {}",
            res_id
        )));
    }
    Ok(allocate_mock_handle())
}

#[cfg(test)]
pub fn load_sound(res_id: u32) -> Result<SoundHandle, ShipsError> {
    if res_id == NULL_RESOURCE {
        return Ok(0);
    }
    if should_fail(res_id) {
        return Err(ShipsError::LoadFailed(format!(
            "Mock failure: load_sound for resource {}",
            res_id
        )));
    }
    Ok(allocate_mock_handle())
}

#[cfg(test)]
pub fn load_string_table(res_id: u32) -> Result<StringTableHandle, ShipsError> {
    if res_id == NULL_RESOURCE {
        return Ok(0);
    }
    if should_fail(res_id) {
        return Err(ShipsError::LoadFailed(format!(
            "Mock failure: load_string_table for resource {}",
            res_id
        )));
    }
    Ok(allocate_mock_handle())
}

#[cfg(test)]
pub fn free_graphic(handle: DrawableHandle) {
    if handle != 0 {
        free_mock_handle(handle);
    }
}

#[cfg(test)]
pub fn free_music(handle: MusicHandle) {
    if handle != 0 {
        free_mock_handle(handle);
    }
}

#[cfg(test)]
pub fn free_sound(handle: SoundHandle) {
    if handle != 0 {
        free_mock_handle(handle);
    }
}

#[cfg(test)]
pub fn free_string_table(handle: StringTableHandle) {
    if handle != 0 {
        free_mock_handle(handle);
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn null_resource_returns_zero() {
        mock_reset();
        assert_eq!(load_graphic(NULL_RESOURCE).unwrap(), 0);
        assert_eq!(load_music(NULL_RESOURCE).unwrap(), 0);
        assert_eq!(load_sound(NULL_RESOURCE).unwrap(), 0);
        assert_eq!(load_string_table(NULL_RESOURCE).unwrap(), 0);
    }

    #[test]
    fn mock_allocates_unique_handles() {
        mock_reset();
        let h1 = load_graphic(1).unwrap();
        let h2 = load_graphic(2).unwrap();
        let h3 = load_music(3).unwrap();
        assert_ne!(h1, 0);
        assert_ne!(h2, 0);
        assert_ne!(h3, 0);
        assert_ne!(h1, h2);
        assert_ne!(h2, h3);
        assert_eq!(mock_allocated_count(), 3);

        free_graphic(h1);
        assert!(!mock_is_allocated(h1));
        assert_eq!(mock_free_count(h1), 1);
        free_graphic(h2);
        free_music(h3);
        assert_eq!(mock_allocated_count(), 0);
    }

    #[test]
    fn free_zero_handle_is_noop() {
        mock_reset();
        free_graphic(0);
        free_music(0);
        free_sound(0);
        free_string_table(0);
        assert_eq!(mock_allocated_count(), 0);
    }

    #[test]
    fn failure_injection_works() {
        mock_reset();
        mock_set_fail_on_res_id(42);
        assert!(load_graphic(42).is_err());
        assert!(load_music(42).is_err());
        assert!(load_sound(42).is_err());
        assert!(load_string_table(42).is_err());
        // Other resource IDs still succeed
        let h = load_graphic(1).unwrap();
        assert_ne!(h, 0);
        free_graphic(h);
        mock_reset();
    }

    #[test]
    fn mock_reset_clears_all_state() {
        let h = load_graphic(1).unwrap();
        assert_eq!(mock_allocated_count(), 1);
        mock_reset();
        assert_eq!(mock_allocated_count(), 0);
        assert_eq!(mock_free_count(h), 0);
    }
}
