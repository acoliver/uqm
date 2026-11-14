//! Pixel access through retained, graphics-thread-owned frames.

use super::{context_abi::ContextFrame, drawable_abi, native_abi::*};

enum Pixels<'a> {
    Read(&'a mut [Color]),
    Write(&'a [Color]),
    Indexes(&'a mut [u8]),
}

impl ContextFrame {
    fn pixels(&self, pixels: Pixels<'_>, width: i32, height: i32) {
        let frame = self.as_raw();
        assert!(!frame.is_null(), "absent frame has no pixels");
        let count = usize::try_from(width)
            .unwrap_or_else(|_| panic!("negative pixel width: {width}"))
            .checked_mul(
                usize::try_from(height)
                    .unwrap_or_else(|_| panic!("negative pixel height: {height}")),
            )
            .unwrap_or_else(|| panic!("pixel buffer size overflow"));
        let length = match &pixels {
            Pixels::Read(buffer) => buffer.len(),
            Pixels::Write(buffer) => buffer.len(),
            Pixels::Indexes(buffer) => buffer.len(),
        };
        assert!(
            length >= count,
            "pixel buffer is shorter than its stride and height"
        );
        // SAFETY: this registered lease retains the descriptor and canvas on their owner thread.
        // The validated buffer covers the complete requested stride; native operations clip to the image.
        unsafe { copy_pixels(frame, pixels, width, height) }
    }

    pub fn read_colors(&self, pixels: &mut [Color], width: i32, height: i32) {
        self.pixels(Pixels::Read(pixels), width, height);
    }

    pub fn write_colors(&self, pixels: &[Color], width: i32, height: i32) {
        self.pixels(Pixels::Write(pixels), width, height);
    }

    pub fn read_indexes(&self, pixels: &mut [u8], width: i32, height: i32) {
        self.pixels(Pixels::Indexes(pixels), width, height);
    }
}

// Called only after frame admission and buffer-size validation above.
unsafe fn copy_pixels(frame: FRAME, pixels: Pixels<'_>, width: i32, height: i32) {
    // SAFETY: admission retains this descriptor; these copied fields do not borrow its pixels.
    let (kind, image) = unsafe { ((*frame).Type, (*frame).image) };
    assert_ne!(
        kind, SCREEN_DRAWABLE as u16,
        "screen frames require readback"
    );
    assert!(!image.is_null(), "frame has no pixel image");
    let result = match pixels {
        Pixels::Read(buffer) => {
            // SAFETY: the caller validated the complete output stride and retained this image.
            unsafe { drawable_abi::ReadFramePixelColors(frame, buffer.as_mut_ptr(), width, height) }
        }
        Pixels::Write(buffer) => {
            // SAFETY: the caller validated the complete input stride and retained this image.
            unsafe { drawable_abi::WriteFramePixelColors(frame, buffer.as_ptr(), width, height) }
        }
        Pixels::Indexes(buffer) => {
            // SAFETY: the caller validated the complete output stride and retained this image.
            unsafe {
                drawable_abi::ReadFramePixelIndexes(frame, buffer.as_mut_ptr(), width, height)
            }
        }
    };
    assert_ne!(result, 0, "frame pixel operation failed");
}
