//! Oscilloscope and playback slider, ported from oscill.c (GPL-2.0-or-later).

fn button_y(y: i32, button_height: i16, slider_height: i16) -> i16 {
    (y - (i32::from(button_height) - i32::from(slider_height)) / 2) as i16
}

pub(crate) mod runtime {
    use super::button_y;
    use crate::comm::locdata::{CColor, CPoint, CStamp};
    use crate::comm::Comm;

    use crate::graphics::context_abi::SetContextClipRect;
    use crate::graphics::drawable_abi::SetContextFGFrame;
    use crate::graphics::native_abi::frame_desc;

    use std::ptr::null_mut;

    use crate::graphics::native_abi::line as Line;

    // Opaque handles and local repr(C) graphics records share the engine ABI.

    use crate::graphics::context_abi::SetContext;

    use crate::graphics::context_abi::SetContextForeGroundColor;

    use crate::graphics::drawable_abi::GetFramePixel;

    use crate::graphics::frame_abi::DrawLine;

    use crate::graphics::frame_abi::DrawStamp;

    use crate::sound::heart_ffi::get_track_position as GetTrackPosition;

    use crate::sound::heart_ffi::GraphForegroundStream;

    use crate::graphics::gfx_common_abi::{BatchGraphics, UnbatchGraphics};

    const EMPTY_STAMP: CStamp = CStamp {
        origin: CPoint { x: 0, y: 0 },
        frame: null_mut(),
    };

    pub(crate) struct RuntimeState {
        pub(crate) scope_frame: *mut frame_desc,
        pub(crate) scope_init: bool,
        pub(crate) scope_work: *mut frame_desc,
        pub(crate) scope_color: CColor,
        pub(crate) scope_width: i16,
        pub(crate) scope_height: i16,
        pub(crate) slider: CStamp,
        pub(crate) button: CStamp,
        pub(crate) slider_changed: bool,
        pub(crate) last_offset: i32,
        pub(crate) slider_space: i32,
    }
    impl Default for RuntimeState {
        fn default() -> Self {
            Self {
                scope_frame: null_mut(),
                scope_init: false,
                scope_work: null_mut(),
                scope_color: CColor {
                    r: 0,
                    g: 0,
                    b: 0,
                    a: 0,
                },
                scope_width: 0,
                scope_height: 0,
                slider: EMPTY_STAMP,
                button: EMPTY_STAMP,
                slider_changed: false,
                last_offset: -1,
                slider_space: 0,
            }
        }
    }
    /// # Safety
    /// Called on the graphics thread with a live background frame.
    pub unsafe fn init_oscilloscope(comm: &mut Comm, background: *mut frame_desc) {
        comm.scope.scope_frame = background;
        if !comm.scope.scope_init {
            let size = (*background).Bounds;
            comm.scope.scope_color = GetFramePixel(
                background,
                CPoint {
                    x: size.width / 2,
                    y: size.height / 2,
                },
            );
            comm.scope.scope_color.a = 255;
            // The retained graphics owner preserves reuse and the native null result on allocation failure.
            comm.scope.scope_work = comm
                .presentation
                .scope_buffer
                .get_or_init(|| {
                    crate::graphics::drawable_abi::LoadedDrawable::create(
                        6,
                        size.width,
                        size.height,
                        1,
                    )
                })
                .as_ref()
                .and_then(|drawable| drawable.lease().frame(0))
                .unwrap_or(null_mut());
            comm.scope.scope_width = size.width - 2;
            comm.scope.scope_height = size.height - 2;
            comm.scope.scope_init = true;
        }
    }

