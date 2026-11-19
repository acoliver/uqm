//! Content mounts, SDL graphics, and SDL input for serialized native fixtures.
//!
//! Each guard undoes exactly what it installed. Fixtures declare the guards
//! after the display resources they back, so the resources drop first.

use crate::{
    graphics::sdl::lifecycle_abi::{GraphicsOptions, TFB_InitGraphics, TFB_UninitGraphics},
    input::{
        frames::{FLIGHT_KEYS, MENU_KEYS, TEMPLATES},
        InputSession,
    },
    input_port::sdl_input,
    io::{handles, uio_bridge as uio},
};
use std::{ffi::CString, path::Path, ptr};

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
        // SAFETY: the serialized fixture owns SDL until this guard drops.
        let status = unsafe {
            TFB_InitGraphics(
                1,
                0,
                ptr::from_ref(&GraphicsOptions::default()).cast(),
                320,
                240,
            )
        };
        assert_eq!(status, 0);
        Self(())
    }
}

impl Drop for Graphics {
    fn drop(&mut self) {
        // SAFETY: every display resource this fixture created has already been released.
        unsafe { TFB_UninitGraphics() };
    }
}

/// A directory mounted as the content directory, and optionally the config
/// directory, with the previous handles restored on drop.
pub struct ContentMount {
    repository: *mut uio::uio_Repository,
    content: *mut uio::uio_DirHandle,
    config: Option<*mut uio::uio_DirHandle>,
    previous_content: *mut crate::io::ffi::uio_DirHandle,
    previous_config: *mut crate::io::ffi::uio_DirHandle,
}

impl ContentMount {
    /// Mount `directory` at `/` and make it the content directory.
    pub fn new(directory: &Path) -> Self {
        let path = cstring(
            directory
                .to_str()
                .unwrap_or_else(|| panic!("fixture path is UTF-8")),
            "fixture path",
        );
        // SAFETY: the serialized fixture owns the repository and mounts until drop.
        unsafe {
            uio::uio_init();
            let repository = uio::uio_openRepository(0);
            assert!(!uio::uio_mountDir(
                repository,
                c"/".as_ptr(),
                1,
                ptr::null_mut(),
                ptr::null(),
                path.as_ptr(),
                ptr::null_mut(),
                6,
                ptr::null_mut()
            )
            .is_null());
            let content = uio::uio_openDir(repository, c"/".as_ptr(), 0);
            let previous_content = handles::contentDir;
            handles::contentDir = content.cast();
            Self {
                repository,
                content,
                config: None,
                previous_content,
                previous_config: handles::configDir,
            }
        }
    }

    /// Also mount `directory` at `/config` and make it the config directory.
    pub fn with_config(mut self, directory: &Path) -> Self {
        let path = cstring(
            directory
                .to_str()
                .unwrap_or_else(|| panic!("fixture path is UTF-8")),
            "fixture path",
        );
        // SAFETY: the mount shares this guard's repository and is closed with it.
        unsafe {
            assert!(!uio::uio_mountDir(
                self.repository,
                c"/config".as_ptr(),
                1,
                ptr::null_mut(),
                ptr::null(),
                path.as_ptr(),
                ptr::null_mut(),
                4,
                ptr::null_mut()
            )
            .is_null());
            let config = uio::uio_openDir(self.repository, c"/config".as_ptr(), 0);
            handles::configDir = config.cast();
            self.config = Some(config);
        }
        self
    }

    /// Reset the resource system and load `index` from the content directory.
    pub fn init_resources(&self, index: &str) {
        // SAFETY: the content directory stays mounted for the guard's life.
        unsafe { crate::resource::ffi_bridge::InitResourceSystem() };
        self.load_index(index);
    }

    /// Load `index` from the content directory into the live resource system.
    pub fn load_index(&self, index: &str) {
        let index = cstring(index, "index name");
        // SAFETY: the content directory stays mounted for the guard's life.
        unsafe {
            crate::resource::ffi_bridge::LoadResourceIndex(
                self.content.cast(),
                index.as_ptr(),
                ptr::null(),
            );
        }
    }
}

impl Drop for ContentMount {
    fn drop(&mut self) {
        // SAFETY: all callbacks have returned and no directory lease escapes the fixture.
        unsafe {
            handles::contentDir = self.previous_content;
            handles::configDir = self.previous_config;
            uio::uio_closeDir(self.content);
            if let Some(config) = self.config {
                uio::uio_closeDir(config);
            }
            uio::uio_closeRepository(self.repository);
            uio::uio_unInit();
        }
    }
}

/// An input session bound to the native SDL input provider.
pub struct NativeInput(InputSession);

impl NativeInput {
    pub fn bind(input: &InputSession) -> Self {
        let state = input.immediate_ptr();
        // SAFETY: the retained input owns both stable vectors until the guard unbinds them.
        unsafe {
            sdl_input::tfb_set_input_vectors(
                ptr::addr_of_mut!((*state).menu).cast(),
                MENU_KEYS as i32,
                ptr::addr_of_mut!((*state).key).cast(),
                TEMPLATES as i32,
                FLIGHT_KEYS as i32,
            );
            assert_eq!(sdl_input::TFB_InitInput(0, 0), 0);
        }
        Self(input.clone())
    }
}

impl Drop for NativeInput {
    fn drop(&mut self) {
        sdl_input::TFB_UninitInput();
        // SAFETY: callbacks have returned, so nothing reads the vectors being detached.
        unsafe { sdl_input::tfb_set_input_vectors(ptr::null_mut(), 0, ptr::null_mut(), 0, 0) };
        self.0.clear_menu_state();
    }
}

/// Deliver a key-down for ASCII `key` to the initialized native provider.
pub fn key_down(key: u8) {
    // SAFETY: an all-zero SDL_Event is valid, and the fields set make it a key-down.
    unsafe {
        let mut event: sdl2_sys::SDL_Event = std::mem::zeroed();
        event.type_ = 0x300;
        event.key.state = 1;
        event.key.keysym.sym = i32::from(key);
        event.key.keysym.scancode = sdl2_sys::SDL_Scancode::SDL_SCANCODE_W;
        sdl_input::process_input_event(&event);
    }
}

/// Deliver one SDL text-input event carrying `text` (at most 31 bytes).
pub fn text_input(text: &str) {
    assert!(text.len() < 32);
    // SAFETY: an all-zero SDL_Event is valid; the text is NUL terminated by the zeroed tail.
    unsafe {
        let mut event: sdl2_sys::SDL_Event = std::mem::zeroed();
        event.type_ = 0x303;
        for (slot, byte) in event.text.text.iter_mut().zip(text.bytes()) {
            *slot = byte as std::ffi::c_char;
        }
        sdl_input::process_input_event(&event);
    }
}

/// Reload `index` from whatever directory is currently the content directory.
pub fn reload_content_index(index: &str) {
    let index = cstring(index, "index name");
    // SAFETY: the serialized fixture keeps a content mount live while it reloads.
    unsafe {
        crate::resource::ffi_bridge::LoadResourceIndex(
            handles::contentDir.cast(),
            index.as_ptr(),
            ptr::null(),
        );
    }
}

/// Drop one entry from the live resource map.
pub fn remove_resource(key: &str) {
    let key = cstring(key, "resource key");
    // SAFETY: the key is NUL terminated and the resource system is initialized by the fixture.
    unsafe { crate::resource::ffi_bridge::res_Remove(key.as_ptr()) };
}

fn cstring(text: &str, what: &str) -> CString {
    CString::new(text).unwrap_or_else(|error| panic!("{what} has no NUL: {error}"))
}
