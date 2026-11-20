//! Music ownership: `MUSICRES` resources and music files.

use std::ffi::{c_void, CStr};

use super::heart_ffi::{destroy_music, load_music};
use super::types::MusicRef;

/// Exclusive release authority for a detached music resource.
pub struct LoadedMusic(MusicRef);

impl LoadedMusic {
    pub fn load(resource: &CStr) -> Option<Self> {
        Self::load_file(&crate::resource::path_of(resource, "MUSICRES")?)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[serial_test::serial]
    fn music_loading_rejects_another_resource_type_before_ownership_transfer() {
        let key = c"test.sound.resource-types.not-music";
        crate::resource::settings::Settings.put_integer(key.to_str().unwrap(), 42);
        let rejected = std::panic::catch_unwind(|| LoadedMusic::load(key));
        assert!(rejected.is_err());
        assert!(crate::resource::remove(key.to_str().unwrap()));
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
