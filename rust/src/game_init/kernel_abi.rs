//! Live game kernel and resource globals, ported from setup.c.
#![allow(non_snake_case)]

use crate::engine::battle::access::lifecycle;
use crate::graphics::native_abi as gfx;
use crate::graphics::{context_abi, drawable_abi, pixmap_abi};
use crate::io::uio_bridge as uio;
use crate::ships::native::abi::*;
use crate::strings::{
    sresins::LoadStringTableInstance,
    string_table::{CaptureStringTable, DestroyStringTable, ReleaseStringTable},
};
use std::{
    ffi::{c_char, CStr},
    ptr,
};

pub use crate::io::handles::{repository, rootDir};

use crate::mainloop::options_abi::{loadAddon, loadIndices, prepareAddons};

unsafe fn context(name: &CStr) -> CONTEXT {
    #[cfg(feature = "debug-process")]
    {
        context_abi::CreateContextAux(name.as_ptr()).cast()
    }
    #[cfg(not(feature = "debug-process"))]
    {
        let _ = name;
        context_abi::CreateContextAux().cast()
    }
}

/// # Safety
/// SDL, IO and resource registration prerequisites must be initialized on the game thread.
pub unsafe fn LoadKernel(
    kernel: &mut crate::engine::Kernel,
    world: &mut crate::engine::World,
    argc: i32,
    argv: *mut *mut c_char,
) -> u32 {
    {
        crate::sound::heart_ffi::InitSound(argc, argv.cast());
        crate::video::control::InitVideoPlayer(1);
        kernel.screen_context = context(c"ScreenContext");
        if kernel.screen_context.is_null() {
            return 0;
        }
        kernel.screen = pixmap_abi::CaptureDrawable(drawable_abi::CreateDisplay(
            3,
            ptr::addr_of_mut!(kernel.screen_width),
            ptr::addr_of_mut!(kernel.screen_height),
        ))
        .cast();
        if kernel.screen.is_null() {
            return 0;
        }
        context_abi::SetContext(kernel.screen_context.cast());
        drawable_abi::SetContextFGFrame(kernel.screen.cast());
        context_abi::SetContextOrigin(gfx::POINT { x: 0, y: 0 });
        kernel.h_res_index = crate::resource::ffi_bridge::InitResourceSystem();
        if kernel.h_res_index.is_null() {
            return 0;
        }
        // SAFETY: the resource system was initialized above and registration runs once on the game thread.
        if loadIndices(contentDir.cast()) == 0 {
            return 0;
        }
        let preferences = kernel.configuration.preferences();
        if preferences.music_3do != 0 {
            loadAddon(c"3domusic".as_ptr());
        }
        kernel.using_speech = preferences.speech;
        if preferences.speech != 0 && loadAddon(c"3dovoice".as_ptr()) == 0 {
            kernel.using_speech = 0;
        }
        if preferences.music_remix != 0 {
            loadAddon(c"remix".as_ptr());
        }
        if preferences.intro == OPT_3DO as i32 {
            loadAddon(c"3dovideo".as_ptr());
        }
        prepareAddons(&kernel.configuration.addons());
        let map = CaptureStringTable(LoadStringTableInstance(c"colortable.main".as_ptr()));
        if map.is_null() {
            return 0;
        }
        crate::graphics::cmap_abi::SetColorMap(crate::graphics::cmap_abi::GetColorMapAddress(
            map.cast(),
        ));
        DestroyStringTable(ReleaseStringTable(map));
        world.set_activity(u16::MAX);
        1
    }
}

/// # Safety
/// Game-thread context creation after LoadKernel.
pub unsafe fn InitContexts(kernel: &mut crate::engine::Kernel) -> u32 {
    {
        kernel.status_context = context(c"StatusContext");
        if kernel.status_context.is_null() {
            return 0;
        }
        context_abi::SetContext(kernel.status_context.cast());
        drawable_abi::SetContextFGFrame(kernel.screen.cast());
        let mut rect = gfx::RECT {
            corner: gfx::POINT {
                x: (ScreenWidth - STATUS_WIDTH as i32 - SAFE_X as i32) as i16,
                y: SAFE_Y as i16,
            },
            extent: gfx::EXTENT {
                width: STATUS_WIDTH as i16,
                height: (ScreenHeight - 2 * SAFE_Y as i32) as i16,
            },
        };
        context_abi::SetContextClipRect(&mut rect);
        kernel.space_context = context(c"SpaceContext");
        if kernel.space_context.is_null() {
            return 0;
        }
        kernel.off_screen_context = context(c"OffScreenContext");
        if kernel.off_screen_context.is_null() {
            return 0;
        }
        u32::from(lifecycle::init_elements(
            &mut *kernel,
            MAX_DISPLAY_ELEMENTS as u16,
        ))
    }
}

