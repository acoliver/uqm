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
    use crate::graphics::context_abi::LoadedContext;
    use std::{cell::Cell, rc::Rc};

    #[test]
    #[serial_test::serial]
    fn suspended_shipyard_hook_retains_its_context_and_restores_selection() {
        let previous = LoadedContext::create(c"previous-shipyard-context");
        let space = LoadedContext::create(c"retained-shipyard-context");
        let context = space.lease();
        let raw = context.with_selected(uqm_engine::test_support::native_probe::selected_context);
        previous.lease().select();
        let previous_raw = uqm_engine::test_support::native_probe::selected_context();
        let input = InputSession::default();
        let draws = Rc::new(Cell::new(0));
        let retained = Rc::downgrade(&draws);
        install(&input, context, move || {
            assert_eq!(
                uqm_engine::test_support::native_probe::selected_context(),
                raw
            );
            draws.set(draws.get() + 1);
        });
        drop(space);
        input.callback().unwrap().call(&mut |_| {}, &input);
        assert_eq!(retained.upgrade().unwrap().get(), 1);
        assert_eq!(
            uqm_engine::test_support::native_probe::selected_context(),
            previous_raw
        );
        let suspended = input.replace_callback(None).unwrap();
        assert!(input.callback().is_none());
        suspended.call(&mut |_| {}, &input);
        assert_eq!(retained.upgrade().unwrap().get(), 2);
        assert_eq!(
            uqm_engine::test_support::native_probe::selected_context(),
            previous_raw
        );
        drop(suspended);
        assert!(retained.upgrade().is_none());
        drop(previous);
        assert!(uqm_engine::test_support::native_probe::selected_context().is_null());
    }
}
