//! One observation per native element callback.

use crate::engine::battle::{Cx, ElementHook, ElementId};

pub fn invoke_element(cx: &mut Cx<'_>, hook: ElementHook, element: ElementId) {
    match hook {
        ElementHook::Native(callback) => {
            let parent = cx.element_parent_address(element) as usize;
            super::dispatch::invoke(callback as usize, parent, || callback(cx, element));
        }
    }
}
