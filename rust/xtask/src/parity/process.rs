use super::*;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};
#[cfg(test)]
#[path = "tests/process_tests.rs"]
mod tests;

pub(super) struct Completion {
    pub status: ExitStatus,
    pub timed_out: bool,
}

pub(super) fn run(command: &mut Command, budget: Duration, logs: &Path) -> Result<Completion> {
    fs::create_dir_all(logs).map_err(|error| error.to_string())?;
    let stdout = fs::File::create(logs.join("stdout.log")).map_err(|error| error.to_string())?;
    let stderr = fs::File::create(logs.join("stderr.log")).map_err(|error| error.to_string())?;
    command.stdin(Stdio::null()).stdout(stdout).stderr(stderr);
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    let started = Instant::now();
    let mut child = command
        .spawn()
        .map_err(|error| format!("launch {command:?}: {error}"))?;
    let result = supervise(&mut child, budget);
    eprintln!(
        "parity process {}: {:.3}s",
        logs.display(),
        started.elapsed().as_secs_f64()
    );
    write_json(
        &logs.join("duration.json"),
        &serde_json::json!({
            "elapsed_seconds": started.elapsed().as_secs_f64(), "budget_seconds": budget.as_secs_f64(),
            "timed_out": result.as_ref().is_ok_and(|result| result.timed_out),
        }),
    )?;
    result
}

fn supervise(child: &mut Child, budget: Duration) -> Result<Completion> {
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                return Ok(Completion {
                    status,
                    timed_out: false,
                })
            }
            Ok(None) => (),
            Err(error) => {
                terminate(child)?;
                return Err(format!("poll replay process: {error}"));
            }
        }
        if started.elapsed() >= budget {
            let status = terminate(child)?;
            return Ok(Completion {
                status,
                timed_out: true,
            });
        }
        std::thread::sleep(Duration::from_millis(5).min(budget.saturating_sub(started.elapsed())));
    }
}

#[cfg(unix)]
pub(crate) fn kill_group(child: &Child) -> std::io::Result<()> {
    let group = libc::pid_t::try_from(child.id())
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidInput, error))?;
    // SAFETY: kill takes integer arguments only. Callers create the child's
    // dedicated process group with process_group(0); the negative ID targets it.
    if unsafe { libc::kill(-group, libc::SIGKILL) } == -1 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}

fn terminate(child: &mut Child) -> Result<ExitStatus> {
    #[cfg(unix)]
    {
        // The process group belongs to this invocation, never a name-wide game kill.
        if let Err(error) = kill_group(child) {
            if let Some(status) = child.try_wait().map_err(|error| error.to_string())? {
                return Ok(status);
            }
            return Err(format!(
                "kill owned process group {} failed: {error}",
                child.id()
            ));
        }
    }
    #[cfg(not(unix))]
    child.kill().map_err(|error| error.to_string())?;
    let started = Instant::now();
    loop {
        if let Some(status) = child.try_wait().map_err(|error| error.to_string())? {
            return Ok(status);
        }
        if started.elapsed() >= Duration::from_secs(2) {
            return Err(format!("process {} did not reap after SIGKILL", child.id()));
        }
        std::thread::sleep(Duration::from_millis(5));
    }
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
