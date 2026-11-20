//! Owned pixel-buffer canvases.
//!
//! A canvas is a Rust pixel buffer in one of the three formats the game draws
//! with: 32-bit RGBX and RGBA (`0xRRGGBBAA`, red in the high byte, the alpha or
//! padding byte low) and 8-bit indexed with a 256-entry palette. Blits, fills
//! and clipping keep the exact SDL 2 surface semantics the C game was built
//! on, including colour keys, the per-surface blend mode and alpha modulation,
//! so canvas contents stay byte-identical to the surfaces they replace.
use super::geometry::{Color, Point, Rect};

pub mod png;
pub mod raster;
pub mod rotate;
pub mod scale;

/// Canvas pixel layout.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PixelFormat {
    /// 32-bit `0xRRGGBBXX`; the low byte carries no alpha.
    Rgbx,
    /// 32-bit `0xRRGGBBAA`.
    Rgba,
    /// 8-bit palette indexes.
    Indexed,
}

impl PixelFormat {
    pub fn has_alpha(self) -> bool {
        self == Self::Rgba
    }

    pub fn bytes_per_pixel(self) -> u8 {
        match self {
            Self::Indexed => 1,
            Self::Rgbx | Self::Rgba => 4,
        }
    }

    pub fn bits_per_pixel(self) -> u8 {
        self.bytes_per_pixel() * 8
    }

    pub const RED_MASK: u32 = 0xff00_0000;
    pub const GREEN_MASK: u32 = 0x00ff_0000;
    pub const BLUE_MASK: u32 = 0x0000_ff00;
    pub const ALPHA_MASK: u32 = 0x0000_00ff;

    pub fn alpha_mask(self) -> u32 {
        if self.has_alpha() {
            Self::ALPHA_MASK
        } else {
            0
        }
    }
}

/// A signed SDL-style rectangle in pixel coordinates.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PixelRect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl PixelRect {
    pub const fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Self { x, y, w, h }
    }

    pub fn is_empty(self) -> bool {
        self.w <= 0 || self.h <= 0
    }

    /// `SDL_IntersectRect`: an empty input leaves the result position
    /// unchanged and zeroes its size.
    pub fn intersect(self, other: Self, result: &mut Self) -> bool {
        if self.is_empty() || other.is_empty() {
            result.w = 0;
            result.h = 0;
            return false;
        }
        let x = self.x.max(other.x);
        let w = (self.x + self.w).min(other.x + other.w) - x;
        let y = self.y.max(other.y);
        let h = (self.y + self.h).min(other.y + other.h) - y;
        *result = Self { x, y, w, h };
        !result.is_empty()
    }
}

#[derive(Clone)]
enum Pixels {
    Rgb(Vec<u32>),
    Indexed {
        data: Vec<u8>,
        palette: Box<[Color; 256]>,
    },
}

/// An owned drawing surface.
#[derive(Clone)]
pub struct Canvas {
    width: i32,
    height: i32,
    format: PixelFormat,
    pixels: Pixels,
    color_key: Option<u32>,
    blend: bool,
    alpha_mod: u8,
    clip: PixelRect,
}

impl std::fmt::Debug for Canvas {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Canvas")
            .field("width", &self.width)
            .field("height", &self.height)
            .field("format", &self.format)
            .field("color_key", &self.color_key)
            .field("blend", &self.blend)
            .field("alpha_mod", &self.alpha_mod)
            .field("clip", &self.clip)
            .finish()
    }
}

const WHITE: Color = Color {
    r: 0xff,
    g: 0xff,
    b: 0xff,
    a: 0xff,
};

impl Canvas {
    /// A zero-filled canvas. Indexed palettes start white, alpha formats
    /// start with blending enabled, as `SDL_CreateRGBSurface` does.
    pub fn new(format: PixelFormat, width: i32, height: i32) -> Self {
        assert!(
            width >= 0 && height >= 0,
            "canvas size {width}x{height} is negative"
        );
        let count = width as usize * height as usize;
        let pixels = match format {
            PixelFormat::Indexed => Pixels::Indexed {
                data: vec![0; count],
                palette: Box::new([WHITE; 256]),
            },
            PixelFormat::Rgbx | PixelFormat::Rgba => Pixels::Rgb(vec![0; count]),
        };
        Self {
            width,
            height,
            format,
            pixels,
            color_key: None,
            blend: format.has_alpha(),
            alpha_mod: 255,
            clip: PixelRect::new(0, 0, width, height),
        }
    }

