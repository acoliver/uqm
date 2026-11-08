//! Explicit dependencies supplied to synchronous input and widget callbacks.

use super::controls::InputControls;
use crate::configuration::{Configuration, Preferences};
use std::{cell::RefCell, ops::Deref, rc::Rc};

type FrameCallback =
    dyn Fn(&mut dyn FnMut(crate::observe_port::CheckpointView<'_>), &InputControls);

#[derive(Clone)]
pub struct InputFrameHook(Rc<FrameCallback>);

impl InputFrameHook {
    pub fn new(
        callback: impl Fn(&mut dyn FnMut(crate::observe_port::CheckpointView<'_>), &InputControls)
            + 'static,
    ) -> Self {
        Self(Rc::new(callback))
    }

    pub fn call(
        &self,
        checkpoint: &mut dyn FnMut(crate::observe_port::CheckpointView<'_>),
        input: &InputControls,
    ) {
        (self.0)(checkpoint, input);
    }
}

#[derive(Clone, Default)]
pub struct InputSession {
    controls: InputControls,
    callback: Rc<RefCell<Option<InputFrameHook>>>,
    pub configuration: Configuration,
}

impl InputSession {
    pub fn new(controls: InputControls, configuration: Configuration) -> Self {
        Self {
            controls,
            callback: Rc::default(),
            configuration,
        }
    }

    pub fn callback(&self) -> Option<InputFrameHook> {
        self.callback.borrow().clone()
    }

    pub fn replace_callback(&self, callback: Option<InputFrameHook>) -> Option<InputFrameHook> {
        let mut callback_slot = self.callback.borrow_mut();
        assert!(
            callback_slot.is_none() || callback.is_none(),
            "input callback already installed"
        );
        std::mem::replace(&mut *callback_slot, callback)
    }

    pub fn controls(&self) -> &InputControls {
        &self.controls
    }

    pub fn preferences(&self) -> Preferences {
        self.configuration.preferences()
    }
}

impl Deref for InputSession {
    type Target = InputControls;

    fn deref(&self) -> &Self::Target {
        &self.controls
    }
}

impl std::fmt::Debug for InputSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InputSession").finish_non_exhaustive()
    }
}

#[cfg(test)]
mod owned_callback_tests {
    use super::*;
    use std::{cell::Cell, rc::Rc};

    #[test]
    fn captured_callbacks_belong_to_each_input_owner_and_do_not_hold_the_state_lock() {
        let first = InputSession::default();
        let second = InputSession::default();
        let first_phase = Rc::new(Cell::new(0));
        let second_phase = Rc::new(Cell::new(40));
        for (input, phase) in [
            (&first, first_phase.clone()),
            (&second, second_phase.clone()),
        ] {
            input.replace_callback(Some(InputFrameHook::new(move |checkpoint, controls| {
                phase.set(phase.get() + 1);
                controls.set_paused(true);
                checkpoint(crate::observe_port::CheckpointView::InputSampled(
                    phase.get(),
                ));
            })));
        }
        let mut observed = Vec::new();
        let mut checkpoint = |view: crate::observe_port::CheckpointView<'_>| {
            if let crate::observe_port::CheckpointView::InputSampled(phase) = view {
                observed.push(phase);
            }
        };
        first.callback().unwrap().call(&mut checkpoint, &first);
        assert_eq!((first_phase.get(), second_phase.get()), (1, 40));
        assert!(first.paused());
        assert!(!second.paused());
        second.callback().unwrap().call(&mut checkpoint, &second);
        assert_eq!((first_phase.get(), second_phase.get()), (1, 41));
        assert_eq!(observed, [1, 41]);
    }

    #[test]
    fn suspended_callbacks_retain_their_capture_until_removed() {
        let input = InputSession::default();
        let resource = Rc::new(Cell::new(0));
        let retained = Rc::downgrade(&resource);
        input.replace_callback(Some(InputFrameHook::new(move |_, _| {
            resource.set(resource.get() + 1);
        })));
        let suspended = input.replace_callback(None);
        assert!(input.callback().is_none());
        assert_eq!(retained.upgrade().unwrap().get(), 0);
        input.replace_callback(suspended);
        input.clone().callback().unwrap().call(&mut |_| {}, &input);
        assert_eq!(retained.upgrade().unwrap().get(), 1);
        drop(input.replace_callback(None));
        assert!(retained.upgrade().is_none());
    }

    #[test]
    fn a_callback_can_remove_itself_before_a_nested_input_loop() {
        let input = InputSession::default();
        let retained = input.clone();
        input.replace_callback(Some(InputFrameHook::new(move |_, controls| {
            assert!(retained.replace_callback(None).is_some());
            controls.set_paused(true);
        })));
        input.callback().unwrap().call(&mut |_| {}, &input);
        assert!(input.callback().is_none());
        assert!(input.paused());
    }

    #[test]
    #[should_panic(expected = "input callback already installed")]
    fn a_live_callback_cannot_be_overwritten() {
        let input = InputSession::default();
        input.replace_callback(Some(InputFrameHook::new(|_, _| {})));
        input.replace_callback(Some(InputFrameHook::new(|_, _| {})));
    }
}
