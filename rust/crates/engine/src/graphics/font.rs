//! Bitmap fonts, ported from font.c and gfxload.c (GPL-2.0-or-later).
use super::canvas::Canvas;
use super::dcq::DrawCommand;
use super::geometry::{Extent, HotSpot, Point, Rect, TextAlign};
use super::state::{self, Graphics};
use crate::strings::unicode::{next_char, UniChar};
use std::ffi::CStr;
use std::num::NonZeroU64;
use std::sync::Arc;

/// `MAX_DELTAS` from font.h: the internal spacing buffer.
pub const MAX_DELTAS: usize = 100;
const CHARACTER_PAGE_MASK: u32 = 0xffff_f800;

/// One character's alpha bitmap and metrics (C: `TFB_Char`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Glyph {
    pub extent: Extent,
    pub disp: Extent,
    pub hot_spot: HotSpot,
    pub data: Arc<[u8]>,
    pub pitch: usize,
}

#[derive(Debug)]
struct Page {
    page_start: u32,
    first_char: u32,
    chars: Vec<Option<Glyph>>,
}

/// C: `FONT_DESC`.
#[derive(Debug, Default)]
pub struct Font {
    pub leading: u16,
    pub leading_width: u16,
    pages: Vec<Page>,
}

/// A string to measure or draw (C: `TEXT`): UTF-8 bytes, at most
/// `char_count` characters, ending early at a NUL.
#[derive(Clone, Copy, Debug)]
pub struct Text<'a> {
    pub baseline: Point,
    pub bytes: &'a [u8],
    pub align: TextAlign,
    pub char_count: u16,
}

impl<'a> Text<'a> {
    pub fn new(baseline: Point, bytes: &'a [u8], align: TextAlign) -> Self {
        Self {
            baseline,
            bytes,
            align,
            char_count: u16::MAX,
        }
    }
}

fn tuned_baseline(height: i16) -> i16 {
    let tune = match height {
        8 => -1,
        9 => -2,
        h if h > 9 => -3,
        _ => 0,
    };
    height.wrapping_add(tune)
}

impl Glyph {
    /// A glyph from a decoded character image.
    pub fn from_canvas(canvas: &Canvas) -> Self {
        let extent = Extent::new(canvas.width() as i16, canvas.height() as i16);
        let pitch = canvas.width() as usize;
        let mut data = vec![0; pitch * canvas.height() as usize];
        canvas.font_char_data(&mut data, pitch);
        Self {
            extent,
            disp: Extent::new(extent.width.wrapping_add(1), extent.height.wrapping_add(1)),
            hot_spot: HotSpot::new(0, tuned_baseline(extent.height)),
            data: data.into(),
            pitch,
        }
    }
}

impl Font {
    /// Build a font from `(character, glyph image)` pairs.
    pub fn build(mut chars: Vec<(u32, Canvas)>) -> Self {
        chars.sort_by_key(|(index, _)| *index);
        let mut font = Self::default();
        let mut start = 0;
        while start < chars.len() {
            let page_start = chars[start].0 & CHARACTER_PAGE_MASK;
            let mut end = start;
            while end < chars.len() && chars[end].0 & CHARACTER_PAGE_MASK == page_start {
                end += 1;
            }
            let first_char = chars[start].0;
            let count = (chars[end - 1].0 + 1 - first_char) as usize;
            let mut page = Page {
                page_start,
                first_char,
                chars: vec![None; count],
            };
            for (index, canvas) in &chars[start..end] {
                let slot = &mut page.chars[(index - first_char) as usize];
                if slot.is_some() {
                    continue;
                }
                let glyph = Glyph::from_canvas(canvas);
                if glyph.disp.height as i32 > i32::from(font.leading) {
                    font.leading = glyph.disp.height as u16;
                }
                if glyph.disp.width as i32 > i32::from(font.leading_width) {
                    font.leading_width = glyph.disp.width as u16;
                }
                *slot = Some(glyph);
            }
            font.pages.push(page);
            start = end;
        }
        font.leading = font.leading.wrapping_add(1);
        font
    }

    /// Each page's start, first character and character count.
    pub fn page_layout(&self) -> Vec<(u32, u32, usize)> {
        self.pages
            .iter()
            .map(|page| (page.page_start, page.first_char, page.chars.len()))
            .collect()
    }