    pub fn width(&self) -> i32 {
        self.width
    }

    pub fn height(&self) -> i32 {
        self.height
    }

    pub fn format(&self) -> PixelFormat {
        self.format
    }

    pub fn is_indexed(&self) -> bool {
        self.format == PixelFormat::Indexed
    }

    /// Bytes per row of the packed pixel buffer.
    pub fn pitch(&self) -> usize {
        self.width as usize * usize::from(self.format.bytes_per_pixel())
    }

    pub fn bounds(&self) -> PixelRect {
        PixelRect::new(0, 0, self.width, self.height)
    }

    // ----- pixel words ---------------------------------------------------

    /// `SDL_MapRGBA` for this canvas.
    pub fn map_rgba(&self, r: u8, g: u8, b: u8, a: u8) -> u32 {
        match &self.pixels {
            Pixels::Indexed { palette, .. } => u32::from(find_color(palette, r, g, b, a)),
            Pixels::Rgb(_) => {
                let rgb = u32::from(r) << 24 | u32::from(g) << 16 | u32::from(b) << 8;
                if self.format.has_alpha() {
                    rgb | u32::from(a)
                } else {
                    rgb
                }
            }
        }
    }

    /// `SDL_GetRGBA` for this canvas.
    pub fn get_rgba(&self, pixel: u32) -> Color {
        match &self.pixels {
            Pixels::Indexed { palette, .. } => palette[(pixel & 0xff) as usize],
            Pixels::Rgb(_) => Color {
                r: (pixel >> 24) as u8,
                g: (pixel >> 16) as u8,
                b: (pixel >> 8) as u8,
                a: if self.format.has_alpha() {
                    pixel as u8
                } else {
                    0xff
                },
            },
        }
    }

    fn index(&self, x: i32, y: i32) -> usize {
        assert!(
            x >= 0 && y >= 0 && x < self.width && y < self.height,
            "pixel ({x}, {y}) outside {}x{} canvas",
            self.width,
            self.height
        );
        y as usize * self.width as usize + x as usize
    }

    /// Raw pixel word: a palette index for indexed canvases.
    pub fn pixel(&self, x: i32, y: i32) -> u32 {
        let index = self.index(x, y);
        match &self.pixels {
            Pixels::Rgb(data) => data[index],
            Pixels::Indexed { data, .. } => u32::from(data[index]),
        }
    }

    pub fn set_pixel(&mut self, x: i32, y: i32, pixel: u32) {
        let index = self.index(x, y);
        match &mut self.pixels {
            Pixels::Rgb(data) => data[index] = pixel,
            Pixels::Indexed { data, .. } => data[index] = pixel as u8,
        }
    }

    /// The 32-bit pixel rows, row-major without padding.
    pub fn words(&self) -> &[u32] {
        match &self.pixels {
            Pixels::Rgb(data) => data,
            Pixels::Indexed { .. } => panic!("indexed canvas has no 32-bit pixel words"),
        }
    }

    pub fn words_mut(&mut self) -> &mut [u32] {
        match &mut self.pixels {
            Pixels::Rgb(data) => data,
            Pixels::Indexed { .. } => panic!("indexed canvas has no 32-bit pixel words"),
        }
    }

    pub fn indexes(&self) -> &[u8] {
        match &self.pixels {
            Pixels::Indexed { data, .. } => data,
            Pixels::Rgb(_) => panic!("32-bit canvas has no palette indexes"),
        }
    }

    pub fn indexes_mut(&mut self) -> &mut [u8] {
        match &mut self.pixels {
            Pixels::Indexed { data, .. } => data,
            Pixels::Rgb(_) => panic!("32-bit canvas has no palette indexes"),
        }
    }

    // ----- palette, key, blending -------------------------------------------

    pub fn palette(&self) -> Option<&[Color; 256]> {
        match &self.pixels {
            Pixels::Indexed { palette, .. } => Some(palette),
            Pixels::Rgb(_) => None,
        }
    }

