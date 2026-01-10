/// Draw a single font character to a canvas.
///
/// Renders a character from a font page with alpha blending support.
/// The character bitmap data is transferred to the canvas, applying
/// the character's alpha channel for transparency.
///
/// # Parameters
///
/// - `canvas`: Destination canvas to draw to
/// - `fg_color`: Foreground color for the character
/// - `page`: Font page containing the character data
/// - `char_index`: Index of the character within the page
/// - `x`: X position for drawing (baseline position)
/// - `y`: Y position for drawing (baseline position)
/// - `use_pixmap`: If true, render with higher quality (currently unused)
///
/// # Returns
///
/// - `Ok(width)` - Returns the character's display width
/// - `Err(CanvasError)` - Drawing failed
///
/// # Notes
///
/// - Character bitmaps are stored as alpha-only data in `TFChar.data`
/// - The alpha channel is applied to the `fg_color` for each pixel
/// - Hot spot offsets are applied to position the glyph correctly
/// - Transparent pixels (alpha = 0) preserve the canvas background
/// - Clipping is performed based on canvas bounds and scissor region
pub fn draw_fontchar(
    canvas: &mut Canvas,
    fg_color: Color,
    page: &FontPage,
    char_index: usize,
    x: i32,
    y: i32,
    use_pixmap: bool,
) -> Result<usize, CanvasError> {
    check_canvas(canvas)?;
    
    // Get character descriptor from page
    let tf_char = page.chars
        .get(char_index)
        .and_then(|opt| opt.as_ref())
        .ok_or_else(|| CanvasError::InvalidOperation(
            "Character not found in font page".to_string()
        ))?;
    
    // Check if we have data to render
    let data = tf_char.data.as_ref().ok_or_else(|| CanvasError::InvalidOperation(
        "Character has no bitmap data".to_string()
    ))?;
    
    let extent_width = tf_char.extent.width as usize;
    let extent_height = tf_char.extent.height as usize;
    let disp_width = tf_char.disp.width as usize;
    let disp_height = tf_char.disp.height as usize;
    let pitch = tf_char.pitch as usize;
    
    // Calculate drawing position applying hot spot offset
    let draw_x = x - tf_char.hotspot.x as i32;
    let draw_y = y - tf_char.hotspot.y as i32;
    
    // Get canvas properties
    let canvas_width = canvas.width() as usize;
    let canvas_height = canvas.height() as usize;
    let bytes_per_pixel = canvas.format().bytes_per_pixel as usize;
    
    // Early exit if character has no dimensions or is off canvas
    if extent_width == 0 || extent_height == 0 || disp_width == 0 || disp_height == 0 {
        return Ok(disp_width);
    }
    
    // Get scissor rect
    let scissor_rect = canvas.scissor().rect;
    
    // Transfer alpha channel to destination pixels
    canvas.with_pixels_mut(|pixels| {
        let fg_bytes = [fg_color.r, fg_color.g, fg_color.b, fg_color.a];
        
        // Iterate through character bitmap
        for char_y in 0..disp_height {
            for char_x in 0..disp_width {
                let src_offset = char_y * pitch + char_x;
                
                if src_offset >= data.len() {
                    continue;
                }
                
                let alpha = data[src_offset] as i32;
                
                // Skip fully transparent pixels
                if alpha == 0 {
                    continue;
                }
                
                // Calculate destination position
                let dst_x = draw_x + char_x as i32;
                let dst_y = draw_y + char_y as i32;
                
                // Check canvas bounds
                if dst_x < 0 || dst_x >= canvas_width as i32 ||
                   dst_y < 0 || dst_y >= canvas_height as i32 {
                    continue;
                }
                
                // Check scissor clip (if enabled)
                if let Some(ref scissor) = scissor_rect {
                    let sc_x = scissor.corner.x;
                    let sc_y = scissor.corner.y;
                    let sc_w = scissor.extent.width as i32;
                    let sc_h = scissor.extent.height as i32;
                    
                    if dst_x < sc_x || dst_x >= sc_x + sc_w ||
                       dst_y < sc_y || dst_y >= sc_y + sc_h {
                        continue;
                    }
                }
                
                // Calculate destination pixel offset
                let dst_offset = (dst_y as usize * canvas_width + dst_x as usize) * bytes_per_pixel;
                
                // Apply color with alpha blending
                if bytes_per_pixel >= 4 {
                    // Alpha blending formula:
                    // dst = fg * alpha + dst * (255 - alpha)
                    // We use i32 to avoid overflow during calculations
                    let alpha_factor = alpha;
                    let inv_alpha = 255 - alpha_factor;
                    
                    for i in 0..4 {
                        if dst_offset + i < pixels.len() {
                            let fg_val = fg_bytes[i] as i32;
                            let dst_val = pixels[dst_offset + i] as i32;
                            let blended = (fg_val * alpha_factor + dst_val * inv_alpha) / 255;
                            pixels[dst_offset + i] = blended as u8;
                        }
                    }
                } else if bytes_per_pixel == 3 {
                    // RGB without alpha channel
                    let alpha_factor = alpha;
                    let inv_alpha = 255 - alpha_factor;
                    
                    for i in 0..3 {
                        if dst_offset + i < pixels.len() {
                            let fg_val = fg_bytes[i] as i32;
                            let dst_val = pixels[dst_offset + i] as i32;
                            let blended = (fg_val * alpha_factor + dst_val * inv_alpha) / 255;
                            pixels[dst_offset + i] = blended as u8;
                        }
                    }
                }
            }
        }
        
        Ok(())
    })?;
    
    // Note: use_pixmap parameter is reserved for future high-quality rendering
    let _ = use_pixmap;
    
    Ok(disp_width)
}

