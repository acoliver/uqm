use super::*;

#[test]
#[cfg(unix)]
fn termination_does_not_need_another_process() {
    use std::os::unix::process::{CommandExt, ExitStatusExt};

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

    let mut child = Command::new("sleep")
        .arg("30")
        .process_group(0)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let limit = libc::rlimit {
        rlim_cur: 0,
        rlim_max: 0,
    };
    // SAFETY: this isolated helper changes only its own process limit.
    let limited = unsafe { libc::setrlimit(libc::RLIMIT_NPROC, &limit) };
    let extra = Command::new("/bin/true").status();
    let started = Instant::now();
    let result = terminate(&mut child);
    let elapsed = started.elapsed();
    // Clean up even when the implementation still tries to spawn /bin/kill.
    let _ = child.kill();
    let _ = child.wait();

    assert_eq!(limited, 0);
    assert_eq!(extra.unwrap_err().raw_os_error(), Some(libc::EAGAIN));
    assert_eq!(result.unwrap().signal(), Some(libc::SIGKILL));
    assert!(elapsed < Duration::from_secs(3));
}
