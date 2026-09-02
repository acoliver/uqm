//! S4 CI command authority.
//!
//! This module owns the `ci` nested command surface of the xtask binary:
//!
//! - `ci doctor`
//! - `ci plan`
//! - `ci workflow-check`
//! - `ci run <gate|all>`
//! - `ci mutations`
//! - `ci validate-evidence <path>`
//!
//! The single machine-readable authority in `rust/ci/gates.json` defines every
//! gate id, owner, exact command vector, feature profile, and threshold. Gate
//! execution and the mutation suite both consume that authority; no command string is
//! duplicated in a workflow-facing API.

pub mod authority;
pub mod bounded_io;
pub mod cache;
pub mod delta;
pub mod doctor;
pub mod evidence;
pub mod exec;
pub mod mutations;
pub mod plan;
pub mod proof;
pub mod run;
pub mod workflow;

use std::path::Path;

pub use authority::load_authority;
use authority::{Authority, Gate};

/// Errors carry the first failed contract id so callers can surface it verbatim.
pub struct CiError {
    pub contract: String,
    pub detail: String,
}

impl std::fmt::Display for CiError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "contract '{}': {}", self.contract, self.detail)
    }
}

impl std::fmt::Debug for CiError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CiError")
            .field("contract", &self.contract)
            .field("detail", &self.detail)
            .finish()
    }
}

impl std::error::Error for CiError {}

impl CiError {
    pub fn new(contract: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            contract: contract.into(),
            detail: detail.into(),
        }
    }
}

impl From<CiError> for String {
    fn from(error: CiError) -> String {
        error.to_string()
    }
}

/// Nested `ci` argument dispatch.
///
/// `arguments` excludes the leading `ci` token. All existing top-level xtask
/// commands remain untouched; only `ci` is routed here.
pub fn run_ci(root: &Path, arguments: &[String]) -> Result<(), String> {
    let subcommand = arguments.first().map(String::as_str).ok_or_else(ci_usage)?;
    match subcommand {
        "doctor" => {
            reject_extra("ci doctor", &arguments[1..])?;
            doctor::doctor(root)
        }
        "plan" => {
            reject_extra("ci plan", &arguments[1..])?;
            plan::plan(root).map(|_| ()).map_err(String::from)
        }
        "workflow-check" => {
            reject_extra("ci workflow-check", &arguments[1..])?;
            workflow::workflow_check(root).map(|_| ())
        }
        "containment-check" => {
            reject_extra("ci containment-check", &arguments[1..])?;
            exec::verify_uid_containment(root)
        }
        "run" => {
            let gate = arguments.get(1).ok_or_else(ci_usage)?;
            reject_extra("ci run", &arguments[2..])?;
            run::run_gate(root, gate)
        }
        "mutations" => {
            reject_extra("ci mutations", &arguments[1..])?;
            mutations::run_mutations(root).map(|_| ())
        }
        "validate-evidence" => {
            let path = arguments.get(1).ok_or_else(ci_usage)?;
            reject_extra("ci validate-evidence", &arguments[2..])?;
            evidence::validate_evidence_command(root, path)
        }
        _ => Err(ci_usage()),
    }
}

fn reject_extra(command: &str, extra: &[String]) -> Result<(), String> {
    if extra.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "unexpected arguments for '{command}': {}",
            extra.join(" ")
        ))
    }
}

fn ci_usage() -> String {
    "usage: xtask ci <doctor|plan|workflow-check|containment-check|run <gate|all>|mutations|validate-evidence <path>>"
        .into()
}

pub fn gate_by_id<'a>(authority: &'a Authority, id: &str) -> Result<&'a Gate, CiError> {
    authority
        .gates
        .iter()
        .find(|gate| gate.id == id)
        .ok_or_else(|| CiError::new("authority.gate", format!("unknown gate id '{id}'")))
}