    /// `SDL_SetPaletteColors`; a no-op on 32-bit canvases.
    pub fn set_palette_colors(&mut self, first: usize, colors: &[Color]) {
        if let Pixels::Indexed { palette, .. } = &mut self.pixels {
            palette[first..first + colors.len()].copy_from_slice(colors);
        }
    }

    pub fn color_key(&self) -> Option<u32> {
        self.color_key
    }

    pub fn set_color_key(&mut self, key: Option<u32>) {
        self.color_key = key;
    }

    pub fn blend(&self) -> bool {
        self.blend
    }

    pub fn set_blend(&mut self, blend: bool) {
        self.blend = blend;
    }

    pub fn alpha_mod(&self) -> u8 {
        self.alpha_mod
    }

    pub fn set_alpha_mod(&mut self, alpha: u8) {
        self.alpha_mod = alpha;
    }

    // ----- clipping and fills ----------------------------------------------

    pub fn clip_rect(&self) -> PixelRect {
        self.clip
    }

    /// `SDL_SetClipRect`.
    pub fn set_clip_rect(&mut self, rect: Option<PixelRect>) -> bool {
        let full = self.bounds();
        match rect {
            None => {
                self.clip = full;
                true
            }
            Some(rect) => {
                let mut clip = self.clip;
                let visible = rect.intersect(full, &mut clip);
                self.clip = clip;
                visible
            }
        }
    }

    /// `SDL_FillRect`.
    pub fn fill_rect(&mut self, rect: Option<PixelRect>, color: u32) {
        let rect = rect.unwrap_or(self.clip);
        if rect.is_empty() {
            return;
        }
        let mut clipped = PixelRect::default();
        if !rect.intersect(self.clip, &mut clipped) {
            return;
        }
        let width = self.width as usize;
        let (x0, x1) = (clipped.x as usize, (clipped.x + clipped.w) as usize);
        for y in clipped.y..clipped.y + clipped.h {
            let row = y as usize * width;
            match &mut self.pixels {
                Pixels::Rgb(data) => data[row + x0..row + x1].fill(color),
                Pixels::Indexed { data, .. } => data[row + x0..row + x1].fill(color as u8),
            }
        }
    }
}

impl Canvas {
    // ----- transparency (sdl/canvas.c) ------------------------------------

    /// The colour key of a 32-bit canvas as a colour.
    pub fn transparent_color(&self) -> Option<Color> {
        if self.is_indexed() {
            return None;
        }
        self.color_key.map(|key| Color {
            a: 0xff,
            ..self.get_rgba(key)
        })
    }

    /// Key out `color` and stop blending, as `TFB_DrawCanvas_SetTransparentColor`.
    pub fn set_transparent_color(&mut self, color: Color) {
        self.color_key = Some(self.map_rgba(color.r, color.g, color.b, 0));
        if !self.is_indexed() {
            self.disable_alpha_mod();
        }
    }

    /// `TFB_DrawCanvas_SetTransparentIndex`: a negative index removes the key.
    pub fn set_transparent_index(&mut self, index: i32) {
        if index >= 0 {
            self.color_key = Some(index as u32);
            if !self.is_indexed() {
                self.disable_alpha_mod();
            }
        } else {
            self.color_key = None;
        }
    }

    /// `TFB_DrawCanvas_GetTransparentIndex` as the engine has always run it:
    /// -1 for a keyed canvas and 0 for an unkeyed one. Rotation and nearest
    /// scale targets inherit this through `copy_transparency`, and the replay
    /// goldens depend on it.
    pub fn transparent_index(&self) -> i32 {
        if self.color_key.is_some() {
            -1
        } else {
            0
        }
    }

    /// `TFB_DrawCanvas_CopyTransparencyInfo`.
    pub fn copy_transparency(&mut self, source: &Canvas) {
        if source.is_indexed() {
            self.set_transparent_index(source.transparent_index());
        } else if let Some(color) = source.transparent_color() {
            self.set_transparent_color(color);
        }
    }

    fn disable_alpha_mod(&mut self) {
        self.alpha_mod = 255;
        self.blend = false;
    }

    /// Replace the palette from `colors`, keeping alpha as given.
    pub fn set_palette(&mut self, colors: &[Color; 256]) {
        self.set_palette_colors(0, colors);
    }

