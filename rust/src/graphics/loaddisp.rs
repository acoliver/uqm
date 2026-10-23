//! Display capture front-end, ported from loaddisp.c (GPL-2.0-or-later).
use super::native_abi::*;
use super::pixmap_abi::*;

/// # Safety
/// Area must be valid; frame must be null or a live drawable frame.
/// Called on the graphics thread with a current foreground frame.
#[no_mangle]
pub unsafe extern "C" fn LoadDisplayPixmap(
    area: *const RECT,
    mut frame: *mut frame_desc,
) -> *mut drawable_desc {
    let mut buffer = GetFrameParentDrawable(frame);
    let index = if buffer.is_null() {
        buffer = CreateDrawable(2 | 4, (*area).extent.width, (*area).extent.height, 1);
        if buffer.is_null() {
            return buffer;
        }
        0
    } else {
        GetFrameIndex(frame)
    };
    frame = SetAbsFrameIndex(CaptureDrawable(buffer), index);
    if (*_CurFramePtr).Type != SCREEN_DRAWABLE as u16
        || (*frame).Type == SCREEN_DRAWABLE as u16
        || (*GetFrameParentDrawable(frame)).Flags & 4 == 0
    {
        log::warn!("Unimplemented function activated: LoadDisplayPixmap()");
    } else {
        TFB_DrawScreen_CopyToImage((*frame).image, area, SCREEN_TFB_SCREEN_MAIN);
    }
    ReleaseDrawable(frame);
    buffer
}
