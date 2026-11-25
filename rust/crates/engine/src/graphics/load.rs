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
use crate::resource::FileError;
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
fn load_image(files: &dyn Files, name: &[u8]) -> Result<Decoded, FileError> {
    let path = String::from_utf8_lossy(name);
    let bytes = files
        .read(Path::new(path.as_ref()))
        .map_err(|error| FileError::at(path.as_ref(), error))?;
    png::decode(&bytes).map_err(|error| FileError::at(path.as_ref(), error))
}

/// C: `get_cel_data`: load the animation `path`, whose stream is `stream`
/// (the first `length` bytes from its position).
pub(crate) fn load_cel(
    files: &Dir,
    stream: &mut FileStream,
    length: u32,
    path: &[u8],
) -> Result<LoadedDrawable, FileError> {
    let mut position = stream.position();
    let (parent, name, prefix) = path_parts(path);
    let mut header = [0u8; 4];
    let path_name = String::from_utf8_lossy(path);
    stream
        .read(&mut header)
        .map_err(|error| FileError::at(path_name.as_ref(), error))?;
    let content = files.snapshot();
    let mut package_stream;
    let (files, file, prefix): (Box<dyn Files>, &mut FileStream, &[u8]) =
        if !path.is_empty() && u32::from_le_bytes(header) == 0x0403_4b50 {
            let parent = String::from_utf8_lossy(parent);
            let package_path = String::from_utf8_lossy(path);
            let files = content
                .with_package(&parent, Path::new(package_path.as_ref()))
                .map_err(|error| FileError::at(package_path.as_ref(), error))?
                .open_relative(Path::new(parent.as_ref()));
            package_stream = files
                .open_read(Path::new(String::from_utf8_lossy(name).as_ref()))
                .map_err(|error| FileError::at(package_path.as_ref(), error))?;
            position = 0;
            (Box::new(files), &mut package_stream, b"")
        } else {
            (Box::new(content), stream, prefix)
        };
    file.seek(SeekFrom::Start(position))
        .map_err(|error| FileError::at(path_name.as_ref(), error))?;
    let mut images = Vec::new();
    let mut failure = None;
    while let Some(line) = file
        .line(1024)
        .map_err(|error| FileError::at(path_name.as_ref(), error))?
    {
        let Some((name, ani)) = parse_ani(&line) else {
            failure = Some(FileError::invalid(
                path_name.as_ref(),
                "invalid animation entry",
            ));
            log::warn!("get_cel_data: invalid animation entry");
            break;
        };
        match load_image(files.as_ref(), &[prefix, &name].concat()) {
            Ok(image) => images.push((image, ani)),
            Err(error) => {
                log::warn!("get_cel_data: {error}");
                failure = Some(error);
            }
        }
        if (file.position() as i32).wrapping_sub(position as i32) >= length as i32 {
            break;
        }
    }
    if images.is_empty() {
        log::warn!("Couldn't get cel data");
        return Err(failure
            .unwrap_or_else(|| FileError::invalid(path_name.as_ref(), "animation has no images")));
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
    Ok(LoadedDrawable::adopt(drawable))
}

/// C: `get_font_data`: load the font directory or package `path`.
pub(crate) fn load_font(files: &Dir, path: &CStr, package: bool) -> Result<LoadedFont, FileError> {
    let content = files.snapshot();
    let name = path
        .to_str()
        .map_err(|error| FileError::at(path.to_string_lossy().as_ref(), error))?;
    let directory = Path::new(name);
    let content = if package {
        content
            .with_package(name, directory)
            .map_err(|error| FileError::at(directory, error))?
    } else {
        content
    };
    let names: Vec<String> = content
        .list(directory)
        .map_err(|error| FileError::at(directory, error))?
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
        return Err(FileError::invalid(directory, "font has no glyph files"));
    }
    let mut chars = Vec::new();
    for name in names {
        let Some(index) = glyph_index(name.as_bytes()) else {
            continue;
        };
        let decoded = match load_image(
            &content,
            directory.join(&name).as_os_str().as_encoded_bytes(),
        ) {
            Ok(decoded) => decoded,
            Err(error) => {
                log::warn!("skipping font glyph: {error}");
                continue;
            }
        };
        let canvas = decoded.canvas();
        if canvas.width() == 0 || canvas.height() == 0 {
            continue;
        }
        chars.push((index, canvas.clone()));
    }
    Ok(LoadedFont::adopt(Font::build(chars)))
}

/// C: `LoadGraphicFile`: an animation or image file outside the resource
/// index.
pub(crate) fn load_graphic_file(files: &Dir, path: &CStr) -> Result<LoadedDrawable, FileError> {
    let name = path
        .to_str()
        .map_err(|error| FileError::at(path.to_string_lossy().as_ref(), error))?;
    let mut stream = files
        .open_read(Path::new(name))
        .map_err(|error| FileError::at(name, error))?;
    let length = stream.len().map_err(|error| FileError::at(name, error))? as u32;
    load_cel(files, &mut stream, length, path.to_bytes())
}

/// C: `LoadFontFile`.
pub(crate) fn load_font_file(files: &Dir, path: &CStr) -> Result<LoadedFont, FileError> {
    let content = files;
    let name = path
        .to_str()
        .map_err(|error| FileError::at(path.to_string_lossy().as_ref(), error))?;
    let directory = Path::new(name);
    let package = !content
        .stat(directory)
        .map_err(|error| FileError::at(directory, error))?
        .is_dir;
    load_font(files, path, package)
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