    /// The palette as opaque colours, as `TFB_DrawCanvas_ExtractPalette`.
    pub fn extract_palette(&self) -> Option<[Color; 256]> {
        self.palette()
            .map(|palette| palette.map(|color| Color { a: 0xff, ..color }))
    }

    // ----- pixel access (sdl/canvas.c) ------------------------------------

    /// The colour at `(x, y)`, or transparent black outside the canvas.
    pub fn color_at(&self, x: i32, y: i32) -> Color {
        if x < 0 || x >= self.width || y < 0 || y >= self.height {
            return Color::default();
        }
        self.get_rgba(self.pixel(x, y))
    }

    /// Alpha-only glyph data, `pitch` bytes per row; opaque formats derive
    /// alpha from intensity.
    pub fn font_char_data(&self, out: &mut [u8], pitch: usize) {
        for y in 0..self.height {
            for x in 0..self.width {
                let c = self.get_rgba(self.pixel(x, y));
                let a = if self.format.has_alpha() {
                    c.a
                } else {
                    ((u32::from(c.r) * 2 + u32::from(c.g) * 4 + u32::from(c.b)) / 7) as u8
                };
                out[y as usize * pitch + x as usize] = a;
            }
        }
    }

    fn collision_mask_and_key(&self) -> (u32, u32) {
        if self.format.has_alpha() {
            (self.format.alpha_mask(), 0)
        } else {
            (u32::MAX, self.color_key.unwrap_or(0))
        }
    }

    /// Whether any pixel inside `area` is opaque in both canvases placed at
    /// their origins.
    pub fn intersects(
        &self,
        origin: Point,
        other: &Canvas,
        other_origin: Point,
        area: Rect,
    ) -> bool {
        let (mask1, key1) = self.collision_mask_and_key();
        let (mask2, key2) = other.collision_mask_and_key();
        let offset = |org: Point| {
            (
                i32::from((i32::from(area.corner.x) - i32::from(org.x)) as i16),
                i32::from((i32::from(area.corner.y) - i32::from(org.y)) as i16),
            )
        };
        let (x1, y1) = offset(origin);
        let (x2, y2) = offset(other_origin);
        for y in 0..i32::from(area.extent.height) {
            for x in 0..i32::from(area.extent.width) {
                let p1 = self.pixel(x + x1, y + y1) & mask1;
                let p2 = other.pixel(x + x2, y + y2) & mask2;
                if p1 != key1 && p2 != key2 {
                    return true;
                }
            }
        }
        false
    }

    /// Read the overlap of the canvas and a `width`-wide colour array.
    pub fn read_colors(&self, pixels: &mut [Color], width: i32, height: i32) {
        for y in 0..height.min(self.height) {
            for x in 0..width.min(self.width) {
                pixels[(y * width + x) as usize] = self.get_rgba(self.pixel(x, y));
            }
        }
    }

    /// Write the overlap of a `width`-wide colour array into the canvas.
    pub fn write_colors(&mut self, pixels: &[Color], width: i32, height: i32) {
        for y in 0..height.min(self.height) {
            for x in 0..width.min(self.width) {
                let c = pixels[(y * width + x) as usize];
                let pixel = self.map_rgba(c.r, c.g, c.b, c.a);
                self.set_pixel(x, y, pixel);
            }
        }
    }

    /// Read the overlap of the palette indexes into a `width`-wide array.
    pub fn read_indexes(&self, data: &mut [u8], width: i32, height: i32) {
        let row = width.min(self.width) as usize;
        for y in 0..height.min(self.height) as usize {
            let source = y * self.width as usize;
            let target = y * width as usize;
            data[target..target + row].copy_from_slice(&self.indexes()[source..source + row]);
        }
    }

    /// Write the overlap of a `width`-wide index array into the canvas.
    pub fn write_indexes(&mut self, data: &[u8], width: i32, height: i32) {
        let row = width.min(self.width) as usize;
        let stride = self.width as usize;
        for y in 0..height.min(self.height) as usize {
            let source = y * width as usize;
            self.indexes_mut()[y * stride..y * stride + row]
                .copy_from_slice(&data[source..source + row]);
        }
    }
}

