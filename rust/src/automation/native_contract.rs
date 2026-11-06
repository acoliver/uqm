//! Runtime deadlines and resource limits shared by a native child and its observer.

use super::native_window::{NativeInventoryLimits, NativeWindowBounds};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeWindowRuntimeContract {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_input_ticks: Option<u64>,
    pub capture_timeout_ms: u64,
    pub capture_kill_grace_ms: u64,
    pub observer_timeout_ms: u64,
    pub observer_kill_grace_ms: u64,
    pub acknowledgement_timeout_ms: u64,
    pub outer_child_timeout_ms: u64,
    pub outer_child_kill_grace_ms: u64,
    pub child_stdout_budget_bytes: u64,
    pub child_stderr_budget_bytes: u64,
    pub observer_response_budget_bytes: u64,
    pub capture_budget_bytes: u64,
    pub content_expansion_budget_bytes: u64,
    pub inventory_limits: NativeInventoryLimits,
    pub expected_client_bounds: NativeWindowBounds,
}

impl NativeWindowRuntimeContract {
    #[must_use]
    pub const fn capture_timeout(self) -> std::time::Duration {
        std::time::Duration::from_millis(self.capture_timeout_ms)
    }

    #[must_use]
    pub const fn capture_kill_grace(self) -> std::time::Duration {
        std::time::Duration::from_millis(self.capture_kill_grace_ms)
    }

    #[must_use]
    pub const fn observer_timeout(self) -> std::time::Duration {
        std::time::Duration::from_millis(self.observer_timeout_ms)
    }

    #[must_use]
    pub const fn observer_kill_grace(self) -> std::time::Duration {
        std::time::Duration::from_millis(self.observer_kill_grace_ms)
    }

    #[must_use]
    pub const fn acknowledgement_timeout(self) -> std::time::Duration {
        std::time::Duration::from_millis(self.acknowledgement_timeout_ms)
    }

    #[must_use]
    pub const fn outer_child_timeout(self) -> std::time::Duration {
        std::time::Duration::from_millis(self.outer_child_timeout_ms)
    }

    #[must_use]
    pub const fn outer_child_kill_grace(self) -> std::time::Duration {
        std::time::Duration::from_millis(self.outer_child_kill_grace_ms)
    }

    #[must_use]
    pub const fn has_valid_deadline_order(self) -> bool {
        let observer = self
            .observer_timeout_ms
            .saturating_add(self.observer_kill_grace_ms);
        let capture = self
            .capture_timeout_ms
            .saturating_add(self.capture_kill_grace_ms);
        !matches!(self.expected_input_ticks, Some(0))
            && self.capture_timeout_ms > 0
            && self.capture_kill_grace_ms > 0
            && self.observer_timeout_ms > capture
            && self.acknowledgement_timeout_ms > observer.saturating_add(observer)
            && self.outer_child_timeout_ms > self.acknowledgement_timeout_ms
            && self.outer_child_kill_grace_ms > 0
            && self.child_stdout_budget_bytes > 0
            && self.child_stderr_budget_bytes > 0
            && self.observer_response_budget_bytes > 0
            && self.capture_budget_bytes > 0
            && self.content_expansion_budget_bytes > 0
            && self.inventory_limits.is_valid()
            && self.expected_client_bounds.width > 0
            && self.expected_client_bounds.height > 0
    }
}
