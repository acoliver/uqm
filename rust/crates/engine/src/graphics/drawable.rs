//! Drawables and their frames, ported from drawable.c and pixmap.c
//! (GPL-2.0-or-later).
//!
//! A drawable is a numbered list of frames; each frame has a hot spot, a size
//! and, unless it is the screen, an image. Game state names frames with the
//! copyable [`FrameId`]. A drawable lives while its owner ([`LoadedDrawable`])
//! or any lease ([`DrawableLease`], [`ContextFrame`], a context's frame or
//! font effect) holds it; its images are released through the draw queue
//! when the last holder lets go, after every draw queued before.
use super::canvas::scale::rescale_nearest;
use super::context::ContextLease;
use super::dcq::DrawCommand;
use super::geometry::{Color, Extent, HotSpot, Point, Rect};
use super::image::{Image, ImageId};
use super::state::{self, Graphics};
use crate::collections::observation::{self, Event};
use std::num::NonZeroU64;

/// C: `ROM_DRAWABLE`, `RAM_DRAWABLE` and `SCREEN_DRAWABLE`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FrameKind {
    #[default]
    Rom,
    Ram,
    Screen,
}

/// C: `FRAME_DESC`.
#[derive(Clone, Debug, Default)]
pub struct Frame {
    pub kind: FrameKind,
    pub hot_spot: HotSpot,
    pub bounds: Extent,
    pub image: Option<ImageId>,
}

/// C: `DRAWABLE_DESC`.
#[derive(Debug)]
pub struct Drawable {
    pub flags: u8,
    pub frames: Vec<Frame>,
    leases: u32,
    owned: bool,
}

/// `CREATE_FLAGS`: the drawable's frames carry an alpha channel.
pub const WANT_ALPHA: u8 = 8;
/// `CREATE_FLAGS`: a pixel image.
pub const WANT_PIXMAP: u8 = 2;

/// A copyable name for a drawable (C: `DRAWABLE`). Names are never reused.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct DrawableId(NonZeroU64);

impl DrawableId {
    /// A stable number for observation journals, distinct from every
    /// [`FrameId::bits`].
    pub fn bits(self) -> usize {
        (self.0.get() as usize) << 16 | 0xffff
    }
}

/// A copyable name for one frame of a drawable (C: `FRAME`); the default is
/// the null frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FrameId(Option<(DrawableId, u16)>);

impl FrameId {
    pub const NULL: Self = Self(None);

    pub(crate) fn new(drawable: DrawableId, index: u16) -> Self {
        Self(Some((drawable, index)))
    }

    /// `None` for the null frame.
    pub fn non_null(self) -> Option<Self> {
        (!self.is_null()).then_some(self)
    }

    pub fn is_null(self) -> bool {
        self.0.is_none()
    }

    /// C: `GetFrameParentDrawable`.
    pub fn drawable(self) -> Option<DrawableId> {
        self.0.map(|(drawable, _)| drawable)
    }

    /// C: `GetFrameIndex`: zero for the null frame.
    pub fn index(self) -> u16 {
        self.0.map_or(0, |(_, index)| index)
    }

    /// A stable number for observation journals and opaque game handles;
    /// zero for the null frame.
    pub fn bits(self) -> usize {
        self.0.map_or(0, |(drawable, index)| {
            (drawable.0.get() as usize) << 16 | usize::from(index)
        })
    }

    /// The frame named by [`Self::bits`].
    pub fn from_bits(bits: usize) -> Self {
        NonZeroU64::new((bits >> 16) as u64).map_or(Self::NULL, |drawable| {
            Self::new(DrawableId(drawable), bits as u16)
        })
    }

    /// C: `GetFrameCount`.
    pub fn count(self) -> u16 {
        self.drawable().map_or(0, |drawable| {
            state::with(|g| g.drawable(drawable).frames.len() as u16)
        })
    }

    /// C: `SetAbsFrameIndex`: the frame `index` modulo the frame count.
    pub fn at(self, index: u16) -> Self {
        let Some((drawable, _)) = self.0 else {
            return self;
        };
        Self::new(drawable, index % self.count())
    }