    /// The glyph for `ch` to adjust, if the font has one.
    pub fn glyph_mut(&mut self, ch: UniChar) -> Option<&mut Glyph> {
        let page_start = ch & CHARACTER_PAGE_MASK;
        let page = self
            .pages
            .iter_mut()
            .find(|page| page.page_start == page_start)?;
        let index = ch.checked_sub(page.first_char)? as usize;
        page.chars.get_mut(index)?.as_mut()
    }

    /// The glyph for `ch`, if the font has one.
    pub fn glyph(&self, ch: UniChar) -> Option<&Glyph> {
        let page_start = ch & CHARACTER_PAGE_MASK;
        let page = self
            .pages
            .iter()
            .find(|page| page.page_start == page_start)?;
        if ch < page.first_char {
            return None;
        }
        page.chars
            .get(ch.wrapping_sub(page.first_char) as usize)?
            .as_ref()
    }

    /// C: `TextRect`: the text's bounding rectangle and per-character
    /// advances. Without `deltas` the text is capped at [`MAX_DELTAS`]
    /// characters. Returns `None` (with the rectangle at the baseline) when
    /// nothing visible is measured.
    pub fn measure(
        font: Option<&Self>,
        text: &mut Text<'_>,
        deltas: Option<&mut [u8]>,
    ) -> (bool, Rect) {
        let mut local = [0u8; MAX_DELTAS];
        let empty = (
            false,
            Rect {
                corner: text.baseline,
                extent: Extent::zero(),
            },
        );
        let Some(font) = font.filter(|_| text.char_count != 0) else {
            return empty;
        };
        let mut count = text.char_count;
        let deltas = match deltas {
            Some(deltas) => deltas,
            None => {
                if usize::from(count) > MAX_DELTAS {
                    count = MAX_DELTAS as u16;
                    text.char_count = count;
                }
                &mut local[..]
            }
        };
        let (mut top, mut bottom, mut width) = (0i16, 0i16, 0i16);
        let mut string = text.bytes;
        let mut next = 0;
        if count > 0 {
            next = next_char(&mut string);
            if next == 0 {
                count = 0;
            }
        }
        let mut written = 0;
        while count > 0 {
            count -= 1;
            let last_width = width;
            let ch = next;
            if count > 0 {
                next = next_char(&mut string);
                if next == 0 {
                    text.char_count -= count;
                    count = 0;
                }
            }
            if let Some(glyph) = font.glyph(ch).filter(|glyph| glyph.disp.width != 0) {
                let y = glyph.hot_spot.y.wrapping_neg();
                top = top.min(y);
                bottom = bottom.max(y.wrapping_add(glyph.disp.height));
                width = width.wrapping_add(glyph.disp.width);
            }
            deltas[written] = width.wrapping_sub(last_width) as u8;
            written += 1;
        }
        if width > 0 {
            bottom = bottom.wrapping_sub(top);
            if bottom > 0 {
                if deltas[written - 1] > 0 {
                    deltas[written - 1] -= 1;
                    width -= 1;
                }
                let x = match text.align {
                    TextAlign::Left => 0,
                    TextAlign::Center => -(width >> 1),
                    TextAlign::Right => -width,
                };
                return (
                    true,
                    Rect {
                        corner: Point::new(
                            x.wrapping_add(text.baseline.x),
                            top.wrapping_add(text.baseline.y),
                        ),
                        extent: Extent::new(width, bottom),
                    },
                );
            }
        }
        empty
    }

    /// The glyphs of `text` and their pen positions, starting at the left
    /// edge of `clip` on the baseline; glyphs outside `clip` are skipped but
    /// still advance the pen.
    pub(crate) fn layout(&self, text: &Text<'_>, clip: Rect) -> Vec<(Point, Glyph)> {
        let mut placed = Vec::new();
        let mut origin = Point::new(clip.corner.x, text.baseline.y);
        let mut count = text.char_count;
        if count == 0 {
            return placed;
        }
        let mut string = text.bytes;
        let mut next = next_char(&mut string);
        if next == 0 {
            count = 0;
        }
        while count > 0 {
            count -= 1;
            let ch = next;
            if count > 0 {
                next = next_char(&mut string);
                if next == 0 {
                    count = 0;
                }
            }
            let Some(glyph) = self.glyph(ch).filter(|glyph| glyph.disp.width != 0) else {
                continue;
            };
            let mut r = Rect {
                corner: Point::new(
                    origin.x.wrapping_sub(glyph.hot_spot.x),
                    origin.y.wrapping_sub(glyph.hot_spot.y),
                ),
                extent: glyph.disp,
            };
            if super::bbox::box_intersect(r, clip, &mut r) != 0 {
                placed.push((origin, glyph.clone()));
            }
            origin.x = origin.x.wrapping_add(glyph.disp.width);
        }
        placed
    }
}

