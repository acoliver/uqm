//! Explicit dependencies supplied to synchronous input and widget callbacks.

use super::controls::InputControls;
use crate::configuration::{Configuration, Preferences};
use std::ops::Deref;

#[derive(Clone, Default)]
pub struct InputSession {
    controls: InputControls,
    pub configuration: Configuration,
}

impl InputSession {
    pub fn new(controls: InputControls, configuration: Configuration) -> Self {
        Self {
            controls,
            configuration,
        }
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
