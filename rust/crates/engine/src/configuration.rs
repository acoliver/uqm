//! Application preferences and startup values retained by their consumers.

use std::ffi::CString;
use std::sync::{Arc, Mutex, MutexGuard};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Preferences {
    pub coarse_scan: i32,
    pub menu: i32,
    pub fonts: i32,
    pub intro: i32,
    pub shield: i32,
    pub smooth_scroll: i32,
    pub melee_scale: i32,
    pub music_3do: u32,
    pub music_remix: u32,
    pub speech: u32,
    pub subtitles: u32,
    pub stereo_sfx: u32,
    pub keep_aspect_ratio: u32,
    pub gamma: f32,
}

#[derive(Default)]
struct Values {
    preferences: Preferences,
    addons: Vec<CString>,
    content_path: Option<CString>,
}

#[derive(Clone, Default)]
pub struct Configuration(Arc<Mutex<Values>>);

impl Configuration {
    fn values(&self) -> MutexGuard<'_, Values> {
        self.0
            .lock()
            .unwrap_or_else(|_| panic!("configuration poisoned"))
    }

    pub fn preferences(&self) -> Preferences {
        self.values().preferences
    }

    pub fn set_preferences(&self, preferences: Preferences) {
        self.values().preferences = preferences;
    }

    pub fn update_preferences(&self, update: impl FnOnce(&mut Preferences)) {
        update(&mut self.values().preferences);
    }

    pub fn addons(&self) -> Vec<CString> {
        self.values().addons.clone()
    }

    pub fn set_addons(&self, addons: Vec<CString>) {
        self.values().addons = addons;
    }

    pub fn content_path(&self) -> Option<CString> {
        self.values().content_path.clone()
    }

    pub fn set_content_path(&self, path: CString) {
        self.values().content_path = Some(path);
    }
}
