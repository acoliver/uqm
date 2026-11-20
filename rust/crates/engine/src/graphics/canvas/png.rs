//! PNG decoding into canvases, preserving UQM's indexed-colour and tRNS
//! contract (png2sdl.c, derived from SDL_image) and the screen-format
//! conversion the C loader applied to true-colour images.
use super::{Canvas, PixelFormat};
use crate::graphics::geometry::Color;
use png::{BitDepth, ColorType};

/// A decoded image before the animation file's transparency is applied.
#[derive(Debug)]
pub struct Decoded {
    canvas: Canvas,
}

fn samples(row: &[u8], depth: BitDepth, count: usize) -> Vec<u16> {
    let bits = depth as usize;
    (0..count)
        .map(|i| match depth {
            BitDepth::Sixteen => u16::from_be_bytes([row[2 * i], row[2 * i + 1]]),
            BitDepth::Eight => u16::from(row[i]),
            _ => u16::from((row[i * bits / 8] >> (8 - bits - i * bits % 8)) & ((1 << bits) - 1)),
        })
        .collect()
}

/// Decode a PNG file.
pub fn decode(bytes: &[u8]) -> Result<Decoded, String> {
    let decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    let mut reader = decoder.read_info().map_err(|e| e.to_string())?;
    let info = reader.info().clone();
    let width = i32::try_from(info.width).map_err(|e| e.to_string())?;
    let height = i32::try_from(info.height).map_err(|e| e.to_string())?;
    let mut buffer = vec![
        0;
        reader
            .output_buffer_size()
            .ok_or("PNG dimensions overflow")?
    ];
    let output = reader.next_frame(&mut buffer).map_err(|e| e.to_string())?;
    let trns = info.trns.as_deref();
    let mut key: Option<u32> = None;
    let mut expand_palette = false;
    if let Some(trans) = trns {
        if info.color_type == ColorType::Indexed {
            // a single fully transparent entry among opaque ones is a key
            let mut transparent: Option<usize> = None;
            let mut j = 0;
            while j < trans.len() {
                if trans[j] == 0 {
                    if transparent.is_some() {
                        break;
                    }
                    transparent = Some(j);
                } else if trans[j] != 255 {
                    break;
                }
                j += 1;
            }
            if j == trans.len() {
                key = transparent.map(|index| index as u32);
            } else {
                expand_palette = true;
            }
        } else if info.color_type == ColorType::Rgb {
            let (r, g, b) = if info.bit_depth == BitDepth::Sixteen {
                (trans[1], trans[3], trans[5])
            } else {
                (trans[0], trans[1], trans[2])
            };
            key = Some(u32::from(r) << 24 | u32::from(g) << 16 | u32::from(b) << 8);
        } else {
            return Err(format!(
                "unsupported PNG transparency for {:?} images",
                info.color_type
            ));
        }
    }
    let format = match info.color_type {
        ColorType::Indexed if !expand_palette => PixelFormat::Indexed,
        ColorType::Grayscale => PixelFormat::Indexed,
        ColorType::Rgb => PixelFormat::Rgbx,
        ColorType::Indexed | ColorType::GrayscaleAlpha | ColorType::Rgba => PixelFormat::Rgba,
    };
    let mut canvas = Canvas::new(format, width, height);
    let to8 = |v: u16| {
        if info.bit_depth == BitDepth::Sixteen {
            (v >> 8) as u8
        } else {
            v as u8
        }
    };
    let input_channels = info.color_type.samples();
    let palette = info.palette.as_deref();
    for y in 0..height as usize {
        let row = &buffer[y * output.line_size..(y + 1) * output.line_size];
        let row = samples(row, info.bit_depth, width as usize * input_channels);
        for x in 0..width as usize {
            let input = &row[x * input_channels..(x + 1) * input_channels];
            let pixel = match info.color_type {
                ColorType::Indexed if expand_palette => {
                    let i = usize::from(input[0]);
                    let rgb = palette
                        .and_then(|p| p.get(i * 3..i * 3 + 3))
                        .ok_or("PNG palette index exceeds PLTE entries")?;
                    let alpha = trns.and_then(|t| t.get(i)).copied().unwrap_or(255);
                    rgba(rgb[0], rgb[1], rgb[2], alpha)
                }
                ColorType::Indexed => u32::from(input[0]),
                ColorType::Grayscale => u32::from(if (info.bit_depth as u8) < 8 {
                    (input[0] * 255 / ((1 << info.bit_depth as u8) - 1)) as u8
                } else {
                    to8(input[0])
                }),
                ColorType::GrayscaleAlpha => {
                    let v = to8(input[0]);
                    rgba(v, v, v, to8(input[1]))
                }
                ColorType::Rgb => rgba(to8(input[0]), to8(input[1]), to8(input[2]), 0),
                ColorType::Rgba => rgba(to8(input[0]), to8(input[1]), to8(input[2]), to8(input[3])),
            };
            canvas.set_pixel(x as i32, y as i32, pixel);
        }
    }
    if format == PixelFormat::Indexed {
        let colors: Vec<Color> = if info.color_type == ColorType::Grayscale {
            (0..=255).map(|v| Color::rgb(v, v, v)).collect()
        } else {
            palette
                .unwrap_or_default()
                .as_chunks::<3>()
                .0
                .iter()
                .take(1 << info.bit_depth as u8)
                .map(|c| Color::rgb(c[0], c[1], c[2]))
                .collect()
        };
        canvas.set_palette_colors(0, &colors);
    }
    canvas.set_color_key(key);
    Ok(Decoded { canvas })
}

