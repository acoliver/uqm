//! Animation and bitmap-font loading, ported from gfxload.c and filegfx.c
//! (GPL-2.0-or-later).
//!
//! An animation (`.ani`) lists one image file a line with its transparency,
//! colour map and hot spot; it may also be a zip package holding the list
//! and its images. A font is a directory or package of glyph images named
//! by their hexadecimal character code.
use super::canvas::png::{self, Decoded};
use super::drawable::{FrameId, FrameKind, LoadedDrawable, WANT_PIXMAP};
use super::font::{Font, LoadedFont};
use super::geometry::HotSpot;
use super::image::Image;
use super::state;
use crate::host::{FileStream, Files};
use crate::io::Dir;
use std::ffi::CStr;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

/// One animation line: `file transparent colormap hotx hoty`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct AniData {
    transparent: i32,
    colormap: i32,
    hot_x: i32,
    hot_y: i32,
}

/// `(parent, name, prefix)` of a resource path, as `get_cel_data` splits it.
fn path_parts(path: &[u8]) -> (&[u8], &[u8], &[u8]) {
    let slash = path
        .iter()
        .rposition(|&b| b == b'/')
        .or_else(|| path.iter().rposition(|&b| b == b'\\'));
    match slash {
        Some(i) => (&path[..i], &path[i + 1..], &path[..=i]),
        None => (b".", path, b""),
    }
}

/// A cursor implementing the `sscanf` conversions the loaders use.
struct Scan<'a>(&'a [u8]);

impl Scan<'_> {
    fn skip_space(&mut self) {
        while let [first, rest @ ..] = self.0 {
            if !first.is_ascii_whitespace() {
                break;
            }
            self.0 = rest;
        }
    }

    /// `%Ns`: up to `limit` non-space bytes.
    fn word(&mut self, limit: usize) -> Option<Vec<u8>> {
        self.skip_space();
        let length = self
            .0
            .iter()
            .take(limit)
            .take_while(|b| !b.is_ascii_whitespace())
            .count();
        (length > 0).then(|| {
            let (word, rest) = self.0.split_at(length);
            self.0 = rest;
            word.to_vec()
        })
    }

    /// `%d`.
    fn int(&mut self) -> Option<i32> {
        self.skip_space();
        let negative = match self.0.first() {
            Some(b'-') => true,
            Some(b'+') => false,
            _ => return self.digits(10).map(|value| value as i32),
        };
        self.0 = &self.0[1..];
        let value = self.digits(10)? as i32;
        Some(if negative {
            value.wrapping_neg()
        } else {
            value
        })
    }

    /// Unsigned digits in `radix`, wrapping like `strtoul` into 32 bits.
    fn digits(&mut self, radix: u32) -> Option<u32> {
        let count = self
            .0
            .iter()
            .take_while(|b| (**b as char).is_digit(radix))
            .count();
        if count == 0 {
            return None;
        }
        let (digits, rest) = self.0.split_at(count);
        self.0 = rest;
        Some(digits.iter().fold(0u32, |value, &digit| {
            value.wrapping_mul(radix).wrapping_add(
                (digit as char)
                    .to_digit(radix)
                    .unwrap_or_else(|| unreachable!()),
            )
        }))
    }
}

/// `sscanf(line, "%1023s %d %d %d %d", ...)`.
fn parse_ani(line: &[u8]) -> Option<(Vec<u8>, AniData)> {
    let mut scan = Scan(line);
    let name = scan.word(1023)?;
    Some((
        name,
        AniData {
            transparent: scan.int()?,
            colormap: scan.int()?,
            hot_x: scan.int()?,
            hot_y: scan.int()?,
        },
    ))
}

/// `sscanf(name, "%x.", &index)`: a glyph file's character code.
fn glyph_index(name: &[u8]) -> Option<u32> {
    let mut scan = Scan(name);
    scan.skip_space();
    if let [b'0', b'x' | b'X', ..] = scan.0 {
        scan.0 = &scan.0[2..];
    }
    let index = scan.digits(16)?;
    (index <= 65535).then_some(index)
}

/// C: `TFB_DrawCanvas_LoadFromFile`: decode an image file of `files`.
fn load_image(files: &dyn Files, name: &[u8]) -> Option<Decoded> {
    let bytes = files
        .read(Path::new(String::from_utf8_lossy(name).as_ref()))
        .ok()?;
    png::decode(&bytes).ok()
}

/// The content directory; loads find nothing before one is mounted.
fn content_files() -> Option<Dir> {
    crate::io::content_dir()
}

