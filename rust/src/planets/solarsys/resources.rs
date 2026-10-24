//! Native interplanetary art, colormaps, music and system RNG ownership.
use super::*;
use crate::graphics::pixmap_abi::{CaptureDrawable, ReleaseDrawable};
use crate::graphics::resgfx::LoadGraphicInstance;
use crate::math::{delete_context as RandomContext_Delete, new_context as RandomContext_New};
use crate::sound::{heart_ffi::DestroyMusic, resource_types::LoadMusicInstance};
use crate::strings::sresins::LoadStringTableInstance;
use crate::strings::string_table::{CaptureStringTable, DestroyStringTable, ReleaseStringTable};

use crate::graphics::drawable_abi::DestroyDrawable;

#[no_mangle]
pub unsafe extern "C" fn FreeIPData() {
    DestroyDrawable(ReleaseDrawable(SISIPFRAME.cast()).cast());
    SISIPFRAME = ptr::null_mut();
    DestroyDrawable(ReleaseDrawable(SUN_FRAME.cast()).cast());
    SUN_FRAME = ptr::null_mut();
    DestroyStringTable(ReleaseStringTable(SUN_CMAP.cast()));
    SUN_CMAP = ptr::null_mut();
    DestroyStringTable(ReleaseStringTable(ORBITAL_CMAP.cast()));
    ORBITAL_CMAP = ptr::null_mut();
    DestroyDrawable(ReleaseDrawable(ORBITAL_FRAME.cast()).cast());
    ORBITAL_FRAME = ptr::null_mut();
    DestroyDrawable(ReleaseDrawable(SpaceJunkFrame.cast()).cast());
    SpaceJunkFrame = ptr::null_mut();
    DestroyMusic(SpaceMusic);
    SpaceMusic = ptr::null_mut();
    RandomContext_Delete(SysGenRNG.cast());
    SysGenRNG = ptr::null_mut();
}

#[no_mangle]
pub(super) unsafe extern "C" fn LoadIPData() {
    if SpaceJunkFrame.is_null() {
        SpaceJunkFrame =
            CaptureDrawable(LoadGraphicInstance(c"graphics.orbitbackground".as_ptr()).cast())
                .cast();
        SISIPFRAME =
            CaptureDrawable(LoadGraphicInstance(c"graphics.flagship".as_ptr()).cast()).cast();
        ORBITAL_CMAP =
            CaptureStringTable(LoadStringTableInstance(c"colortable.orbplan".as_ptr())).cast();
        ORBITAL_FRAME =
            CaptureDrawable(LoadGraphicInstance(c"graphics.planets".as_ptr()).cast()).cast();
        SUN_CMAP =
            CaptureStringTable(LoadStringTableInstance(c"colortable.truespace".as_ptr())).cast();
        SUN_FRAME =
            CaptureDrawable(LoadGraphicInstance(c"graphics.truespacesun".as_ptr()).cast()).cast();
        SpaceMusic = LoadMusicInstance(c"music.space".as_ptr());
    }
    if SysGenRNG.is_null() {
        SysGenRNG = RandomContext_New().cast();
    }
}
