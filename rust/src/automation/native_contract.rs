//! Runtime deadlines and resource limits shared by a native child and its observer.

use super::native_trace_bound::NativeTraceProfile;
use super::native_window::{NativeInventoryLimits, NativeWindowBounds};
use super::script::ValidatedScript;
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
    /// Whether the deadlines are ordered so each outlives what it supervises.
    pub fn validate(self) -> Result<(), String> {
        if !self.has_valid_deadline_order() {
            return Err("native runtime contract has invalid deadline order".to_string());
        }
        Ok(())
    }

    /// Admission of a scenario's contract before any child launches: ordered
    /// deadlines, and a trace that fits for the script's combat reach.
    pub fn admit(self, script: &ValidatedScript) -> Result<(), String> {
        self.validate()?;
        self.check_trace_fits(NativeTraceProfile::of_script(script))
    }

    /// The trace of a scenario with a recorded length must fit one inventory
    /// member, since the journal, the live reader and retention all stop there.
    /// Rejecting it before launch beats a run that dies partway through.
    fn check_trace_fits(self, profile: NativeTraceProfile) -> Result<(), String> {
        let Some(ticks) = self.expected_input_ticks else {
            return Ok(());
        };
        let bound = profile
            .max_trace_bytes(ticks)
            .ok_or("native trace bound overflows")?;
        if bound > self.inventory_limits.member_bytes {
            return Err(format!(
                "native trace for {ticks} input ticks needs up to {bound} bytes, over the {} byte member limit",
                self.inventory_limits.member_bytes
            ));
        }
        Ok(())
    }

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

#[cfg(test)]
mod tests {
    use super::*;

    /// `campaign-s07c-war-cleared`, the longest connected-campaign scenario.
    const S07C_INPUT_TICKS: u64 = 101_050;
    const MEMBER_BYTES: u64 = 64 * 1024 * 1024;

    fn contract(expected_input_ticks: Option<u64>) -> NativeWindowRuntimeContract {
        NativeWindowRuntimeContract {
            expected_input_ticks,
            capture_timeout_ms: 30_000,
            capture_kill_grace_ms: 5_000,
            observer_timeout_ms: 40_000,
            observer_kill_grace_ms: 5_000,
            acknowledgement_timeout_ms: 95_000,
            outer_child_timeout_ms: 300_000,
            outer_child_kill_grace_ms: 5_000,
            child_stdout_budget_bytes: 16 * 1024 * 1024,
            child_stderr_budget_bytes: 16 * 1024 * 1024,
            observer_response_budget_bytes: 64 * 1024,
            capture_budget_bytes: MEMBER_BYTES,
            content_expansion_budget_bytes: 32 * 1024 * 1024,
            inventory_limits: NativeInventoryLimits {
                member_count: 10_000,
                member_bytes: MEMBER_BYTES,
                aggregate_bytes: 1024 * 1024 * 1024,
                path_bytes: 4096,
                aggregate_path_bytes: 8 * 1024 * 1024,
            },
            expected_client_bounds: NativeWindowBounds {
                x: 80,
                y: 48,
                width: 1280,
                height: 960,
            },
        }
    }

    #[test]
    fn a_trace_for_the_longest_campaign_scenario_fits_one_member() {
        contract(Some(S07C_INPUT_TICKS))
            .check_trace_fits(NativeTraceProfile::WITHOUT_COMBAT)
            .unwrap();
    }

    #[test]
    fn a_scenario_whose_trace_cannot_fit_is_rejected_before_launch() {
        let error = contract(Some(S07C_INPUT_TICKS))
            .check_trace_fits(NativeTraceProfile::WITH_COMBAT)
            .unwrap_err();
        assert!(error.contains("member limit"), "{error}");
        assert!(contract(Some(u64::MAX))
            .check_trace_fits(NativeTraceProfile::WITHOUT_COMBAT)
            .is_err());
        contract(None)
            .check_trace_fits(NativeTraceProfile::WITH_COMBAT)
            .unwrap();
    }
}
