//! Music ownership and the loaders and free routines behind the `SNDRES` and
//! `MUSICRES` resource types, whose data the resource system stores as opaque
//! pointers.

use std::ffi::{c_void, CStr};

use super::heart_ffi::{destroy_music, load_music, load_sound_bank};
use super::owned_bank::LoadedSoundBank;
use super::types::MusicRef;

/// Exclusive release authority for a detached music resource.
pub struct LoadedMusic(MusicRef);

impl LoadedMusic {
    pub fn load(resource: &CStr) -> Option<Self> {
        let handle = crate::resource::ffi_bridge::detach_resource_of_type(resource, "MUSICRES")?;
        // SAFETY: the MUSICRES loader stores one boxed `MusicRef` and detaching transfers it.
        Some(Self(*unsafe {
            Box::from_raw(handle.as_ptr().cast::<MusicRef>())
        }))
    }

    pub fn load_file(path: &CStr) -> Option<Self> {
        load_music(path.to_str().ok()?).map(Self)
    }

    pub fn playback(&self) -> MusicRef {
        self.0.clone()
    }

    /// An address identifying this music while a video clip plays it; never dereferenced.
    pub fn identity(&self) -> *mut c_void {
        std::sync::Arc::as_ptr(&self.0 .0).cast_mut().cast()
    }
}

impl std::fmt::Debug for LoadedMusic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("LoadedMusic")
            .field(&self.identity())
            .finish()
    }
}

impl Drop for LoadedMusic {
    fn drop(&mut self) {
        destroy_music(self.0.clone());
    }
}

/// The `SNDRES` loader: a bank handle for the resource system, or null.
pub fn sound_bank_handle(name: &str) -> *mut c_void {
    load_sound_bank(name).map_or(std::ptr::null_mut(), LoadedSoundBank::into_raw)
}

/// The `MUSICRES` loader: a music handle for the resource system, or null.
pub fn music_handle(name: &str) -> *mut c_void {
    load_music(name).map_or(std::ptr::null_mut(), |music| {
        Box::into_raw(Box::new(music)).cast()
    })
}

/// Free an `SNDRES` handle that was never detached.
///
/// # Safety
/// `handle` is null or came from [`sound_bank_handle`] and was not detached or freed.
pub unsafe fn free_sound_bank_handle(handle: *mut c_void) {
    if !handle.is_null() {
        // SAFETY: the caller supplies the one reference the loader released.
        drop(unsafe { LoadedSoundBank::from_raw(handle) });
    }
}

/// Free a `MUSICRES` handle that was never detached.
///
/// # Safety
/// `handle` is null or came from [`music_handle`] and was not detached or freed.
pub unsafe fn free_music_handle(handle: *mut c_void) {
    if !handle.is_null() {
        // SAFETY: the caller supplies the one box the loader released.
        destroy_music(*unsafe { Box::from_raw(handle.cast::<MusicRef>()) });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn music_handles_round_trip_and_release_their_sample_reference() {
        let sample = std::sync::Arc::new(parking_lot::Mutex::new(
            super::super::types::SoundSample::new(),
        ));
        let handle = Box::into_raw(Box::new(MusicRef(sample.clone()))).cast::<c_void>();
        assert_eq!(std::sync::Arc::strong_count(&sample), 2);
        // SAFETY: the handle was boxed above and is freed once.
        unsafe { free_music_handle(handle) };
        assert_eq!(std::sync::Arc::strong_count(&sample), 1);
    }

    #[test]
    fn detached_music_releases_its_sample_reference_on_drop() {
        let sample = std::sync::Arc::new(parking_lot::Mutex::new(
            super::super::types::SoundSample::new(),
        ));
        let music = LoadedMusic(MusicRef(sample.clone()));
        assert_eq!(std::sync::Arc::strong_count(&sample), 2);
        let playback = music.playback();
        assert_eq!(std::sync::Arc::strong_count(&sample), 3);
        drop(music);
        assert_eq!(std::sync::Arc::strong_count(&sample), 2);
        assert!(std::sync::Arc::ptr_eq(&playback.0, &sample));
        drop(playback);
        assert_eq!(std::sync::Arc::strong_count(&sample), 1);
    }
}