    /// C: `SetRelFrameIndex`.
    pub fn relative(self, mut offset: i16) -> Self {
        let Some((drawable, index)) = self.0 else {
            return self;
        };
        let count = self.count();
        if offset < 0 {
            loop {
                offset = (i32::from(offset) + i32::from(count)) as i16;
                if offset >= 0 {
                    break;
                }
            }
        }
        let index = (i32::from(index as i16) + i32::from(offset)) % i32::from(count);
        Self::new(drawable, index as u16)
    }

    /// C: `SetEquFrameIndex`.
    pub fn equal_index(self, source: Self) -> Self {
        if self.is_null() || source.is_null() {
            Self::NULL
        } else {
            self.at(source.index())
        }
    }

    /// C: `IncFrameIndex`.
    pub fn next(self) -> Self {
        let Some((drawable, index)) = self.0 else {
            return self;
        };
        if index + 1 < self.count() {
            Self::new(drawable, index + 1)
        } else {
            Self::new(drawable, 0)
        }
    }

    /// C: `DecFrameIndex`.
    pub fn previous(self) -> Self {
        let Some((drawable, index)) = self.0 else {
            return self;
        };
        if index > 0 {
            Self::new(drawable, index - 1)
        } else {
            Self::new(drawable, self.count() - 1)
        }
    }

    /// C: `GetFrameRect`: the frame's bounds placed at minus its hot spot.
    pub fn rect(self) -> Option<Rect> {
        self.0.map(|_| state::with(|g| g.frame(self).rect()))
    }

    /// C: `GetFrameHot`.
    pub fn hot_spot(self) -> HotSpot {
        if self.is_null() {
            return HotSpot::default();
        }
        state::with(|g| g.frame(self).hot_spot)
    }

    /// C: `SetFrameHot`: returns the previous hot spot.
    pub fn set_hot_spot(self, hot: HotSpot) -> HotSpot {
        if self.is_null() {
            return HotSpot::default();
        }
        state::with(|g| std::mem::replace(&mut g.frame_mut(self).hot_spot, hot))
    }

    /// Replace the frame's bounds (a fixture's collision box); returns the
    /// previous bounds.
    pub fn set_bounds(self, bounds: Extent) -> Extent {
        state::with(|g| std::mem::replace(&mut g.frame_mut(self).bounds, bounds))
    }

    /// The image identity of a pixel frame and the identity of its mipmap
    /// companion, for zoom-level diagnostics.
    pub fn image_identity(self) -> Option<(u64, Option<u64>)> {
        let (_, index) = self.0?;
        let _ = index;
        state::with(|g| {
            let image = g.frame(self).image?;
            Some((image.key(), g.images.get(image).mipmap_key()))
        })
    }

    /// C: `TFB_DrawScreen_SetMipmap` with the mipmap frame's hot spot: queue
    /// `mipmap` as this frame's trilinear companion.
    pub fn set_mipmap(self, mipmap: Self) {
        state::with(|g| {
            let image = g.frame(self).pixel_image();
            let source = g.frame(mipmap);
            let hot = source.hot_spot;
            let mipmap = source.pixel_image();
            g.enqueue(DrawCommand::SetMipmap { image, mipmap, hot });
        });
    }

    /// The frame's kind; `None` for the null frame.
    pub fn kind(self) -> Option<FrameKind> {
        self.0.map(|_| state::with(|g| g.frame(self).kind))
    }
}

impl Frame {
    fn rect(&self) -> Rect {
        Rect {
            corner: Point::new(
                self.hot_spot.x.wrapping_neg(),
                self.hot_spot.y.wrapping_neg(),
            ),
            extent: self.bounds,
        }
    }

    fn pixel_image(&self) -> ImageId {
        assert_ne!(
            self.kind,
            FrameKind::Screen,
            "screen frames require readback"
        );
        self.image
            .unwrap_or_else(|| panic!("frame has no pixel image"))
    }
}

/// The drawable registry: serial names, never reused.
#[derive(Debug, Default)]
pub(crate) struct Drawables {
    pub(crate) entries: std::collections::HashMap<NonZeroU64, Drawable>,
    next: u64,
}

