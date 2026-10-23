//! Restart-menu rendering, ported from restart.c (GPL-2.0-or-later).
#![allow(non_snake_case)]
#[cfg(any(not(test), feature = "linked_c_archive"))]
use crate::ships::native::abi::*;

/// # Safety
/// Menu and its frame are live; the SDL game thread owns the current context.
#[cfg(any(not(test), feature = "linked_c_archive"))]
pub unsafe fn DrawRestartMenuGraphic(menu: *mut MENU_STATE) {
    let mut rect: RECT = std::mem::zeroed();
    GetFrameRect((*menu).CurFrame, &mut rect);
    let mut stamp = STAMP {
        frame: (*menu).CurFrame,
        origin: POINT {
            x: centered_coordinate(ScreenWidth, rect.extent.width),
            y: centered_coordinate(ScreenHeight, rect.extent.height),
        },
    };
    SetContextBackGroundColor(Color {
        r: 0,
        g: 0,
        b: 0,
        a: 255,
    });
    BatchGraphics();
    ClearDrawable();
    crate::graphics::cmap_abi::FlushColorXForms();
    DrawStamp(&mut stamp);
    SetContextFont(TinyFont);
    let version = std::ffi::CString::new(concat!("v", env!("CARGO_PKG_VERSION"))).unwrap();
    let mut text = TEXT {
        baseline: POINT {
            x: (ScreenWidth - 3) as i16,
            y: (ScreenHeight - 2) as i16,
        },
        pStr: version.as_ptr(),
        align: crate::graphics::native_abi::TEXT_ALIGN_ALIGN_RIGHT,
        CharCount: u16::MAX,
    };
    SetContextForeGroundColor(Color {
        r: 255,
        g: 255,
        b: 255,
        a: 255,
    });
    font_DrawText(&mut text);
    UnbatchGraphics();
}

/// # Safety
/// The menu's flash context and frame must be live on the graphics owner thread.
#[cfg(any(not(test), feature = "linked_c_archive"))]
pub unsafe fn DrawRestartMenu(menu: *mut MENU_STATE, state: u8, frame: FRAME) {
    let origin = crate::navigation::abi::POINT { x: 0, y: 0 };
    crate::ui::flash::Flash_setOverlay(
        (*menu).flashContext.cast(),
        &origin,
        SetAbsFrameIndex(frame, u16::from(state) + 1).cast(),
    );
}

#[cfg(any(not(test), feature = "linked_c_archive"))]
pub fn StartGame() -> u32 {
    super::orchestration::rust_start_game() as u32
}

fn centered_coordinate(screen: i32, image: i16) -> i16 {
    ((screen - i32::from(image)) >> 1) as i16
}

#[cfg(test)]
mod tests {
    use super::centered_coordinate;

    #[test]
    fn centers_restart_frame_with_original_signed_shift_rounding() {
        assert_eq!(centered_coordinate(320, 320), 0);
        assert_eq!(centered_coordinate(320, 301), 9);
        assert_eq!(centered_coordinate(200, 203), -2);
    }
}