/// `SDL_FindColor`.
fn find_color(palette: &[Color; 256], r: u8, g: u8, b: u8, a: u8) -> u8 {
    let mut smallest = u32::MAX;
    let mut pixel = 0;
    for (index, color) in palette.iter().enumerate() {
        let delta = |x: u8, y: u8| {
            let d = i32::from(x) - i32::from(y);
            (d * d) as u32
        };
        let distance =
            delta(color.r, r) + delta(color.g, g) + delta(color.b, b) + delta(color.a, a);
        if distance < smallest {
            pixel = index as u8;
            if distance == 0 {
                break;
            }
            smallest = distance;
        }
    }
    pixel
}

/// `SDL_UpperBlit`: clip `source_rect` to the source and the destination
/// clip rectangle, update `destination_rect` to the drawn area and copy.
pub fn blit(
    source: &Canvas,
    source_rect: Option<PixelRect>,
    destination: &mut Canvas,
    destination_rect: Option<&mut PixelRect>,
) {
    upper_blit(source, source_rect, destination, destination_rect, None);
}

/// [`blit`] with the source blended at a constant surface alpha, as the
/// engine's `DRAW_ALPHA` mode does for opaque images.
pub fn blit_modulated(
    source: &Canvas,
    source_rect: Option<PixelRect>,
    destination: &mut Canvas,
    destination_rect: Option<&mut PixelRect>,
    alpha: u8,
) {
    upper_blit(
        source,
        source_rect,
        destination,
        destination_rect,
        Some(alpha),
    );
}

fn upper_blit(
    source: &Canvas,
    source_rect: Option<PixelRect>,
    destination: &mut Canvas,
    destination_rect: Option<&mut PixelRect>,
    alpha: Option<u8>,
) {
    let mut full = PixelRect::new(0, 0, destination.width, destination.height);
    let dst = destination_rect.unwrap_or(&mut full);
    let (mut sx, mut sy, mut w, mut h);
    if let Some(rect) = source_rect {
        sx = rect.x;
        w = rect.w;
        if sx < 0 {
            w += sx;
            dst.x -= sx;
            sx = 0;
        }
        w = w.min(source.width - sx);
        sy = rect.y;
        h = rect.h;
        if sy < 0 {
            h += sy;
            dst.y -= sy;
            sy = 0;
        }
        h = h.min(source.height - sy);
    } else {
        sx = 0;
        sy = 0;
        w = source.width;
        h = source.height;
    }
    let clip = destination.clip;
    let dx = clip.x - dst.x;
    if dx > 0 {
        w -= dx;
        dst.x += dx;
        sx += dx;
    }
    let dx = dst.x + w - clip.x - clip.w;
    if dx > 0 {
        w -= dx;
    }
    let dy = clip.y - dst.y;
    if dy > 0 {
        h -= dy;
        dst.y += dy;
        sy += dy;
    }
    let dy = dst.y + h - clip.y - clip.h;
    if dy > 0 {
        h -= dy;
    }
    if w > 0 && h > 0 {
        dst.w = w;
        dst.h = h;
        lower_blit(
            source,
            PixelRect::new(sx, sy, w, h),
            destination,
            dst.x,
            dst.y,
            alpha,
        );
    } else {
        dst.w = 0;
        dst.h = 0;
    }
}

/// The per-pixel transfer of an already clipped blit.
fn lower_blit(
    source: &Canvas,
    area: PixelRect,
    destination: &mut Canvas,
    dx: i32,
    dy: i32,
    alpha: Option<u8>,
) {
    let alpha_mod = alpha.unwrap_or(source.alpha_mod);
    let span = Span {
        area,
        src_width: source.width as usize,
        dst_width: destination.width as usize,
        dx,
        dy,
    };
    let dst_format = destination.format;
    let Pixels::Rgb(target) = &mut destination.pixels else {
        panic!("blits onto indexed canvases are not supported");
    };
    match &source.pixels {
        Pixels::Indexed { data, palette } => blit_indexed(
            data,
            palette,
            source.color_key,
            alpha_mod,
            dst_format.has_alpha(),
            span,
            target,
        ),
        Pixels::Rgb(data) => {
            let src_alpha = source.format.has_alpha();
            let modulate = alpha_mod != 255;
            let blend = (source.blend || alpha.is_some()) && src_alpha || modulate;
            match (blend, src_alpha) {
                (false, _) => blit_opaque(source, data, dst_format, span, target),
                (true, true) => {
                    blit_pixel_alpha(data, source.color_key, alpha_mod, dst_format, span, target)
                }
                (true, false) => blit_surface_alpha(
                    data,
                    source.color_key,
                    alpha_mod,
                    dst_format.has_alpha(),
                    span,
                    target,
                ),
            }
        }
    }
}

