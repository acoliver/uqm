//! Images: a canvas with its cached scaled, filled and mipmap companions,
//! ported from tfb_draw.c and the image paths of sdl/canvas.c
//! (GPL-2.0-or-later).
use super::arena::{Arena, Handle};
use super::canvas::raster::{self, Plot, DRAW_ALPHA, DRAW_REPLACE};
use super::canvas::rotate::{rotate, rotated_extent, rotation_target};
use super::canvas::scale::{
    rescale_bilinear, rescale_nearest, rescale_trilinear, GSCALE_IDENTITY, TFB_SCALE_BILINEAR,
    TFB_SCALE_NEAREST, TFB_SCALE_TRILINEAR,
};
use super::canvas::{self, Canvas, PixelFormat, PixelRect};
use super::colormap::ColorMap;
use super::font::Glyph;
use super::geometry::{Color, DrawMode, Extent, HotSpot, Point, Rect};
use std::sync::Arc;

pub type ImageId = Handle<Image>;

/// C: `TFB_Image`.
#[derive(Debug)]
pub struct Image {
    normal: Canvas,
    scaled: Option<Canvas>,
    mipmap: Option<ImageId>,
    filled: Option<Canvas>,
    pub colormap_index: i32,
    colormap_version: i32,
    pub normal_hs: HotSpot,
    mipmap_hs: HotSpot,
    last_scale_hs: HotSpot,
    last_scale: i32,
    last_scale_type: i32,
    last_fill: Color,
    pub extent: Extent,
    dirty: bool,
}

impl Image {
    /// C: `TFB_DrawImage_New` for a canvas already in display format.
    pub fn new(normal: Canvas) -> Self {
        let extent = Extent::new(normal.width() as i16, normal.height() as i16);
        Self {
            normal,
            scaled: None,
            mipmap: None,
            filled: None,
            colormap_index: -1,
            colormap_version: 0,
            normal_hs: HotSpot::default(),
            mipmap_hs: HotSpot::default(),
            last_scale_hs: HotSpot::default(),
            last_scale: 0,
            last_scale_type: -1,
            last_fill: Color::default(),
            extent,
            dirty: false,
        }
    }

    /// C: `TFB_DrawImage_CreateForScreen`: an opaque or alpha 32-bit image.
    pub fn for_screen(width: i32, height: i32, alpha: bool) -> Self {
        let format = if alpha {
            PixelFormat::Rgba
        } else {
            PixelFormat::Rgbx
        };
        Self::new(Canvas::new(format, width, height))
    }

    /// C: `TFB_DrawCanvas_New_TrueColor` wrapped in an image.
    pub fn true_color(width: i32, height: i32, alpha: bool) -> Self {
        Self::for_screen(width, height, alpha)
    }

    pub fn canvas(&self) -> &Canvas {
        &self.normal
    }

    pub fn canvas_mut(&mut self) -> &mut Canvas {
        self.dirty = true;
        &mut self.normal
    }

    /// C: `SetFrameTransparentColor`: changes the key without regenerating
    /// the scaled companion.
    pub fn set_transparent_color(&mut self, color: Color) {
        self.normal.set_transparent_color(color);
    }

    /// C: `TFB_DrawImage_New_Rotated`.
    pub fn rotated(&self, angle: i32) -> Self {
        let size = rotated_extent(&self.normal, angle);
        let mut target = rotation_target(&self.normal, angle);
        rotate(&self.normal, &mut target, angle, size);
        Self::new(target)
    }

    fn scale_target(&mut self, kind: i32) {
        // bilinear and trilinear share their target
        let normalize = |kind| {
            if kind == TFB_SCALE_TRILINEAR {
                TFB_SCALE_BILINEAR
            } else {
                kind
            }
        };
        if self.scaled.is_some() && normalize(kind) != normalize(self.last_scale_type) {
            self.scaled = None;
        }
        if self.scaled.is_some() {
            return;
        }
        let src = &self.normal;
        self.scaled = Some(if kind == TFB_SCALE_NEAREST {
            let mut target = Canvas::new(src.format(), src.width(), src.height());
            target.copy_transparency(src);
            target
        } else {
            // the scaled image may be one pixel larger because of hot-spot
            // alignment and fractional edge pixels
            Canvas::new(PixelFormat::Rgba, src.width() + 1, src.height() + 1)
        });
    }

