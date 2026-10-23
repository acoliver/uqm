//! Splash, introduction, victory and ship-spin presentations, ported from uqm/fmv.c.
#![allow(non_snake_case)]

#[cfg(any(not(test), feature = "linked_c_archive"))]
mod runtime {
    use super::super::intro::ShowPresentation;
    use crate::navigation::abi::*;
    use crate::navigation::globals::GlobData;
    use std::ptr;

    // uqm/fmv.h defines WANT_SHIP_SPINS unconditionally in this configuration.
    const WANT_SHIP_SPINS: bool = true;

    /// # Safety
    /// Graphics, resource and input systems must be initialized on the game thread;
    /// `music` must be null or a live music reference.
    #[no_mangle]
    pub unsafe extern "C" fn DoShipSpin(index: COUNT, music: MUSIC_REF) {
        if !WANT_SHIP_SPINS {
            return;
        }
        let mut name = [0 as std::ffi::c_char; 32];
        let mut old_rect: RECT = std::mem::zeroed();
        LoadIntoExtraScreen(ptr::null_mut());
        if !music.is_null() {
            StopMusic();
        }
        FreeHyperData();
        libc::sprintf(
            name.as_mut_ptr(),
            c"slides.spins.%02u".as_ptr(),
            index as u32,
        );
        ShowPresentation(name.as_ptr());
        SleepThreadUntil(FadeScreen(
            ScreenFadeType_FadeAllToBlack,
            (ONE_SECOND / 4) as SIZE,
        ));
        FlushColorXForms();
        GetContextClipRect(&mut old_rect);
        SetContextClipRect(ptr::null_mut());
        DrawFromExtraScreen(ptr::null_mut());
        SetContextClipRect(&mut old_rect);
        if !music.is_null() {
            PlayMusic(music, 1, 1);
        }
        SleepThreadUntil(FadeScreen(
            ScreenFadeType_FadeAllToColor,
            (ONE_SECOND / 4) as SIZE,
        ));
        FlushColorXForms();
    }

    /// # Safety
    /// The game systems must be initialized on the game thread. A supplied
    /// processing callback must remain valid throughout this call.
    #[no_mangle]
    pub unsafe extern "C" fn SplashScreen(process: Option<unsafe extern "C" fn(DWORD)>) {
        SleepThreadUntil(FadeScreen(
            ScreenFadeType_FadeAllToBlack,
            (ONE_SECOND / 120) as SIZE,
        ));
        SetContext(ScreenContext);
        let mut stamp = STAMP {
            origin: POINT { x: 0, y: 0 },
            frame: CaptureDrawable(LoadGraphicInstance(TITLE_ANIM.as_ptr().cast()).cast()),
        };
        DrawStamp(&mut stamp);
        DestroyDrawable(ReleaseDrawable(stamp.frame));
        let timeout = FadeScreen(ScreenFadeType_FadeAllToColor, (ONE_SECOND / 2) as SIZE);
        if let Some(callback) = process {
            callback(timeout);
        }
        if GlobData.Game_state.CurrentActivity & CHECK_ABORT as u16 != 0 {
            return;
        }
        WaitForAnyButton(0, ONE_SECOND * 3, 1);
        if GlobData.Game_state.CurrentActivity & CHECK_ABORT as u16 != 0 {
            return;
        }
        GlobData.Game_state.CurrentActivity &= !(CHECK_ABORT as u16);
        SleepThreadUntil(FadeScreen(
            ScreenFadeType_FadeAllToBlack,
            (ONE_SECOND / 2) as SIZE,
        ));
    }

    /// # Safety
    /// Graphics, audio, resources and input must be initialized on the game thread.
    #[no_mangle]
    pub unsafe extern "C" fn Introduction() {
        ShowPresentation(INTROPRES_STRTAB.as_ptr().cast());
        SleepThreadUntil(FadeScreen(
            ScreenFadeType_FadeAllToBlack,
            (ONE_SECOND / 2) as SIZE,
        ));
    }

    /// # Safety
    /// Graphics, audio, resources and input must be initialized on the game thread.
    #[no_mangle]
    pub unsafe extern "C" fn Victory() {
        SleepThreadUntil(FadeScreen(
            ScreenFadeType_FadeAllToBlack,
            (ONE_SECOND / 2) as SIZE,
        ));
        ShowPresentation(FINALPRES_STRTAB.as_ptr().cast());
        FadeScreen(ScreenFadeType_FadeAllToBlack, 0);
    }
}

#[cfg(any(not(test), feature = "linked_c_archive"))]
pub use runtime::{DoShipSpin, Introduction, SplashScreen, Victory};
