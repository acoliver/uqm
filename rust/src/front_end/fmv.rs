//! Splash, introduction, victory and ship-spin presentations, ported from uqm/fmv.c.

#[cfg(not(test))]
pub(crate) mod runtime {
    use super::super::intro::runtime::show_presentation;
    use crate::engine::{battle::Cx, observe::Io};
    use crate::navigation::abi::*;
    use crate::ui::util::wait_for_any_button;

    use std::ptr;

    // uqm/fmv.h defines WANT_SHIP_SPINS unconditionally in this configuration.
    const WANT_SHIP_SPINS: bool = true;

    /// # Safety
    /// Game-thread legacy boundary with the caller's context published and
    /// `music` null or a live music reference.
    #[no_mangle]
    pub unsafe extern "C" fn DoShipSpin(index: COUNT, music: MUSIC_REF) {
        crate::app::bridge::enter(|cx| {
            let mut observer = cx.kernel.input_runtime().observer();
            let mut io = Io {
                observer: &mut *observer,
            };
            ship_spin(cx, &mut io, index, music);
        });
    }

    pub fn do_ship_spin(cx: &mut Cx<'_>, io: &mut Io<'_>, index: COUNT) {
        // SAFETY: Cx retains the lifecycle; this entry supplies no borrowed raw music resource.
        unsafe { ship_spin(cx, io, index, ptr::null_mut()) }
    }

    unsafe fn ship_spin(cx: &mut Cx<'_>, io: &mut Io<'_>, index: COUNT, music: MUSIC_REF) {
        if !WANT_SHIP_SPINS {
            return;
        }
        let mut name = [0 as std::ffi::c_char; 32];
        cx.with_legacy(|| {
            LoadIntoExtraScreen(ptr::null_mut());
            if !music.is_null() {
                stop_music();
            }
        });
        crate::navigation::hyper_data::free_hyper_data(cx.kernel);
        cx.with_legacy(|| {
            libc::sprintf(
                name.as_mut_ptr(),
                c"slides.spins.%02u".as_ptr(),
                index as u32,
            );
        });
        show_presentation(cx, io, name.as_ptr());
        let music_enabled = cx.world.data.game.glob_flags & 8 == 0;
        cx.with_legacy(|| {
            let mut old_rect: RECT = std::mem::zeroed();
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
                crate::sound::music::remember_native_music(
                    std::ptr::NonNull::new(music.cast()),
                    true,
                    1,
                    music_enabled,
                );
            }
            SleepThreadUntil(FadeScreen(
                ScreenFadeType_FadeAllToColor,
                (ONE_SECOND / 4) as SIZE,
            ));
            FlushColorXForms();
        });
    }

    /// # Safety
    /// The game systems must be initialized on the game thread. A supplied
    /// processing callback must remain valid throughout this call.
    pub unsafe fn splash_screen(
        cx: &mut Cx<'_>,
        io: &mut Io<'_>,
        process: Option<unsafe fn(&mut Cx<'_>, DWORD)>,
    ) {
        let screen_context = cx.kernel.screen_context;
        let timeout = cx.with_legacy(|| {
            SleepThreadUntil(FadeScreen(
                ScreenFadeType_FadeAllToBlack,
                (ONE_SECOND / 120) as SIZE,
            ));
            SetContext(screen_context);
            let mut stamp = STAMP {
                origin: POINT { x: 0, y: 0 },
                frame: CaptureDrawable(LoadGraphicInstance(TITLE_ANIM.as_ptr().cast()).cast()),
            };
            DrawStamp(&mut stamp);
            DestroyDrawable(ReleaseDrawable(stamp.frame));
            FadeScreen(ScreenFadeType_FadeAllToColor, (ONE_SECOND / 2) as SIZE)
        });
        if let Some(callback) = process {
            callback(cx, timeout);
        }
        if cx.world.activity() & CHECK_ABORT as u16 != 0 {
            return;
        }
        wait_for_any_button(cx, io, 0, ONE_SECOND * 3, 1);
        if cx.world.activity() & CHECK_ABORT as u16 != 0 {
            return;
        }
        cx.world
            .set_activity(cx.world.activity() & !(CHECK_ABORT as u16));
        cx.with_legacy(|| {
            SleepThreadUntil(FadeScreen(
                ScreenFadeType_FadeAllToBlack,
                (ONE_SECOND / 2) as SIZE,
            ));
        });
    }

    /// # Safety
    /// Graphics, audio, resources and input must be initialized on the game thread.
    pub unsafe fn introduction(cx: &mut Cx<'_>, io: &mut Io<'_>) {
        show_presentation(cx, io, INTROPRES_STRTAB.as_ptr().cast());
        cx.with_legacy(|| {
            SleepThreadUntil(FadeScreen(
                ScreenFadeType_FadeAllToBlack,
                (ONE_SECOND / 2) as SIZE,
            ));
        });
    }

    /// # Safety
    /// Graphics, audio, resources and input must be initialized on the game thread.
    pub unsafe fn victory(cx: &mut Cx<'_>, io: &mut Io<'_>) {
        cx.with_legacy(|| {
            SleepThreadUntil(FadeScreen(
                ScreenFadeType_FadeAllToBlack,
                (ONE_SECOND / 2) as SIZE,
            ));
        });
        show_presentation(cx, io, FINALPRES_STRTAB.as_ptr().cast());
        cx.with_legacy(|| {
            FadeScreen(ScreenFadeType_FadeAllToBlack, 0);
        });
    }
}

#[cfg(not(test))]
pub use runtime::{do_ship_spin, introduction, splash_screen, victory, DoShipSpin};
