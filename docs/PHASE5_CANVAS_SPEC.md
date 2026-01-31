# Phase 5: Canvas Drawing Primitives Port to Rust

## Overview
Port `sc2/src/libs/graphics/sdl/canvas.c` (2176 lines) to Rust. The canvas system provides drawing primitives (lines, rectangles, blits) used throughout the game.

## C Source Files
- `sc2/src/libs/graphics/sdl/canvas.c` - Main implementation
- `sc2/src/libs/graphics/tfb_draw.h` - Drawing interface
- `sc2/src/libs/graphics/sdl/primitives.h` - Primitive rendering

## Key Data Structures

### Color
```rust
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}
```

### DrawMode
```rust
#[derive(Clone, Copy, Debug)]
pub enum DrawMode {
    Replace,           // Direct pixel write
    Alpha(u8),         // Alpha blend with factor
    Additive,          // Additive blending
    Subtractive,       // Subtractive blending
}
```

### Rect
```rust
#[derive(Clone, Copy, Debug)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
}
```

### Canvas (wraps SDL_Surface or raw buffer)
```rust
pub struct Canvas {
    pub width: u32,
    pub height: u32,
    pub pitch: u32,
    pub format: PixelFormat,
    pub pixels: Vec<u8>,  // Or reference to SDL surface
}
```

## Core Functions to Implement

### Initialization
- `TFB_DrawCanvas_Initialize()` - Build lookup tables
- `TFB_DrawCanvas_GetError() -> &str`

### Primitive Drawing
- `TFB_DrawCanvas_Line(x1, y1, x2, y2, color, mode, target)`
- `TFB_DrawCanvas_Rect(rect, color, mode, target)`
- `TFB_DrawCanvas_FilledRect(rect, color, mode, target)`  // Not in C, but useful
- `TFB_DrawCanvas_Image(src, src_rect, dst, dst_rect, mode)`

### Surface Operations
- `TFB_DrawCanvas_New_TrueColor(w, h, has_alpha) -> Canvas`
- `TFB_DrawCanvas_New_Paletted(w, h, palette, transparent_idx) -> Canvas`
- `TFB_DrawCanvas_Delete(canvas)`
- `TFB_DrawCanvas_GetExtent(canvas) -> Rect`
- `TFB_DrawCanvas_SetClipRect(canvas, rect)`
- `TFB_DrawCanvas_GetClipRect(canvas) -> Rect`

### Pixel Operations
- `TFB_DrawCanvas_GetPixel(canvas, x, y) -> Color`
- `TFB_DrawCanvas_SetPixel(canvas, x, y, color, mode)`
- `TFB_DrawCanvas_GetPixelColors(canvas, pixels, count) -> Vec<Color>`

### Format Conversion
- `TFB_DrawCanvas_ToScreenFormat(canvas) -> Canvas`
- `TFB_DrawCanvas_Rescale(canvas, scale_type) -> Canvas`
- `TFB_DrawCanvas_GetScreenFormat() -> PixelFormat`

### Palette Operations (for paletted surfaces)
- `TFB_DrawCanvas_SetPalette(canvas, first, count, colors)`
- `TFB_DrawCanvas_GetPalette(canvas) -> Palette`
- `TFB_DrawCanvas_SetTransparentIndex(canvas, index)`
- `TFB_DrawCanvas_GetTransparentIndex(canvas) -> Option<u8>`

### Blitting
- `TFB_DrawCanvas_CopyRect(src, src_rect, dst, dst_pos, mode)`
- `TFB_DrawCanvas_FontChar(font_char, dst, x, y, color)`

### Advanced
- `TFB_DrawCanvas_SetMipmap(canvas, mipmap, index)`
- `TFB_DrawCanvas_GetMipmap(canvas, index) -> Canvas`
- `TFB_DrawCanvas_Lock(canvas) -> &mut [u8]`
- `TFB_DrawCanvas_Unlock(canvas)`