fn rgba(r: u8, g: u8, b: u8, a: u8) -> u32 {
    u32::from(r) << 24 | u32::from(g) << 16 | u32::from(b) << 8 | u32::from(a)
}

impl Decoded {
    pub fn is_indexed(&self) -> bool {
        self.canvas.is_indexed()
    }

    /// The decoded pixels without the screen-format conversion, for font
    /// glyphs which only read their intensity or alpha.
    pub fn canvas(&self) -> &Canvas {
        &self.canvas
    }

    /// Apply an animation entry's transparency: a palette index for indexed
    /// images, black for `0` on true-colour images, none for `-1`.
    pub fn apply_transparency(&mut self, transparent: i32) {
        if self.canvas.is_indexed() {
            if transparent >= 0 {
                self.canvas.set_transparent_index(transparent);
            }
        } else if transparent == 0 {
            self.canvas.set_color_key(Some(0));
        }
        if transparent == -1 {
            self.canvas.set_transparent_index(-1);
        }
    }

    /// The image in display format: indexed images unchanged, true-colour
    /// images as `TFB_DisplayFormatAlpha` converted them. A keyed RGBA image
    /// loses the alpha of its key-coloured pixels and stops blending.
    pub fn into_display_canvas(self) -> Canvas {
        let mut canvas = self.canvas;
        if canvas.format() == PixelFormat::Rgba {
            match canvas.color_key() {
                Some(key) => {
                    let key = key & 0xffff_ff00;
                    for pixel in canvas.words_mut() {
                        if *pixel & 0xffff_ff00 == key {
                            *pixel &= 0xffff_ff00;
                        }
                    }
                    canvas.set_color_key(Some(key));
                    canvas.set_blend(false);
                }
                None => canvas.set_blend(true),
            }
        } else if canvas.format() == PixelFormat::Rgbx {
            canvas.set_blend(false);
        }
        canvas
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn encode(
        color: ColorType,
        depth: BitDepth,
        width: u32,
        data: &[u8],
        plte: Option<&[u8]>,
        trns: Option<&[u8]>,
    ) -> Vec<u8> {
        let mut out = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut out, width, 1);
            encoder.set_color(color);
            encoder.set_depth(depth);
            if let Some(plte) = plte {
                encoder.set_palette(plte.to_vec());
            }
            if let Some(trns) = trns {
                encoder.set_trns(trns.to_vec());
            }
            let mut writer = encoder.write_header().unwrap();
            writer.write_image_data(data).unwrap();
        }
        out
    }

    #[test]
    fn indexed_with_one_transparent_entry_becomes_a_keyed_palette() {
        let png = encode(
            ColorType::Indexed,
            BitDepth::Eight,
            2,
            &[0, 1],
            Some(&[1, 2, 3, 4, 5, 6]),
            Some(&[255, 0]),
        );
        let decoded = decode(&png).unwrap();
        assert!(decoded.is_indexed());
        assert_eq!(decoded.canvas().color_key(), Some(1));
        assert_eq!(decoded.canvas().palette().unwrap()[1], Color::rgb(4, 5, 6));
        assert_eq!(
            decoded.canvas().palette().unwrap()[2],
            Color::rgb(255, 255, 255)
        );
    }

    #[test]
    fn partially_transparent_palettes_expand_to_rgba() {
        let png = encode(
            ColorType::Indexed,
            BitDepth::Eight,
            1,
            &[0],
            Some(&[9, 8, 7]),
            Some(&[128]),
        );
        let canvas = decode(&png).unwrap().into_display_canvas();
        assert_eq!(canvas.format(), PixelFormat::Rgba);
        assert_eq!(canvas.words(), &[0x0908_0780]);
        assert!(canvas.blend());
    }

    #[test]
    fn rgb_images_become_rgbx_and_black_keys_follow_the_animation_entry() {
        let png = encode(
            ColorType::Rgb,
            BitDepth::Eight,
            2,
            &[0, 0, 0, 1, 2, 3],
            None,
            None,
        );
        let mut decoded = decode(&png).unwrap();
        decoded.apply_transparency(0);
        let canvas = decoded.into_display_canvas();
        assert_eq!(canvas.words(), &[0, 0x0102_0300]);
        assert_eq!(canvas.color_key(), Some(0));
        assert!(!canvas.blend());
    }

    #[test]
    fn keyed_rgba_images_clear_key_alpha_and_stop_blending() {
        let png = encode(
            ColorType::Rgba,
            BitDepth::Eight,
            2,
            &[0, 0, 0, 200, 1, 2, 3, 255],
            None,
            None,
        );
        let mut decoded = decode(&png).unwrap();
        decoded.apply_transparency(0);
        let canvas = decoded.into_display_canvas();
        assert_eq!(canvas.words(), &[0, 0x0102_03ff]);
        assert!(!canvas.blend());
    }
}