    /// C: `TFB_DrawImage_FixScaling`: regenerate the scaled canvas when the
    /// scale, scaler or pixels changed.
    fn fix_scaling(&mut self, mipmap: Option<&Canvas>, target: i32, kind: i32) {
        if !self.dirty
            && self.scaled.is_some()
            && target == self.last_scale
            && kind == self.last_scale_type
        {
            return;
        }
        self.dirty = false;
        self.scale_target(kind);
        let mut scaled = self
            .scaled
            .take()
            .unwrap_or_else(|| unreachable!("scale target was just created"));
        if kind == TFB_SCALE_NEAREST {
            rescale_nearest(
                &self.normal,
                &mut scaled,
                target,
                self.normal_hs,
                &mut self.extent,
                &mut self.last_scale_hs,
            );
        } else if kind == TFB_SCALE_BILINEAR {
            rescale_bilinear(
                &self.normal,
                &mut scaled,
                target,
                self.normal_hs,
                &mut self.extent,
                &mut self.last_scale_hs,
            );
        } else {
            let mipmap = mipmap.unwrap_or_else(|| panic!("trilinear scaling without a mipmap"));
            rescale_trilinear(
                (&self.normal, self.normal_hs),
                (mipmap, self.mipmap_hs),
                &mut scaled,
                target,
                &mut self.extent,
                &mut self.last_scale_hs,
            );
        }
        self.scaled = Some(scaled);
        self.last_scale_type = kind;
        self.last_scale = target;
    }

    /// The handle key of the trilinear mipmap, if one is set.
    pub fn mipmap_key(&self) -> Option<u64> {
        self.mipmap.map(Handle::key)
    }

    pub fn last_scale_hot_spot(&self) -> HotSpot {
        self.last_scale_hs
    }

    pub fn scaled_extent(&self) -> Extent {
        self.scaled.as_ref().map_or(self.extent, |c| {
            Extent::new(c.width() as i16, c.height() as i16)
        })
    }

    pub fn canvas_extent(&self) -> Extent {
        Extent::new(self.normal.width() as i16, self.normal.height() as i16)
    }

    fn take_colormap(&mut self, cmap: Option<&ColorMap>) {
        if let Some(cmap) = cmap {
            self.colormap_version = cmap.version;
        }
    }
}

/// Draw onto the normal canvas of image `target` (C: the `TFB_DrawImage_*`
/// functions). The target is out of the arena while `draw` runs, so `draw`
/// must not read it.
pub(crate) fn draw_into<R>(
    images: &mut Arena<Image>,
    target: ImageId,
    draw: impl FnOnce(&mut Arena<Image>, &mut Canvas) -> R,
) -> R {
    let mut canvas = std::mem::replace(
        &mut images.get_mut(target).normal,
        Canvas::new(PixelFormat::Rgbx, 0, 0),
    );
    let result = draw(images, &mut canvas);
    let image = images.get_mut(target);
    image.normal = canvas;
    image.dirty = true;
    result
}

/// Blit with a draw mode, as the canvas blit of sdl/canvas.c.
pub(crate) fn blit_mode(
    src: &Canvas,
    src_rect: Option<PixelRect>,
    dst: &mut Canvas,
    dst_rect: &mut PixelRect,
    mode: DrawMode,
) {
    let surface_alpha = mode.kind == DRAW_ALPHA && !src.format().has_alpha();
    if mode.kind == DRAW_REPLACE || surface_alpha {
        if surface_alpha {
            assert!(!src.blend(), "image already carries a surface alpha");
            canvas::blit_modulated(src, src_rect, dst, Some(dst_rect), mode.factor as u8);
        } else {
            canvas::blit(src, src_rect, dst, Some(dst_rect));
        }
        return;
    }
    let Some(plot) = Plot::for_canvas(dst, mode.kind) else {
        log::warn!(
            "ERROR: TFB_DrawCanvas_Blit unsupported draw mode ({})",
            mode.kind
        );
        return;
    };
    let src_rect = src_rect.unwrap_or(src.bounds());
    raster::blit(src, src_rect, plot, i32::from(mode.factor), dst, *dst_rect);
}

/// Where and at what scale an image draw lands.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Placement {
    pub x: i32,
    pub y: i32,
    pub scale: i32,
    pub scale_mode: i32,
}

impl Placement {
    /// Unscaled at `(x, y)`.
    pub(crate) fn at(x: i32, y: i32) -> Self {
        Self {
            x,
            y,
            scale: 0,
            scale_mode: 0,
        }
    }
}

