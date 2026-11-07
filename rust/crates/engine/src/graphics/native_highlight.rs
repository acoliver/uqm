//! Exclusive flashing-region ownership with its retained drawing context.
use super::{ffi, flash, native_abi::*, native_context::ContextLease};
use crate::{
    callback::ffi::{AlarmId, Alarm_addAbsoluteMs, Alarm_remove},
    time::counter_abi::ONE_SECOND,
};
use std::{cell::RefCell, ptr::NonNull, rc::Rc};

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
    NextTime,
    PreUpdate,
    PostUpdate,
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

    fn apply(&mut self, operation: Operation) -> u32 {
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
                Operation::NextTime => return flash::Flash_nextTime(native),
                Operation::PreUpdate => flash::Flash_preUpdate(native),
                Operation::PostUpdate => flash::Flash_postUpdate(native),
                Operation::Terminate => flash::Flash_terminate(native),
            }
        }
        0
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
    pub fn pre_update(&mut self) {
        self.apply(Operation::PreUpdate);
    }
    pub fn post_update(&mut self) {
        self.apply(Operation::PostUpdate);
    }

    pub fn schedule(mut self) -> ScheduledHighlight {
        self.start();
        let mut registry = ffi::native_allocations();
        registry.next_highlight_id = registry
            .next_highlight_id
            .checked_add(1)
            .unwrap_or_else(|| panic!("highlight registration ID exhausted"));
        let id = HighlightId(registry.next_highlight_id);
        let state = Rc::new(RefCell::new(ScheduledState {
            id,
            highlight: self,
            alarm: None,
            paused: false,
            update_depth: 0,
        }));
        registry.highlights.insert(id, Rc::downgrade(&state));
        drop(registry);
        state.borrow_mut().schedule();
        ScheduledHighlight(state)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct HighlightId(u64);

pub(super) struct ScheduledState {
    id: HighlightId,
    highlight: Highlight,
    alarm: Option<AlarmId>,
    paused: bool,
    update_depth: u16,
}

impl ScheduledState {
    fn schedule(&mut self) {
        let next = self.highlight.apply(Operation::NextTime);
        let ms = (next / ONE_SECOND)
            .wrapping_mul(1000)
            .wrapping_add((next % ONE_SECOND) * 1000 / ONE_SECOND);
        self.alarm = Some(
            Alarm_addAbsoluteMs(ms, tick, self.id)
                .unwrap_or_else(|| panic!("scheduled highlight requires initialized alarms")),
        );
    }
}

impl Drop for ScheduledState {
    fn drop(&mut self) {
        Alarm_remove(self.alarm.take());
        ffi::native_allocations().highlights.remove(&self.id);
    }
}

fn tick(id: HighlightId) {
    // The alarm crosses only an identity. Registry access checks the graphics thread
    // before upgrading its weak reference to the non-Send animation owner.
    let state = ffi::native_allocations()
        .highlights
        .get(&id)
        .and_then(std::rc::Weak::upgrade)
        .unwrap_or_else(|| panic!("alarm fired for an unregistered highlight"));
    let mut state = state.borrow_mut();
    state.alarm = None;
    state.highlight.process();
    state.schedule();
}

pub struct ScheduledHighlight(Rc<RefCell<ScheduledState>>);

impl ScheduledHighlight {
    pub fn set_rect(&mut self, rect: RECT) {
        self.0.borrow_mut().highlight.set_rect(rect);
    }

    pub fn pause(&mut self) {
        let mut state = self.0.borrow_mut();
        Alarm_remove(state.alarm.take());
        state.paused = true;
    }

    pub fn resume(&mut self) {
        let mut state = self.0.borrow_mut();
        if state.paused {
            state.schedule();
            state.paused = false;
        }
    }

    pub fn pre_update(&mut self) {
        let mut state = self.0.borrow_mut();
        if state.alarm.is_some() {
            state.update_depth = state.update_depth.wrapping_add(1);
            if state.update_depth <= 1 {
                state.highlight.apply(Operation::PreUpdate);
            }
        }
    }

    pub fn post_update(&mut self) {
        let mut state = self.0.borrow_mut();
        if state.alarm.is_some() {
            state.update_depth = state.update_depth.wrapping_sub(1);
            if state.update_depth == 0 {
                state.highlight.apply(Operation::PostUpdate);
            }
        }
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
        // SAFETY: the serialized fixture has released its only highlight.
        unsafe { super::super::flash::release_work_context_for_test() };
        assert!(
            ffi::native_allocations().contexts.is_empty(),
            "highlight fixture must release its cached work context"
        );
        std::thread::spawn(|| {
            // SAFETY: the fixture holds the serial guard and only this joined thread
            // accesses graphics while verifying the next fixture's ownership handoff.
            unsafe {
                let drawable = super::super::drawable_abi::AllocDrawable(2);
                assert!(!drawable.is_null());
                assert_eq!(super::super::drawable_abi::DestroyDrawable(drawable), 1);
            }
        })
        .join()
        .unwrap_or_else(|_| panic!("graphics fixture ownership handoff failed"));
    }
}