/// The rows a clipped blit copies: source and destination start offsets.
#[derive(Clone, Copy)]
struct Span {
    area: PixelRect,
    src_width: usize,
    dst_width: usize,
    dx: i32,
    dy: i32,
}

impl Span {
    fn rows(self) -> impl Iterator<Item = (usize, usize)> {
        (0..self.area.h).map(move |row| {
            (
                (self.area.y + row) as usize * self.src_width + self.area.x as usize,
                (self.dy + row) as usize * self.dst_width + self.dx as usize,
            )
        })
    }

    fn width(self) -> usize {
        self.area.w as usize
    }
}

/// An indexed source through its palette, skipping the key index.
fn blit_indexed(
    data: &[u8],
    palette: &[Color; 256],
    key: Option<u32>,
    alpha_mod: u8,
    dst_alpha: bool,
    span: Span,
    target: &mut [u32],
) {
    let modulate = alpha_mod != 255;
    let alpha_mod = u32::from(alpha_mod);
    let map: Vec<u32> = palette
        .iter()
        .map(|c| {
            let rgb = (u32::from(c.r) << 24) | (u32::from(c.g) << 16) | (u32::from(c.b) << 8);
            if dst_alpha {
                rgb | (u32::from(c.a) * alpha_mod / 255)
            } else {
                rgb
            }
        })
        .collect();
    for (s, d) in span.rows() {
        for i in 0..span.width() {
            let index = data[s + i];
            if key == Some(u32::from(index)) {
                continue;
            }
            target[d + i] = if modulate {
                let a = u32::from(palette[usize::from(index)].a) * alpha_mod / 255;
                blend_rgb(map[usize::from(index)], target[d + i], a, dst_alpha)
            } else {
                map[usize::from(index)]
            };
        }
    }
}

/// Opaque copies: SDL_BlitCopy, Blit4to4MaskAlpha and BlitNtoNKey.
fn blit_opaque(
    source: &Canvas,
    data: &[u32],
    dst_format: PixelFormat,
    span: Span,
    target: &mut [u32],
) {
    let rgb = PixelFormat::RED_MASK | PixelFormat::GREEN_MASK | PixelFormat::BLUE_MASK;
    let (src_alpha, dst_alpha) = (source.format.has_alpha(), dst_format.has_alpha());
    let same = source.format == dst_format;
    let key_mask = !source.format.alpha_mask();
    for (s, d) in span.rows() {
        for i in 0..span.width() {
            let pixel = data[s + i];
            target[d + i] = match source.color_key {
                Some(key) if pixel & key_mask == key & key_mask => continue,
                Some(_) if dst_alpha && src_alpha => pixel,
                Some(_) if dst_alpha => pixel | 0xff,
                Some(_) => pixel & rgb,
                None if same => pixel,
                None if dst_alpha => pixel & rgb | 0xff,
                None => pixel & rgb,
            };
        }
    }
}

/// Per-pixel alpha: the scalar composition established for every
/// architecture, with any surface alpha folded into each pixel's alpha.
fn blit_pixel_alpha(
    data: &[u32],
    key: Option<u32>,
    alpha_mod: u8,
    dst_format: PixelFormat,
    span: Span,
    target: &mut [u32],
) {
    let key = key.map(|key| key & 0xffff_ff00);
    let modulate = alpha_mod != 255;
    let alpha_mod = u32::from(alpha_mod);
    for (s, d) in span.rows() {
        for i in 0..span.width() {
            let mut pixel = data[s + i];
            if modulate {
                pixel = (pixel & 0xffff_ff00) | ((pixel & 0xff) * alpha_mod / 255);
            }
            target[d + i] = low_alpha(pixel, target[d + i], key, dst_format);
        }
    }
}