/// C: `get_cel_data`: load the animation `path`, whose stream is `stream`
/// (the first `length` bytes from its position).
pub(crate) fn load_cel(
    stream: &mut FileStream,
    length: u32,
    path: &[u8],
) -> Option<LoadedDrawable> {
    let mut position = stream.position();
    let (parent, name, prefix) = path_parts(path);
    let mut header = [0u8; 4];
    let _ = stream.read(&mut header);
    let content = content_files()?.snapshot();
    let mut package_stream;
    let (files, file, prefix): (Box<dyn Files>, &mut FileStream, &[u8]) =
        if !path.is_empty() && u32::from_le_bytes(header) == 0x0403_4b50 {
            let parent = String::from_utf8_lossy(parent);
            let package_path = String::from_utf8_lossy(path);
            let files = content
                .with_package(&parent, Path::new(package_path.as_ref()))
                .unwrap_or(content)
                .open_relative(Path::new(parent.as_ref()));
            package_stream = files
                .open_read(Path::new(String::from_utf8_lossy(name).as_ref()))
                .ok()?;
            position = 0;
            (Box::new(files), &mut package_stream, b"")
        } else {
            (Box::new(content), stream, prefix)
        };
    file.seek(SeekFrom::Start(position)).ok()?;
    let mut images = Vec::new();
    while let Some(line) = file.line(1024).ok().flatten() {
        let Some((name, ani)) = parse_ani(&line) else {
            log::warn!("get_cel_data: invalid animation entry");
            break;
        };
        match load_image(files.as_ref(), &[prefix, &name].concat()) {
            Some(image) => images.push((image, ani)),
            None => log::warn!("get_cel_data: Unable to load image!"),
        }
        if (file.position() as i32).wrapping_sub(position as i32) >= length as i32 {
            break;
        }
    }
    if images.is_empty() {
        log::warn!("Couldn't get cel data");
        return None;
    }
    let drawable = state::with(|g| {
        let drawable = g.alloc_drawable(images.len() as u16, WANT_PIXMAP);
        for (index, (mut decoded, ani)) in images.into_iter().enumerate() {
            decoded.apply_transparency(ani.transparent);
            let mut image = Image::new(decoded.into_display_canvas());
            image.colormap_index = ani.colormap;
            let bounds = image.extent;
            let image = g.images.insert(image);
            let frame = g.frame_mut(FrameId::new(drawable, index as u16));
            frame.kind = FrameKind::Rom;
            frame.hot_spot = HotSpot::new(ani.hot_x as i16, ani.hot_y as i16);
            frame.bounds = bounds;
            frame.image = Some(image);
        }
        drawable
    });
    Some(LoadedDrawable::adopt(drawable))
}

/// C: `get_font_data`: load the font directory or package `path`.
pub(crate) fn load_font(path: &CStr, package: bool) -> Option<LoadedFont> {
    let content = content_files()?.snapshot();
    let directory = Path::new(path.to_str().ok()?);
    let content = if package {
        content
            .with_package(path.to_str().ok()?, directory)
            .unwrap_or(content)
    } else {
        content
    };
    let names: Vec<String> = content
        .list(directory)
        .unwrap_or_default()
        .into_iter()
        .filter(|name| {
            !name.starts_with('.')
                && name.contains('.')
                && content
                    .stat(&directory.join(name))
                    .is_ok_and(|stat| stat.is_file)
        })
        .collect();
    if names.is_empty() {
        return None;
    }
    let mut chars = Vec::new();
    for name in names {
        let Some(index) = glyph_index(name.as_bytes()) else {
            continue;
        };
        let Some(decoded) = load_image(
            &content,
            directory.join(&name).as_os_str().as_encoded_bytes(),
        ) else {
            continue;
        };
        let canvas = decoded.canvas();
        if canvas.width() == 0 || canvas.height() == 0 {
            continue;
        }
        chars.push((index, canvas.clone()));
    }
    Some(LoadedFont::adopt(Font::build(chars)))
}

/// C: `LoadGraphicFile`: an animation or image file outside the resource
/// index.
pub(crate) fn load_graphic_file(path: &CStr) -> Option<LoadedDrawable> {
    let mut stream = content_files()?
        .open_read(Path::new(path.to_str().ok()?))
        .ok()?;
    let length = stream.len().ok()? as u32;
    load_cel(&mut stream, length, path.to_bytes())
}

/// C: `LoadFontFile`.
pub(crate) fn load_font_file(path: &CStr) -> Option<LoadedFont> {
    let content = content_files()?;
    let directory = Path::new(path.to_str().ok()?);
    let package = !content.stat(directory).ok()?.is_dir;
    load_font(path, package)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn animation_lines_scan_like_sscanf() {
        assert_eq!(
            parse_ani(b"  frame.png -1 3 +4 -5\n"),
            Some((
                b"frame.png".to_vec(),
                AniData {
                    transparent: -1,
                    colormap: 3,
                    hot_x: 4,
                    hot_y: -5
                }
            ))
        );
        assert_eq!(parse_ani(b"frame.png 1 2 3"), None);
        assert_eq!(parse_ani(b"frame.png 1 2 3 x"), None);
        assert_eq!(
            parse_ani(b"a 1 2 3 4junk").map(|(_, ani)| ani.hot_y),
            Some(4)
        );
    }

    #[test]
    fn glyph_names_are_hexadecimal_character_codes() {
        assert_eq!(glyph_index(b"41.png"), Some(0x41));
        assert_eq!(glyph_index(b"0x7e.png"), Some(0x7e));
        assert_eq!(glyph_index(b"10000.png"), None);
        assert_eq!(glyph_index(b"readme.txt"), None);
    }

    #[test]
    fn paths_split_into_parent_name_and_prefix() {
        assert_eq!(
            path_parts(b"base/ui/menu.ani"),
            (&b"base/ui"[..], &b"menu.ani"[..], &b"base/ui/"[..])
        );
        assert_eq!(
            path_parts(b"menu.ani"),
            (&b"."[..], &b"menu.ani"[..], &b""[..])
        );
    }
}
