//! Controller-owned GPU diagnostics, kept outside the child's sealed inventory.

use std::path::Path;
use std::time::SystemTime;

pub(crate) const LOG_LIMIT: u64 = 64 * 1024;
#[cfg(target_os = "macos")]
const QUERY_OUTPUT_LIMIT: usize = 16 * 1024;
const ERROR: &str = "kIOGPUCommandBufferCallbackError";

pub(crate) struct Report {
    pub errors: String,
    pub status: String,
    collection_error: Option<String>,
}

impl Report {
    fn from_output(output: &[u8], collection: Result<String, String>) -> Self {
        let errors: String = String::from_utf8_lossy(output)
            .lines()
            .filter(|line| line.contains(ERROR))
            .flat_map(|line| [line, "\n"])
            .collect();
        let (status, collection_error) = match collection {
            Ok(status) => (status, None),
            Err(error) => (error.clone(), Some(error)),
        };
        Self {
            errors,
            status,
            collection_error,
        }
    }

    fn not_applicable(reason: &str) -> Self {
        Self::from_output(
            b"",
            Ok(format!("GPU diagnostics: not applicable ({reason})\n")),
        )
    }

    pub fn decide(&self, scenario: Result<(), String>) -> Result<(), String> {
        if let Some(first) = self.errors.lines().next() {
            return Err(format!("GPU command buffer error: {first}"));
        }
        if let Some(error) = &self.collection_error {
            let original = scenario
                .err()
                .map(|error| format!("{error}; "))
                .unwrap_or_default();
            return Err(format!(
                "{original}GPU diagnostic collection failed: {error}"
            ));
        }
        scenario
    }
}

#[cfg(any(target_os = "macos", test))]
fn query_args(pid: u32, start: SystemTime, end: SystemTime) -> Result<Vec<String>, String> {
    let start = start
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|error| error.to_string())?;
    let end = end
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|error| error.to_string())?;
    // log show's epoch boundaries are whole seconds. Enclose the observed run
    // rather than dropping events in its first or last fractional second.
    Ok(vec![
        "show".into(),
        "--style".into(),
        "compact".into(),
        "--info".into(),
        "--debug".into(),
        "--start".into(),
        format!("@{}", start.as_secs()),
        "--end".into(),
        format!("@{}", end.as_secs() + u64::from(end.subsec_nanos() != 0)),
        "--predicate".into(),
        format!("processID == {pid} AND eventMessage CONTAINS \"{ERROR}\""),
    ])
}

#[cfg(target_os = "macos")]
fn game_pid(scenario: &Path) -> Result<Option<u32>, String> {
    for name in ["native-acceptance.json", "native-acceptance-failure.json"] {
        let path = scenario.join(name);
        if !path.exists() {
            continue;
        }
        let bytes = crate::read_regular_file_bounded(
            &path,
            crate::native_config::runtime_contract()
                .inventory_limits
                .member_bytes,
        )?;
        let receipt: serde_json::Value =
            serde_json::from_slice(&bytes).map_err(|error| error.to_string())?;
        if receipt["schema"]
            == uqm_rust::automation::native_window::NATIVE_ACCEPTANCE_SETUP_FAILURE_SCHEMA
        {
            return Ok(None);
        }
        let pid = receipt["child"]["process"]["pid"]
            .as_u64()
            .and_then(|pid| u32::try_from(pid).ok())
            .filter(|pid| *pid != 0)
            .ok_or("native child receipt has no game PID")?;
        return Ok(Some(pid));
    }
    Err("scenario has no child receipt identifying the game's PID".into())
}

pub(crate) fn run_scenario(
    command: &mut std::process::Command,
    timeout: std::time::Duration,
    suite: &mut crate::native_suite::SuiteAccounting,
    index: usize,
    label: &str,
) -> Result<(), String> {
    run_scenario_with(command, timeout, suite, index, label, collect)
}

fn run_scenario_with(
    command: &mut std::process::Command,
    timeout: std::time::Duration,
    suite: &mut crate::native_suite::SuiteAccounting,
    index: usize,
    label: &str,
    collect: impl FnOnce(&Path, SystemTime, SystemTime) -> Report,
) -> Result<(), String> {
    let started = SystemTime::now();
    let captured = crate::process::run(command, Some(timeout));
    let ended = SystemTime::now();
    crate::echo_output(&captured);
    let supervision = crate::classify_supervision(&captured, label);
    let outcome = if captured.succeeded() {
        Ok(())
    } else {
        Err(supervision.detail.clone())
    };
    let gpu = collect(&suite.directory(index)?, started, ended);
    let gpu_retention = suite.retain_gpu_diagnostics(index, &gpu);
    let retention =
        suite.record_supervision(index, supervision, &captured.stdout, &captured.stderr);
    let accounting = suite.account_scenario(index);
    gpu.decide(outcome)
        .and(gpu_retention)
        .and(retention)
        .and(accounting)
}

pub(crate) fn collect(scenario: &Path, start: SystemTime, end: SystemTime) -> Report {
    #[cfg(target_os = "macos")]
    let report = match game_pid(scenario) {
        Ok(Some(pid)) => query(pid, start, end),
        Ok(None) => Report::not_applicable("setup failed before a game was launched"),
        Err(error) => Report::from_output(b"", Err(error)),
    };
    #[cfg(not(target_os = "macos"))]
    let report = {
        let _ = (scenario, start, end);
        Report::not_applicable("macOS unified log is not available on this platform")
    };
    eprintln!("{}", report.status.trim_end());
    report
}

#[cfg(target_os = "macos")]
fn query(pid: u32, start: SystemTime, end: SystemTime) -> Report {
    let args = match query_args(pid, start, end) {
        Ok(args) => args,
        Err(error) => return Report::from_output(b"", Err(error)),
    };
    let output = crate::process::run_bounded(
        std::process::Command::new("/usr/bin/log").args(&args),
        Some(std::time::Duration::from_secs(30)),
        QUERY_OUTPUT_LIMIT,
    );
    let command = format!("/usr/bin/log {}", args.join(" "));
    let status = if !output.succeeded() {
        Err(output.failure_detail(&command))
    } else if output.stdout_bytes_seen > QUERY_OUTPUT_LIMIT as u64
        || output.stderr_bytes_seen > QUERY_OUTPUT_LIMIT as u64
    {
        Err(format!(
            "{command}: query output exceeded {QUERY_OUTPUT_LIMIT} bytes"
        ))
    } else {
        Ok(format!(
            "GPU diagnostics: query passed\n{command}\n{}",
            String::from_utf8_lossy(&output.stderr)
        ))
    };
    Report::from_output(&output.stdout, status)
}

#[cfg(test)]
#[path = "native_gpu_tests.rs"]
mod tests;
