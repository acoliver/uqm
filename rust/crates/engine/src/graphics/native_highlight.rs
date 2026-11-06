//! Exclusive flashing-region ownership with its retained drawing context.
use super::{flash, native_abi::*, native_context::ContextLease};
use std::ptr::NonNull;

pub struct Highlight {
    native: NonNull<flash::FlashContext>,
    context: ContextLease,
}

enum Operation {
    Rect(RECT),
    Merge(i32, i32, i32),
    FrameTime(u32),
    Speed(u32, u32, u32, u32),
    Start,
    Process,
    Terminate,
}

impl Highlight {
    pub(super) fn new(
        context: ContextLease,
        native_context: NonNull<context_desc>,
        rect: RECT,
    ) -> Self {
        // SAFETY: the context lease retains this registered context on its graphics thread.
        let native = unsafe { flash::Flash_createHighlight(native_context.as_ptr(), &rect) };
        Self {
            native: NonNull::new(native).unwrap_or_else(|| panic!("highlight allocation failed")),
            context,
        }
    }

    fn apply(&mut self, operation: Operation) {
        self.context.assert_thread();
        let native = self.native.as_ptr();
        // SAFETY: this exclusive owner keeps both the flash state and drawing context live.
        unsafe {
            match operation {
                Operation::Rect(rect) => flash::Flash_setRect(native, &rect),
                Operation::Merge(start, end, denominator) => {
                    flash::Flash_setMergeFactors(native, start, end, denominator)
                }
                Operation::FrameTime(time) => flash::Flash_setFrameTime(native, time),
                Operation::Speed(fade_in, on, fade_out, off) => {
                    flash::Flash_setSpeed(native, fade_in, on, fade_out, off)
                }
                Operation::Start => flash::Flash_start(native),
                Operation::Process => flash::Flash_process(native),
                Operation::Terminate => flash::Flash_terminate(native),
            }
        }
    }

    pub fn set_rect(&mut self, rect: RECT) {
        self.apply(Operation::Rect(rect));
    }
    pub fn set_merge_factors(&mut self, start: i32, end: i32, denominator: i32) {
        assert_ne!(
            denominator, 0,
            "highlight merge denominator must be nonzero"
        );
        self.apply(Operation::Merge(start, end, denominator));
    }
    pub fn set_frame_time(&mut self, time: u32) {
        self.apply(Operation::FrameTime(time));
    }
    pub fn set_speed(&mut self, fade_in: u32, on: u32, fade_out: u32, off: u32) {
        self.apply(Operation::Speed(fade_in, on, fade_out, off));
    }
    pub fn start(&mut self) {
        self.apply(Operation::Start);
    }
    pub fn process(&mut self) {
        self.apply(Operation::Process);
    }
}

impl Drop for Highlight {
    fn drop(&mut self) {
        self.apply(Operation::Terminate);
    }
}

#[cfg(test)]
mod tests {
    use super::super::{context_abi, ffi, native_abi::*};
    use std::{ptr::NonNull, rc::Rc};

    #[test]
    #[serial_test::serial]
    fn highlight_retains_its_context_until_its_own_release() {
        // SAFETY: the serialized fixture exclusively owns context allocation and release.
        let raw = unsafe {
            #[cfg(feature = "debug-process")]
            {
                context_abi::CreateContextAux(c"highlight-owner-test".as_ptr())
            }
            #[cfg(not(feature = "debug-process"))]
            {
                context_abi::CreateContextAux()
            }
        };
        let context = context_abi::ContextLease::retain(NonNull::new(raw).unwrap());
        let allocation = ffi::native_allocations()
            .contexts
            .get(&(raw as usize))
            .cloned()
            .unwrap();
        let weak = Rc::downgrade(&allocation);
        drop(allocation);
        let rect = RECT {
            corner: POINT { x: 17, y: 29 },
            extent: EXTENT {
                width: 18,
                height: 18,
            },
        };
        let mut highlight = context.highlight(rect);
        // SAFETY: the fixture consumes the native authority while the typed context stays retained.
        assert_eq!(unsafe { context_abi::DestroyContext(raw) }, 1);
        drop(context);
        assert!(weak.upgrade().is_some());
        highlight.set_rect(RECT {
            corner: POINT { x: 35, y: 29 },
            ..rect
        });
        highlight.set_merge_factors(2, 3, 2);
        highlight.set_frame_time(840 / 16);
        highlight.set_speed(0, 840 / 16, 0, 840 / 16);
        highlight.process();
        drop(highlight);
        assert!(weak.upgrade().is_none());
    }
}