/// A copyable name for a loaded font (C: `FONT`). Names are never reused.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FontId(NonZeroU64);

impl FontId {
    pub fn bits(self) -> usize {
        self.0.get() as usize
    }
}

#[derive(Debug)]
struct FontEntry {
    font: Font,
    leases: u32,
    owned: bool,
}

/// The font registry.
#[derive(Debug, Default)]
pub(crate) struct Fonts {
    entries: std::collections::HashMap<NonZeroU64, FontEntry>,
    next: u64,
}

impl Graphics {
    pub(crate) fn font(&self, font: FontId) -> &Font {
        &self
            .fonts
            .entries
            .get(&font.0)
            .unwrap_or_else(|| panic!("font {font:?} was released"))
            .font
    }

    #[cfg(all(feature = "test-support", not(test)))]
    pub(crate) fn font_mut(&mut self, font: FontId) -> &mut Font {
        &mut self
            .fonts
            .entries
            .get_mut(&font.0)
            .unwrap_or_else(|| panic!("font {font:?} was released"))
            .font
    }

    fn add_font(&mut self, font: Font) -> FontId {
        self.fonts.next += 1;
        let id = NonZeroU64::new(self.fonts.next).unwrap_or_else(|| unreachable!());
        self.fonts.entries.insert(
            id,
            FontEntry {
                font,
                leases: 1,
                owned: true,
            },
        );
        FontId(id)
    }

    pub(crate) fn retain_font(&mut self, font: FontId) {
        self.fonts
            .entries
            .get_mut(&font.0)
            .unwrap_or_else(|| panic!("font {font:?} was released"))
            .leases += 1;
    }

    /// Let go of a lease; the last one releases the glyphs through the
    /// queue (C: `release_font_data`).
    pub(crate) fn release_font(&mut self, font: FontId) {
        let entry = self
            .fonts
            .entries
            .get_mut(&font.0)
            .unwrap_or_else(|| panic!("font {font:?} was released"));
        entry.leases -= 1;
        if entry.leases != 0 {
            return;
        }
        let entry = self
            .fonts
            .entries
            .remove(&font.0)
            .unwrap_or_else(|| unreachable!());
        if self.current.font == Some(font) {
            self.set_context_font(None);
        }
        for page in entry.font.pages {
            for glyph in page.chars.into_iter().flatten() {
                self.enqueue(DrawCommand::DeleteData(glyph.data));
            }
        }
    }
}

/// A held font.
#[derive(Debug, PartialEq, Eq)]
pub struct FontLease(FontId);

impl FontLease {
    /// Hold a loaded font whose owner has not released it.
    pub fn retain(font: FontId) -> Self {
        state::with(|g| {
            let entry = g
                .fonts
                .entries
                .get_mut(&font.0)
                .filter(|entry| entry.owned)
                .unwrap_or_else(|| panic!("font is unowned or retired: {font:?}"));
            entry.leases += 1;
        });
        Self(font)
    }

    /// Hold a font another holder keeps alive (a context's selection),
    /// whether or not its owner has released it.
    pub(crate) fn held(font: FontId) -> Self {
        state::with(|g| g.retain_font(font));
        Self(font)
    }

    pub fn font(&self) -> FontId {
        self.0
    }

    pub fn same_font(&self, other: &Self) -> bool {
        self.0 == other.0
    }

    pub fn leading(&self) -> i16 {
        state::with(|g| g.font(self.0).leading as i16)
    }

    pub fn measure(
        &self,
        string: &CStr,
        baseline: Point,
        align: TextAlign,
        characters: u16,
    ) -> TextMeasurement {
        self.measure_bounded::<MAX_DELTAS>(string, baseline, align, characters)
    }

    /// C: `TextRect` with an `N`-entry delta buffer.
    pub fn measure_bounded<const N: usize>(
        &self,
        string: &CStr,
        baseline: Point,
        align: TextAlign,
        characters: u16,
    ) -> TextMeasurement<N> {
        let mut measured = TextMeasurement {
            rect: Rect {
                corner: baseline,
                extent: Extent::zero(),
            },
            characters: characters.min(N.min(u16::MAX as usize) as u16),
            deltas: [0; N],
            leading: 0,
            visible: false,
        };
        let mut text = Text {
            baseline,
            bytes: string.to_bytes(),
            align,
            char_count: measured.characters,
        };
        state::with(|g| {
            let font = g.font(self.0);
            measured.leading = font.leading as i16;
            (measured.visible, measured.rect) =
                Font::measure(Some(font), &mut text, Some(&mut measured.deltas));
        });
        measured.characters = text.char_count;
        measured
    }
}

