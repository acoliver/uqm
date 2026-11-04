//! Splash, introduction, victory and ship-spin presentations, ported from uqm/fmv.c.

#[cfg(not(test))]
pub(crate) mod runtime {
    use super::super::intro::{runtime::show_presentation, ShowPresentation};
    use crate::engine::{battle::Cx, observe::Io};
    use crate::engine::{Kernel, WorldData};
    use crate::navigation::abi::*;

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
            stop_music();
        }
        free_hyper_data();
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
        SetContext(Kernel::current().read(|owner| owner.screen_context));
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
        if WorldData::current().read(|owner| owner.game.current_activity) & CHECK_ABORT as u16 != 0
        {
            return;
        }
        WaitForAnyButton(0, ONE_SECOND * 3, 1);
        if WorldData::current().read(|owner| owner.game.current_activity) & CHECK_ABORT as u16 != 0
        {
            return;
        }
        (**WorldData::current()).game.current_activity &= !(CHECK_ABORT as u16);
        SleepThreadUntil(FadeScreen(
            ScreenFadeType_FadeAllToBlack,
            (ONE_SECOND / 2) as SIZE,
        ));
    }

    pub fn introduction(cx: &mut Cx<'_>, io: &mut Io<'_>) {
        // SAFETY: the registered introduction resource is used on the initialized game thread.
        unsafe {
            show_presentation(cx, io, INTROPRES_STRTAB.as_ptr().cast());
        }
        // SAFETY: the presentation has completed before its final fade is drained.
        cx.with_legacy(|| unsafe {
            SleepThreadUntil(FadeScreen(
                ScreenFadeType_FadeAllToBlack,
                (ONE_SECOND / 2) as SIZE,
            ));
        });
    }

    pub fn victory(cx: &mut Cx<'_>, io: &mut Io<'_>) {
        // SAFETY: the initialized game thread owns the fade transaction.
        cx.with_legacy(|| unsafe {
            SleepThreadUntil(FadeScreen(
                ScreenFadeType_FadeAllToBlack,
                (ONE_SECOND / 2) as SIZE,
            ));
        });
        // SAFETY: the registered victory resource remains live throughout the presentation.
        unsafe {
            show_presentation(cx, io, FINALPRES_STRTAB.as_ptr().cast());
        }
        // SAFETY: the presentation has completed before the final fade is applied.
        cx.with_legacy(|| unsafe {
            FadeScreen(ScreenFadeType_FadeAllToBlack, 0);
        });
    }
}

#[cfg(not(test))]
pub use runtime::{introduction, victory, DoShipSpin, SplashScreen};
