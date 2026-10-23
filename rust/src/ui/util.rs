//! UI drawing and nested input waits, ported from uqm/util.c.

use crate::navigation::abi::{Color, RECT};

fn box_rectangles(
    rect: RECT,
    mut border: i16,
    top: Color,
    bottom: Color,
    interior: Option<Color>,
    mut draw: impl FnMut(Color, RECT),
) {
    let mut r = rect;
    if border == 0 {
        border = 2;
    } else {
        r.extent.height = 1;
        draw(top, r);
        if border == 2 {
            r.corner.x += 1;
            r.corner.y += 1;
            r.extent.width -= 2;
            draw(top, r);
        }
        r = rect;
        r.extent.width = 1;
        draw(top, r);
        if border == 2 {
            r.corner.x += 1;
            r.corner.y += 1;
            r.extent.height -= 2;
            draw(top, r);
        }
        r.corner.x = rect.corner.x + rect.extent.width - 1;
        r.corner.y = rect.corner.y + 1;
        r.extent.height = rect.extent.height - 1;
        draw(bottom, r);
        if border == 2 {
            r.corner.x -= 1;
            r.corner.y += 1;
            r.extent.height -= 2;
            draw(bottom, r);
        }
        r.corner.x = rect.corner.x;
        r.extent.width = rect.extent.width;
        r.corner.y = rect.corner.y + rect.extent.height - 1;
        r.extent.height = 1;
        draw(bottom, r);
        if border == 2 {
            r.corner.x += 1;
            r.corner.y -= 1;
            r.extent.width -= 2;
            draw(bottom, r);
        }
    }
    if let Some(color) = interior {
        r.corner.x = rect.corner.x + border;
        r.corner.y = rect.corner.y + border;
        r.extent.width = rect.extent.width - (border << 1);
        r.extent.height = rect.extent.height - (border << 1);
        draw(color, r);
    }
}

#[cfg(not(test))]
#[allow(non_snake_case)]
pub(crate) mod live {
    use super::*;
    use crate::navigation::abi::*;
    use crate::navigation::globals::GlobData;
    use std::{mem, ptr};

    /// # Safety
    /// The rectangle must be valid and the graphics context active on the game thread.
    #[no_mangle]
    pub unsafe extern "C" fn DrawStarConBox(
        rect: *mut RECT,
        border: SIZE,
        top: Color,
        bottom: Color,
        fill: BOOLEAN,
        interior: Color,
    ) {
        let mut current = None;
        box_rectangles(
            *rect,
            border,
            top,
            bottom,
            (fill != 0).then_some(interior),
            |color, mut r| {
                let rgba = (color.r, color.g, color.b, color.a);
                if current != Some(rgba) {
                    SetContextForeGroundColor(color);
                    current = Some(rgba);
                }
                DrawFilledRectangle(&mut r);
            },
        );
    }

    /// # Safety
    /// The random generator must be accessed only from the game thread.
    #[no_mangle]
    pub unsafe extern "C" fn SeedRandomNumbers() -> DWORD {
        let now = GetTimeCounter();
        TFB_SeedRandom(now);
        now
    }

    /// # Safety
    /// The active context must have a frame; rect is null or points to a valid rectangle.
    #[no_mangle]
    pub unsafe extern "C" fn SaveContextFrame(rect: *const RECT) -> STAMP {
        STAMP {
            origin: if rect.is_null() {
                POINT { x: 0, y: 0 }
            } else {
                (*rect).corner
            },
            frame: CaptureDrawable(CopyContextRect(rect)),
        }
    }

    /// # Safety
    /// Game-thread access to initialized input, graphics and audio state.
    #[no_mangle]
    pub unsafe extern "C" fn PauseGame() -> BOOLEAN {
        if ActivityFrame.is_null()
            || GlobData.Game_state.CurrentActivity & (CHECK_ABORT | CHECK_PAUSE) as u16 != 0
            || LastActivity & (CHECK_LOAD | CHECK_RESTART) as u16 != 0
        {
            return 0;
        }
        GlobData.Game_state.CurrentActivity |= CHECK_PAUSE as u16;
        if PlayingTrack() != 0 {
            PauseTrack();
        }
        let old_context = SetContext(ScreenContext);
        let old_origin = SetContextOrigin(POINT { x: 0, y: 0 });
        let mut old_rect: RECT = mem::zeroed();
        GetContextClipRect(&mut old_rect);
        SetContextClipRect(ptr::null_mut());
        let mut context_rect: RECT = mem::zeroed();
        GetContextClipRect(&mut context_rect);
        let mut rect: RECT = mem::zeroed();
        GetFrameRect(ActivityFrame, &mut rect);
        rect.corner.x = (context_rect.extent.width - rect.extent.width) >> 1;
        rect.corner.y = (context_rect.extent.height - rect.extent.height) >> 1;
        let mut saved = SaveContextFrame(&rect);
        let mut stamp = STAMP {
            origin: rect.corner,
            frame: ActivityFrame,
        };
        SetSystemRect(&rect);
        DrawStamp(&mut stamp);
        FlushGraphics();
        while ImmediateInputState.menu[KEY_PAUSE as usize] != 0 && GamePaused != 0 {
            BeginInputFrame();
            TaskSwitch();
        }
        while ImmediateInputState.menu[KEY_PAUSE as usize] == 0 && GamePaused != 0 {
            BeginInputFrame();
            TaskSwitch();
        }
        while ImmediateInputState.menu[KEY_PAUSE as usize] != 0 && GamePaused != 0 {
            BeginInputFrame();
            TaskSwitch();
        }
        GamePaused = 0;
        DrawStamp(&mut saved);
        DestroyDrawable(ReleaseDrawable(saved.frame));
        ClearSystemRect();
        SetContextClipRect(&mut old_rect);
        SetContextOrigin(old_origin);
        SetContext(old_context);
        WaitForNoInput(ONE_SECOND / 4, 1);
        if PlayingTrack() != 0 {
            ResumeTrack();
        }
        TaskSwitch();
        GlobData.Game_state.CurrentActivity &= !(CHECK_PAUSE as u16);
        1
    }