unsafe fn init_kernel(kernel: &mut crate::engine::Kernel, activity: u16) -> u32 {
    // SAFETY: only resource loaders run during the owner borrow.
    let loaded = {
        crate::battle::context_access::lifecycle::init_ships(&mut *kernel);
        let Some(font) = crate::graphics::native_font::LoadedFont::load(c"font.starcon") else {
            return 0;
        };
        kernel.install_battle_font(font);
        let Some(font) = crate::graphics::native_font::LoadedFont::load(c"font.tiny") else {
            return 0;
        };
        kernel.install_tiny_font(font);
        kernel.activity_frame = pixmap_abi::CaptureDrawable(
            crate::graphics::resgfx::LoadGraphicInstance(c"graphics.activity".as_ptr()).cast(),
        )
        .cast();
        if kernel.activity_frame.is_null() {
            return 0;
        }
        kernel.status_frame = pixmap_abi::CaptureDrawable(
            crate::graphics::resgfx::LoadGraphicInstance(c"graphics.status".as_ptr()).cast(),
        )
        .cast();
        if kernel.status_frame.is_null() {
            return 0;
        }
        kernel.game_strings =
            CaptureStringTable(LoadStringTableInstance(c"text.starcon".as_ptr())).cast();
        if kernel.game_strings.is_null() {
            return 0;
        }
        kernel.micro_font =
            crate::graphics::resgfx::LoadGraphicInstance(c"font.micro".as_ptr()).cast();
        if kernel.micro_font.is_null() {
            return 0;
        }
        kernel.menu_sounds = CaptureStringTable(
            crate::sound::resource_types::LoadSoundInstance(c"sounds.menu".as_ptr()).cast(),
        )
        .cast();
        if kernel.menu_sounds.is_null() {
            return 0;
        }
        1
    };
    if loaded == 0 {
        return 0;
    }
    init_status_offsets(kernel, ScreenHeight);
    super::space_resources::init_space(kernel, activity)
}

/// # Safety
/// Called on the game thread with base resources and graphics initialized.
pub unsafe fn InitGameKernel(kernel: &mut crate::engine::Kernel, activity: u16) -> u32 {
    // SAFETY: the copied resource handle retains no owner borrow.
    let needs_init = kernel.activity_frame.is_null();
    if needs_init && (init_kernel(kernel, activity) == 0 || InitContexts(kernel) == 0) {
        return 0;
    }
    1
}

pub fn UninitPlayerInput() {}

/// # Safety
/// Player index is below NUM_PLAYERS and its input context is not already installed.
pub fn SetPlayerInput(kernel: &mut crate::engine::Kernel, player: u16) -> bool {
    {
        let slot = &mut kernel.player_input[player as usize];
        assert!(slot.is_none());
        *slot = Some(crate::engine::battle::input::InputContext::new(
            player,
            kernel.player_control[player as usize],
        ));
        true
    }
}

/// # Safety
/// Both player input contexts must be uninstalled.
pub fn SetPlayerInputAll(kernel: &mut crate::engine::Kernel) -> bool {
    for player in 0..2 {
        if !SetPlayerInput(kernel, player) {
            return false;
        }
    }
    true
}

/// # Safety
/// Player index is below NUM_PLAYERS and context is null or owned by this player.
pub fn ClearPlayerInput(kernel: &mut crate::engine::Kernel, player: u16) {
    kernel.player_input[player as usize] = None;
}

/// # Safety
/// Called on the game thread when the contexts are no longer in use.
pub fn ClearPlayerInputAll(kernel: &mut crate::engine::Kernel) {
    for player in 0..2 {
        ClearPlayerInput(kernel, player);
    }
}

/// # Safety
/// Called once during startup on the IO owner thread.
#[no_mangle]
pub unsafe extern "C" fn initIO() -> i32 {
    uio::uio_init();
    repository = uio::uio_openRepository(0);
    rootDir = uio::uio_openDir(repository, c"/".as_ptr(), 0);
    if rootDir.is_null() {
        log::error!("Could not open '/' dir.");
        -1
    } else {
        0
    }
}

/// # Safety
/// No directory users remain; root and repository are live.
pub unsafe fn uninitIO() {
    uio::uio_closeDir(rootDir);
    uio::uio_closeRepository(repository);
    uio::uio_unInit();
}