impl Graphics {
    pub(crate) fn drawable(&self, drawable: DrawableId) -> &Drawable {
        self.drawables
            .entries
            .get(&drawable.0)
            .unwrap_or_else(|| panic!("drawable {drawable:?} was released"))
    }

    fn drawable_mut(&mut self, drawable: DrawableId) -> &mut Drawable {
        self.drawables
            .entries
            .get_mut(&drawable.0)
            .unwrap_or_else(|| panic!("drawable {drawable:?} was released"))
    }

    pub(crate) fn frame(&self, frame: FrameId) -> &Frame {
        let (drawable, index) = frame.0.unwrap_or_else(|| panic!("null frame"));
        &self.drawable(drawable).frames[usize::from(index)]
    }

    pub(crate) fn frame_mut(&mut self, frame: FrameId) -> &mut Frame {
        let (drawable, index) = frame.0.unwrap_or_else(|| panic!("null frame"));
        &mut self.drawable_mut(drawable).frames[usize::from(index)]
    }

    /// C: `AllocDrawable`: `count` empty frames, owned by the caller.
    pub(crate) fn alloc_drawable(&mut self, count: u16, flags: u8) -> DrawableId {
        self.drawables.next += 1;
        let id = NonZeroU64::new(self.drawables.next).unwrap_or_else(|| unreachable!());
        let drawable = DrawableId(id);
        observation::record(Event::Drawable(drawable.bits()));
        for index in 0..count {
            observation::record(Event::Frame {
                address: FrameId::new(drawable, index).bits(),
                owner: drawable.bits(),
                index,
            });
        }
        self.drawables.entries.insert(
            id,
            Drawable {
                flags,
                frames: vec![Frame::default(); usize::from(count)],
                leases: 1,
                owned: true,
            },
        );
        drawable
    }

    /// C: `request_drawable`.
    fn request_drawable(
        &mut self,
        count: u16,
        kind: FrameKind,
        flags: u8,
        width: i16,
        height: i16,
    ) -> DrawableId {
        let drawable = self.alloc_drawable(count, flags);
        for index in 0..usize::from(count) {
            let image = (kind == FrameKind::Ram && width > 0 && height > 0).then(|| {
                self.images.insert(Image::true_color(
                    i32::from(width),
                    i32::from(height),
                    flags & WANT_ALPHA != 0,
                ))
            });
            self.drawable_mut(drawable).frames[index] = Frame {
                kind,
                hot_spot: HotSpot::default(),
                bounds: Extent::new(width, height),
                image,
            };
        }
        drawable
    }

    /// C: `CreateDrawable`.
    pub(crate) fn create_drawable(
        &mut self,
        flags: u8,
        width: i16,
        height: i16,
        count: u16,
    ) -> DrawableId {
        self.request_drawable(count, FrameKind::Ram, flags & 15, width, height)
    }

    /// C: `CreateDisplay`: the screen as a one-frame drawable.
    pub(crate) fn create_display(&mut self, flags: u8) -> DrawableId {
        self.request_drawable(
            1,
            FrameKind::Screen,
            flags & 3,
            state::SCREEN_WIDTH as i16,
            state::SCREEN_HEIGHT as i16,
        )
    }

    /// Hold a drawable for a lease.
    pub(crate) fn retain_drawable(&mut self, drawable: DrawableId) {
        let entry = self
            .drawables
            .entries
            .get_mut(&drawable.0)
            .unwrap_or_else(|| panic!("frame belongs to an unowned drawable: {drawable:?}"));
        entry.leases += 1;
    }

    /// Let go of a lease; the last one releases the drawable's images
    /// through the returned queue commands (C: `release_cel_data`).
    #[must_use]
    pub(crate) fn release_drawable(&mut self, drawable: DrawableId) -> Vec<DrawCommand> {
        let entry = self.drawable_mut(drawable);
        entry.leases -= 1;
        if entry.leases != 0 {
            return Vec::new();
        }
        let entry = self
            .drawables
            .entries
            .remove(&drawable.0)
            .unwrap_or_else(|| unreachable!());
        entry
            .frames
            .into_iter()
            .filter_map(|frame| frame.image)
            .map(DrawCommand::DeleteImage)
            .collect()
    }

