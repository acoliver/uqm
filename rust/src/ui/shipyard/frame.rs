//! The shipyard hook retains its drawing context until registration is removed.

use crate::engine::{input::InputFrameHook, input::InputSession, Kernel};
use crate::graphics::context_abi::ContextLease;

pub(super) fn install(input: &InputSession, kernel: &Kernel) {
    let context = ContextLease::require(kernel.space_context);
    input.replace_callback(Some(InputFrameHook::new(move |_, _| {
        // SAFETY: the shipyard retains its published menu until this hook is removed.
        context.with_selected(|| unsafe { super::animate_power_lines(None) });
    })));
}
