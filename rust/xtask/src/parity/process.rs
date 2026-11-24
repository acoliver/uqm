use super::*;
use std::process::{Command, ExitStatus};
use std::time::{Duration, Instant};
#[cfg(unix)]
use uqm_rust::automation::child_session::{
    command_executable_digest, ChildSession, ChildSessionConfig, ChildSessionError,
};

pub(super) struct Completion {
    pub status: ExitStatus,
    pub timed_out: bool,
}

pub(super) fn run(command: &mut Command, budget: Duration, logs: &Path) -> Result<Completion> {
    fs::create_dir_all(logs).map_err(|error| error.to_string())?;
    // Existing invocations overwrite their diagnostic files, not their evidence.
    for name in ["stdout.log", "stderr.log"] {
        match fs::remove_file(logs.join(name)) {
            Ok(()) => (),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => (),
            Err(error) => return Err(error.to_string()),
        }
    }
    let started = Instant::now();
    let mut timed_out = false;
    let result = supervise(command, budget, logs, &mut timed_out);
    eprintln!(
        "parity process {}: {:.3}s",
        logs.display(),
        started.elapsed().as_secs_f64()
    );
    let publication = write_json(
        &logs.join("duration.json"),
        &serde_json::json!({
            "elapsed_seconds": started.elapsed().as_secs_f64(), "budget_seconds": budget.as_secs_f64(),
            "timed_out": timed_out,
        }),
    );
    match (result, publication) {
        (Err(error), Err(publication)) => {
            Err(format!("{error}; duration publication: {publication}"))
        }
        (Err(error), _) => Err(error),
        (Ok(_), Err(error)) => Err(error),
        (Ok(result), Ok(())) => Ok(result),
    }
}

#[cfg(unix)]
fn supervise(
    command: &mut Command,
    budget: Duration,
    logs: &Path,
    timed_out: &mut bool,
) -> Result<Completion> {
    let config = ChildSessionConfig {
        stdout_log: logs.join("stdout.log"),
        stderr_log: logs.join("stderr.log"),
        stdout_budget: u64::MAX,
        stderr_budget: u64::MAX,
        timeout: budget,
        grace: Duration::from_secs(2),
        executable_digest: command_executable_digest(command).map_err(|error| error.to_string())?,
    };
    let session = ChildSession::spawn_outer_logged(command, config)
        .map_err(|error| format!("{command:?}: {error}"))?;
    let (receipt, timed_out) = match session.finish() {
        Ok(receipt) => (receipt, false),
        Err(failure) => {
            *timed_out = matches!(failure.error, ChildSessionError::Timeout { .. });
            // A timeout is an ordinary unsuccessful completion only when cleanup was observed.
            if !matches!(failure.error, ChildSessionError::Timeout { .. })
                || !failure.receipt.cleanup_diagnostics.is_empty()
                || !failure.receipt.orphan_check_passed
                || !failure.receipt.output_drained
            {
                return Err(failure.to_string());
            }
            (*failure.receipt, true)
        }
    };
    let status = receipt
        .reaped_status
        .ok_or("replay child has no observed exit/reap status")?;
    Ok(Completion { status, timed_out })
}

#[cfg(not(unix))]
fn supervise(
    command: &mut Command,
    budget: Duration,
    logs: &Path,
    timed_out: &mut bool,
) -> Result<Completion> {
    command
        .stdin(std::process::Stdio::null())
        .stdout(fs::File::create(logs.join("stdout.log")).map_err(|error| error.to_string())?)
        .stderr(fs::File::create(logs.join("stderr.log")).map_err(|error| error.to_string())?);
    let child = crate::quality::tool::spawn(command)?;
    let (status, timeout) = uqm_rust::automation::child_session::finish_outer_child(child, budget)?;
    *timed_out = timeout;
    Ok(Completion {
        status,
        timed_out: timeout,
    })
}

pub(super) fn require_success(result: Completion, label: &str, logs: &Path) -> Result<()> {
    if result.timed_out || !result.status.success() {
        return Err(format!(
            "{label}: status={}, watchdog_timeout={}; logs {}",
            result.status,
            result.timed_out,
            logs.display()
        ));
    }
    Ok(())
}

#[cfg(all(test, unix))]
#[path = "tests/process_tests.rs"]
mod tests;
