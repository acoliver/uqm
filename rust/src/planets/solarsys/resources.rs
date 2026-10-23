//! Native interplanetary art, colormaps, music and system RNG ownership.
use super::*;
use crate::graphics::pixmap_abi::{CaptureDrawable, ReleaseDrawable};
use crate::graphics::resgfx::LoadGraphicInstance;
use crate::math::{RandomContext_Delete, RandomContext_New};
use crate::sound::{heart_ffi::DestroyMusic, resource_types::LoadMusicInstance};
use crate::strings::sresins::LoadStringTableInstance;
use crate::strings::string_table::{CaptureStringTable, DestroyStringTable, ReleaseStringTable};

extern "C" {
    fn DestroyDrawable(drawable: *mut c_void) -> u32;
}

#[no_mangle]
unsafe extern "C" fn FreeIPData() {
    DestroyDrawable(ReleaseDrawable(SISIPFrame.cast()).cast());
    SISIPFrame = ptr::null_mut();
    DestroyDrawable(ReleaseDrawable(SunFrame.cast()).cast());
    SunFrame = ptr::null_mut();
    DestroyStringTable(ReleaseStringTable(SunCMap.cast()));
    SunCMap = ptr::null_mut();
    DestroyStringTable(ReleaseStringTable(OrbitalCMap.cast()));
    OrbitalCMap = ptr::null_mut();
    DestroyDrawable(ReleaseDrawable(OrbitalFrame.cast()).cast());
    OrbitalFrame = ptr::null_mut();
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
        SISIPFrame =
            CaptureDrawable(LoadGraphicInstance(c"graphics.flagship".as_ptr()).cast()).cast();
        OrbitalCMap =
            CaptureStringTable(LoadStringTableInstance(c"colortable.orbplan".as_ptr())).cast();
        OrbitalFrame =
            CaptureDrawable(LoadGraphicInstance(c"graphics.planets".as_ptr()).cast()).cast();
        SunCMap =
            CaptureStringTable(LoadStringTableInstance(c"colortable.truespace".as_ptr())).cast();
        SunFrame =
            CaptureDrawable(LoadGraphicInstance(c"graphics.truespacesun".as_ptr()).cast()).cast();
        SpaceMusic = LoadMusicInstance(c"music.space".as_ptr());
    }
    if SysGenRNG.is_null() {
        SysGenRNG = RandomContext_New().cast();
    }
}
