//! Shared allocation leases for native drawable handles.
use super::native_abi::{context_desc, drawable_desc, font_desc, FRAME};
use std::{
    collections::BTreeMap,
    ptr::NonNull,
    rc::{Rc, Weak},
    sync::Arc,
    thread::ThreadId,
};

pub(super) struct Allocations {
    pub owner: Option<Arc<ThreadId>>,
    entries: BTreeMap<usize, Rc<Allocation>>,
    frames: BTreeMap<usize, Weak<Allocation>>,
    pub(super) contexts: BTreeMap<usize, Rc<super::native_context::Allocation>>,
    pub(super) fonts: BTreeMap<usize, Rc<super::native_font::Allocation>>,
    pub(super) next_highlight_id: u64,
    pub(super) highlights: BTreeMap<
        super::native_highlight::HighlightId,
        Weak<std::cell::RefCell<super::native_highlight::ScheduledState>>,
    >,
}
impl Allocations {
    pub const fn new() -> Self {
        Self {
            owner: None,
            entries: BTreeMap::new(),
            frames: BTreeMap::new(),
            contexts: BTreeMap::new(),
            fonts: BTreeMap::new(),
            next_highlight_id: 0,
            highlights: BTreeMap::new(),
        }
    }
    pub fn assert_thread(&mut self) {
        let current = std::thread::current().id();
        // Allocations keep the owner pinned even after logical release removes their map entry.
        match &self.owner {
            Some(owner) if Arc::strong_count(owner) > 1 => assert_eq!(
                **owner, current,
                "drawable allocation registry accessed from a non-owner thread"
            ),
            _ => self.owner = Some(Arc::new(current)),
        }
    }
    fn insert(&mut self, drawable: NonNull<drawable_desc>, frames: Vec<FRAME>) {
        let owner = self
            .owner
            .clone()
            .unwrap_or_else(|| panic!("drawable registry has no owning thread"));
        let allocation = Rc::new(Allocation {
            _owned: OwnedAllocation::new(AllocationKind::Drawable(drawable), owner),
            frames,
        });
        self.frames
            .retain(|_, allocation| allocation.strong_count() != 0);
        for address in &allocation.frames {
            self.frames
                .insert(*address as usize, Rc::downgrade(&allocation));
        }
        assert!(
            self.entries
                .insert(drawable.as_ptr() as usize, allocation)
                .is_none(),
            "drawable allocation is already registered"
        );
    }
}

pub(super) enum AllocationKind {
    Drawable(NonNull<drawable_desc>),
    Context(NonNull<context_desc>),
    Font(NonNull<font_desc>),
}

pub(super) struct OwnedAllocation {
    kind: AllocationKind,
    owner: Arc<ThreadId>,
}
impl OwnedAllocation {
    pub(super) fn new(kind: AllocationKind, owner: Arc<ThreadId>) -> Self {
        Self { kind, owner }
    }
    pub(super) fn assert_thread(&self) {
        assert_eq!(*self.owner, std::thread::current().id());
    }
}
impl Drop for OwnedAllocation {
    fn drop(&mut self) {
        self.assert_thread();
        // SAFETY: this is the final graphics-thread owner of the registered allocation.
        unsafe {
            match self.kind {
                AllocationKind::Drawable(drawable) => {
                    super::gfxload::release_cel_data_now(drawable)
                }
                AllocationKind::Font(font) => super::gfxload::release_font_data_now(font),
                AllocationKind::Context(context) => super::context_abi::free_context(context),
            }
        }
    }
}

pub(super) struct Allocation {
    _owned: OwnedAllocation,
    frames: Vec<FRAME>,
}

#[derive(Clone)]
pub struct DrawableLease {
    pub(super) _allocation: Rc<Allocation>,
}

pub(super) unsafe fn register(drawable: NonNull<drawable_desc>, count: u16) {
    // SAFETY: AllocDrawable has initialized this descriptor and every one of its frame slots.
    let frames = unsafe { drawable.as_ref().Frame };
    let addresses = (0..count)
        .map(|i| frames.wrapping_add(usize::from(i)).cast())
        .collect();
    super::ffi::native_allocations().insert(drawable, addresses);
}

