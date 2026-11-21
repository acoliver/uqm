//! Content mounts, SDL graphics, and SDL input for serialized native fixtures.
//!
//! Each guard undoes exactly what it installed. Fixtures declare the guards
//! after the display resources they back, so the resources drop first.

use crate::{
    graphics::present::{init_graphics, uninit_graphics, GraphicsOptions},
    input::InputSession,
    input_port::sdl_input,
    io::{install, uninstall, Dir, Placement, Vfs},
};
use std::{ffi::CString, path::Path};

/// A temporary directory of fixture content under the repository's tmp directory.
pub struct FixtureFiles(tempfile::TempDir);

impl FixtureFiles {
    pub fn new() -> Self {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../tmp");
        std::fs::create_dir_all(&root).unwrap_or_else(|e| panic!("fixture tmp root: {e}"));
        let root = root
            .canonicalize()
            .unwrap_or_else(|e| panic!("fixture tmp root: {e}"));
        let directory = tempfile::tempdir_in(root).unwrap_or_else(|e| panic!("fixture dir: {e}"));
        Self(directory)
    }

    pub fn path(&self) -> &Path {
        self.0.path()
    }

    pub fn write(&self, name: &str, contents: &str) {
        std::fs::write(self.path().join(name), contents)
            .unwrap_or_else(|e| panic!("write fixture {name}: {e}"));
    }

    /// `font/<code>.png` white 1x1 glyphs for every code in `codes`.
    pub fn write_font(&self, codes: std::ops::RangeInclusive<u32>) {
        std::fs::create_dir(self.path().join("font"))
            .unwrap_or_else(|e| panic!("fixture font dir: {e}"));
        for code in codes {
            image::RgbaImage::from_pixel(1, 1, image::Rgba([255, 255, 255, 255]))
                .save(self.path().join(format!("font/{code:04x}.png")))
                .unwrap_or_else(|e| panic!("write glyph {code:04x}: {e}"));
        }
    }
}

impl Default for FixtureFiles {
    fn default() -> Self {
        Self::new()
    }
}

/// The SDL dummy video and audio drivers with a 320x240 software display.
pub struct Graphics(());

impl Graphics {
    pub fn start() -> Self {
        std::env::set_var("SDL_VIDEODRIVER", "dummy");
        std::env::set_var("SDL_AUDIODRIVER", "dummy");
        init_graphics(1, 0, GraphicsOptions::default(), 320, 240);
        Self(())
    }
}

impl Drop for Graphics {
    fn drop(&mut self) {
        uninit_graphics();
    }
}

/// A directory mounted as the content directory, and optionally the config
/// directory, with the previous process file system restored on drop.
pub struct ContentMount {
    vfs: Vfs,
    content: Dir,
    previous: Option<Vfs>,
}

impl ContentMount {
    /// Mount `directory` at `/` and make it the content directory.
    pub fn new(directory: &Path) -> Self {
        let vfs = Vfs::new();
        vfs.mount_dir("/", directory, Placement::Top, false);
        let content = vfs.open_dir(Path::new("/"));
        vfs.set_content_dir(content.clone());
        let previous = install(vfs.clone());
        Self {
            vfs,
            content,
            previous,
        }
    }

    /// Also mount `directory` at `/config` and make it the config directory.
    pub fn with_config(self, directory: &Path) -> Self {
        self.vfs
            .mount_dir("/config", directory, Placement::Top, false);
        self.vfs
            .set_config_dir(self.vfs.open_dir(Path::new("/config")));
        self
    }

    /// Reset the resource system and load `index` from the content directory.
    pub fn init_resources(&self, index: &str) {
        crate::resource::init();
        self.load_index(index);
    }

    /// Load `index` from the content directory into the live resource system.
    pub fn load_index(&self, index: &str) {
        crate::resource::load_index(&self.content, &cstring(index, "index name"), None);
    }
}

impl Drop for ContentMount {
    fn drop(&mut self) {
        match self.previous.take() {
            Some(previous) => {
                install(previous);
            }
            None => {
                uninstall();
            }
        }
    }
}

/// An input session bound to the native SDL input provider.
pub struct NativeInput(InputSession);

impl NativeInput {
    pub fn bind(input: &InputSession) -> Self {
        if let Err(error) = sdl_input::init_input(input.controls()) {
            panic!("native input did not start: {error}");
        }
        Self(input.clone())
    }
}

impl Drop for NativeInput {
    fn drop(&mut self) {
        sdl_input::uninit_input();
        self.0.clear_menu_state();
    }
}

/// Deliver a key-down for ASCII `key` to the initialized native provider.
pub fn key_down(key: u8) {
    sdl_input::process_input_event(&crate::host::HostEvent::KeyDown {
        key: i32::from(key),
        numlock: false,
        repeat: false,
    });
}

/// Deliver one text-input event carrying `text`.
pub fn text_input(text: &str) {
    sdl_input::process_input_event(&crate::host::HostEvent::TextInput(text.to_owned()));
}

/// Reload `index` from whatever directory is currently the content directory.
pub fn reload_content_index(index: &str) {
    let content = crate::io::content_dir().unwrap_or_else(|| panic!("no content directory"));
    crate::resource::load_index(&content, &cstring(index, "index name"), None);
}

/// Drop one entry from the live resource map.
pub fn remove_resource(key: &str) {
    crate::resource::remove(key);
}

fn cstring(text: &str, what: &str) -> CString {
    CString::new(text).unwrap_or_else(|error| panic!("{what} has no NUL: {error}"))
}
