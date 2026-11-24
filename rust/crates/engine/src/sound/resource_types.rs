//! Music ownership: `MUSICRES` resources and music files.

use std::ffi::{c_void, CStr};

use super::heart_ffi::{destroy_music, load_music};
use super::types::MusicRef;

/// Exclusive release authority for a detached music resource.
pub struct LoadedMusic(MusicRef);

impl LoadedMusic {
    pub fn load(content: &crate::resource::Content, resource: &CStr) -> Option<Self> {
        Self::load_file(
            content.files(),
            &content.path_of(resource, crate::resource::AssetKind::Music)?,
        )
    }

    pub fn load_file(files: &crate::io::Dir, path: &CStr) -> Option<Self> {
        load_music(files, path.to_str().ok()?).map(Self)
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
        let mount = crate::io::fixture::TempMount::new();
        mount.write(
            "wrong.rmp",
            b"test.sound.resource-types.not-music = INT32:42\n",
        );
        let content = mount.content(c"wrong.rmp");
        let rejected = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            LoadedMusic::load(&content, key)
        }));
        assert!(rejected.is_err());
        let mut revised = content.revise();
        assert!(revised.remove(key.to_str().unwrap()));
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