/// Surface alpha on an opaque source.
fn blit_surface_alpha(
    data: &[u32],
    key: Option<u32>,
    alpha_mod: u8,
    dst_alpha: bool,
    span: Span,
    target: &mut [u32],
) {
    for (s, d) in span.rows() {
        for i in 0..span.width() {
            let pixel = data[s + i];
            if key == Some(pixel) {
                continue;
            }
            target[d + i] =
                surface_alpha(pixel, target[d + i], alpha_mod) | if dst_alpha { 0xff } else { 0 };
        }
    }
}

/// Blend one opaque 32-bit source colour over the destination with `alpha`.
fn blend_rgb(source: u32, destination: u32, alpha: u32, dst_alpha: bool) -> u32 {
    let mut result = 0;
    for shift in [8, 16, 24] {
        let s = (source >> shift) & 0xff;
        let d = (destination >> shift) & 0xff;
        let c = (((s as i32 - d as i32) * alpha as i32) / 255 + d as i32) as u32;
        result |= c << shift;
    }
    if dst_alpha {
        let d = destination & 0xff;
        result | (alpha + d - alpha * d / 255)
    } else {
        result
    }
}

/// RGBA (`0xRRGGBBAA`) source composed over a 32-bit destination; the
/// colour key compares the colour bytes only.
pub(crate) fn low_alpha(
    source: u32,
    destination: u32,
    key: Option<u32>,
    format: PixelFormat,
) -> u32 {
    if key == Some(source & 0xffff_ff00) {
        return destination;
    }
    let alpha = (source & 255) as i32;
    if alpha == 0 {
        return destination;
    }
    let mut result = 0;
    for shift in [8, 16, 24] {
        let a = ((source >> shift) & 255) as i32;
        let b = ((destination >> shift) & 255) as i32;
        let channel = if key.is_some() {
            a * alpha / 255 + (255 - alpha) * b / 255
        } else {
            b + (a - b) * alpha / 255
        };
        result |= (channel as u32) << shift;
    }
    let alpha_mask = format.alpha_mask();
    let d = if alpha_mask == 0 {
        255
    } else {
        destination & 255
    };
    result | (alpha as u32 + d - alpha as u32 * d / 255) & alpha_mask
}

