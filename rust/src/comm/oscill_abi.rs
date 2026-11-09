//! Oscilloscope and playback slider, ported from oscill.c (GPL-2.0-or-later).

fn button_y(y: i32, button_height: i16, slider_height: i16) -> i16 {
    (y - (i32::from(button_height) - i32::from(slider_height)) / 2) as i16
}

pub(crate) mod runtime {
    use super::button_y;
    use crate::comm::locdata::{CColor, CPoint, CStamp};
    use crate::comm::Comm;
    use crate::graphics::context_abi::{SetContext, SetContextClipRect, SetContextForeGroundColor};
    use crate::graphics::drawable_abi::{CreateDrawable, GetFramePixel, SetContextFGFrame};
    use crate::graphics::frame_abi::{DrawLine, DrawStamp};
    use crate::graphics::gfx_common_abi::{BatchGraphics, UnbatchGraphics};
    use crate::graphics::native_abi::{frame_desc, line as Line};
    use crate::graphics::pixmap_abi::CaptureDrawable;
    use crate::sound::heart_ffi::get_track_position as GetTrackPosition;
    use crate::sound::heart_ffi::GraphForegroundStream;
    use std::ptr::null_mut;

    const EMPTY_STAMP: CStamp = CStamp {
        origin: CPoint { x: 0, y: 0 },
        frame: null_mut(),
    };
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
    static mut SLIDER: CStamp = EMPTY_STAMP;
    static mut BUTTON: CStamp = EMPTY_STAMP;
    static mut SLIDER_CHANGED: bool = false;
    static mut LAST_OFFSET: i32 = -1;
    static mut SLIDER_SPACE: i32 = 0;

    /// # Safety
    /// Called on the graphics thread with a live background frame.
    pub unsafe fn init_oscilloscope(background: *mut frame_desc) {
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
    /// Oscilloscope and graphics contexts must be initialized.
    pub unsafe fn draw_oscilloscope(comm: &mut Comm) {
        if !comm.options.oscilloscope {
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
            comm.presentation.using_speech as i32,
        ) != 0
        {
            let old = SetContext(comm.presentation.off_screen_context);
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
    pub unsafe fn init_slider(
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
    pub unsafe fn set_slider_image(frame: *mut frame_desc) {
        SLIDER_CHANGED = true;
        BUTTON.frame = frame.cast();
    }

    /// # Safety
    /// Slider and graphics context must be initialized.
    pub unsafe fn draw_slider(comm: &mut Comm) {
        if !comm.options.slider {
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

    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::graphics::{
            drawable_abi::{DestroyDrawable, LoadedDrawable},
            pixmap_abi::ReleaseDrawable,
            sdl::lifecycle_abi,
        };

        #[test]
        #[serial_test::serial]
        fn legacy_widget_initialization_preserves_cache_and_slider_geometry(
        ) -> Result<(), Box<dyn std::error::Error>> {
            struct Restore(
                *mut crate::graphics::ffi::SDL_Surface,
                *mut crate::graphics::ffi::SDL_Surface,
            );
            impl Drop for Restore {
                fn drop(&mut self) {
                    // SAFETY: only this serialized fixture initializes the legacy widget cache.
                    unsafe {
                        DestroyDrawable(ReleaseDrawable(SCOPE_WORK));
                        SCOPE_FRAME = null_mut();
                        SCOPE_WORK = null_mut();
                        SCOPE_INIT = false;
                        SCOPE_COLOR = CColor {
                            r: 0,
                            g: 0,
                            b: 0,
                            a: 0,
                        };
                        SCOPE_WIDTH = 0;
                        SCOPE_HEIGHT = 0;
                        SLIDER = EMPTY_STAMP;
                        BUTTON = EMPTY_STAMP;
                        SLIDER_CHANGED = false;
                        LAST_OFFSET = -1;
                        SLIDER_SPACE = 0;
                        lifecycle_abi::format_conv_surf = self.0;
                        lifecycle_abi::SDL_Screen = self.1;
                    }
                }
            }
            let format = sdl2::surface::Surface::new(8, 6, sdl2::pixels::PixelFormatEnum::RGBA32)?;
            let screen = sdl2::surface::Surface::new(8, 6, sdl2::pixels::PixelFormatEnum::RGB888)?;
            // SAFETY: the fixture owns both surfaces and restores the borrowed globals before they drop.
            unsafe {
                assert!(!{ SCOPE_INIT });
                let restore = Restore(lifecycle_abi::format_conv_surf, lifecycle_abi::SDL_Screen);
                lifecycle_abi::format_conv_surf = format.raw().cast();
                lifecycle_abi::SDL_Screen = screen.raw().cast();
                let background = LoadedDrawable::create(6, 8, 6, 1).ok_or("missing background")?;
                let frame = background.lease().frame(0).ok_or("missing frame")?;
                init_oscilloscope(frame);
                let work = SCOPE_WORK;
                assert!(!work.is_null());
                init_oscilloscope(frame);
                assert_eq!((SCOPE_WORK, SCOPE_WIDTH, SCOPE_HEIGHT), (work, 6, 4));
                init_slider(3, 4, 20, frame, frame);
                set_slider_image(frame);
                assert_eq!((SLIDER.origin.x, BUTTON.origin.y, SLIDER_SPACE), (3, 4, 12));
                assert!({ SLIDER_CHANGED });
                let mut disabled = Comm::with_options(crate::comm::CommOptions {
                    oscilloscope: false,
                    slider: false,
                    ..Default::default()
                });
                // Disabled widgets must not inspect an unusable waveform cache or consume the slider update.
                SCOPE_WIDTH = -1;
                draw_oscilloscope(&mut disabled);
                draw_slider(&mut disabled);
                assert_eq!((LAST_OFFSET, SLIDER_CHANGED), (-1, true));
                drop(restore);
            }
            Ok(())
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