/// C: `TFB_DrawCanvas_Image`.
pub(crate) fn draw(
    images: &mut Arena<Image>,
    id: ImageId,
    place: Placement,
    cmap: Option<Arc<ColorMap>>,
    mode: DrawMode,
    dst: &mut Canvas,
) {
    let (Placement { x, y, scale, .. }, mut scale_mode) = (place, place.scale_mode);
    let changed = |image: &Image| {
        cmap.as_ref()
            .is_some_and(|c| image.colormap_version != c.version)
    };
    {
        let image = images.get_mut(id);
        // only install the new palette if it changed
        if image.normal.is_indexed() && changed(image) {
            let palette = cmap
                .as_ref()
                .map(|c| c.palette)
                .unwrap_or_else(|| unreachable!());
            image.normal.set_palette(&palette);
        }
    }
    let scaled = scale != 0 && scale != GSCALE_IDENTITY;
    if scaled {
        let mipmap = images.get(id).mipmap;
        if scale_mode == TFB_SCALE_TRILINEAR {
            match mipmap {
                Some(mipmap) => {
                    if images.get(mipmap).normal.is_indexed() && changed(images.get(id)) {
                        let palette = cmap
                            .as_ref()
                            .map(|c| c.palette)
                            .unwrap_or_else(|| unreachable!());
                        images.get_mut(mipmap).normal.set_palette(&palette);
                    }
                }
                // bilinear when no mipmap is available
                None => scale_mode = TFB_SCALE_BILINEAR,
            }
        }
        match mipmap.filter(|_| scale_mode == TFB_SCALE_TRILINEAR) {
            Some(mipmap) if mipmap != id => {
                let (image, mipmap) = images.pair_mut(id, mipmap);
                image.fix_scaling(Some(&mipmap.normal), scale, scale_mode);
            }
            Some(_) => {
                let image = images.get_mut(id);
                let mipmap = image.normal.clone();
                image.fix_scaling(Some(&mipmap), scale, scale_mode);
            }
            None => images.get_mut(id).fix_scaling(None, scale, scale_mode),
        }
    }
    let image = images.get_mut(id);
    let (source, src_rect, hot) = if scaled {
        let palette = image.normal.palette().copied();
        let scaled = image.scaled.as_mut().unwrap_or_else(|| unreachable!());
        if scaled.is_indexed() {
            // only a paletted source scales to a paletted target
            scaled.set_palette(
                &palette.unwrap_or_else(|| panic!("paletted scale of a true-colour image")),
            );
        }
        let rect = PixelRect::new(
            0,
            0,
            i32::from(image.extent.width),
            i32::from(image.extent.height),
        );
        (&*scaled, Some(rect), image.last_scale_hs)
    } else {
        (&image.normal, None, image.normal_hs)
    };
    let mut target_rect = PixelRect::new(x - i32::from(hot.x), y - i32::from(hot.y), 0, 0);
    blit_mode(source, src_rect, dst, &mut target_rect, mode);
    image.take_colormap(cmap.as_deref());
}