/// Opaque 32-bit source blended with a constant surface alpha.
fn surface_alpha(source: u32, destination: u32, alpha: u8) -> u32 {
    let rgb = 0xffff_ff00;
    if alpha == 128 {
        let (s, d) = (source >> 8, destination >> 8);
        return ((((s & 0xfefefe) + (d & 0xfefefe)) >> 1) + (s & d & 0x010101)) << 8 & rgb;
    }
    let alpha = u32::from(alpha);
    let blend = |mask: u32| {
        let s = (source >> 8) & mask;
        let d = (destination >> 8) & mask;
        d.wrapping_add(s.wrapping_sub(d).wrapping_mul(alpha) >> 8) & mask
    };
    (blend(0xff00ff) | blend(0xff00)) << 8 & rgb
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rgbx(w: i32, h: i32) -> Canvas {
        Canvas::new(PixelFormat::Rgbx, w, h)
    }

    #[test]
    fn new_canvases_match_sdl_surface_defaults() {
        let indexed = Canvas::new(PixelFormat::Indexed, 2, 2);
        assert_eq!(indexed.palette().unwrap()[17], WHITE);
        assert!(!indexed.blend());
        assert!(Canvas::new(PixelFormat::Rgba, 1, 1).blend());
        assert!(!rgbx(1, 1).blend());
        assert_eq!(rgbx(3, 2).clip_rect(), PixelRect::new(0, 0, 3, 2));
    }

    #[test]
    fn mapping_round_trips_each_format() {
        let canvas = rgbx(1, 1);
        assert_eq!(canvas.map_rgba(1, 2, 3, 4), 0x0102_0300);
        assert_eq!(canvas.get_rgba(0x0102_0377).a, 0xff);
        let rgba = Canvas::new(PixelFormat::Rgba, 1, 1);
        assert_eq!(rgba.map_rgba(1, 2, 3, 4), 0x0102_0304);
        assert_eq!(
            rgba.get_rgba(0x0102_0304),
            Color {
                r: 1,
                g: 2,
                b: 3,
                a: 4
            }
        );
        let mut indexed = Canvas::new(PixelFormat::Indexed, 1, 1);
        indexed.set_palette_colors(
            0,
            &[Color {
                r: 0,
                g: 0,
                b: 0,
                a: 255,
            }; 3],
        );
        indexed.set_palette_colors(
            2,
            &[Color {
                r: 10,
                g: 10,
                b: 10,
                a: 255,
            }],
        );
        assert_eq!(indexed.map_rgba(9, 9, 9, 255), 2);
    }

    #[test]
    fn clip_and_fill_follow_sdl_intersection_rules() {
        let mut canvas = rgbx(4, 3);
        assert!(canvas.set_clip_rect(Some(PixelRect::new(1, 1, 10, 1))));
        assert_eq!(canvas.clip_rect(), PixelRect::new(1, 1, 3, 1));
        canvas.fill_rect(Some(PixelRect::new(0, 0, 3, 3)), 7);
        assert_eq!(canvas.words(), &[0, 0, 0, 0, 0, 7, 7, 0, 0, 0, 0, 0]);
        assert!(!canvas.set_clip_rect(Some(PixelRect::new(9, 9, 0, 1))));
        assert_eq!(canvas.clip_rect(), PixelRect::new(1, 1, 0, 0));
    }

    #[test]
    fn indexed_keyed_blit_maps_the_palette_and_skips_the_key() {
        let mut source = Canvas::new(PixelFormat::Indexed, 2, 1);
        source.set_palette_colors(
            1,
            &[Color {
                r: 1,
                g: 2,
                b: 3,
                a: 255,
            }],
        );
        source.indexes_mut().copy_from_slice(&[1, 0]);
        source.set_color_key(Some(0));
        let mut target = rgbx(3, 1);
        target.words_mut().fill(9);
        let mut rect = PixelRect::new(1, 0, 0, 0);
        blit(&source, None, &mut target, Some(&mut rect));
        assert_eq!(target.words(), &[9, 0x0102_0300, 9]);
        assert_eq!(rect, PixelRect::new(1, 0, 2, 1));
    }

    #[test]
    fn opaque_copies_clear_padding_for_keys_and_set_alpha_for_rgba() {
        let mut source = rgbx(2, 1);
        source
            .words_mut()
            .copy_from_slice(&[0x1122_3344, 0x5566_7700]);
        source.set_color_key(Some(0x5566_7700));
        let mut target = rgbx(2, 1);
        blit(&source, None, &mut target, None);
        assert_eq!(target.words(), &[0x1122_3300, 0]);
        source.set_color_key(None);
        let mut rgba = Canvas::new(PixelFormat::Rgba, 2, 1);
        blit(&source, None, &mut rgba, None);
        assert_eq!(rgba.words(), &[0x1122_33ff, 0x5566_77ff]);
        let mut copy = rgbx(2, 1);
        blit(&source, None, &mut copy, None);
        assert_eq!(copy.words(), source.words());
    }

    #[test]
    fn source_rect_and_clip_shift_the_destination_like_sdl_upper_blit() {
        let mut source = rgbx(3, 2);
        source.words_mut().fill(0xffff_ff00);
        let mut target = rgbx(3, 2);
        target.set_clip_rect(Some(PixelRect::new(1, 0, 2, 2)));
        let mut rect = PixelRect::new(0, 0, 100, 100);
        blit(
            &source,
            Some(PixelRect::new(-1, 0, 4, 2)),
            &mut target,
            Some(&mut rect),
        );
        assert_eq!(rect, PixelRect::new(1, 0, 2, 2));
        assert_eq!(target.words()[0], 0);
        assert_eq!(target.words()[1], 0xffff_ff00);
    }

    #[test]
    fn per_pixel_alpha_composes_like_the_scalar_blitter() {
        let mut source = Canvas::new(PixelFormat::Rgba, 1, 1);
        source.words_mut()[0] = 0x1122_3380;
        let mut target = Canvas::new(PixelFormat::Rgbx, 1, 1);
        target.words_mut()[0] = 0x4433_2200;
        blit(&source, None, &mut target, None);
        assert_eq!(target.words()[0], 0x2b2b_2a00);
    }
}
