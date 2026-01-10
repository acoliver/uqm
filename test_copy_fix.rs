// Standalone test to verify copy_canvas clipping logic
// This can be run with: rustc --edition 2021 test_copy_fix.rs && ./test_copy_fix

use std::fmt;

// Simplified types for testing
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CanvasError;

impl fmt::Display for CanvasError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Canvas error")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CanvasFormat {
    pub bytes_per_pixel: i32,
}

impl CanvasFormat {
    pub const fn rgba() -> Self {
        Self { bytes_per_pixel: 4 }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Extent {
    pub width: i32,
    pub height: i32,
}

impl Extent {
    pub const fn new(width: i32, height: i32) -> Self {
        Self { width, height }
    }
}

struct CanvasInner {
    extent: Extent,
    format: CanvasFormat,
    pixels: Vec<u8>,
}

#[derive(Clone)]
struct Canvas {
    inner: std::rc::Rc<std::cell::RefCell<CanvasInner>>,
}

impl Canvas {
    fn new_rgba(width: i32, height: i32) -> Self {
        let pixel_count = (width * height) as usize;
        let pixels = vec![0u8; pixel_count * 4];
        Self {
            inner: std::rc::Rc::new(std::cell::RefCell::new(CanvasInner {
                extent: Extent::new(width, height),
                format: CanvasFormat::rgba(),
                pixels,
            })),
        }
    }

    fn width(&self) -> i32 {
        self.inner.borrow().extent.width
    }

    fn height(&self) -> i32 {
        self.inner.borrow().extent.height
    }

    fn format(&self) -> CanvasFormat {
        self.inner.borrow().format
    }

    fn pixels(&self) -> Vec<u8> {
        self.inner.borrow().pixels.clone()
    }

    fn with_pixels_mut<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut [u8]) -> R,
    {
        let mut inner = self.inner.borrow_mut();
        f(&mut inner.pixels)
    }

    fn fill_rect(&self, x1: i32, y1: i32, x2: i32, y2: i32, r: u8, g: u8, b: u8) {
        let mut inner = self.inner.borrow_mut();
        let width = inner.extent.width;
        let height = inner.extent.height;
        let (x_start, x_end) = if x1 <= x2 { (x1, x2) } else { (x2, x1) };
        let (y_start, y_end) = if y1 <= y2 { (y1, y2) } else { (y2, y1) };

        let pixels = &mut inner.pixels;

        for y in y_start.max(0).min(height - 1)..=y_end.max(0).min(height - 1) {
            for x in x_start.max(0).min(width - 1)..=x_end.max(0).min(width - 1) {
                let offset = (y * width + x) as usize * 4;
                pixels[offset] = r;
                pixels[offset + 1] = g;
                pixels[offset + 2] = b;
                pixels[offset + 3] = 255;
            }
        }
    }
}

// THE FIXED COPY CANVAS FUNCTION
fn copy_canvas(
    dst: &Canvas,
    src: &Canvas,
    dst_x: i32,
    dst_y: i32,
    src_x: i32,
    src_y: i32,
    width: i32,
    height: i32,
) {
    let mut dst_inner = dst.inner.borrow_mut();
    let src_inner = src.inner.borrow();
    let dst_width = dst_inner.extent.width;
    let dst_height = dst_inner.extent.height;
    let src_width = src_inner.extent.width;
    let src_height = src_inner.extent.height;
    let bytes_per_pixel = dst_inner.format.bytes_per_pixel as usize;

    // Handle default parameters (copy entire source)
    let copy_width = if width <= 0 { src_width } else { width };
    let copy_height = if height <= 0 { src_height } else { height };

    // Define the source region to copy from
    let src_x1 = src_x;
    let src_y1 = src_y;
    let src_x2 = src_x + copy_width;
    let src_y2 = src_y + copy_height;

    // Define the destination region to copy to
    let dst_x1 = dst_x;
    let dst_y1 = dst_y;
    let dst_x2 = dst_x + copy_width;
    let dst_y2 = dst_y + copy_height;

    // Clip source region to source canvas bounds
    let src_clipped_x1 = src_x1.max(0).min(src_width);
    let src_clipped_x2 = src_x2.max(0).min(src_width);
    let src_clipped_y1 = src_y1.max(0).min(src_height);
    let src_clipped_y2 = src_y2.max(0).min(src_height);

    // Clip destination region to destination canvas bounds
    let dst_clipped_x1 = dst_x1.max(0).min(dst_width);
    let dst_clipped_x2 = dst_x2.max(0).min(dst_width);
    let dst_clipped_y1 = dst_y1.max(0).min(dst_height);
    let dst_clipped_y2 = dst_y2.max(0).min(dst_height);

    // Calculate the widths/heights of clipped regions
    let src_clip_w = src_clipped_x2 - src_clipped_x1;
    let src_clip_h = src_clipped_y2 - src_clipped_y1;
    let dst_clip_w = dst_clipped_x2 - dst_clipped_x1;
    let dst_clip_h = dst_clipped_y2 - dst_clipped_y1;

    // The actual copy width/height is the minimum of the clipped dimensions
    let copy_w = src_clip_w.min(dst_clip_w).max(0);
    let copy_h = src_clip_h.min(dst_clip_h).max(0);

    // Early exit if nothing to copy
    if copy_w <= 0 || copy_h <= 0 {
        return;
    }

    // Calculate the offset from the clipped source start to the original source start
    // This is needed when src_x1 is negative (we need to skip those pixels)
    let src_offset_x = src_clipped_x1 - src_x1;
    let src_offset_y = src_clipped_y1 - src_y1;

    // Calculate the offset from the clipped destination start to the original destination start
    // This is needed when dst_x1 is negative (we need to skip those source pixels)
    let dst_offset_x = dst_clipped_x1 - dst_x1;
    let dst_offset_y = dst_clipped_y1 - dst_y1;

    // Calculate actual source position: start from clipped position, then adjust for dst offset
    let src_actual_x = src_clipped_x1 + dst_offset_x;
    let src_actual_y = src_clipped_y1 + dst_offset_y;

    // Get pixel buffers
    let src_pixels = src_inner.pixels.clone();
    let dst_pixels = &mut dst_inner.pixels;

    // Copy row by row
    for y in 0..copy_h {
        let src_y = src_actual_y + y;
        let dst_y = dst_clipped_y1 + y;

        let src_row_offset = (src_y * src_width + src_actual_x) as usize * bytes_per_pixel;
        let dst_row_offset = (dst_y * dst_width + dst_clipped_x1) as usize * bytes_per_pixel;

        let row_bytes = copy_w as usize * bytes_per_pixel;

        // Sanity check to ensure we're within bounds
        let src_end = src_row_offset + row_bytes;
        let dst_end = dst_row_offset + row_bytes;

        if src_end > src_pixels.len() || dst_end > dst_pixels.len() {
            break; // Skip this row if out of bounds
        }

        // Copy this row
        dst_pixels[dst_row_offset..dst_row_offset + row_bytes]
            .copy_from_slice(&src_pixels[src_row_offset..src_row_offset + row_bytes]);
    }
}

fn main() {
    println!("Testing copy_canvas fix...\n");

    // Test 1: Negative dst_y shifts the source correctly
    println!("Test 1: Negative dst_y");
    {
        let src = Canvas::new_rgba(10, 10);
        src.fill_rect(0, 0, 9, 9, 255, 0, 255); // fill entire source with magenta
        let dst = Canvas::new_rgba(10, 10);
        copy_canvas(&dst, &src, 0, -2, 0, 0, 10, 10);
        let pixels = dst.pixels();
        // When copying with dst_y=-2, rows 0-1 of source map to negative (invisible) positions in destination
        // Source row 2 maps to dst row 0, source row 3 maps to dst row 1, etc.
        // Let's check pixel at source (0, 3) -> dest (0, 1)
        let offset = (1 * 10) as usize * 4;
        assert_eq!(pixels[offset], 255, "Pixel at (0,1) should be magenta R=255");
        println!("[OK] Test 1 passed\n");
    }

    // Test 2: Entirely outside destination
    println!("Test 2: Entirely outside destination");
    {
        let src = Canvas::new_rgba(10, 10);
        src.fill_rect(0, 0, 9, 9, 0, 255, 0); // green
        let dst = Canvas::new_rgba(10, 10);
        copy_canvas(&dst, &src, 20, 20, 0, 0, 10, 10);
        let pixels = dst.pixels();
        for i in (0..pixels.len()).step_by(4) {
            assert_eq!(pixels[i], 0, "Pixel should be black");
        }
        println!("[OK] Test 2 passed\n");
    }

    // Test 3: Partial offset copy
    println!("Test 3: Partial offset copy");
    {
        let src = Canvas::new_rgba(10, 10);
        src.fill_rect(5, 5, 7, 7, 255, 255, 0); // yellow at (5,5)-(7,7)
        let dst = Canvas::new_rgba(10, 10);
        // Copy from (5,5) size 6x6 to position (3,3)
        copy_canvas(&dst, &src, 3, 3, 5, 5, 6, 6);
        let pixels = dst.pixels();
        // Source (5,5) maps to dest (3,3)
        let dst_offset = (3 * 10 + 3) as usize * 4;
        assert_eq!(pixels[dst_offset], 255, "R should be 255");
        assert_eq!(pixels[dst_offset + 1], 255, "G should be 255");
        assert_eq!(pixels[dst_offset + 2], 0, "B should be 0");
        println!("[OK] Test 3 passed\n");
    }

    // Test 4: Large source to small destination
    println!("Test 4: Large source to small destination");
    {
        let src = Canvas::new_rgba(20, 20);
        src.fill_rect(5, 5, 6, 6, 0, 0, 255); // blue at (5,5)-(6,6)
        let dst = Canvas::new_rgba(10, 10);
        copy_canvas(&dst, &src, 0, 0, 0, 0, 20, 20);
        let pixels = dst.pixels();
        // Source (5,5) maps to dest (5,5)
        let dst_offset = (5 * 10 + 5) as usize * 4;
        assert_eq!(pixels[dst_offset], 0, "R should be 0");
        assert_eq!(pixels[dst_offset + 1], 0, "G should be 0");
        assert_eq!(pixels[dst_offset + 2], 255, "B should be 255");
        println!("[OK] Test 4 passed\n");
    }

    println!("All tests passed! [OK]");
}