    pub(crate) fn retain_frame(&mut self, frame: FrameId) {
        if let Some(drawable) = frame.drawable() {
            self.retain_drawable(drawable);
        }
    }

    #[must_use]
    pub(crate) fn release_frame(&mut self, frame: FrameId) -> Vec<DrawCommand> {
        frame
            .drawable()
            .map_or_else(Vec::new, |drawable| self.release_drawable(drawable))
    }

    /// C: `DestroyDrawable`: the owner's release. Leases keep the frames.
    #[must_use]
    pub(crate) fn destroy_drawable(&mut self, drawable: DrawableId) -> Vec<DrawCommand> {
        if self.current.frame.drawable() == Some(drawable) {
            self.set_context_fg_frame(FrameId::NULL);
        }
        observation::record(Event::DrawableFree(drawable.bits()));
        let entry = self.drawable_mut(drawable);
        assert!(entry.owned, "drawable {drawable:?} destroyed twice");
        entry.owned = false;
        self.release_drawable(drawable)
    }

    pub(crate) fn frame_image(&self, frame: FrameId) -> &Image {
        self.images.get(self.frame(frame).pixel_image())
    }

    pub(crate) fn frame_image_mut(&mut self, frame: FrameId) -> &mut Image {
        let id = self.frame(frame).pixel_image();
        self.images.get_mut(id)
    }

    /// C: `RotateFrame`.
    pub(crate) fn rotate_frame(&mut self, frame: FrameId, angle: i32) -> DrawableId {
        let source = self.frame(frame).clone();
        let image = self.images.get(source.pixel_image()).rotated(angle);
        let bounds = image.extent;
        let drawable = self.request_drawable(1, FrameKind::Ram, WANT_PIXMAP, 0, 0);
        let image = self.images.insert(image);
        let rotated = &mut self.drawable_mut(drawable).frames[0];
        rotated.bounds = bounds;
        rotated.hot_spot = rotate_hotspot(source.hot_spot, source.bounds, bounds, angle);
        rotated.image = Some(image);
        drawable
    }

    /// C: `matching_frame`: a one-frame RAM drawable with the source's flags.
    fn matching_drawable(&mut self, frame: FrameId, width: i32, height: i32) -> DrawableId {
        let flags = frame
            .drawable()
            .map_or(0, |drawable| self.drawable(drawable).flags);
        self.create_drawable(flags, width as i16, height as i16, 1)
    }

    /// C: `CopyFrameRect`.
    pub(crate) fn copy_frame_rect(&mut self, frame: FrameId, area: Rect) -> DrawableId {
        let source = self.frame(frame).pixel_image();
        let drawable = self.matching_drawable(
            frame,
            i32::from(area.extent.width),
            i32::from(area.extent.height),
        );
        let target = self.frame(FrameId::new(drawable, 0)).pixel_image();
        super::image::copy_image_rect(&mut self.images, source, area, target, Point::default());
        drawable
    }

    /// C: `RescaleFrame`.
    pub(crate) fn rescale_frame(&mut self, frame: FrameId, width: i32, height: i32) -> DrawableId {
        let source = self.frame(frame).clone();
        let drawable = self.matching_drawable(frame, width, height);
        let target = FrameId::new(drawable, 0);
        self.frame_mut(target).hot_spot = HotSpot::new(
            (i32::from(source.hot_spot.x) * width / i32::from(source.bounds.width)) as i16,
            (i32::from(source.hot_spot.y) * height / i32::from(source.bounds.height)) as i16,
        );
        let target = self.frame(target).pixel_image();
        let (from, to) = self.images.pair_mut(source.pixel_image(), target);
        let mut extent = Extent::zero();
        let mut hot = HotSpot::default();
        rescale_nearest(
            from.canvas(),
            to.canvas_mut(),
            -1,
            HotSpot::default(),
            &mut extent,
            &mut hot,
        );
        drawable
    }
}

