pub mod bridge;
pub mod observe;

use std::sync::Arc;

/// Owns the observer for the full application lifecycle, including finalization.
pub struct Application {
    observer: Option<Arc<crate::automation::AutomationObserver>>,
}

impl Application {
    pub fn new(observer: Option<Arc<crate::automation::AutomationObserver>>) -> Self {
        if let Some(observer) = &observer {
            bridge::install_observer(observer);
        }
        Self { observer }
    }

    pub fn observer(&self) -> Option<&Arc<crate::automation::AutomationObserver>> {
        self.observer.as_ref()
    }
}

impl Drop for Application {
    fn drop(&mut self) {
        if let Some(observer) = &self.observer {
            bridge::uninstall_observer(observer);
        }
    }
}
