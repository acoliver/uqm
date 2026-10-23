//! Live game kernel and resource globals, ported from setup.c.
#![allow(non_snake_case)]

use crate::graphics::native_abi as gfx;
use crate::graphics::{context_abi, drawable_abi, pixmap_abi};
use crate::io::uio_bridge as uio;
use crate::ships::native::abi::*;
use crate::strings::{
    sresins::LoadStringTableInstance,
    string_table::{CaptureStringTable, DestroyStringTable, ReleaseStringTable},
};
use std::{
    ffi::{c_char, c_void, CStr},
    mem, ptr,
};

macro_rules! global {
    ($name:ident: $ty:ty) => {
        #[no_mangle]
        pub static mut $name: $ty = unsafe { mem::zeroed() };
    };
}
global!(LastActivity: u16);
global!(PlayerControl: [u8; 2]);
global!(hResIndex: *mut c_void);
global!(ScreenContext: CONTEXT);
global!(SpaceContext: CONTEXT);
global!(StatusContext: CONTEXT);
global!(OffScreenContext: CONTEXT);
global!(screen_width: i16);
global!(screen_height: i16);
global!(Screen: FRAME);
global!(StarConFont: FONT);
global!(MicroFont: FONT);
global!(TinyFont: FONT);
global!(race_q: [QUEUE; 2]);
global!(ActivityFrame: FRAME);
global!(StatusFrame: FRAME);
global!(FlagStatFrame: FRAME);
global!(MiscDataFrame: FRAME);
global!(FontGradFrame: FRAME);
global!(GameStrings: STRING);
global!(disp_q: QUEUE);
global!(repository: *mut uio::uio_Repository);
global!(rootDir: *mut uio::uio_DirHandle);
global!(usingSpeech: u32);

extern "C" {
    fn loadIndices(dir: *mut c_void) -> i32;
    fn loadAddon(name: *const c_char) -> u32;
    fn prepareAddons(addons: *const *const c_char);
}

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
    crate::sound::heart_ffi::InitSound(argc, argv.cast());
    crate::video::control::InitVideoPlayer(1);
    ScreenContext = context(c"ScreenContext");
    if ScreenContext.is_null() {
        return 0;
    }
    Screen = pixmap_abi::CaptureDrawable(drawable_abi::CreateDisplay(
        3,
        ptr::addr_of_mut!(screen_width),
        ptr::addr_of_mut!(screen_height),
    ))
    .cast();
    if Screen.is_null() {
        return 0;
    }
    context_abi::SetContext(ScreenContext.cast());
    drawable_abi::SetContextFGFrame(Screen.cast());
    context_abi::SetContextOrigin(gfx::POINT { x: 0, y: 0 });
    hResIndex = crate::resource::ffi_bridge::InitResourceSystem();
    if hResIndex.is_null() {
        return 0;
    }
    if loadIndices(contentDir.cast()) == 0 {
        return 0;
    }
    if opt3doMusic != 0 {
        loadAddon(c"3domusic".as_ptr());
    }
    usingSpeech = optSpeech;
    if optSpeech != 0 && loadAddon(c"3dovoice".as_ptr()) == 0 {
        usingSpeech = 0;
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
    crate::navigation::globals::GlobData
        .Game_state
        .CurrentActivity = u16::MAX;
    1
}

/// # Safety
/// Game-thread context creation after LoadKernel.
#[no_mangle]
pub unsafe extern "C" fn InitContexts() -> u32 {
    StatusContext = context(c"StatusContext");
    if StatusContext.is_null() {
        return 0;
    }
    context_abi::SetContext(StatusContext.cast());
    drawable_abi::SetContextFGFrame(Screen.cast());
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
    SpaceContext = context(c"SpaceContext");
    if SpaceContext.is_null() {
        return 0;
    }
    OffScreenContext = context(c"OffScreenContext");
    if OffScreenContext.is_null() {
        return 0;
    }
    InitQueue(
        ptr::addr_of_mut!(disp_q),
        MAX_DISPLAY_ELEMENTS as u16,
        mem::size_of::<ELEMENT>() as u16,
    )
}

unsafe fn init_kernel() -> u32 {
    for queue in &mut *ptr::addr_of_mut!(race_q) {
        InitQueue(
            queue,
            MAX_SHIPS_PER_SIDE as u16,
            mem::size_of::<STARSHIP>() as u16,
        );
    }
    StarConFont = crate::graphics::resgfx::LoadGraphicInstance(c"font.starcon".as_ptr()).cast();
    if StarConFont.is_null() {
        return 0;
    }
    TinyFont = crate::graphics::resgfx::LoadGraphicInstance(c"font.tiny".as_ptr()).cast();
    if TinyFont.is_null() {
        return 0;
    }
    ActivityFrame = pixmap_abi::CaptureDrawable(
        crate::graphics::resgfx::LoadGraphicInstance(c"graphics.activity".as_ptr()).cast(),
    )
    .cast();
    if ActivityFrame.is_null() {
        return 0;
    }
    StatusFrame = pixmap_abi::CaptureDrawable(
        crate::graphics::resgfx::LoadGraphicInstance(c"graphics.status".as_ptr()).cast(),
    )
    .cast();
    if StatusFrame.is_null() {
        return 0;
    }
    GameStrings = CaptureStringTable(LoadStringTableInstance(c"text.starcon".as_ptr())).cast();
    if GameStrings.is_null() {
        return 0;
    }
    MicroFont = crate::graphics::resgfx::LoadGraphicInstance(c"font.micro".as_ptr()).cast();
    if MicroFont.is_null() {
        return 0;
    }
    MenuSounds = CaptureStringTable(
        crate::sound::resource_types::LoadSoundInstance(c"sounds.menu".as_ptr()).cast(),
    )
    .cast();
    if MenuSounds.is_null() {
        return 0;
    }
    InitStatusOffsets();
    super::space_resources::InitSpace()
}

/// # Safety
/// Called on the game thread with base resources and graphics initialized.
#[no_mangle]
pub unsafe extern "C" fn InitGameKernel() -> u32 {
    if ActivityFrame.is_null() && (init_kernel() == 0 || InitContexts() == 0) {
        return 0;
    }
    1
}

#[no_mangle]
pub extern "C" fn UninitPlayerInput() {}

/// # Safety
/// Player index is below NUM_PLAYERS and its input context is not already installed.
#[no_mangle]
pub unsafe extern "C" fn SetPlayerInput(player: u16) -> bool {
    let slot = ptr::addr_of_mut!(PlayerInput)
        .cast::<*mut crate::battle::battle_controls::InputContext>()
        .add(player as usize);
    assert!((*slot).is_null());
    *slot = match PlayerControl[player as usize] & 15 {
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
    let slot = ptr::addr_of_mut!(PlayerInput)
        .cast::<*mut crate::battle::battle_controls::InputContext>()
        .add(player as usize);
    if (*slot).is_null() {
        log::debug!("ClearPlayerInput(): PlayerInput[{player}] was NULL.");
        return;
    }
    ((*(**slot).handlers).delete_context.unwrap())(*slot);
    *slot = ptr::null_mut();
}

/// # Safety
/// Called on the game thread when the contexts are no longer in use.
#[no_mangle]
pub unsafe extern "C" fn ClearPlayerInputAll() {
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
#[no_mangle]
pub unsafe extern "C" fn uninitIO() {
    uio::uio_closeDir(rootDir);
    uio::uio_closeRepository(repository);
    uio::uio_unInit();
}
