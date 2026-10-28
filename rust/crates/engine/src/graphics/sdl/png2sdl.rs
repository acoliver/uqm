//! PNG-to-SDL conversion preserving UQM's indexed-color and tRNS contract.
//! Ported from png2sdl.c (GPL-2.0-or-later), derived from SDL_image.
use png::{BitDepth, ColorType};
use sdl2_sys::{
    SDL_Color, SDL_CreateRGBSurface, SDL_FreeSurface, SDL_LockSurface, SDL_MapRGB, SDL_RWops,
    SDL_RWread, SDL_RWseek, SDL_RWtell, SDL_SetColorKey, SDL_SetPaletteColors, SDL_Surface,
    SDL_UnlockSurface, SDL_bool,
};
#[cfg(test)]
use sdl2_sys::{SDL_GetColorKey, SDL_RWFromConstMem, SDL_RWclose};
use std::io::{self, BufReader, Read, Seek, SeekFrom};
use std::ptr;

struct RwStream(*mut SDL_RWops);
impl Read for RwStream {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        unsafe { Ok(SDL_RWread(self.0, out.as_mut_ptr().cast(), 1, out.len())) }
    }
}
impl Seek for RwStream {
    fn seek(&mut self, from: SeekFrom) -> io::Result<u64> {
        let (offset, whence) = match from {
            SeekFrom::Start(n) => (i64::try_from(n).map_err(io::Error::other)?, 0),
            SeekFrom::Current(n) => (n, 1),
            SeekFrom::End(n) => (n, 2),
        };
        let result = unsafe { SDL_RWseek(self.0, offset, whence) };
        if result < 0 {
            Err(io::Error::other("SDL_RWseek failed"))
        } else {
            Ok(result as u64)
        }
    }
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

unsafe fn load(src: *mut SDL_RWops) -> Result<*mut SDL_Surface, String> {
    let decoder = png::Decoder::new(BufReader::new(RwStream(src)));
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
    let mut ckey: i32 = -1;
    let mut expand_palette = false;
    if let Some(trans) = trns {
        if info.color_type == ColorType::Indexed {
            let mut t = -1;
            let mut j = 0;
            while j < trans.len() {
                if trans[j] == 0 {
                    if t >= 0 {
                        break;
                    }
                    t = j as i32;
                } else if trans[j] != 255 {
                    break;
                }
                j += 1;
            }
            if j == trans.len() {
                ckey = t;
            } else {
                expand_palette = true;
            }
        } else {
            ckey = 0;
        }
    }
    let channels = match info.color_type {
        ColorType::Indexed if expand_palette => 4,
        ColorType::Indexed => 1,
        ColorType::Grayscale if trns.is_some() => 2,
        ColorType::Grayscale => 1,
        ColorType::Rgb => 3,
        ColorType::GrayscaleAlpha | ColorType::Rgba => 4,
    };
    let (rmask, gmask, bmask, amask) = if channels >= 3 {
        if cfg!(target_endian = "little") {
            (
                0x000000ff,
                0x0000ff00,
                0x00ff0000,
                if channels == 4 { 0xff000000 } else { 0 },
            )
        } else {
            let shift = if channels == 4 { 0 } else { 8 };
            (
                0xff000000 >> shift,
                0x00ff0000 >> shift,
                0x0000ff00 >> shift,
                0x000000ff >> shift,
            )
        }
    } else {
        (0, 0, 0, 0)
    };
    let surface = SDL_CreateRGBSurface(0, width, height, channels * 8, rmask, gmask, bmask, amask);
    if surface.is_null() {
        return Err("SDL_CreateRGBSurface failed".into());
    }
    if ckey != -1 {
        if info.color_type != ColorType::Indexed {
            let (r, g, b) = if info.color_type == ColorType::Rgb {
                let t = trns.expect("RGB key requires tRNS");
                if info.bit_depth == BitDepth::Sixteen {
                    (t[1], t[3], t[5])
                } else {
                    (t[0], t[1], t[2])
                }
            } else {
                (0, 0, 0)
            };
            ckey = SDL_MapRGB((*surface).format, r, g, b) as i32;
        }
        SDL_SetColorKey(surface, SDL_bool::SDL_TRUE as i32, ckey as u32);
    }
    SDL_LockSurface(surface);
    let input_channels = info.color_type.samples();
    for y in 0..height as usize {
        let row = &buffer[y * output.line_size..(y + 1) * output.line_size];
        let row = samples(row, info.bit_depth, width as usize * input_channels);
        let dst = std::slice::from_raw_parts_mut(
            (*surface)
                .pixels
                .cast::<u8>()
                .add(y * (*surface).pitch as usize),
            width as usize * channels as usize,
        );
        for x in 0..width as usize {
            let input = &row[x * input_channels..(x + 1) * input_channels];
            let output = &mut dst[x * channels as usize..(x + 1) * channels as usize];
            let to8 = |v: u16| {
                if info.bit_depth == BitDepth::Sixteen {
                    (v >> 8) as u8
                } else {
                    v as u8
                }
            };
            match info.color_type {
                ColorType::Indexed if expand_palette => {
                    let i = input[0] as usize;
                    let palette = info.palette.as_deref().expect("indexed PNG has PLTE");
                    let Some(rgb) = palette.get(i * 3..i * 3 + 3) else {
                        SDL_UnlockSurface(surface);
                        SDL_FreeSurface(surface);
                        return Err("PNG palette index exceeds PLTE entries".into());
                    };
                    output[..3].copy_from_slice(rgb);
                    output[3] = trns.and_then(|t| t.get(i)).copied().unwrap_or(255);
                }
                ColorType::Indexed => {
                    output[0] = input[0] as u8;
                }
                ColorType::Grayscale => {
                    output[0] = if (info.bit_depth as u8) < 8 {
                        (input[0] * 255 / ((1 << info.bit_depth as u8) - 1)) as u8
                    } else {
                        to8(input[0])
                    };
                    if let Some(t) = trns {
                        let transparent = if info.bit_depth == BitDepth::Sixteen {
                            u16::from_be_bytes([t[0], t[1]])
                        } else {
                            u16::from(t[0])
                        };
                        output[1] = if input[0] == transparent { 0 } else { 255 };
                    }
                }
                ColorType::GrayscaleAlpha => {
                    output[..3].fill(to8(input[0]));
                    output[3] = to8(input[1]);
                }
                ColorType::Rgb | ColorType::Rgba => {
                    for (out, sample) in output.iter_mut().zip(input) {
                        *out = to8(*sample);
                    }
                }
            }
        }
    }
    SDL_UnlockSurface(surface);
    if !(*(*surface).format).palette.is_null() {
        let palette: Vec<SDL_Color> = if info.color_type == ColorType::Grayscale {
            (0..=255)
                .map(|v| SDL_Color {
                    r: v,
                    g: v,
                    b: v,
                    a: 255,
                })
                .collect()
        } else {
            info.palette
                .as_deref()
                .unwrap_or_default()
                .as_chunks::<3>()
                .0
                .iter()
                .take(1 << info.bit_depth as u8)
                .map(|c| SDL_Color {
                    r: c[0],
                    g: c[1],
                    b: c[2],
                    a: 255,
                })
                .collect()
        };
        SDL_SetPaletteColors(
            (*(*surface).format).palette,
            palette.as_ptr(),
            0,
            palette.len() as i32,
        );
    }
    Ok(surface)
}

/// # Safety
/// Source must be null or a live seekable SDL_RWops with read/seek callbacks.
/// The returned surface belongs to the caller. The input stream stays open.
#[no_mangle]
pub unsafe extern "C" fn TFB_png_to_sdl(src: *mut SDL_RWops) -> *mut SDL_Surface {
    if src.is_null() {
        return ptr::null_mut();
    }
    let start = SDL_RWtell(src);
    match load(src) {
        Ok(surface) => surface,
        Err(error) => {
            SDL_RWseek(src, start, 0);
            eprint!("{error}");
            ptr::null_mut()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn encode(
        color: ColorType,
        depth: BitDepth,
        data: &[u8],
        palette: Option<&[u8]>,
        trns: Option<&[u8]>,
    ) -> Vec<u8> {
        let mut bytes = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut bytes, 2, 1);
            encoder.set_color(color);
            encoder.set_depth(depth);
            if let Some(p) = palette {
                encoder.set_palette(p);
            }
            if let Some(t) = trns {
                encoder.set_trns(t);
            }
            encoder
                .write_header()
                .unwrap()
                .write_image_data(data)
                .unwrap();
        }
        bytes
    }
    unsafe fn decode(data: &[u8]) -> *mut SDL_Surface {
        let rw = SDL_RWFromConstMem(data.as_ptr().cast(), data.len() as i32);
        let surface = TFB_png_to_sdl(rw);
        SDL_RWclose(rw);
        assert!(!surface.is_null());
        surface
    }
    #[test]
    fn indexed_single_key_preserves_indices_and_palette() {
        let bytes = encode(
            ColorType::Indexed,
            BitDepth::One,
            &[0b01000000],
            Some(&[1, 2, 3, 9, 8, 7]),
            Some(&[255, 0]),
        );
        unsafe {
            let surface = decode(&bytes);
            assert_eq!((*(*surface).format).BitsPerPixel, 8);
            assert_eq!(
                std::slice::from_raw_parts((*surface).pixels.cast::<u8>(), 2),
                &[0, 1]
            );
            let mut key = 99;
            assert_eq!(SDL_GetColorKey(surface, &mut key), 0);
            assert_eq!(key, 1);
            assert_eq!((*(*(*surface).format).palette).colors.add(1).read().g, 8);
            SDL_FreeSurface(surface);
        }
    }
    #[test]
    fn fractional_alpha_and_multiple_keys_expand_palette() {
        for trns in [&[0u8, 0][..], &[255u8, 128][..]] {
            let bytes = encode(
                ColorType::Indexed,
                BitDepth::Eight,
                &[0, 1],
                Some(&[1, 2, 3, 9, 8, 7]),
                Some(trns),
            );
            unsafe {
                let s = decode(&bytes);
                assert_eq!((*(*s).format).BitsPerPixel, 32);
                assert_eq!(
                    std::slice::from_raw_parts((*s).pixels.cast::<u8>(), 8),
                    &[1, 2, 3, trns[0], 9, 8, 7, trns[1]]
                );
                SDL_FreeSurface(s);
            }
        }
    }
    #[test]
    fn sixteen_bit_gray_alpha_becomes_rgba_with_high_bytes() {
        let bytes = encode(
            ColorType::GrayscaleAlpha,
            BitDepth::Sixteen,
            &[0x12, 0xab, 0x34, 0xcd, 0x56, 0xef, 0x78, 0x91],
            None,
            None,
        );
        unsafe {
            let s = decode(&bytes);
            assert_eq!(
                std::slice::from_raw_parts((*s).pixels.cast::<u8>(), 8),
                &[0x12, 0x12, 0x12, 0x34, 0x56, 0x56, 0x56, 0x78]
            );
            SDL_FreeSurface(s);
        }
    }
    #[test]
    fn malformed_palette_index_returns_error_without_panicking() {
        let bytes = encode(
            ColorType::Indexed,
            BitDepth::Eight,
            &[0, 3],
            Some(&[1, 2, 3, 4, 5, 6]),
            Some(&[128, 255]),
        );
        unsafe {
            let rw = SDL_RWFromConstMem(bytes.as_ptr().cast(), bytes.len() as i32);
            assert!(TFB_png_to_sdl(rw).is_null());
            assert_eq!(SDL_RWtell(rw), 0);
            SDL_RWclose(rw);
        }
    }
    #[test]
    fn malformed_png_restores_stream_position() {
        unsafe {
            let bytes = b"prefixbad png";
            let rw = SDL_RWFromConstMem(bytes.as_ptr().cast(), bytes.len() as i32);
            SDL_RWseek(rw, 6, 0);
            assert!(TFB_png_to_sdl(rw).is_null());
            assert_eq!(SDL_RWtell(rw), 6);
            SDL_RWclose(rw);
        }
    }
}
