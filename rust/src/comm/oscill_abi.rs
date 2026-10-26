//! Oscilloscope and playback slider, ported from oscill.c (GPL-2.0-or-later).

fn button_y(y: i32, button_height: i16, slider_height: i16) -> i16 {
    (y - (i32::from(button_height) - i32::from(slider_height)) / 2) as i16
}

#[cfg(not(test))]
pub(crate) mod runtime {
    use super::button_y;
    use crate::comm::locdata::{CColor, CPoint, CStamp};
    use crate::graphics::context_abi::SetContextClipRect;
    use crate::graphics::drawable_abi::SetContextFGFrame;
    use crate::graphics::native_abi::frame_desc;

    use std::ptr::null_mut;

    use crate::graphics::native_abi::line as Line;

    // Opaque handles and local repr(C) graphics records share the engine ABI.

    use crate::graphics::context_abi::SetContext;

    use crate::graphics::context_abi::SetContextForeGroundColor;

    use crate::graphics::drawable_abi::CreateDrawable;

    use crate::graphics::drawable_abi::DestroyDrawable;

    use crate::graphics::drawable_abi::GetFramePixel;

    use crate::graphics::frame_abi::DrawLine;

    use crate::graphics::frame_abi::DrawStamp;

    use crate::graphics::pixmap_abi::CaptureDrawable;

    use crate::graphics::pixmap_abi::ReleaseDrawable;
    use crate::sound::heart_ffi::get_track_position as GetTrackPosition;

    use crate::sound::heart_ffi::GraphForegroundStream;

    extern "C" {
        static mut OffScreenContext: *mut crate::graphics::native_abi::context_desc;
        static mut usingSpeech: u32;
    }
    use crate::graphics::gfx_common_abi::{BatchGraphics, UnbatchGraphics};

    static mut SCOPE_FRAME: *mut frame_desc = null_mut();
    static mut SCOPE_INIT: bool = false;
    static mut SCOPE_WORK: *mut frame_desc = null_mut();
    static mut SCOPE_COLOR: CColor = CColor {
        r: 0,
        g: 0,
        b: 0,
        a: 0,
    };
    static mut SCOPE_WIDTH: i16 = 0;
    static mut SCOPE_HEIGHT: i16 = 0;
    const EMPTY_STAMP: CStamp = CStamp {
        origin: CPoint { x: 0, y: 0 },
        frame: null_mut(),
    };
    static mut SLIDER: CStamp = EMPTY_STAMP;
    static mut BUTTON: CStamp = EMPTY_STAMP;
    static mut SLIDER_CHANGED: bool = false;
    static mut LAST_OFFSET: i32 = -1;
    #[no_mangle]
    pub static mut oscillDisabled: i32 = 0;
    #[no_mangle]
    pub static mut sliderDisabled: i32 = 0;
    pub static mut SLIDER_SPACE: i32 = 0;

    /// # Safety
    /// Called on the graphics thread with a live background frame.
    #[no_mangle]
    pub unsafe extern "C" fn InitOscilloscope(background: *mut frame_desc) {
        SCOPE_FRAME = background;
        if !SCOPE_INIT {
            let size = (*background).Bounds;
            SCOPE_COLOR = GetFramePixel(
                background,
                CPoint {
                    x: size.width / 2,
                    y: size.height / 2,
                },
            );
            SCOPE_COLOR.a = 255;
            SCOPE_WORK = CaptureDrawable(CreateDrawable(6, size.width, size.height, 1));
            SCOPE_WIDTH = size.width - 2;
            SCOPE_HEIGHT = size.height - 2;
            SCOPE_INIT = true;
        }
    }

    /// # Safety
    /// Called on the graphics thread, with no concurrent drawing.
    #[no_mangle]
    pub unsafe extern "C" fn UninitOscilloscope() {
        DestroyDrawable(ReleaseDrawable(SCOPE_WORK));
        SCOPE_WORK = null_mut();
        SCOPE_INIT = false;
    }

    /// # Safety
    /// Oscilloscope and graphics contexts must be initialized.
    #[no_mangle]
    pub unsafe extern "C" fn DrawOscilloscope() {
        if oscillDisabled != 0 {
            return;
        }
        let mut data = [0u8; 128];
        assert!(SCOPE_WIDTH as usize <= data.len());
        assert!(SCOPE_HEIGHT < 256);
        let mut stamp = EMPTY_STAMP;
        if GraphForegroundStream(
            data.as_mut_ptr(),
            SCOPE_WIDTH.into(),
            SCOPE_HEIGHT.into(),
            usingSpeech as i32,
        ) != 0
        {
            let old = SetContext(OffScreenContext);
            SetContextFGFrame(SCOPE_WORK);
            SetContextClipRect(std::ptr::null_mut());
            stamp.frame = SCOPE_FRAME.cast();
            DrawStamp((&raw mut stamp).cast());
            SetContextForeGroundColor(SCOPE_COLOR);
            for i in 0..i32::from(SCOPE_WIDTH) - 1 {
                DrawLine(&mut Line {
                    first: CPoint {
                        x: (i + 1) as i16,
                        y: i16::from(data[i as usize]) + 1,
                    },
                    second: CPoint {
                        x: (i + 2) as i16,
                        y: i16::from(data[i as usize + 1]) + 1,
                    },
                });
            }
            SetContext(old);
            stamp.frame = SCOPE_WORK.cast();
        } else {
            stamp.frame = SCOPE_FRAME.cast();
        }
        DrawStamp((&raw mut stamp).cast());
    }

    /// # Safety
    /// Frames must be live; called only on the graphics thread.
    #[no_mangle]
    pub unsafe extern "C" fn InitSlider(
        x: i32,
        y: i32,
        width: i32,
        slider: *mut frame_desc,
        button: *mut frame_desc,
    ) {
        let slider_size = (*slider).Bounds;
        let button_size = (*button).Bounds;
        SLIDER = CStamp {
            origin: CPoint {
                x: x as i16,
                y: y as i16,
            },
            frame: slider.cast(),
        };
        BUTTON = CStamp {
            origin: CPoint {
                x: x as i16,
                y: button_y(y, button_size.height, slider_size.height),
            },
            frame: button.cast(),
        };
        SLIDER_SPACE = width - i32::from(button_size.width);
    }

    /// # Safety
    /// Frame must remain live until replaced; called on the graphics thread.
    #[no_mangle]
    pub unsafe extern "C" fn SetSliderImage(frame: *mut frame_desc) {
        SLIDER_CHANGED = true;
        BUTTON.frame = frame.cast();
    }

    /// # Safety
    /// Slider and graphics context must be initialized.
    #[no_mangle]
    pub unsafe extern "C" fn DrawSlider() {
        if sliderDisabled != 0 {
            return;
        }
        let offset = GetTrackPosition(SLIDER_SPACE);
        if offset != LAST_OFFSET || SLIDER_CHANGED {
            SLIDER_CHANGED = false;
            LAST_OFFSET = offset;
            BUTTON.origin.x = (i32::from(SLIDER.origin.x) + offset) as i16;
            BatchGraphics();
            DrawStamp((&raw mut SLIDER).cast());
            DrawStamp((&raw mut BUTTON).cast());
            UnbatchGraphics();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn slider_button_centering_uses_c_truncation() {
        assert_eq!(button_y(107, 15, 10), 105);
        assert_eq!(button_y(107, 5, 10), 109);
        assert_eq!(button_y(107, 10, 10), 107);
    }
}