    unsafe fn wait_until(released: bool, timeout: TimeCount, reset: BOOLEAN) -> BOOLEAN {
        let mut pressed = AnyButtonPress(1);
        while (pressed != 0) == released
            && (timeout == TimePeriod::MAX || GetTimeCounter() < timeout)
            && GlobData.Game_state.CurrentActivity & CHECK_ABORT as u16 == 0
            && QuitPosted == 0
        {
            if crate::automation::input_ffi::rust_automation_service_boundary() != 0 {
                break;
            }
            SleepThread(ONE_SECOND / 40);
            pressed = AnyButtonPress(1);
        }
        if reset != 0 {
            FlushInput();
        }
        if released {
            u32::from(pressed == 0)
        } else {
            pressed
        }
    }

    /// # Safety
    /// Game-thread access to initialized input and automation state.
    #[no_mangle]
    pub unsafe extern "C" fn WaitForAnyButtonUntil(
        new_button: BOOLEAN,
        timeout: TimeCount,
        reset: BOOLEAN,
    ) -> BOOLEAN {
        if new_button != 0 && WaitForNoInputUntil(timeout, 0) == 0 {
            return 0;
        }
        wait_until(false, timeout, reset)
    }
    /// # Safety
    /// Game-thread access to initialized input and automation state.
    #[no_mangle]
    pub unsafe extern "C" fn WaitForAnyButton(
        new_button: BOOLEAN,
        duration: TimePeriod,
        reset: BOOLEAN,
    ) -> BOOLEAN {
        let timeout = if duration == TimePeriod::MAX {
            duration
        } else {
            duration.wrapping_add(GetTimeCounter())
        };
        WaitForAnyButtonUntil(new_button, timeout, reset)
    }
    /// # Safety
    /// Game-thread access to initialized input and automation state.
    #[no_mangle]
    pub unsafe extern "C" fn WaitForNoInputUntil(timeout: TimeCount, reset: BOOLEAN) -> BOOLEAN {
        wait_until(true, timeout, reset)
    }
    /// # Safety
    /// Game-thread access to initialized input and automation state.
    #[no_mangle]
    pub unsafe extern "C" fn WaitForNoInput(duration: TimePeriod, reset: BOOLEAN) -> BOOLEAN {
        let timeout = if duration == TimePeriod::MAX {
            duration
        } else {
            duration.wrapping_add(GetTimeCounter())
        };
        WaitForNoInputUntil(timeout, reset)
    }
    /// # Safety
    /// Game-thread access to initialized input and audio state.
    #[no_mangle]
    pub unsafe extern "C" fn SleepGame() {
        if QuitPosted != 0 {
            return;
        }
        log::debug!("Game is going to sleep");
        if PlayingTrack() != 0 {
            PauseTrack();
        }
        PauseMusic();
        while GameActive == 0 && QuitPosted == 0 {
            SleepThread(ONE_SECOND / 2);
        }
        log::debug!("Game is waking up");
        WaitForNoInput(ONE_SECOND / 10, 1);
        ResumeMusic();
        if PlayingTrack() != 0 {
            ResumeTrack();
        }
        TaskSwitch();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::navigation::abi::{EXTENT, POINT};

    fn rectangles(border: i16, fill: bool) -> Vec<(u8, i16, i16, i16, i16)> {
        let mut draws = Vec::new();
        let color = |r| Color {
            r,
            g: 0,
            b: 0,
            a: 255,
        };
        box_rectangles(
            RECT {
                corner: POINT { x: 10, y: 20 },
                extent: EXTENT {
                    width: 30,
                    height: 40,
                },
            },
            border,
            color(1),
            color(2),
            fill.then_some(color(3)),
            |c, r| draws.push((c.r, r.corner.x, r.corner.y, r.extent.width, r.extent.height)),
        );
        draws
    }

    #[test]
    fn double_border_preserves_corner_overlap_and_draw_order() {
        assert_eq!(
            rectangles(2, true),
            [
                (1, 10, 20, 30, 1),
                (1, 11, 21, 28, 1),
                (1, 10, 20, 1, 40),
                (1, 11, 21, 1, 38),
                (2, 39, 21, 1, 39),
                (2, 38, 22, 1, 37),
                (2, 10, 59, 30, 1),
                (2, 11, 58, 28, 1),
                (3, 12, 22, 26, 36),
            ]
        );
    }
    #[test]
    fn zero_border_only_fills_with_two_pixel_inset() {
        assert_eq!(rectangles(0, true), [(3, 12, 22, 26, 36)]);
        assert!(rectangles(0, false).is_empty());
    }
    #[test]
    fn single_border_uses_one_pixel_inset() {
        assert_eq!(
            rectangles(1, true),
            [
                (1, 10, 20, 30, 1),
                (1, 10, 20, 1, 40),
                (2, 39, 21, 1, 39),
                (2, 10, 59, 30, 1),
                (3, 11, 21, 28, 38)
            ]
        );
    }
}
