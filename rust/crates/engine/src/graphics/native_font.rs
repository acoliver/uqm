//! Font allocation ownership on the serialized graphics thread.
use super::{ffi, native_abi::font_desc, native_drawables};
use std::{ffi::CStr, ptr::NonNull, rc::Rc};

pub(super) struct Allocation {
    font: NonNull<font_desc>,
    owned: native_drawables::OwnedAllocation,
}

#[derive(Clone)]
pub struct FontLease(Rc<Allocation>);
impl FontLease {
    pub fn retain(font: NonNull<font_desc>) -> Self {
        let allocation = ffi::native_allocations()
            .fonts
            .get(&(font.as_ptr() as usize))
            .cloned();
        let allocation =
            allocation.unwrap_or_else(|| panic!("font is unowned or retired: {font:p}"));
        Self(allocation)
    }

    pub fn measure(
        &self,
        string: &CStr,
        baseline: super::native_abi::POINT,
        align: super::native_abi::TEXT_ALIGN,
        characters: u16,
    ) -> TextMeasurement {
        self.measure_bounded::<100>(string, baseline, align, characters)
    }

    pub fn measure_bounded<const N: usize>(
        &self,
        string: &CStr,
        baseline: super::native_abi::POINT,
        align: super::native_abi::TEXT_ALIGN,
        characters: u16,
    ) -> TextMeasurement<N> {
        let mut measured = TextMeasurement {
            rect: super::native_abi::RECT {
                corner: baseline,
                extent: super::native_abi::EXTENT {
                    width: 0,
                    height: 0,
                },
            },
            characters: characters.min(N.min(u16::MAX as usize) as u16),
            deltas: [0; N],
            leading: 0,
            visible: false,
        };
        let mut text = super::native_abi::TEXT {
            baseline,
            pStr: string.as_ptr(),
            align,
            CharCount: measured.characters,
        };
        let font = self.as_raw();
        // SAFETY: the lease retains this font; the terminated string and the
        // bounded character count keep all input and delta accesses in their buffers.
        unsafe {
            measured.leading = (*font).Leading as i16;
            measured.visible = super::font_abi::measure(
                font,
                &mut text,
                &mut measured.rect,
                measured.deltas.as_mut_ptr(),
            ) != 0;
        }
        measured.characters = text.CharCount;
        measured
    }

    pub(super) fn as_raw(&self) -> *mut font_desc {
        self.0.owned.assert_thread();
        self.0.font.as_ptr()
    }
}

pub struct TextMeasurement<const N: usize = 100> {
    pub rect: super::native_abi::RECT,
    pub characters: u16,
    pub deltas: [u8; N],
    pub leading: i16,
    pub visible: bool,
}

/// Exclusive logical release authority, with leases retaining the physical allocation.
pub struct LoadedFont {
    lease: FontLease,
}
impl LoadedFont {
    pub fn load(resource: &CStr) -> Option<Self> {
        crate::resource::ffi_bridge::detach_resource_of_type(resource, "FONTRES").map(|font| Self {
            lease: FontLease::retain(font.cast()),
        })
    }

    pub fn lease(&self) -> FontLease {
        self.lease.clone()
    }

    pub fn as_raw(&self) -> *mut font_desc {
        self.lease.as_raw()
    }
}
impl Drop for LoadedFont {
    fn drop(&mut self) {
        release(self.lease.0.font);
    }
}

/// # Safety
/// The loader must transfer a fully initialized, uniquely owned font allocation.
pub(super) unsafe fn register(font: NonNull<font_desc>) {
    let mut allocations = ffi::native_allocations();
    let owner = allocations
        .owner
        .clone()
        .unwrap_or_else(|| panic!("font registry has no owner"));
    let allocation = Rc::new(Allocation {
        font,
        owned: native_drawables::OwnedAllocation::new(
            native_drawables::AllocationKind::Font(font),
            owner,
        ),
    });
    assert!(
        allocations
            .fonts
            .insert(font.as_ptr() as usize, allocation)
            .is_none(),
        "font already registered"
    );
}