/// C: `RotateFrame`'s hot-spot rotation about the integer centre.
fn rotate_hotspot(hot: HotSpot, bounds: Extent, rotated: Extent, angle: i32) -> HotSpot {
    let radians = f64::from(angle) * std::f64::consts::PI / 180.0;
    let mut dx = f64::from(i32::from(hot.x) - i32::from(bounds.width) / 2);
    let mut dy = f64::from(i32::from(hot.y) - i32::from(bounds.height) / 2);
    let distance = (dx * dx + dy * dy).sqrt();
    if distance as i32 != 0 {
        let original = (-dy).atan2(dx);
        dx = (original + radians).cos() * distance;
        dy = -(original + radians).sin() * distance;
    }
    HotSpot::new(
        (i32::from(rotated.width) / 2 + dx as i32) as i16,
        (i32::from(rotated.height) / 2 + dy as i32) as i16,
    )
}

/// A held drawable: its frames stay valid while any lease is alive.
#[derive(Debug, PartialEq, Eq)]
pub struct DrawableLease(DrawableId);

impl DrawableLease {
    /// Hold `drawable`; panics if it was released.
    pub fn retain(drawable: DrawableId) -> Self {
        state::with(|g| g.retain_drawable(drawable));
        Self(drawable)
    }

    /// Adopt the reference a new drawable starts with.
    fn adopt(drawable: DrawableId) -> Self {
        Self(drawable)
    }

    pub fn drawable(&self) -> DrawableId {
        self.0
    }

    pub fn frame_count(&self) -> u16 {
        state::with(|g| g.drawable(self.0).frames.len() as u16)
    }

    /// The frame `index`, if the drawable has it.
    pub fn frame(&self, index: usize) -> Option<FrameId> {
        (index < usize::from(self.frame_count())).then(|| FrameId::new(self.0, index as u16))
    }

    /// The frame `index` modulo the frame count.
    pub fn wrapped_frame(&self, index: usize) -> FrameId {
        FrameId::new(self.0, (index % usize::from(self.frame_count())) as u16)
    }

    pub fn context_frame(&self, index: usize) -> ContextFrame {
        let frame = self
            .frame(index)
            .unwrap_or_else(|| panic!("drawable frame index {index}"));
        ContextFrame::retain(frame)
    }

    pub fn frame_rect(&self, index: usize) -> Rect {
        self.context_frame(index).rect()
    }

    pub fn set_hotspot(&self, index: usize, hot: HotSpot) {
        self.context_frame(index).replace_hotspot(hot);
    }

    pub fn owns_frame(&self, frame: FrameId) -> bool {
        self.frame_index(frame).is_some()
    }

    pub fn frame_index(&self, frame: FrameId) -> Option<usize> {
        (frame.drawable() == Some(self.0) && frame.index() < self.frame_count())
            .then(|| usize::from(frame.index()))
    }
}

impl Clone for DrawableLease {
    fn clone(&self) -> Self {
        Self::retain(self.0)
    }
}

impl Drop for DrawableLease {
    fn drop(&mut self) {
        let released = state::with(|g| g.release_drawable(self.0));
        state::enqueue_all(released);
    }
}

/// Unique release authority for a loaded or generated drawable.
#[derive(Debug)]
pub struct LoadedDrawable {
    lease: Option<DrawableLease>,
}

impl LoadedDrawable {
    /// Adopt a drawable just created by the caller.
    pub(crate) fn adopt(drawable: DrawableId) -> Self {
        Self {
            lease: Some(DrawableLease::adopt(drawable)),
        }
    }

    pub fn drawable(&self) -> DrawableId {
        self.lease_ref().0
    }

    /// Whether no lease other than the owner's holds the drawable.
    pub fn is_exclusive(&self) -> bool {
        state::with(|g| g.drawable(self.drawable()).leases == 1)
    }

    fn lease_ref(&self) -> &DrawableLease {
        self.lease
            .as_ref()
            .unwrap_or_else(|| unreachable!("owner lease is held until drop"))
    }

    pub fn lease(&self) -> DrawableLease {
        self.lease_ref().clone()
    }

    /// The first frame (C: `CaptureDrawable`).
    pub fn first_frame(&self) -> FrameId {
        FrameId::new(self.lease_ref().0, 0)
    }

    /// Hand ownership to game state that names the drawable by its first
    /// frame (C: a `FRAME` field); [`Self::from_frame`] takes it back.
    pub fn into_frame(self) -> FrameId {
        // the owner's release moves to whoever calls `from_frame`
        std::mem::ManuallyDrop::new(self).first_frame()
    }

