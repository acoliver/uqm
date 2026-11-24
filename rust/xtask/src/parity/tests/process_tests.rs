use super::*;

#[test]
#[cfg(unix)]
fn termination_does_not_need_another_process() {
    use std::os::unix::process::ExitStatusExt;

    const HELPER: &str = "UQM_WATCHDOG_PROCESS_LIMIT_HELPER";
    if std::env::var_os(HELPER).is_none() {
        let output = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "parity::process::tests::termination_does_not_need_another_process",
                "--nocapture",
            ])
            .env(HELPER, "1")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        return;
    }

    let logs = tempfile::tempdir().unwrap();
    let command = Command::new("sleep");
    let config = ChildSessionConfig {
        stdout_log: logs.path().join("out"),
        stderr_log: logs.path().join("err"),
        stdout_budget: u64::MAX,
        stderr_budget: u64::MAX,
        timeout: Duration::from_millis(20),
        grace: Duration::from_secs(2),
        executable_digest: command_executable_digest(&command).unwrap(),
    };
    let session =
        ChildSession::spawn_outer_logged(Command::new("sleep").arg("30"), config).unwrap();
    let limited = rustix::process::setrlimit(
        rustix::process::Resource::Nproc,
        rustix::process::Rlimit {
            current: Some(0),
            maximum: Some(0),
        },
    );
    let extra = Command::new(std::env::current_exe().unwrap())
        .arg("--list")
        .status();
    let started = Instant::now();
    let failure = session.finish().unwrap_err();
    let elapsed = started.elapsed();

    assert_eq!(limited, Ok(()));
    assert_eq!(extra.unwrap_err().raw_os_error(), Some(libc::EAGAIN));
    assert_eq!(
        failure.receipt.reaped_status.unwrap().signal(),
        Some(libc::SIGKILL)
    );
    assert!(failure.receipt.orphan_check_passed && failure.receipt.output_drained);
    assert!(elapsed < Duration::from_secs(3));
}