pub(super) fn release(font: NonNull<font_desc>) {
    let allocation = ffi::native_allocations()
        .fonts
        .remove(&(font.as_ptr() as usize));
    let allocation = allocation.unwrap_or_else(|| panic!("release of unowned font: {font:p}"));
    drop(allocation);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[serial_test::serial]
    fn bounded_measurement_preserves_empty_and_invalid_legacy_name_behavior() {
        // SAFETY: the fixture transfers its initialized empty font to the registry.
        let raw = unsafe {
            let raw = NonNull::new(
                crate::memory::rust_hcalloc(std::mem::size_of::<font_desc>()).cast::<font_desc>(),
            )
            .unwrap();
            (*raw.as_ptr()).Leading = 17;
            register(raw);
            raw
        };
        let owner = LoadedFont {
            lease: FontLease::retain(raw),
        };
        let lease = owner.lease();
        let baseline = super::super::native_abi::POINT { x: 20, y: 30 };
        for name in [c"", CStr::from_bytes_with_nul(&[0xe9, 0]).unwrap()] {
            let measured = lease.measure(name, baseline, 0, 30);
            assert!(!measured.visible);
            assert_eq!((measured.rect.corner.x, measured.rect.corner.y), (20, 30));
            assert_eq!(
                (measured.rect.extent.width, measured.rect.extent.height),
                (0, 0)
            );
            assert_eq!(measured.characters, 30);
            assert_eq!(measured.deltas, [0; 100]);
            assert_eq!(measured.leading, 17);
        }
        drop(owner);
        assert_eq!(lease.measure(c"", baseline, 0, u16::MAX).characters, 100);
    }

    #[test]
    #[serial_test::serial]
    fn font_release_waits_for_the_last_lease_and_rejects_unknown_addresses() {
        // SAFETY: this fixture allocates an empty font and transfers it to the registry.
        let font = unsafe {
            let font = NonNull::new(
                crate::memory::rust_hcalloc(std::mem::size_of::<font_desc>()).cast::<font_desc>(),
            )
            .unwrap();
            (*font.as_ptr()).Leading = 17;
            register(font);
            font
        };
        let owner = LoadedFont {
            lease: FontLease::retain(font),
        };
        let lease = owner.lease();
        let allocation = Rc::downgrade(&lease.0);
        drop(owner);
        // SAFETY: the lease keeps the font allocated after logical resource release.
        assert_eq!(unsafe { (*lease.as_raw()).Leading }, 17);
        assert!(std::panic::catch_unwind(|| release(font)).is_err());
        assert!(std::panic::catch_unwind(|| release(NonNull::dangling())).is_err());
        // SAFETY: this serial fixture owns the context and its selected font.
        let raw_context = unsafe {
            #[cfg(feature = "debug-process")]
            {
                super::super::context_abi::CreateContextAux(c"font-lease".as_ptr())
            }
            #[cfg(not(feature = "debug-process"))]
            {
                super::super::context_abi::CreateContextAux()
            }
        };
        let context =
            super::super::context_abi::ContextLease::retain(NonNull::new(raw_context).unwrap());
        let original = super::super::native_abi::Color {
            r: 17,
            g: 31,
            b: 47,
            a: 255,
        };
        context.select();
        // SAFETY: this fixture owns the selected context on the serialized graphics thread.
        unsafe { super::super::context_abi::SetContextForeGroundColor(original) };
        for unwind in [false, true] {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                context.with_text_state(|| {
                    context.set_font(&lease);
                    context.fill_rectangle(
                        super::super::native_abi::Color {
                            r: 91,
                            g: 73,
                            b: 55,
                            a: 255,
                        },
                        super::super::native_abi::RECT {
                            corner: super::super::native_abi::POINT { x: 0, y: 0 },
                            extent: super::super::native_abi::EXTENT {
                                width: 1,
                                height: 1,
                            },
                        },
                    );
                    if unwind {
                        panic!("text fixture failure");
                    }
                });
            }));
            assert_eq!(result.is_err(), unwind);
            // SAFETY: the fixture still owns this context after both normal and unwound scopes.
            unsafe {
                assert!((*raw_context).Font.is_null());
                assert_eq!((*raw_context).ForeGroundColor.r, original.r);
                assert_eq!((*raw_context).ForeGroundColor.g, original.g);
                assert_eq!((*raw_context).ForeGroundColor.b, original.b);
            }
        }
        context.set_font(&lease);
        // SAFETY: the context owns the selected font after its original release.
        assert_eq!(unsafe { (*raw_context).Font }, font.as_ptr());
        drop(lease);
        assert!(allocation.upgrade().is_some());
        context.with_text_state(|| ());
        // SAFETY: logical release cannot retire the retained context or its font.
        unsafe { super::super::context_abi::DestroyContext(raw_context) };
        assert!(allocation.upgrade().is_some());
        drop(context);
        assert!(allocation.upgrade().is_none());
        assert!(std::panic::catch_unwind(|| FontLease::retain(font)).is_err());
    }

    #[test]
    #[serial_test::serial]
    fn loading_rejects_non_font_resources_before_transfer() {
        let key = c"test.font.not-a-font";
        // SAFETY: this fixture installs and removes its own terminated resource key.
        unsafe { crate::resource::ffi_bridge::res_PutInteger(key.as_ptr(), 7) };
        assert!(std::panic::catch_unwind(|| LoadedFont::load(key)).is_err());
        // SAFETY: this fixture still owns the integer resource after rejection.
        unsafe { crate::resource::ffi_bridge::res_Remove(key.as_ptr()) };
    }
}