pub(super) fn release(drawable: NonNull<drawable_desc>) {
    let allocation = super::ffi::native_allocations()
        .entries
        .remove(&(drawable.as_ptr() as usize));
    let allocation =
        allocation.unwrap_or_else(|| panic!("release of unowned drawable {drawable:p}"));
    drop(allocation);
}

/// Retain registered frame allocations without dereferencing caller-provided addresses.
/// Unknown or expired frames are rejected on the serialized graphics thread.
pub fn retain_frame_allocations(frames: &[FRAME]) -> Vec<DrawableLease> {
    let allocations = super::ffi::native_allocations();
    let retained = frames
        .iter()
        .filter(|frame| !frame.is_null())
        .map(|frame| {
            allocations
                .frames
                .get(&(*frame as usize))
                .and_then(Weak::upgrade)
                .map(|allocation| DrawableLease {
                    _allocation: allocation,
                })
                .ok_or(*frame as usize)
        })
        .collect::<Result<Vec<_>, _>>();
    drop(allocations);
    retained.unwrap_or_else(|frame| panic!("frame belongs to an unowned drawable: {frame:#x}"))
}
/// Exclusive release authority for a loaded or generated drawable.
pub struct LoadedDrawable {
    drawable: NonNull<drawable_desc>,
    lease: DrawableLease,
}
impl LoadedDrawable {
    pub fn load(resource: &std::ffi::CStr) -> Option<Self> {
        let drawable = crate::resource::ffi_bridge::detach_resource_of_type(resource, "GFXRES")?;
        Some(Self::retain_loaded(drawable.cast()))
    }

    pub fn create(flags: u8, width: i16, height: i16, count: u16) -> Option<Self> {
        // SAFETY: the provider creates and registers every frame before transferring ownership.
        let drawable = unsafe { super::drawable_abi::CreateDrawable(flags, width, height, count) };
        NonNull::new(drawable).map(Self::retain_loaded)
    }

    /// Rescale one pixel frame into a separate owner, preserving the source.
    /// Empty frames, invalid indices and dimensions outside native bounds return `None`.
    pub fn rescaled(&self, index: usize, width: i32, height: i32) -> Option<Self> {
        let (width, height) = (i16::try_from(width).ok()?, i16::try_from(height).ok()?);
        let frame = self.lease.frame(index)?;
        let bounds = self.lease.frame_rect(index).extent;
        if width <= 0 || height <= 0 || bounds.width <= 0 || bounds.height <= 0 {
            return None;
        }
        // SAFETY: this lease retains a factory-initialized pixel frame on its graphics thread.
        // RescaleFrame returns a new registered drawable with independent release authority.
        let drawable = unsafe {
            assert!(!(*frame).image.is_null(), "frame has no pixels");
            super::drawable_abi::RescaleFrame(frame, i32::from(width), i32::from(height))
        };
        NonNull::new(drawable).map(Self::retain_loaded)
    }

    /// Copy an area relative to the context's clip into an independently owned drawable.
    /// An absent area copies the current clip; an empty intersection returns `None`.
    pub fn capture(
        context: &super::native_context::ContextLease,
        area: Option<super::native_abi::RECT>,
    ) -> Option<Self> {
        context.with_selected(|| {
            let _target = context.target();
            // SAFETY: the context and target remain retained on their graphics thread;
            // the native copy returns a newly registered drawable with unique release authority.
            let drawable = unsafe {
                super::context_abi::CopyContextRect(
                    area.as_ref().map_or(std::ptr::null(), std::ptr::from_ref),
                )
            };
            NonNull::new(drawable).map(Self::retain_loaded)
        })
    }

    fn retain_loaded(drawable: NonNull<drawable_desc>) -> Self {
        let allocation = super::ffi::native_allocations()
            .entries
            .get(&(drawable.as_ptr() as usize))
            .cloned()
            .unwrap_or_else(|| panic!("loaded drawable is not registered"));
        Self {
            drawable,
            lease: DrawableLease {
                _allocation: allocation,
            },
        }
    }