    /// # Safety
    /// Oscilloscope and graphics contexts must be initialized.
    pub unsafe fn draw_oscilloscope(comm: &mut Comm) {
        if !comm.options.oscilloscope {
            return;
        }
        let mut data = [0u8; 128];
        assert!(comm.scope.scope_width as usize <= data.len());
        assert!(comm.scope.scope_height < 256);
        let mut stamp = EMPTY_STAMP;
        if GraphForegroundStream(
            data.as_mut_ptr(),
            comm.scope.scope_width.into(),
            comm.scope.scope_height.into(),
            comm.presentation.using_speech as i32,
        ) != 0
        {
            let old = SetContext(comm.presentation.off_screen_context);
            SetContextFGFrame(comm.scope.scope_work);
            SetContextClipRect(std::ptr::null_mut());
            stamp.frame = comm.scope.scope_frame.cast();
            DrawStamp((&raw mut stamp).cast());
            SetContextForeGroundColor(comm.scope.scope_color);
            for i in 0..i32::from(comm.scope.scope_width) - 1 {
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
            stamp.frame = comm.scope.scope_work.cast();
        } else {
            stamp.frame = comm.scope.scope_frame.cast();
        }
        DrawStamp((&raw mut stamp).cast());
    }

    /// # Safety
    /// Frames must be live; called only on the graphics thread.
    pub unsafe fn init_slider(
        comm: &mut Comm,
        x: i32,
        y: i32,
        width: i32,
        slider: *mut frame_desc,
        button: *mut frame_desc,
    ) {
        let slider_size = (*slider).Bounds;
        let button_size = (*button).Bounds;
        comm.scope.slider = CStamp {
            origin: CPoint {
                x: x as i16,
                y: y as i16,
            },
            frame: slider.cast(),
        };
        comm.scope.button = CStamp {
            origin: CPoint {
                x: x as i16,
                y: button_y(y, button_size.height, slider_size.height),
            },
            frame: button.cast(),
        };
        comm.scope.slider_space = width - i32::from(button_size.width);
    }

    /// # Safety
    /// Frame must remain live until replaced; called on the graphics thread.
    pub unsafe fn set_slider_image(comm: &mut Comm, frame: *mut frame_desc) {
        comm.scope.slider_changed = true;
        comm.scope.button.frame = frame.cast();
    }

    /// # Safety
    /// Slider and graphics context must be initialized.
    pub unsafe fn draw_slider(comm: &mut Comm) {
        if !comm.options.slider {
            return;
        }
        let offset = GetTrackPosition(comm.scope.slider_space);
        if offset != comm.scope.last_offset || comm.scope.slider_changed {
            comm.scope.slider_changed = false;
            comm.scope.last_offset = offset;
            comm.scope.button.origin.x = (i32::from(comm.scope.slider.origin.x) + offset) as i16;
            BatchGraphics();
            DrawStamp((&raw mut comm.scope.slider).cast());
            DrawStamp((&raw mut comm.scope.button).cast());
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

#[cfg(test)]
mod owner_tests {
    use super::runtime::*;
    use crate::collections::observation::{self, Event};
    use crate::comm::Comm;
    use crate::graphics::{
        drawable_abi::{CreateDrawable, DestroyDrawable},
        pixmap_abi::{CaptureDrawable, ReleaseDrawable},
    };

    #[test]
    #[serial_test::serial]
    fn scope_buffer_survives_two_conversations_until_kernel_drop() {
        use crate::graphics::{ffi::SDL_Surface, sdl::lifecycle_abi};
        struct Format(*mut SDL_Surface, *mut SDL_Surface);
        impl Drop for Format {
            fn drop(&mut self) {
                // SAFETY: the serialized fixture restores borrowed surface globals before their owners drop.
                unsafe {
                    lifecycle_abi::format_conv_surf = self.0;
                    lifecycle_abi::SDL_Screen = self.1;
                }
            }
        }
        let surface =
            sdl2::surface::Surface::new(8, 6, sdl2::pixels::PixelFormatEnum::RGBA32).unwrap();
        let screen =
            sdl2::surface::Surface::new(8, 6, sdl2::pixels::PixelFormatEnum::RGB888).unwrap();
        observation::enable();
        observation::drain();
        // SAFETY: both SDL surfaces remain owned by this serialized fixture until after global restoration.
        unsafe {
            let _restore = Format(lifecycle_abi::format_conv_surf, lifecycle_abi::SDL_Screen);
            lifecycle_abi::format_conv_surf = surface.raw().cast();
            lifecycle_abi::SDL_Screen = screen.raw().cast();
            let background = CreateDrawable(6, 8, 6, 1);
            assert!(!background.is_null());
            let frame = CaptureDrawable(background);
            let kernel = crate::engine::Kernel::new();
            let mut comm = Comm::new();
            comm.presentation = crate::comm::presentation::Presentation::from(&kernel);
            init_oscilloscope(&mut comm, frame);
            init_slider(&mut comm, 3, 4, 20, frame, frame);
            set_slider_image(&mut comm, frame);
            assert_eq!(comm.scope.slider.origin.x, 3);
            assert_eq!(comm.scope.slider_space, 12);
            assert!(comm.scope.slider_changed);
            let work = ReleaseDrawable(comm.scope.scope_work) as usize;
            assert_ne!(work, background as usize);
            let moved = Box::new(comm);
            assert!(moved.scope.scope_init);
            drop(moved);
            assert!(observation::inspect_pending(|events| events
                .iter()
                .all(|event| !matches!(event, Event::DrawableFree(_)))));
            let mut next = Comm::new();
            next.presentation = crate::comm::presentation::Presentation::from(&kernel);
            init_oscilloscope(&mut next, frame);
            assert_eq!(ReleaseDrawable(next.scope.scope_work) as usize, work);
            drop(next);
            assert!(observation::inspect_pending(|events| events
                .iter()
                .all(|event| !matches!(event, Event::DrawableFree(_)))));
            drop(kernel);
            let frees = observation::inspect_pending(|events| {
                events
                    .iter()
                    .filter_map(|event| match event {
                        Event::DrawableFree(address) => Some(*address),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
            });
            assert_eq!(frees, [work]);
            DestroyDrawable(background);
        }
        observation::drain();
    }
}
