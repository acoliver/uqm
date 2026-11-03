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

use crate::mainloop::options_abi::loadIndices;

use crate::mainloop::options_abi::loadAddon;

use crate::mainloop::options_abi::prepareAddons;

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
#[no_mangle]
pub unsafe extern "C" fn LoadKernel(argc: i32, argv: *mut *mut c_char) -> u32 {
    use crate::engine::{Kernel, WorldData};
    crate::sound::heart_ffi::InitSound(argc, argv.cast());
    crate::video::control::InitVideoPlayer(1);
    (*Kernel::current()).screen_context = context(c"ScreenContext");
    if (*Kernel::current()).screen_context.is_null() {
        return 0;
    }
    (*Kernel::current()).screen = pixmap_abi::CaptureDrawable(drawable_abi::CreateDisplay(
        3,
        ptr::addr_of_mut!((*Kernel::current()).screen_width),
        ptr::addr_of_mut!((*Kernel::current()).screen_height),
    ))
    .cast();
    if (*Kernel::current()).screen.is_null() {
        return 0;
    }
    context_abi::SetContext((*Kernel::current()).screen_context.cast());
    drawable_abi::SetContextFGFrame((*Kernel::current()).screen.cast());
    context_abi::SetContextOrigin(gfx::POINT { x: 0, y: 0 });
    (*Kernel::current()).h_res_index = crate::resource::ffi_bridge::InitResourceSystem();
    if (*Kernel::current()).h_res_index.is_null() {
        return 0;
    }
    // SAFETY: the resource system was initialized above and registration runs once on the game thread.
    if loadIndices(contentDir.cast()) == 0 {
        return 0;
    }
    if opt3doMusic != 0 {
        loadAddon(c"3domusic".as_ptr());
    }
    (*Kernel::current()).using_speech = optSpeech;
    if optSpeech != 0 && loadAddon(c"3dovoice".as_ptr()) == 0 {
        (*Kernel::current()).using_speech = 0;
    }
    if optRemixMusic != 0 {
        loadAddon(c"remix".as_ptr());
    }
    if optWhichIntro == OPT_3DO as i32 {
        loadAddon(c"3dovideo".as_ptr());
    }
    prepareAddons(optAddons);
    let map = CaptureStringTable(LoadStringTableInstance(c"colortable.main".as_ptr()));
    if map.is_null() {
        return 0;
    }
    crate::graphics::cmap_abi::SetColorMap(crate::graphics::cmap_abi::GetColorMapAddress(
        map.cast(),
    ));
    DestroyStringTable(ReleaseStringTable(map));
    (*WorldData::current()).game.current_activity = u16::MAX;
    1
}

/// # Safety
/// Game-thread context creation after LoadKernel.
#[no_mangle]
pub unsafe extern "C" fn InitContexts() -> u32 {
    use crate::engine::Kernel;
    (*Kernel::current()).status_context = context(c"StatusContext");
    if (*Kernel::current()).status_context.is_null() {
        return 0;
    }
    context_abi::SetContext((*Kernel::current()).status_context.cast());
    drawable_abi::SetContextFGFrame((*Kernel::current()).screen.cast());
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
    (*Kernel::current()).space_context = context(c"SpaceContext");
    if (*Kernel::current()).space_context.is_null() {
        return 0;
    }
    (*Kernel::current()).off_screen_context = context(c"OffScreenContext");
    if (*Kernel::current()).off_screen_context.is_null() {
        return 0;
    }
    u32::from(lifecycle::init_elements(
        &mut *Kernel::current(),
        MAX_DISPLAY_ELEMENTS as u16,
    ))
}