/// C: `TFB_DrawCanvas_FilledImage`.
pub(crate) fn draw_filled(
    images: &mut Arena<Image>,
    id: ImageId,
    place: Placement,
    mut color: Color,
    mut mode: DrawMode,
    dst: &mut Canvas,
) {
    let (Placement { x, y, scale, .. }, mut scale_mode) = (place, place.scale_mode);
    check_primitive_mode(dst, &mut color, &mut mode);
    let image = images.get_mut(id);
    let mut force_fill = false;
    let scaled = scale != 0 && scale != GSCALE_IDENTITY;
    let (src_rect, hot) = if scaled {
        if scale_mode == TFB_SCALE_TRILINEAR {
            // no point in trilinear for filled images
            scale_mode = TFB_SCALE_BILINEAR;
        }
        if scale != image.last_scale || scale_mode != image.last_scale_type {
            force_fill = true;
        }
        image.fix_scaling(None, scale, scale_mode);
        (
            Some(PixelRect::new(
                0,
                0,
                i32::from(image.extent.width),
                i32::from(image.extent.height),
            )),
            image.last_scale_hs,
        )
    } else {
        if image.last_scale != 0 {
            // remember that the last fill was from an unscaled image
            force_fill = true;
            image.last_scale = 0;
        }
        (None, image.normal_hs)
    };
    let mut target_rect = PixelRect::new(x - i32::from(hot.x), y - i32::from(hot.y), 0, 0);
    let source_is_indexed = if scaled {
        image.scaled.as_ref().is_some_and(Canvas::is_indexed)
    } else {
        image.normal.is_indexed()
    };
    if source_is_indexed {
        // a palette of the fill colour; the image palette now differs from
        // its colour map
        let palette = [color; 256];
        let source = if scaled {
            image.scaled.as_mut().unwrap_or_else(|| unreachable!())
        } else {
            &mut image.normal
        };
        source.set_palette(&palette);
        image.colormap_version = image.colormap_version.wrapping_sub(1);
        let source = if scaled {
            image.scaled.as_ref().unwrap_or_else(|| unreachable!())
        } else {
            &image.normal
        };
        blit_mode(source, src_rect, dst, &mut target_rect, mode);
        return;
    }
    let source = if scaled {
        image.scaled.as_ref().unwrap_or_else(|| unreachable!())
    } else {
        &image.normal
    };
    if image
        .filled
        .as_ref()
        .is_some_and(|f| f.width() < source.width() || f.height() < source.height())
    {
        image.filled = None;
    }
    if image.filled.is_none() {
        image.filled = Some(Canvas::new(
            source.format(),
            source.width(),
            source.height(),
        ));
        force_fill = true;
    }
    let filled = image.filled.as_mut().unwrap_or_else(|| unreachable!());
    if force_fill || image.last_fill != color {
        // image or fill colour changed: regenerate
        if mode.kind == DRAW_ALPHA && filled.format().has_alpha() {
            // per-pixel alpha and surface alpha do not combine: modulate
            // the alpha channel instead
            color.a = mode.factor as u8;
            mode.kind = DRAW_REPLACE;
        } else {
            // do not modulate the alpha channel
            color.a = 0xff;
        }
        let fill_color = filled.map_rgba(color.r, color.g, color.b, color.a);
        fill(source, fill_color, filled);
        image.last_fill = color;
    }
    blit_mode(filled, src_rect, dst, &mut target_rect, mode);
}

/// Primitives may draw `DRAW_ALPHA` onto non-alpha canvases via `Color.a`.
pub(crate) fn check_primitive_mode(dst: &Canvas, color: &mut Color, mode: &mut DrawMode) {
    if mode.kind == DRAW_REPLACE && !dst.format().has_alpha() && color.a != 0xff {
        mode.kind = DRAW_ALPHA;
        mode.factor = i16::from(color.a);
        color.a = 0xff;
    }
}

/// Fill the opaque pixels of `src` with `fill_color` into `dst` (same format).
fn fill(src: &Canvas, mut fill_color: u32, dst: &mut Canvas) {
    if src.is_indexed() || dst.is_indexed() {
        log::warn!(
            "TFB_DrawCanvas_Fill: Unsupported surface formats: {} bytes/pixel source, {} bytes/pixel destination",
            src.format().bytes_per_pixel(),
            dst.format().bytes_per_pixel()
        );
        return;
    }
    let amask = src.format().alpha_mask();
    let alpha = fill_color & amask;
    // the alpha channel is processed separately
    fill_color &= !amask;
    let mut dst_key = 0;
    if dst_key == fill_color {
        // colour collision: the key becomes grey (1/2, 1/2, 1/2)
        dst_key = dst.map_rgba(127, 127, 127, 0);
    }
    let width = src.width() as usize;
    let dst_width = dst.width() as usize;
    if amask != 0 {
        for y in 0..src.height() as usize {
            for x in 0..width {
                let coverage = src.words()[y * width + x] & amask;
                dst.words_mut()[y * dst_width + x] = if coverage == 0 {
                    dst_key
                } else if alpha == 255 {
                    coverage | fill_color
                } else {
                    ((coverage * alpha) >> 8) | fill_color
                };
            }
        }
    } else if let Some(key) = src.color_key() {
        for y in 0..src.height() as usize {
            for x in 0..width {
                let pixel = src.words()[y * width + x];
                dst.words_mut()[y * dst_width + x] =
                    if pixel == key { dst_key } else { fill_color };
            }
        }
    } else {
        log::warn!("TFB_DrawCanvas_Fill: Unsupported sourcesurface format");
    }
    // a dynamic image keeps its key without RLE; an RGBA image still blends
    dst.set_color_key(Some(dst_key));
}

