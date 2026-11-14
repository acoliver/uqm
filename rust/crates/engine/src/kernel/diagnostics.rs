//! Per-campaign diagnostic scopes, independent of publication or process globals.
use std::{cell::Cell, rc::Rc};

#[derive(Default)]
pub struct Diagnostics {
    quiet: Rc<Cell<bool>>,
    contexts: Rc<Cell<bool>>,
}

pub struct DiagnosticGuard {
    flag: Rc<Cell<bool>>,
    previous: bool,
}
impl Drop for DiagnosticGuard {
    fn drop(&mut self) {
        self.flag.set(self.previous);
    }
}
impl DiagnosticGuard {
    fn enter(flag: &Rc<Cell<bool>>) -> Self {
        Self {
            previous: flag.replace(true),
            flag: flag.clone(),
        }
    }
}
impl Diagnostics {
    pub fn interactive(&self) -> bool {
        !self.quiet.get()
    }
    pub fn suppress_interaction(&self) -> DiagnosticGuard {
        DiagnosticGuard::enter(&self.quiet)
    }
    pub fn enter_contexts(&self) -> Option<DiagnosticGuard> {
        if self.contexts.get() {
            None
        } else {
            Some(DiagnosticGuard::enter(&self.contexts))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn scopes_restore_nested_and_independent_campaigns_after_unwind() {
        let first = Diagnostics::default();
        let second = Diagnostics::default();
        let outer = first.suppress_interaction();
        assert!(!first.interactive());
        assert!(second.interactive());
        let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _inner = first.suppress_interaction();
            panic!("diagnostic failure");
        }));
        assert!(panic.is_err());
        assert!(!first.interactive());
        drop(outer);
        assert!(first.interactive());
        let contexts = first.enter_contexts().unwrap();
        assert!(first.enter_contexts().is_none());
        assert!(second.enter_contexts().is_some());
        drop(contexts);
        assert!(first.enter_contexts().is_some());
    }
}