    /// Take back ownership handed out by [`Self::into_frame`]; the null
    /// frame owns nothing.
    pub fn from_frame(frame: FrameId) -> Option<Self> {
        frame.drawable().map(Self::adopt)
    }

    /// Load a graphics resource.
    pub fn load(
        content: &crate::resource::Content,
        resource: &std::ffi::CStr,
    ) -> Result<Self, crate::resource::AssetLoadError> {
        let resolved = content.require(resource, crate::resource::AssetKind::Graphic)?;
        let path = std::ffi::CString::new(resolved.descriptor)
            .map_err(|error| resolved.load_error(resource, error))?;
        Self::load_file(resolved.files, &path).map_err(|error| resolved.load_error(resource, error))
    }

    /// Load an animation or image file.
    pub fn load_file(
        files: &crate::io::Dir,
        path: &std::ffi::CStr,
    ) -> Result<Self, crate::resource::FileError> {
        super::load::load_graphic_file(files, path)
    }

    /// C: `CreateDrawable`.
    pub fn create(flags: u8, width: i16, height: i16, count: u16) -> Option<Self> {
        Some(Self::adopt(state::with(|g| {
            g.create_drawable(flags, width, height, count)
        })))
    }

    /// C: `CreateDisplay`.
    pub fn display(flags: u8) -> Self {
        Self::adopt(state::with(|g| g.create_display(flags)))
    }

    /// C: `RotateFrame`.
    pub fn rotated(frame: &ContextFrame, angle: i32) -> Option<Self> {
        let source = frame.frame();
        if source.is_null() {
            return None;
        }
        assert_ne!(source.kind(), Some(FrameKind::Screen));
        Some(Self::adopt(state::with(|g| g.rotate_frame(source, angle))))
    }

    /// Rescale one pixel frame into a separate owner, preserving the source.
    /// Empty frames, invalid indices and dimensions outside native bounds return `None`.
    pub fn rescaled(&self, index: usize, width: i32, height: i32) -> Option<Self> {
        let (width, height) = (i16::try_from(width).ok()?, i16::try_from(height).ok()?);
        let frame = self.lease_ref().frame(index)?;
        let bounds = frame.rect()?.extent;
        if width <= 0 || height <= 0 || bounds.width <= 0 || bounds.height <= 0 {
            return None;
        }
        assert_ne!(frame.kind(), Some(FrameKind::Screen));
        Some(Self::adopt(state::with(|g| {
            g.rescale_frame(frame, i32::from(width), i32::from(height))
        })))
    }

    /// Copy an area relative to the context's clip into an independently owned drawable.
    /// An absent area copies the current clip; an empty intersection returns `None`.
    pub fn capture(context: &ContextLease, area: Option<Rect>) -> Option<Self> {
        context.copy_rect(area).map(Self::adopt)
    }

    /// C: `CloneFrame`.
    pub fn clone_frame(frame: &ContextFrame) -> Option<Self> {
        let source = frame.frame();
        if source.is_null() {
            return None;
        }
        assert_ne!(source.kind(), Some(FrameKind::Screen));
        let hot = source.hot_spot();
        let bounds = source.rect()?.extent;
        let drawable = state::with(|g| {
            let drawable = g.copy_frame_rect(
                source,
                Rect {
                    corner: Point::default(),
                    extent: bounds,
                },
            );
            g.frame_mut(FrameId::new(drawable, 0)).hot_spot = hot;
            drawable
        });
        Some(Self::adopt(drawable))
    }
}

impl Drop for LoadedDrawable {
    fn drop(&mut self) {
        let lease = self
            .lease
            .take()
            .unwrap_or_else(|| unreachable!("owner lease is held until drop"));
        // the store's destroy below releases the owner's reference
        let drawable = std::mem::ManuallyDrop::new(lease).0;
        let released = state::with(|g| g.destroy_drawable(drawable));
        state::enqueue_all(released);
    }
}