    pub fn lease(&self) -> DrawableLease {
        self.lease.clone()
    }
}
impl Drop for LoadedDrawable {
    fn drop(&mut self) {
        // SAFETY: this is the unique logical release owner; leases retain the physical allocation.
        unsafe { super::drawable_abi::DestroyDrawable(self.drawable.as_ptr()) };
    }
}

impl DrawableLease {
    pub(super) fn frame_count(&self) -> u16 {
        self._allocation.frames.len() as u16
    }

    pub fn context_frame(&self, index: usize) -> super::native_context::ContextFrame {
        let frame = self
            .frame(index)
            .unwrap_or_else(|| panic!("drawable frame index {index}"));
        super::native_context::ContextFrame::retain(NonNull::new(frame))
    }

    pub fn frame_rect(&self, index: usize) -> super::native_abi::RECT {
        self.context_frame(index).rect()
    }

    pub fn set_hotspot(&self, index: usize, hot: super::native_abi::HOT_SPOT) {
        self.context_frame(index).replace_hotspot(hot);
    }

    pub(super) fn assert_thread(&self) {
        self._allocation._owned.assert_thread();
    }

    pub fn owns_frame(&self, frame: FRAME) -> bool {
        self.frame_at_address(frame).is_some()
    }
    pub fn frame_index(&self, frame: FRAME) -> Option<usize> {
        self._allocation
            .frames
            .iter()
            .position(|owned| *owned == frame)
    }
    pub(super) fn wrapped_frame(&self, index: usize) -> FRAME {
        self.assert_thread();
        self._allocation.frames[index % self._allocation.frames.len()]
    }
    pub fn frame(&self, index: usize) -> Option<FRAME> {
        self._allocation.frames.get(index).copied()
    }
    pub fn frame_at_address(&self, frame: FRAME) -> Option<FRAME> {
        self._allocation
            .frames
            .iter()
            .copied()
            .find(|owned| *owned == frame)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[serial_test::serial]
    fn retained_next_frame_wraps_single_and_multi_frame_portraits() {
        for count in [1, 3] {
            // SAFETY: the fixture transfers the registered allocation to its unique owner.
            let raw = unsafe {
                let raw = super::super::drawable_abi::AllocDrawable(count);
                (*raw).MaxIndex = count - 1;
                for index in 0..count {
                    (*(*raw).Frame.add(usize::from(index))).Index = index;
                }
                raw
            };
            let owner = LoadedDrawable::retain_loaded(NonNull::new(raw).unwrap());
            let lease = owner.lease();
            for index in 0..usize::from(count) {
                let next = lease.context_frame(index).next_frame();
                assert_eq!(
                    next.frame,
                    lease.frame((index + 1) % usize::from(count)).unwrap()
                );
            }
        }
    }

    #[test]
    #[serial_test::serial]
    fn drawable_owner_release_keeps_its_leased_frames_and_bounds_alive() {
        // SAFETY: the fixture transfers this newly registered allocation to its owner.
        let drawable = unsafe { super::super::drawable_abi::AllocDrawable(2) };
        let owner = LoadedDrawable::retain_loaded(NonNull::new(drawable).unwrap());
        let lease = owner.lease();
        let frame = lease.frame(1).unwrap();
        // SAFETY: the fixture exclusively initializes its new frame's bounds.
        unsafe {
            (*frame).Bounds = super::super::native_abi::EXTENT {
                width: 149,
                height: 55,
            }
        };
        lease.set_hotspot(1, super::super::native_abi::HOT_SPOT { x: -12, y: 7 });
        drop(owner);
        let first = lease.context_frame(1).indexed_frame(0).rect();
        assert_eq!((first.extent.width, first.extent.height), (0, 0));
        let rect = lease.frame_rect(1);
        assert_eq!((rect.corner.x, rect.corner.y), (12, -7));
        assert_eq!((rect.extent.width, rect.extent.height), (149, 55));
        let retained = lease.context_frame(1);
        assert!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| lease.context_frame(2)))
                .is_err()
        );
        let allocation = Rc::downgrade(&lease._allocation);
        drop(lease);
        assert!(allocation.upgrade().is_some());
        drop(retained);
        assert!(allocation.upgrade().is_none());
    }

    #[test]
    #[serial_test::serial]
    fn registry_release_validates_addresses_and_keeps_retained_frames_live() {
        // SAFETY: this serialized fixture exclusively owns the newly allocated drawable.
        let drawable = unsafe { super::super::drawable_abi::AllocDrawable(1) };
        // SAFETY: drawable was just allocated and remains owned by this fixture.
        let frame = unsafe { super::super::pixmap_abi::CaptureDrawable(drawable) };
        let retained = retain_frame_allocations(&[frame]);
        let drawable = NonNull::new(drawable).unwrap();
        release(drawable);
        assert_eq!(retained[0].frame(0), Some(frame));
        assert!(std::panic::catch_unwind(|| release(drawable)).is_err());
        assert_eq!(retain_frame_allocations(&[frame])[0].frame(0), Some(frame));
        assert!(std::panic::catch_unwind(|| release(NonNull::dangling())).is_err());
        assert_eq!(retain_frame_allocations(&[frame])[0].frame(0), Some(frame));
        drop(retained);
        assert!(std::panic::catch_unwind(|| retain_frame_allocations(&[frame])).is_err());
    }

    #[test]
    #[serial_test::serial]
    fn context_capture_owns_its_pixels_and_restores_the_previous_context() {
        use super::super::{context_abi, native_abi::*, sdl::lifecycle_abi};
        struct ConversionSurface {
            previous: *mut super::super::ffi::SDL_Surface,
            previous_screen: *mut super::super::ffi::SDL_Surface,
            created: *mut sdl2_sys::SDL_Surface,
            screen: *mut sdl2_sys::SDL_Surface,
        }
        impl Drop for ConversionSurface {
            fn drop(&mut self) {
                // SAFETY: the serialized fixture restores the prior provider before freeing its surface.
                unsafe {
                    lifecycle_abi::format_conv_surf = self.previous;
                    lifecycle_abi::SDL_Screen = self.previous_screen;
                    sdl2_sys::SDL_FreeSurface(self.created);
                    sdl2_sys::SDL_FreeSurface(self.screen);
                }
            }
        }
        // SAFETY: the serialized fixture owns this format surface until all its images drop.
        let _format = unsafe {
            let created =
                sdl2_sys::SDL_CreateRGBSurface(0, 1, 1, 32, 0xff, 0xff00, 0xff0000, 0xff000000);
            assert!(!created.is_null());
            let screen = sdl2_sys::SDL_CreateRGBSurface(0, 1, 1, 32, 0xff, 0xff00, 0xff0000, 0);
            assert!(!screen.is_null());
            let previous = lifecycle_abi::format_conv_surf;
            let previous_screen = lifecycle_abi::SDL_Screen;
            lifecycle_abi::format_conv_surf = created.cast();
            lifecycle_abi::SDL_Screen = screen.cast();
            ConversionSurface {
                previous,
                previous_screen,
                created,
                screen,
            }
        };
        let make_context = || {
            // SAFETY: the serialized fixture owns every context until its final release.
            let raw = unsafe {
                #[cfg(feature = "debug-process")]
                {
                    context_abi::CreateContextAux(c"capture-test".as_ptr())
                }
                #[cfg(not(feature = "debug-process"))]
                {
                    context_abi::CreateContextAux()
                }
            };
            (
                raw,
                context_abi::ContextLease::retain(NonNull::new(raw).unwrap()),
            )
        };
        let (raw, context) = make_context();
        let (previous_raw, previous) = make_context();
        let source = LoadedDrawable::create(2, 16, 12, 1).unwrap();
        let source_allocation = Rc::downgrade(&source.lease._allocation);
        let source_frame = source.lease().context_frame(0);
        drop(source);
        context.set_frame(source_frame);
        context.fill_rectangle(
            Color {
                r: 23,
                g: 45,
                b: 67,
                a: 255,
            },
            RECT {
                corner: POINT { x: 0, y: 0 },
                extent: EXTENT {
                    width: 16,
                    height: 12,
                },
            },
        );
        assert!(source_allocation.upgrade().is_some());
        previous.select();
        let whole = LoadedDrawable::capture(&context, None).unwrap();
        // SAFETY: graphics operations are serialized by the fixture.
        let selected = unsafe { context_abi::_pCurContext };
        assert_eq!(selected, previous_raw);
        assert_eq!(whole.lease().frame_rect(0).extent.width, 16);
        assert_eq!(whole.lease().frame_rect(0).extent.height, 12);
        let captured = context.with_clip(
            RECT {
                corner: POINT { x: 2, y: 3 },
                extent: EXTENT {
                    width: 8,
                    height: 7,
                },
            },
            || {
                LoadedDrawable::capture(
                    &context,
                    Some(RECT {
                        corner: POINT { x: 1, y: 2 },
                        extent: EXTENT {
                            width: 4,
                            height: 3,
                        },
                    }),
                )
                .unwrap()
            },
        );
        previous.select();
        assert!(LoadedDrawable::capture(
            &context,
            Some(RECT {
                corner: POINT { x: 100, y: 100 },
                extent: EXTENT {
                    width: 4,
                    height: 3
                },
            })
        )
        .is_none());
        // SAFETY: the fixture reads its serialized graphics selection and releases its own contexts.
        unsafe {
            let selected = context_abi::_pCurContext;
            assert_eq!(selected, previous_raw);
            context_abi::DestroyContext(raw);
        }
        drop(context);
        assert!(source_allocation.upgrade().is_none());
        let frame = captured.lease().context_frame(0);
        assert_eq!(frame.rect().extent.width, 4);
        assert_eq!(frame.rect().extent.height, 3);
        // SAFETY: the captured allocation retains its frame and (0, 0) is within its bounds.
        let pixel = unsafe {
            super::super::drawable_abi::GetFramePixel(
                captured.lease().frame(0).unwrap(),
                POINT { x: 0, y: 0 },
            )
        };
        assert_eq!((pixel.r, pixel.g, pixel.b), (23, 45, 67));
        whole.lease().set_hotspot(0, HOT_SPOT { x: -2, y: 3 });
        assert!(whole.rescaled(1, 32, 24).is_none());
        assert!(whole.rescaled(0, 0, 24).is_none());
        assert!(whole.rescaled(0, 32, -1).is_none());
        let scaled = whole.rescaled(0, 32, 24).unwrap();
        let scaled_lease = scaled.lease();
        let allocation = Rc::downgrade(&scaled_lease._allocation);
        drop(scaled);
        let bounds = scaled_lease.frame_rect(0);
        assert_eq!((bounds.extent.width, bounds.extent.height), (32, 24));
        assert_eq!((bounds.corner.x, bounds.corner.y), (4, -6));
        assert_eq!(whole.lease().frame_rect(0).extent.width, 16);
        assert_eq!(whole.lease().frame_rect(0).corner.x, 2);
        // SAFETY: the lease retains a live pixel image and this point is within its bounds.
        let scaled_pixel = unsafe {
            super::super::drawable_abi::GetFramePixel(
                scaled_lease.frame(0).unwrap(),
                POINT { x: 0, y: 0 },
            )
        };
        assert_eq!(
            (scaled_pixel.r, scaled_pixel.g, scaled_pixel.b),
            (23, 45, 67)
        );
        assert!(allocation.upgrade().is_some());
        drop(scaled_lease);
        assert!(allocation.upgrade().is_none());
        drop(frame);
        drop(captured);
        drop(whole);
        // SAFETY: the fixture releases its remaining context on the same graphics thread.
        unsafe { context_abi::DestroyContext(previous_raw) };
        drop(previous);
    }
}
