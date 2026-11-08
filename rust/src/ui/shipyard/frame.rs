//! The shipyard hook retains its drawing context until registration is removed.

use crate::engine::input::{InputFrameHook, InputSession};
use crate::graphics::context_abi::ContextLease;

pub(super) fn install(input: &InputSession, context: ContextLease, draw: impl Fn() + 'static) {
    input.replace_callback(Some(InputFrameHook::new(move |_, _| {
        context.with_selected(&draw);
    })));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graphics::context_abi::{_pCurContext, LoadedContext};
    use std::{cell::Cell, rc::Rc};

    #[test]
    #[serial_test::serial]
    fn suspended_shipyard_hook_retains_its_context_and_restores_selection() {
        let previous = LoadedContext::create(c"previous-shipyard-context");
        let space = LoadedContext::create(c"retained-shipyard-context");
        let context = space.lease();
        // SAFETY: this serialized fixture reads selection while the lease is selected.
        let raw = context.with_selected(|| unsafe { _pCurContext });
        previous.lease().select();
        // SAFETY: the previous owner remains selected on this thread.
        let previous_raw = unsafe { _pCurContext };
        let input = InputSession::default();
        let draws = Rc::new(Cell::new(0));
        let retained = Rc::downgrade(&draws);
        install(&input, context, move || {
            // SAFETY: the hook selects its retained context on this serialized thread.
            assert_eq!(unsafe { _pCurContext }, raw);
            draws.set(draws.get() + 1);
        });
        drop(space);
        input.callback().unwrap().call(&mut |_| {}, &input);
        assert_eq!(retained.upgrade().unwrap().get(), 1);
        // SAFETY: the previous context remains owned throughout the callback.
        assert_eq!(unsafe { _pCurContext }, previous_raw);
        let suspended = input.replace_callback(None).unwrap();
        assert!(input.callback().is_none());
        suspended.call(&mut |_| {}, &input);
        assert_eq!(retained.upgrade().unwrap().get(), 2);
        // SAFETY: the previous owner remains live after the suspended callback.
        assert_eq!(unsafe { _pCurContext }, previous_raw);
        drop(suspended);
        assert!(retained.upgrade().is_none());
        drop(previous);
        // SAFETY: the serialized fixture has released both context owners.
        assert!(unsafe { _pCurContext }.is_null());
    }
}