/// Hold the distinct drawables of `frames`.
pub fn retain_frames(frames: &[FrameId]) -> Vec<DrawableLease> {
    let mut drawables: Vec<DrawableId> =
        frames.iter().filter_map(|frame| frame.drawable()).collect();
    drawables.sort_unstable();
    drawables.dedup();
    drawables.into_iter().map(DrawableLease::retain).collect()
}

/// A held frame (or the null frame) for drawing.
#[derive(Clone, Debug, Default)]
pub struct ContextFrame {
    frame: FrameId,
    _lease: Option<DrawableLease>,
}

impl PartialEq for ContextFrame {
    fn eq(&self, other: &Self) -> bool {
        self.frame == other.frame
    }
}

impl ContextFrame {
    /// Hold `frame`'s drawable; panics if it was released.
    pub fn retain(frame: FrameId) -> Self {
        Self {
            frame,
            _lease: frame.drawable().map(DrawableLease::retain),
        }
    }

    pub fn frame(&self) -> FrameId {
        self.frame
    }

    pub fn is_null(&self) -> bool {
        self.frame.is_null()
    }

    pub fn same_frame(&self, other: &Self) -> bool {
        self.frame == other.frame
    }

    /// The next frame, wrapping (C: `IncFrameIndex`).
    pub fn next_frame(&self) -> Self {
        if self.frame.is_null() {
            return self.clone();
        }
        self.with_frame(self.frame.next())
    }

    /// The frame `index` modulo the frame count.
    pub fn wrapped_frame(&self, index: u16) -> Self {
        assert!(!self.frame.is_null(), "absent frame has no allocation");
        self.with_frame(self.frame.at(index))
    }

    /// The frame `index`; panics past the frame count.
    pub fn indexed_frame(&self, index: usize) -> Self {
        assert!(!self.frame.is_null(), "absent frame has no allocation");
        assert!(
            index < usize::from(self.frame_count()),
            "drawable frame index {index}"
        );
        self.with_frame(self.frame.at(index as u16))
    }

    /// The frame offset by `offset`, wrapping (C: `SetRelFrameIndex`).
    pub fn relative_frame(&self, offset: i16) -> Self {
        self.with_frame(self.frame.relative(offset))
    }

    fn with_frame(&self, frame: FrameId) -> Self {
        Self {
            frame,
            _lease: self._lease.clone(),
        }
    }

    pub fn rect(&self) -> Rect {
        self.frame
            .rect()
            .unwrap_or_else(|| panic!("absent frame has no bounds"))
    }

    pub fn frame_count(&self) -> u16 {
        self.frame.count()
    }

    pub fn index(&self) -> u16 {
        self.frame.index()
    }

    pub fn replace_hotspot(&self, hotspot: HotSpot) -> HotSpot {
        self.frame.set_hot_spot(hotspot)
    }

    /// Read a frame-local pixel without applying its hotspot. Flush pending drawing first.
    pub fn pixel(&self, point: Point) -> Color {
        let extent = self.rect().extent;
        assert!(
            point.x >= 0 && point.y >= 0 && point.x < extent.width && point.y < extent.height,
            "frame pixel is outside its extent"
        );
        state::with(|g| {
            g.frame_image(self.frame)
                .canvas()
                .color_at(i32::from(point.x), i32::from(point.y))
        })
    }

    /// Read the normal canvas alpha mask, ignoring its hotspot and draw mode.
    pub fn alpha_occupancy(&self) -> Option<Vec<u8>> {
        if self.frame.is_null() {
            return None;
        }
        state::with(|g| {
            let frame = g.frame(self.frame);
            let (width, height) = (frame.bounds.width, frame.bounds.height);
            let image = frame.image?;
            if width <= 0 || height <= 0 {
                return None;
            }
            let canvas = g.images.get(image).canvas();
            let mut occupancy = Vec::with_capacity(width as usize * height as usize);
            for y in 0..i32::from(height) {
                for x in 0..i32::from(width) {
                    occupancy.push(u8::from(canvas.color_at(x, y).a != 0));
                }
            }
            Some(occupancy)
        })
    }

    /// C: `SetFrameTransparentColor`.
    pub fn transparent_color(&self, color: Color) {
        state::with(|g| g.frame_image_mut(self.frame).set_transparent_color(color));
    }