/// C: `TFB_DrawCanvas_FontChar`.
pub(crate) fn draw_font_char(
    images: &mut Arena<Image>,
    glyph: &Glyph,
    backing: ImageId,
    x: i32,
    y: i32,
    mut mode: DrawMode,
    dst: &mut Canvas,
) {
    let (w, h) = (
        i32::from(glyph.extent.width),
        i32::from(glyph.extent.height),
    );
    let surface = &mut images.get_mut(backing).normal;
    if !surface.format().has_alpha() || surface.width() < w || surface.height() < h {
        log::warn!(
            "ERROR: TFB_DrawCanvas_FontChar bad backing surface: {}x{}x{}; char: {}x{}",
            surface.width(),
            surface.height(),
            surface.format().bytes_per_pixel(),
            w,
            h
        );
        return;
    }
    let stride = surface.width() as usize;
    let words = surface.words_mut();
    if mode.kind == DRAW_ALPHA {
        // per-pixel alpha and surface alpha do not combine: modulate the
        // backing alpha ourselves (>> 8, so 255 is not exact)
        let alpha = u32::from(mode.factor as u16);
        mode.kind = DRAW_REPLACE;
        for row in 0..h as usize {
            for col in 0..w as usize {
                let pixel = &mut words[row * stride + col];
                let mut a = u32::from(glyph.data[row * glyph.pitch + col]);
                if alpha != 0xff {
                    a = a.wrapping_mul(alpha) >> 8;
                }
                *pixel = *pixel & 0xffff_ff00 | (a & 0xff);
            }
        }
    } else {
        for row in 0..h as usize {
            for col in 0..w as usize {
                let pixel = &mut words[row * stride + col];
                *pixel = *pixel & 0xffff_ff00 | u32::from(glyph.data[row * glyph.pitch + col]);
            }
        }
    }
    let mut target_rect = PixelRect::new(
        x - i32::from(glyph.hot_spot.x),
        y - i32::from(glyph.hot_spot.y),
        0,
        0,
    );
    let src_rect = PixelRect::new(0, 0, w, h);
    blit_mode(
        &images.get(backing).normal,
        Some(src_rect),
        dst,
        &mut target_rect,
        mode,
    );
}

/// C: `TFB_DrawImage_SetMipmap`.
pub(crate) fn set_mipmap(images: &mut Arena<Image>, id: ImageId, mipmap: ImageId, hot: HotSpot) {
    let (image_indexed, image_cmap) = {
        let image = images.get(id);
        (image.normal.is_indexed(), image.colormap_index)
    };
    let mipmap_image = images.get(mipmap);
    let accepted = !mipmap_image.normal.is_indexed()
        || image_indexed && image_cmap == mipmap_image.colormap_index;
    let image = images.get_mut(id);
    if accepted {
        image.mipmap = Some(mipmap);
        image.mipmap_hs = hot;
    } else {
        image.mipmap = None;
    }
}

/// C: `TFB_DrawImage_Intersect`.
pub(crate) fn intersect(
    images: &Arena<Image>,
    a: ImageId,
    ao: Point,
    b: ImageId,
    bo: Point,
    area: Rect,
) -> bool {
    images
        .get(a)
        .normal
        .intersects(ao, &images.get(b).normal, bo, area)
}

/// `SDL_UpperBlit` of `area` from one canvas to another at `to`.
pub(crate) fn copy_rect(source: &Canvas, area: Rect, target: &mut Canvas, to: Point) {
    let source_rect = PixelRect::new(
        i32::from(area.corner.x),
        i32::from(area.corner.y),
        i32::from(area.extent.width),
        i32::from(area.extent.height),
    );
    let mut target_rect = PixelRect::new(
        i32::from(to.x),
        i32::from(to.y),
        i32::from(area.extent.width),
        i32::from(area.extent.height),
    );
    canvas::blit(source, Some(source_rect), target, Some(&mut target_rect));
}

/// C: `COPYTOIMAGE`: copy a screen area into an image's normal canvas. As in
/// C, the image's scaled cache is not invalidated.
pub(crate) fn copy_screen_into(
    images: &mut Arena<Image>,
    id: ImageId,
    screen: &Canvas,
    area: Rect,
) {
    copy_rect(
        screen,
        area,
        &mut images.get_mut(id).normal,
        Point::default(),
    );
}