## Blend Table
The C code pre-computes a 256x256 blend table:
```rust
// btable[weight][value] = (value * weight + 0x80) >> 8
static BLEND_TABLE: [[u8; 256]; 256] = ...;
```
This can be computed at compile time with `const fn` or lazy_static.

## Pixel Format Support
```rust
pub enum PixelFormat {
    RGBA8888,
    BGRA8888,
    RGB888,
    BGR888,
    Indexed8 { palette: Box<[Color; 256]> },
}
```

## Drawing Algorithms

### Line (Bresenham)
Already implemented in `rust/src/graphics/tfb_draw.rs` - can reuse.

### Filled Rectangle
Simple scanline fill with clipping.

### Alpha Blending
```rust
fn blend(src: Color, dst: Color, mode: DrawMode) -> Color {
    match mode {
        DrawMode::Replace => src,
        DrawMode::Alpha(factor) => {
            let a = (src.a as u32 * factor as u32) / 255;
            Color {
                r: ((src.r as u32 * a + dst.r as u32 * (255 - a)) / 255) as u8,
                g: ((src.g as u32 * a + dst.g as u32 * (255 - a)) / 255) as u8,
                b: ((src.b as u32 * a + dst.b as u32 * (255 - a)) / 255) as u8,
                a: dst.a, // or compute properly
            }
        }
        // ... other modes
    }
}
```

## Thread Safety
Canvas operations are typically single-threaded (graphics thread only), but we should still use `Send + Sync` where appropriate for future flexibility.

## Test Plan (TDD)

### Unit Tests
1. `test_color_creation` - Create colors
2. `test_canvas_new_truecolor` - Create RGBA canvas
3. `test_canvas_new_paletted` - Create indexed canvas
4. `test_canvas_get_set_pixel` - Single pixel ops
5. `test_canvas_clip_rect` - Clipping rectangle
6. `test_draw_line_horizontal` - Horizontal line
7. `test_draw_line_vertical` - Vertical line
8. `test_draw_line_diagonal` - Diagonal line
9. `test_draw_line_clipped` - Line with clipping
10. `test_draw_rect_outline` - Rectangle outline
11. `test_draw_rect_filled` - Filled rectangle
12. `test_blend_replace` - Replace mode
13. `test_blend_alpha` - Alpha blending
14. `test_blend_additive` - Additive blending
15. `test_blit_simple` - Simple copy
16. `test_blit_with_alpha` - Alpha blit
17. `test_blit_clipped` - Clipped blit
18. `test_palette_operations` - Palette get/set
19. `test_format_conversion` - RGBA to screen format
20. `test_rescale` - Canvas rescaling

### Integration Tests
1. `test_canvas_with_sdl_surface` - Wrap SDL surface
2. `test_canvas_to_texture` - Upload to GPU
3. `test_font_rendering` - Render font characters

## File Structure
```
rust/src/graphics/
├── mod.rs              (add canvas module)
├── canvas/
│   ├── mod.rs          (public exports)
│   ├── types.rs        (Color, Rect, DrawMode, PixelFormat)
│   ├── canvas.rs       (Canvas struct and methods)
│   ├── primitives.rs   (line, rect drawing)
│   ├── blend.rs        (blending operations)
│   ├── blit.rs         (copy/blit operations)
│   ├── palette.rs      (palette operations)
│   └── ffi.rs          (C FFI bindings)
```

## FFI Functions to Export
```rust
#[no_mangle]
pub extern "C" fn TFB_DrawCanvas_Initialize();

#[no_mangle]
pub extern "C" fn TFB_DrawCanvas_Line(
    x1: c_int, y1: c_int, x2: c_int, y2: c_int,
    color: Color, mode: DrawMode, target: *mut c_void
);
// ... etc
```

## Dependencies
- Existing `sdl2` crate for surface interop
- Existing graphics modules in `rust/src/graphics/`

## Acceptance Criteria
1. All unit tests pass
2. Pixel-perfect output matching C implementation
3. Proper clipping on all operations
4. Alpha blending matches C behavior
5. FFI bindings work with C code
6. No visual artifacts or off-by-one errors