unsafe fn init_kernel() -> u32 {
    use crate::engine::Kernel;
    crate::battle::context_access::lifecycle::init_ships(&mut *Kernel::current());
    (*Kernel::current()).star_con_font =
        crate::graphics::resgfx::LoadGraphicInstance(c"font.starcon".as_ptr()).cast();
    if (*Kernel::current()).star_con_font.is_null() {
        return 0;
    }
    (*Kernel::current()).tiny_font =
        crate::graphics::resgfx::LoadGraphicInstance(c"font.tiny".as_ptr()).cast();
    if (*Kernel::current()).tiny_font.is_null() {
        return 0;
    }
    (*Kernel::current()).activity_frame = pixmap_abi::CaptureDrawable(
        crate::graphics::resgfx::LoadGraphicInstance(c"graphics.activity".as_ptr()).cast(),
    )
    .cast();
    if (*Kernel::current()).activity_frame.is_null() {
        return 0;
    }
    (*Kernel::current()).status_frame = pixmap_abi::CaptureDrawable(
        crate::graphics::resgfx::LoadGraphicInstance(c"graphics.status".as_ptr()).cast(),
    )
    .cast();
    if (*Kernel::current()).status_frame.is_null() {
        return 0;
    }
    (*Kernel::current()).game_strings =
        CaptureStringTable(LoadStringTableInstance(c"text.starcon".as_ptr())).cast();
    if (*Kernel::current()).game_strings.is_null() {
        return 0;
    }
    (*Kernel::current()).micro_font =
        crate::graphics::resgfx::LoadGraphicInstance(c"font.micro".as_ptr()).cast();
    if (*Kernel::current()).micro_font.is_null() {
        return 0;
    }
    (*Kernel::current()).menu_sounds = CaptureStringTable(
        crate::sound::resource_types::LoadSoundInstance(c"sounds.menu".as_ptr()).cast(),
    )
    .cast();
    if (*Kernel::current()).menu_sounds.is_null() {
        return 0;
    }
    InitStatusOffsets();
    super::space_resources::InitSpace()
}

/// # Safety
/// Called on the game thread with base resources and graphics initialized.
#[no_mangle]
pub unsafe extern "C" fn InitGameKernel() -> u32 {
    use crate::engine::Kernel;
    if (*Kernel::current()).activity_frame.is_null() && (init_kernel() == 0 || InitContexts() == 0)
    {
        return 0;
    }
    1
}

pub fn UninitPlayerInput() {}

/// # Safety
/// Player index is below NUM_PLAYERS and its input context is not already installed.
#[no_mangle]
pub unsafe extern "C" fn SetPlayerInput(player: u16) -> bool {
    use crate::engine::Kernel;
    let slot = &mut (*Kernel::current()).player_input[player as usize];
    assert!((*slot).is_null());
    *slot = match (*Kernel::current()).player_control[player as usize] & 15 {
        1 => crate::battle::battle_controls::HumanInputContext_new(player).cast(),
        6 | 2 => crate::battle::battle_controls::ComputerInputContext_new(player).cast(),
        8 => crate::battle::battle_controls::NetworkInputContext_new(player).cast(),
        _ => {
            log::error!("Invalid control method in SetPlayerInput().");
            std::process::abort();
        }
    };
    !(*slot).is_null()
}

/// # Safety
/// Both player input contexts must be uninstalled.
#[no_mangle]
pub unsafe extern "C" fn SetPlayerInputAll() -> bool {
    for player in 0..2 {
        if !SetPlayerInput(player) {
            return false;
        }
    }
    true
}

/// # Safety
/// Player index is below NUM_PLAYERS and context is null or owned by this player.
#[no_mangle]
pub unsafe extern "C" fn ClearPlayerInput(player: u16) {
    use crate::engine::Kernel;
    let input = (*Kernel::current()).player_input[player as usize]
        .cast::<crate::battle::battle_controls::InputContext>();
    if input.is_null() {
        log::debug!("ClearPlayerInput(): PlayerInput[{player}] was NULL.");
        return;
    }
    ((*(*input).handlers).delete_context.unwrap())(input);
    (*Kernel::current()).player_input[player as usize] = ptr::null_mut();
}

/// # Safety
/// Called on the game thread when the contexts are no longer in use.
pub unsafe fn ClearPlayerInputAll() {
    for player in 0..2 {
        ClearPlayerInput(player);
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
