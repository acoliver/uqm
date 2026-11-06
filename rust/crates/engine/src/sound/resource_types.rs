//! Sound bank and music instance loading. The `SNDRES` and `MUSICRES`
//! resource types are registered by the resource system itself.

use std::ffi::{c_char, c_void};

use crate::resource::ffi_bridge::{res_DetachResource, res_GetResource};

/// Exclusive release authority for a detached music resource.
#[derive(Debug)]
pub struct LoadedMusic(std::ptr::NonNull<c_void>);

impl LoadedMusic {
    pub fn load(resource: &std::ffi::CStr) -> Option<Self> {
        crate::resource::ffi_bridge::detach_resource_of_type(resource, "MUSICRES").map(Self)
    }

    pub fn as_raw(&self) -> *mut c_void {
        self.0.as_ptr()
    }
}

impl Drop for LoadedMusic {
    fn drop(&mut self) {
        // SAFETY: this non-cloneable owner frees its detached resource exactly once.
        unsafe { super::heart_ffi::destroy_music(self.0.as_ptr()) };
    }
}

/// Loads the sound bank behind `res` and takes ownership of it from the
/// resource system.
///
/// # Safety
/// `res` must be a NUL-terminated resource key.
#[no_mangle]
pub unsafe extern "C" fn LoadSoundInstance(res: *const c_char) -> *mut c_void {
    let data = res_GetResource(res);
    if !data.is_null() {
        res_DetachResource(res);
    }
    data
}

/// Loads the music behind `res` and takes ownership of it from the
/// resource system.
///
/// # Safety
/// `res` must be a NUL-terminated resource key.
#[no_mangle]
pub unsafe extern "C" fn LoadMusicInstance(res: *const c_char) -> *mut c_void {
    let data = res_GetResource(res);
    if !data.is_null() {
        res_DetachResource(res);
    }
    data
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    #[serial_test::serial]
    fn music_loading_rejects_another_resource_type_before_ownership_transfer() {
        let key = c"test.sound.resource-types.not-music";
        // SAFETY: this serialized fixture installs and removes its own integer resource.
        unsafe { crate::resource::ffi_bridge::res_PutInteger(key.as_ptr(), 42) };
        let rejected = std::panic::catch_unwind(|| LoadedMusic::load(key));
        assert!(rejected.is_err());
        // SAFETY: the fixture key is terminated and still identifies its integer resource.
        unsafe { crate::resource::ffi_bridge::res_Remove(key.as_ptr()) };
    }

    #[test]
    fn detached_music_releases_its_sample_reference_on_drop() {
        let sample = Arc::new(parking_lot::Mutex::new(
            super::super::types::SoundSample::new(),
        ));
        // SAFETY: this fixture constructs the same allocator-owned Arc slot as the music loader.
        let music = unsafe {
            let slot = crate::memory::rust_hmalloc(std::mem::size_of::<*const c_void>())
                .cast::<*const parking_lot::Mutex<super::super::types::SoundSample>>();
            slot.write(Arc::into_raw(sample.clone()));
            LoadedMusic(std::ptr::NonNull::new(slot.cast()).unwrap())
        };
        assert_eq!(Arc::strong_count(&sample), 2);
        assert!(!music.as_raw().is_null());
        drop(music);
        assert_eq!(Arc::strong_count(&sample), 1);
    }
}