/// C: `TFB_DrawImage_CopyRect` between two images.
pub(crate) fn copy_image_rect(
    images: &mut Arena<Image>,
    source: ImageId,
    area: Rect,
    target: ImageId,
    to: Point,
) {
    let (source, target) = images.pair_mut(source, target);
    copy_rect(&source.normal, area, &mut target.normal, to);
    target.dirty = true;
}

/// C: `TFB_DrawCanvas_Line` with the primitive alpha rule.
pub(crate) fn draw_line(dst: &mut Canvas, points: [i32; 4], mut color: Color, mut mode: DrawMode) {
    check_primitive_mode(dst, &mut color, &mut mode);
    let pixel = dst.map_rgba(color.r, color.g, color.b, color.a);
    let Some(plot) = Plot::for_canvas(dst, mode.kind) else {
        log::warn!(
            "ERROR: TFB_DrawCanvas_Line unsupported draw mode ({})",
            mode.kind
        );
        return;
    };
    raster::line(dst, points, pixel, plot, i32::from(mode.factor));
}

/// C: `TFB_DrawCanvas_Rect`: a filled rectangle.
pub(crate) fn draw_rect(dst: &mut Canvas, rect: Rect, mut color: Color, mut mode: DrawMode) {
    let area = PixelRect::new(
        i32::from(rect.corner.x),
        i32::from(rect.corner.y),
        i32::from(rect.extent.width),
        i32::from(rect.extent.height),
    );
    check_primitive_mode(dst, &mut color, &mut mode);
    let mut pixel = dst.map_rgba(color.r, color.g, color.b, color.a);
    if mode.kind == DRAW_REPLACE {
        let amask = dst.format().alpha_mask();
        if let Some(key) = dst.color_key().filter(|_| amask != 0) {
            // an alpha canvas with a key: key-coloured rectangles are transparent
            if pixel & !amask == key & !amask {
                pixel &= !amask;
            }
        }
        dst.fill_rect(Some(area), pixel);
        return;
    }
    let Some(plot) = Plot::for_canvas(dst, mode.kind) else {
        log::warn!(
            "ERROR: TFB_DrawCanvas_Rect unsupported draw mode ({})",
            mode.kind
        );
        return;
    };
    raster::fill_rect(dst, area, pixel, plot, i32::from(mode.factor));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unscaled_images_are_drawn_relative_to_their_hot_spot() {
        let mut images = Arena::new();
        let mut canvas = Canvas::new(PixelFormat::Rgbx, 2, 2);
        canvas.set_pixel(0, 0, 0x0102_0300);
        let mut image = Image::new(canvas);
        image.normal_hs = HotSpot::new(1, 1);
        let id = images.insert(image);
        let mut screen = Canvas::new(PixelFormat::Rgbx, 4, 4);
        draw(
            &mut images,
            id,
            Placement::at(2, 2),
            None,
            DrawMode::REPLACE,
            &mut screen,
        );
        assert_eq!(screen.pixel(1, 1), 0x0102_0300);
        assert_eq!(screen.pixel(2, 2), 0);
    }

    #[test]
    fn scaled_images_reuse_their_cache_until_marked_dirty() {
        use super::super::canvas::scale::TFB_SCALE_NEAREST;
        let mut source = Canvas::new(PixelFormat::Rgbx, 4, 4);
        source.set_pixel(0, 0, 0x0c22_3800);
        let mut image = Image::new(source);
        image.fix_scaling(None, 128, TFB_SCALE_NEAREST);
        assert_eq!(image.extent, Extent::new(2, 2));
        assert_eq!(
            (image.last_scale, image.last_scale_type, image.dirty),
            (128, TFB_SCALE_NEAREST, false)
        );
        let pixel = |image: &Image| image.scaled.as_ref().map(|c| c.pixel(0, 0));
        assert_eq!(pixel(&image), Some(0x0c22_3800));
        image.normal.set_pixel(0, 0, 0x4e5a_0c00);
        image.fix_scaling(None, 128, TFB_SCALE_NEAREST);
        assert_eq!(pixel(&image), Some(0x0c22_3800));
        image.dirty = true;
        image.fix_scaling(None, 128, TFB_SCALE_NEAREST);
        assert!(!image.dirty);
        assert_eq!(pixel(&image), Some(0x4e5a_0c00));
    }

    #[test]
    fn rotation_keeps_pixels_at_zero_and_carries_the_transparent_colour() {
        let mut canvas = Canvas::new(PixelFormat::Rgbx, 2, 2);
        canvas
            .words_mut()
            .copy_from_slice(&[1 << 8, 2 << 8, 3 << 8, 4 << 8]);
        canvas.set_transparent_color(Color::rgb(1, 2, 3));
        let image = Image::new(canvas);
        let rotated = image.rotated(0);
        assert_eq!(rotated.canvas().words(), image.canvas().words());
        let key = rotated
            .canvas()
            .transparent_color()
            .unwrap_or_else(|| panic!("rotation dropped the key"));
        assert_eq!((key.r, key.g, key.b), (1, 2, 3));
    }

    #[test]
    fn transparency_copies_follow_the_source_format() {
        let mut true_colour = Canvas::new(PixelFormat::Rgbx, 1, 1);
        true_colour.set_transparent_color(Color::rgb(11, 22, 33));
        let mut copy = Canvas::new(PixelFormat::Rgbx, 1, 1);
        copy.copy_transparency(&true_colour);
        assert!(copy.palette().is_none());
        let key = copy.transparent_color().unwrap_or_else(|| panic!("key"));
        assert_eq!((key.r, key.g, key.b, key.a), (11, 22, 33, 255));

        let mut indexed = Canvas::new(PixelFormat::Indexed, 1, 1);
        let mut palette = [Color::default(); 256];
        palette[1] = Color::rgb(12, 34, 56);
        indexed.set_palette(&palette);
        assert_eq!(
            indexed.palette().map(|p| p[1]),
            Some(Color::rgb(12, 34, 56))
        );
        indexed.set_transparent_index(7);
        // a keyed indexed canvas reports -1, so its copy carries no key
        assert_eq!(indexed.transparent_index(), -1);
        let mut copy = Canvas::new(PixelFormat::Indexed, 1, 1);
        copy.copy_transparency(&indexed);
        assert_eq!(copy.color_key(), None);
    }

    #[test]
    fn filled_images_reuse_their_fill_until_the_colour_changes() {
        let mut images = Arena::new();
        let mut canvas = Canvas::new(PixelFormat::Rgba, 2, 1);
        canvas
            .words_mut()
            .copy_from_slice(&[0x0000_0000, 0x0102_03ff]);
        let id = images.insert(Image::new(canvas));
        let mut screen = Canvas::new(PixelFormat::Rgbx, 2, 1);
        draw_filled(
            &mut images,
            id,
            Placement::at(0, 0),
            Color::rgb(9, 8, 7),
            DrawMode::REPLACE,
            &mut screen,
        );
        assert_eq!(screen.words(), &[0, 0x0908_0700]);
        assert_eq!(images.get(id).last_fill, Color::rgb(9, 8, 7));
    }

    #[test]
    fn colour_maps_install_once_per_version() {
        let mut images = Arena::new();
        let mut canvas = Canvas::new(PixelFormat::Indexed, 1, 1);
        canvas.indexes_mut()[0] = 4;
        let id = images.insert(Image::new(canvas));
        let mut palette = [Color::default(); 256];
        palette[4] = Color::rgb(1, 2, 3);
        let map = Arc::new(ColorMap {
            index: 0,
            version: 7,
            palette,
        });
        let mut screen = Canvas::new(PixelFormat::Rgbx, 1, 1);
        draw(
            &mut images,
            id,
            Placement::at(0, 0),
            Some(map),
            DrawMode::REPLACE,
            &mut screen,
        );
        assert_eq!(screen.words(), &[0x0102_0300]);
        assert_eq!(images.get(id).colormap_version, 7);
    }

    #[test]
    fn glyphs_write_backing_alpha_before_blitting() {
        let mut images = Arena::new();
        let mut backing = Canvas::new(PixelFormat::Rgba, 2, 1);
        backing.words_mut().fill(0xff00_00ff);
        let backing = images.insert(Image::new(backing));
        let glyph = Glyph {
            extent: Extent::new(2, 1),
            disp: Extent::new(3, 2),
            hot_spot: HotSpot::new(0, 0),
            data: Arc::from(vec![255, 0]),
            pitch: 2,
        };
        let mut screen = Canvas::new(PixelFormat::Rgbx, 2, 1);
        draw_font_char(
            &mut images,
            &glyph,
            backing,
            0,
            0,
            DrawMode::REPLACE,
            &mut screen,
        );
        assert_eq!(screen.words(), &[0xff00_0000, 0]);
    }
}
