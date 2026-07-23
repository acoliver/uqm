//! Error types for the runtime automation subsystem.
//!
//! All errors retain the originating script path and the offending step index
//! (where applicable) so callers can report precise failures without
//! re-deriving context (REQ-SCRIPT-001).
//!
//! @plan PLAN-20260723-RUNTIME-AUTOMATION.P01
//! @requirement REQ-SCRIPT-001, REQ-SCRIPT-004

use thiserror::Error;

/// The path of a step within the script, used for precise error reporting.
///
/// `None` indicates an error that applies to the document root (for example a
/// malformed version field) rather than to a specific step.
pub type StepIndex = Option<usize>;

/// Every error that can arise while parsing or validating an automation
/// script.
///
/// Each variant carries enough context to produce a message of the form
/// `<path>:<step>: <reason>`. `path` is the source file path supplied to the
/// parser; `step` is the zero-based index of the offending step, or `None`
/// for root-level errors.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P01
/// @requirement REQ-SCRIPT-001, REQ-SCRIPT-004
#[derive(Debug, Error)]
pub enum AutomationError {
    /// The input bytes were not valid UTF-8.
    #[error("{path}: not valid UTF-8: {reason}")]
    NotUtf8 { path: String, reason: String },

    /// The input bytes were not valid JSON.
    #[error("{path}: invalid JSON: {reason}")]
    InvalidJson { path: String, reason: String },

    /// The document version is missing or unsupported. Only version `1` is
    /// accepted (REQ-SCRIPT-002).
    #[error("{path}: unsupported or missing version: {found}")]
    UnsupportedVersion { path: String, found: String },

    /// A required root field is missing (REQ-SCRIPT-002).
    #[error("{path}: missing required field: {field}")]
    MissingField { path: String, field: &'static str },

    /// The document or a nested object contained an unknown field. The root
    /// and every step object use `#[serde(deny_unknown_fields)]` so this is a
    /// hard rejection (REQ-SCRIPT-002, REQ-SCRIPT-003).
    #[error("{path}: unknown field: {field}")]
    UnknownField { path: String, field: String },

    /// A field value was out of its allowed range or otherwise invalid at the
    /// document root.
    #[error("{path}: invalid value for {field}: {reason}")]
    InvalidValue {
        path: String,
        field: &'static str,
        reason: String,
    },

    /// A step-level validation error. `step` is the zero-based step index.
    #[error("{path}: step {step}: {reason}")]
    Step {
        path: String,
        step: usize,
        reason: String,
    },

    /// The `steps` array was empty (REQ-SCRIPT-002).
    #[error("{path}: steps must be nonempty")]
    EmptySteps { path: String },

    /// Exactly one `finish` step is required and it must be last
    /// (REQ-SCRIPT-004).
    #[error("{path}: {reason}")]
    FinishSemantics { path: String, reason: String },

    /// A declared budget is insufficient for the number of admitted callbacks
    /// a step statically requires. Per the inclusive-limit contract, a maximum
    /// `M` admits at most `M-1` callbacks; a step needing `N` admitted
    /// callbacks requires the corresponding maximum to be at least `N+1`
    /// (REQ-SCRIPT-004).
    #[error("{path}: step {step}: {reason}")]
    InsufficientBudget {
        path: String,
        step: usize,
        reason: String,
    },

    /// A checked arithmetic operation (count/duration sum) overflowed
    /// (REQ-SCRIPT-004).
    #[error("{path}: {reason}")]
    ArithmeticOverflow { path: String, reason: String },

    /// The linked binary does not provide a required build capability
    /// (REQ-BUILD-001).
    #[error("missing required build capability: {flag}")]
    MissingBuildCapability { flag: &'static str },
}

impl AutomationError {
    /// Construct a step-level error with precise path and index.
    #[must_use]
    pub fn step(path: impl Into<String>, step: usize, reason: impl Into<String>) -> Self {
        Self::Step {
            path: path.into(),
            step,
            reason: reason.into(),
        }
    }

    /// The source path carried by this error, if any.
    #[must_use]
    pub fn path(&self) -> Option<&str> {
        match self {
            Self::NotUtf8 { path, .. }
            | Self::InvalidJson { path, .. }
            | Self::UnsupportedVersion { path, .. }
            | Self::MissingField { path, .. }
            | Self::UnknownField { path, .. }
            | Self::InvalidValue { path, .. }
            | Self::Step { path, .. }
            | Self::EmptySteps { path, .. }
            | Self::FinishSemantics { path, .. }
            | Self::InsufficientBudget { path, .. }
            | Self::ArithmeticOverflow { path, .. } => Some(path),
            Self::MissingBuildCapability { .. } => None,
        }
    }

    /// The offending step index, if this is a step-level error.
    #[must_use]
    pub fn step_index(&self) -> StepIndex {
        match self {
            Self::Step { step, .. } | Self::InsufficientBudget { step, .. } => Some(*step),
            _ => None,
        }
    }
}

// ===========================================================================
//  Unit tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn step_error_carries_path_and_index() {
        let err = AutomationError::step("script.json", 3, "bad count");
        assert_eq!(err.path(), Some("script.json"));
        assert_eq!(err.step_index(), Some(3));
        let msg = format!("{err}");
        assert!(msg.contains("script.json"));
        assert!(msg.contains("step 3"));
        assert!(msg.contains("bad count"));
    }

    #[test]
    fn root_error_has_no_step() {
        let err = AutomationError::UnsupportedVersion {
            path: "x.json".into(),
            found: "2".into(),
        };
        assert_eq!(err.path(), Some("x.json"));
        assert_eq!(err.step_index(), None);
    }

    #[test]
    fn build_capability_error_has_no_path() {
        let err = AutomationError::MissingBuildCapability {
            flag: "RUST_OWNS_MAIN",
        };
        assert_eq!(err.path(), None);
        let msg = format!("{err}");
        assert!(msg.contains("RUST_OWNS_MAIN"));
    }

    #[test]
    fn budget_error_reports_step() {
        let err = AutomationError::InsufficientBudget {
            path: "s.json".into(),
            step: 2,
            reason: "need 5 admitted input ticks but max is 5".into(),
        };
        assert_eq!(err.step_index(), Some(2));
        let msg = format!("{err}");
        assert!(msg.contains("need 5"));
    }
}