impl Clone for FontLease {
    fn clone(&self) -> Self {
        state::with(|g| g.retain_font(self.0));
        Self(self.0)
    }
}

impl Drop for FontLease {
    fn drop(&mut self) {
        state::with(|g| g.release_font(self.0));
    }
}

/// A measured string (C: `TextRect` results).
#[derive(Debug)]
pub struct TextMeasurement<const N: usize = MAX_DELTAS> {
    pub rect: Rect,
    pub characters: u16,
    pub deltas: [u8; N],
    pub leading: i16,
    pub visible: bool,
}

/// Unique release authority for a loaded font; leases keep its glyphs.
#[derive(Debug)]
pub struct LoadedFont {
    lease: FontLease,
}

impl LoadedFont {
    /// Register a built font.
    pub(crate) fn adopt(font: Font) -> Self {
        Self {
            lease: FontLease(state::with(|g| g.add_font(font))),
        }
    }

    /// Load a font resource.
    pub fn load(
        content: &crate::resource::Content,
        resource: &std::ffi::CStr,
    ) -> Result<Self, crate::resource::AssetLoadError> {
        let resolved = content.require(resource, crate::resource::AssetKind::Font)?;
        let path = std::ffi::CString::new(resolved.descriptor)
            .map_err(|error| resolved.load_error(resource, error))?;
        Self::load_file(resolved.files, &path).map_err(|error| resolved.load_error(resource, error))
    }

    /// Load a font directory or package.
    pub fn load_file(
        files: &crate::io::Dir,
        path: &CStr,
    ) -> Result<Self, crate::resource::FileError> {
        super::load::load_font_file(files, path)
    }

    pub fn lease(&self) -> FontLease {
        self.lease.clone()
    }

    pub fn font(&self) -> FontId {
        self.lease.0
    }
}

impl Drop for LoadedFont {
    fn drop(&mut self) {
        let font = self.lease.0;
        state::with(|g| {
            let entry = g
                .fonts
                .entries
                .get_mut(&font.0)
                .unwrap_or_else(|| panic!("font {font:?} was released"));
            assert!(entry.owned, "font {font:?} released twice");
            entry.owned = false;
        });
    }
}

#[cfg(test)]
mod tests {
    use super::super::canvas::PixelFormat;
    use super::*;

    fn glyph_canvas(width: i32, height: i32) -> Canvas {
        let mut canvas = Canvas::new(PixelFormat::Rgba, width, height);
        canvas.words_mut().fill(0xffff_ffff);
        canvas
    }

    #[test]
    fn font_baseline_tuning_preserves_legacy_spacing() {
        assert_eq!([7, 8, 9, 10, 15].map(tuned_baseline), [7, 7, 7, 7, 12]);
    }

    #[test]
    fn measurement_caps_internal_deltas_and_aligns() {
        let font = Font::build(vec![
            (u32::from(b'a'), glyph_canvas(3, 10)),
            (u32::from(b'b'), glyph_canvas(2, 10)),
        ]);
        assert_eq!((font.leading, font.leading_width), (12, 4));
        let mut text = Text::new(Point::new(10, 20), b"ab", TextAlign::Center);
        let mut deltas = [0; 2];
        let (visible, rect) = Font::measure(Some(&font), &mut text, Some(&mut deltas));
        assert!(visible);
        assert_eq!(deltas, [4, 2]);
        assert_eq!(rect, Rect::new(7, 13, 6, 11));
        let long = [b'a'; 150];
        let mut text = Text::new(Point::default(), &long, TextAlign::Left);
        Font::measure(Some(&font), &mut text, None);
        assert_eq!(text.char_count, MAX_DELTAS as u16);
    }

    #[test]
    fn sparse_pages_and_missing_glyphs_measure_as_empty() {
        let font = Font::build(vec![
            (0x41, glyph_canvas(1, 1)),
            (0x900, glyph_canvas(1, 1)),
        ]);
        assert!(font.glyph(0x41).is_some());
        assert!(font.glyph(0x42).is_none());
        assert!(font.glyph(0x900).is_some());
        let mut text = Text::new(Point::new(1, 2), b"", TextAlign::Left);
        assert_eq!(
            Font::measure(Some(&font), &mut text, None),
            (false, Rect::new(1, 2, 0, 0))
        );
    }
}