    fn checked_count(pixels: usize, width: i32, height: i32) {
        let count = usize::try_from(width)
            .unwrap_or_else(|_| panic!("negative pixel width: {width}"))
            .checked_mul(
                usize::try_from(height)
                    .unwrap_or_else(|_| panic!("negative pixel height: {height}")),
            )
            .unwrap_or_else(|| panic!("pixel buffer size overflow"));
        assert!(
            pixels >= count,
            "pixel buffer is shorter than its stride and height"
        );
    }

    /// C: `ReadFramePixelColors`.
    pub fn read_colors(&self, pixels: &mut [Color], width: i32, height: i32) {
        Self::checked_count(pixels.len(), width, height);
        state::with(|g| {
            g.frame_image(self.frame)
                .canvas()
                .read_colors(pixels, width, height)
        });
    }

    /// C: `WriteFramePixelColors`.
    pub fn write_colors(&self, pixels: &[Color], width: i32, height: i32) {
        Self::checked_count(pixels.len(), width, height);
        state::with(|g| {
            g.frame_image_mut(self.frame)
                .canvas_mut()
                .write_colors(pixels, width, height)
        });
    }

    /// C: `ReadFramePixelIndexes`.
    pub fn read_indexes(&self, pixels: &mut [u8], width: i32, height: i32) {
        Self::checked_count(pixels.len(), width, height);
        state::with(|g| {
            g.frame_image(self.frame)
                .canvas()
                .read_indexes(pixels, width, height)
        });
    }

    /// C: `WriteFramePixelIndexes`.
    pub fn write_indexes(&self, pixels: &[u8], width: i32, height: i32) {
        Self::checked_count(pixels.len(), width, height);
        state::with(|g| {
            g.frame_image_mut(self.frame)
                .canvas_mut()
                .write_indexes(pixels, width, height)
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_hotspot_and_bounds_preserve_signed_coordinates() {
        let owner = LoadedDrawable::create(0, 7, 5, 1).unwrap();
        let frame = owner.first_frame();
        assert_eq!(frame.set_hot_spot(HotSpot::new(-2, 3)), HotSpot::default());
        assert_eq!(frame.rect(), Some(Rect::new(2, -3, 7, 5)));
        assert_eq!(FrameId::NULL.rect(), None);
    }

    #[test]
    fn rotation_truncates_hotspot_after_rotating_about_integer_center() {
        let bounds = Extent::new(8, 6);
        let hot = rotate_hotspot(HotSpot::new(6, 3), bounds, Extent::new(6, 8), 90);
        assert_eq!((hot.x, hot.y), (3, 2));
        let center = rotate_hotspot(HotSpot::new(4, 3), bounds, bounds, 123);
        assert_eq!((center.x, center.y), (4, 3));
    }

    #[test]
    fn frame_indexing_wraps_like_pixmap_c() {
        let owner = LoadedDrawable::create(0, 1, 1, 3).unwrap();
        let first = owner.first_frame();
        assert_eq!(first.count(), 3);
        assert_eq!(first.at(4).index(), 1);
        assert_eq!(first.previous().index(), 2);
        assert_eq!(first.at(2).next().index(), 0);
        assert_eq!(first.relative(-4).index(), 2);
        assert_eq!(first.relative(5).index(), 2);
        assert_eq!(first.at(1).equal_index(first.at(2)).index(), 2);
        assert!(FrameId::NULL.next().is_null());
        assert_eq!(FrameId::from_bits(first.at(2).bits()), first.at(2));
        assert_eq!(FrameId::from_bits(0), FrameId::NULL);
    }

    #[test]
    fn owner_release_keeps_leased_frames_until_the_last_lease_drops() {
        let owner = LoadedDrawable::create(0, 4, 4, 2).unwrap();
        let drawable = owner.drawable();
        let lease = owner.lease();
        let frame = lease.context_frame(1);
        frame.replace_hotspot(HotSpot::new(-12, 7));
        drop(owner);
        assert_eq!(lease.frame_rect(1), Rect::new(12, -7, 4, 4));
        drop(lease);
        assert_eq!(frame.next_frame().index(), 0);
        drop(frame);
        assert!(std::panic::catch_unwind(|| DrawableLease::retain(drawable)).is_err());
    }
}
