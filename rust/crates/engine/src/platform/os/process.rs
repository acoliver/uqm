//! Process identity, signalling, waiting and process-table inspection.

use std::io;
use std::os::unix::process::CommandExt as _;
use std::process::Command;

/// The real user ID of this process.
#[must_use]
pub fn real_user_id() -> u32 {
    // SAFETY: getuid has no preconditions and cannot fail.
    unsafe { libc::getuid() }
}

/// The effective user ID of this process.
#[must_use]
pub fn effective_user_id() -> u32 {
    // SAFETY: geteuid has no preconditions and cannot fail.
    unsafe { libc::geteuid() }
}

/// Send `signal` to the process `pid`.
///
/// The caller owns the question of whether `pid` still names the intended
/// process; a PID that was reaped and reused receives the signal.
///
/// # Errors
///
/// Returns the OS error from `kill`, for example `ESRCH` or `EPERM`.
pub fn send_signal(pid: libc::pid_t, signal: libc::c_int) -> io::Result<()> {
    // SAFETY: kill takes integers and never dereferences memory.
    if unsafe { libc::kill(pid, signal) } == -1 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

/// Send `signal` to every member of the process group `group`.
///
/// # Errors
///
/// Returns the OS error from `kill`, for example `ESRCH` for an empty group.
pub fn send_signal_to_group(group: libc::pid_t, signal: libc::c_int) -> io::Result<()> {
    send_signal(-group, signal)
}

/// Whether a process with `pid` exists, by the zero-signal check.
///
/// A process the caller may not signal still exists, so `EPERM` is `true`.
///
/// # Errors
///
/// Returns the OS error from `kill` for any failure other than `ESRCH` and `EPERM`.
pub fn exists(pid: libc::pid_t) -> io::Result<bool> {
    match send_signal(pid, 0) {
        Ok(()) => Ok(true),
        Err(error) => match error.raw_os_error() {
            Some(libc::ESRCH) => Ok(false),
            Some(libc::EPERM) => Ok(true),
            _ => Err(error),
        },
    }
}

/// Make this process the leader of a new process group.
///
/// Async-signal-safe: one `setpgid`.
///
/// # Errors
///
/// Returns the OS error from `setpgid`.
pub fn start_new_process_group() -> io::Result<()> {
    // SAFETY: setpgid(0, 0) takes integers and never dereferences memory.
    if unsafe { libc::setpgid(0, 0) } == -1 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

/// Make this process the leader of a new session.
///
/// Async-signal-safe: one `setsid`.
///
/// # Errors
///
/// Returns the OS error from `setsid`, for example `EPERM` for a group leader.
pub fn start_new_session() -> io::Result<()> {
    // SAFETY: setsid takes no arguments and never dereferences memory.
    if unsafe { libc::setsid() } == -1 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

/// Whether the child `pid` has exited, without reaping it.
///
/// The exited child stays waitable, so its PID and process-group identity
/// cannot be reused until the caller reaps it.
///
/// # Errors
///
/// Returns the OS error from `waitid`; `ECHILD` when `pid` is not a child of
/// this process, or was already reaped.
pub fn has_exited_without_reaping(pid: libc::pid_t) -> io::Result<bool> {
    // SAFETY: siginfo_t is plain data for which all-zero is a valid value.
    let mut info: libc::siginfo_t = unsafe { std::mem::zeroed() };
    // SAFETY: info is writable siginfo_t storage; WNOWAIT leaves the child waitable.
    let result = unsafe {
        libc::waitid(
            libc::P_PID,
            pid as libc::id_t,
            &raw mut info,
            libc::WEXITED | libc::WNOHANG | libc::WNOWAIT,
        )
    };
    if result == -1 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: waitid initialized info on success.
    Ok(unsafe { info.si_pid() } != 0)
}

/// Run `hook` in the child between `fork` and `exec`.
///
/// The hook runs in a forked copy of a multi-threaded process, so it must stay
/// async-signal-safe: no allocation, no lock, no formatting. The functions of
/// this module documented as async-signal-safe, `std::process::id` and
/// arithmetic on values captured by copy qualify.
pub fn before_exec<F>(command: &mut Command, hook: F)
where
    F: FnMut() -> io::Result<()> + Send + Sync + 'static,
{
    // SAFETY: the documented contract confines the hook to async-signal-safe calls.
    unsafe { command.pre_exec(hook) };
}

/// Cap the size of any file the child writes to `bytes`, before it executes.
pub fn limit_file_size_before_exec(command: &mut Command, bytes: u64) {
    let limit = libc::rlimit {
        rlim_cur: bytes as libc::rlim_t,
        rlim_max: bytes as libc::rlim_t,
    };
    before_exec(command, move || {
        // SAFETY: limit is a fully initialized rlimit that outlives the call.
        if unsafe { libc::setrlimit(libc::RLIMIT_FSIZE, &raw const limit) } != 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    });
}

/// Clock ticks per second used by the kernel's `/proc` accounting.
#[cfg(target_os = "linux")]
#[must_use]
pub fn clock_ticks_per_second() -> Option<u64> {
    // SAFETY: sysconf is a read-only configuration query.
    let ticks = unsafe { libc::sysconf(libc::_SC_CLK_TCK) };
    u64::try_from(ticks).ok().filter(|value| *value > 0)
}

/// The kernel's `KERN_PROC_PID` record for `pid`, at least `minimum_len` bytes.
///
/// The record survives for a process that has exited but is not yet reaped.
/// `None` means the kernel has no such record or it is shorter than expected.
#[cfg(target_os = "macos")]
#[must_use]
pub fn kernel_process_record(pid: libc::pid_t, minimum_len: usize) -> Option<Vec<u8>> {
    let mut mib: [libc::c_int; 4] = [libc::CTL_KERN, libc::KERN_PROC, libc::KERN_PROC_PID, pid];
    let mut required: usize = 0;
    // SAFETY: mib names KERN_PROC_PID for one PID, and a null buffer with a
    // writable length requests the record size without writing any bytes.
    let sized = unsafe {
        libc::sysctl(
            mib.as_mut_ptr(),
            mib.len() as u32,
            std::ptr::null_mut(),
            &raw mut required,
            std::ptr::null_mut(),
            0,
        )
    };
    if sized != 0 || required < minimum_len {
        return None;
    }
    let mut record = vec![0_u8; required];
    let mut written = required;
    // SAFETY: record is writable for written bytes, and the kernel reports the
    // byte count it produced through written.
    let read = unsafe {
        libc::sysctl(
            mib.as_mut_ptr(),
            mib.len() as u32,
            record.as_mut_ptr().cast(),
            &raw mut written,
            std::ptr::null_mut(),
            0,
        )
    };
    if read != 0 || written < minimum_len {
        return None;
    }
    record.truncate(written);
    Some(record)
}

/// The path of the executable backing the live or unreaped process `pid`.
///
/// # Errors
///
/// `InvalidInput` when `pid` does not fit a C `int`, otherwise the OS error
/// from `proc_pidpath`.
#[cfg(target_os = "macos")]
pub fn executable_path(pid: u32) -> io::Result<std::path::PathBuf> {
    use std::os::unix::ffi::OsStringExt as _;

    let pid = libc::c_int::try_from(pid)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "PID does not fit c_int"))?;
    let mut bytes = vec![0_u8; libc::PROC_PIDPATHINFO_MAXSIZE as usize];
    // SAFETY: bytes is writable for the length passed to proc_pidpath.
    let count =
        unsafe { libc::proc_pidpath(pid, bytes.as_mut_ptr().cast(), bytes.len() as libc::c_uint) };
    let count = usize::try_from(count)
        .ok()
        .filter(|count| *count > 0 && *count <= bytes.len())
        .ok_or_else(io::Error::last_os_error)?;
    bytes.truncate(count);
    Ok(std::path::PathBuf::from(std::ffi::OsString::from_vec(
        bytes,
    )))
}

/// Every PID the kernel lists in the process group `group`.
///
/// The listing is a filter, not a guarantee: a listed PID may already have left
/// the group, exited, or been reused.
///
/// # Errors
///
/// The OS error from `proc_listpids`, or an error when the process table kept
/// growing across every retry.
#[cfg(target_os = "macos")]
pub fn process_group_pids(group: u32) -> io::Result<Vec<libc::pid_t>> {
    const PROC_PGRP_ONLY: u32 = 2;
    // Query and retry if the process table grows between calls.
    for _ in 0..3 {
        // SAFETY: a null buffer requests the required byte count.
        let required =
            unsafe { libc::proc_listpids(PROC_PGRP_ONLY, group, std::ptr::null_mut(), 0) };
        if required < 0 {
            return Err(io::Error::last_os_error());
        }
        let slots = usize::try_from(required)
            .ok()
            .and_then(|bytes| bytes.checked_div(std::mem::size_of::<libc::pid_t>()))
            .and_then(|count| count.checked_add(16))
            .ok_or_else(|| io::Error::other("process-group member count overflow"))?;
        let mut pids = vec![0 as libc::pid_t; slots];
        let bytes = i32::try_from(pids.len() * std::mem::size_of::<libc::pid_t>())
            .map_err(|_| io::Error::other("process-group buffer does not fit c_int"))?;
        // SAFETY: pids is writable for exactly bytes bytes.
        let written =
            unsafe { libc::proc_listpids(PROC_PGRP_ONLY, group, pids.as_mut_ptr().cast(), bytes) };
        if written < 0 {
            return Err(io::Error::last_os_error());
        }
        if written < bytes {
            let count = usize::try_from(written)
                .ok()
                .and_then(|value| value.checked_div(std::mem::size_of::<libc::pid_t>()))
                .ok_or_else(|| io::Error::other("invalid process-group byte count"))?;
            pids.truncate(count);
            return Ok(pids);
        }
    }
    Err(io::Error::other(
        "process-group membership changed during every inspection",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Stdio;

    fn sleeping_child() -> std::process::Child {
        Command::new("/bin/sleep")
            .arg("30")
            .stdin(Stdio::null())
            .spawn()
            .unwrap()
    }

    fn pid_of(child: &std::process::Child) -> libc::pid_t {
        libc::pid_t::try_from(child.id()).unwrap()
    }

    #[test]
    fn user_ids_are_the_owner_of_files_this_process_creates() {
        use std::os::unix::fs::MetadataExt as _;
        // The effective ID can differ from the real one only under setuid,
        // which a test binary is not.
        assert_eq!(real_user_id(), effective_user_id());
        let root = tempfile::tempdir().unwrap();
        assert_eq!(
            std::fs::metadata(root.path()).unwrap().uid(),
            effective_user_id()
        );
    }

    #[test]
    fn a_live_process_exists_and_a_signalled_one_receives_it() {
        let mut child = sleeping_child();
        let pid = pid_of(&child);
        assert!(exists(pid).unwrap());
        send_signal(pid, libc::SIGKILL).unwrap();
        let status = child.wait().unwrap();
        assert_eq!(
            std::os::unix::process::ExitStatusExt::signal(&status),
            Some(libc::SIGKILL)
        );
    }

    #[test]
    fn a_reaped_process_no_longer_exists_and_cannot_be_signalled() {
        let mut child = sleeping_child();
        let pid = pid_of(&child);
        child.kill().unwrap();
        child.wait().unwrap();
        assert!(!exists(pid).unwrap());
        let error = send_signal(pid, libc::SIGTERM).unwrap_err();
        assert_eq!(error.raw_os_error(), Some(libc::ESRCH));
    }

    #[test]
    fn pid_one_exists_whoever_owns_it() {
        // PID 1 is another user's process, or ours; either way it exists.
        assert!(exists(1).unwrap());
    }

    #[test]
    fn an_empty_group_cannot_be_signalled() {
        let mut child = sleeping_child();
        let pid = pid_of(&child);
        child.kill().unwrap();
        child.wait().unwrap();
        let error = send_signal_to_group(pid, libc::SIGTERM).unwrap_err();
        assert_eq!(error.raw_os_error(), Some(libc::ESRCH));
    }

    #[test]
    fn exited_children_stay_waitable_until_reaped() {
        let mut child = Command::new("/usr/bin/true").spawn().unwrap();
        let pid = pid_of(&child);
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while !has_exited_without_reaping(pid).unwrap() {
            assert!(std::time::Instant::now() < deadline, "child never exited");
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        // Observing it did not reap it, so it can still be observed and waited.
        assert!(has_exited_without_reaping(pid).unwrap());
        child.wait().unwrap();
        let error = has_exited_without_reaping(pid).unwrap_err();
        assert_eq!(error.raw_os_error(), Some(libc::ECHILD));
    }

    #[test]
    fn a_running_child_has_not_exited() {
        let mut child = sleeping_child();
        assert!(!has_exited_without_reaping(pid_of(&child)).unwrap());
        child.kill().unwrap();
        child.wait().unwrap();
    }

    #[test]
    fn a_process_that_is_not_a_child_cannot_be_waited_on() {
        let error = has_exited_without_reaping(1).unwrap_err();
        assert_eq!(error.raw_os_error(), Some(libc::ECHILD));
    }

    #[test]
    fn a_child_started_in_a_new_session_or_group_leads_a_group_named_by_its_pid() {
        for start in [start_new_session, start_new_process_group] {
            let mut command = Command::new("/bin/sleep");
            command.arg("30");
            before_exec(&mut command, start);
            let mut child = command.spawn().unwrap();
            send_signal_to_group(pid_of(&child), 0).unwrap();
            child.kill().unwrap();
            child.wait().unwrap();
        }
    }

    #[test]
    fn a_failing_hook_fails_the_spawn() {
        let mut command = Command::new("/bin/sleep");
        command.arg("30");
        before_exec(&mut command, || {
            Err(io::Error::from_raw_os_error(libc::EACCES))
        });
        let error = command.spawn().unwrap_err();
        assert_eq!(error.raw_os_error(), Some(libc::EACCES));
    }

    #[test]
    fn a_group_leader_cannot_start_a_new_session() {
        // The calling test thread is not a group leader, but a child that first
        // became one cannot setsid.
        let mut command = Command::new("/bin/sleep");
        command.arg("30");
        before_exec(&mut command, || {
            start_new_process_group()?;
            start_new_session()
        });
        let error = command.spawn().unwrap_err();
        assert_eq!(error.raw_os_error(), Some(libc::EPERM));
    }

    #[test]
    fn a_child_cannot_write_past_its_file_size_limit() {
        let root = tempfile::tempdir().unwrap();
        let target = root.path().join("out");
        let mut command = Command::new("/bin/sh");
        command
            .arg("-c")
            .arg("trap '' XFSZ; head -c 4096 /dev/zero > \"$1\"")
            .arg("sh")
            .arg(&target);
        limit_file_size_before_exec(&mut command, 1_024);
        let status = command.status().unwrap();
        assert!(!status.success());
        assert_eq!(std::fs::metadata(&target).unwrap().len(), 1_024);
    }

    #[cfg(target_os = "macos")]
    mod macos {
        use super::*;

        fn current_pid() -> libc::pid_t {
            libc::pid_t::try_from(std::process::id()).unwrap()
        }

        #[test]
        fn the_kernel_record_describes_this_process() {
            let record = kernel_process_record(current_pid(), 44).unwrap();
            // kp_proc.p_pid sits at offset 40.
            let pid = i32::from_ne_bytes(record[40..44].try_into().unwrap());
            assert_eq!(pid, current_pid());
        }

        #[test]
        fn a_record_shorter_than_requested_or_for_no_process_is_absent() {
            assert!(kernel_process_record(current_pid(), usize::MAX).is_none());
            assert!(kernel_process_record(libc::pid_t::MAX, 44).is_none());
        }

        #[test]
        fn the_executable_path_names_the_running_binary() {
            let path = executable_path(std::process::id()).unwrap();
            assert_eq!(
                path.canonicalize().unwrap(),
                std::env::current_exe().unwrap().canonicalize().unwrap()
            );
        }

        #[test]
        fn the_executable_path_of_a_missing_process_is_an_error() {
            assert!(executable_path(i32::MAX as u32).is_err());
            let error = executable_path(u32::MAX).unwrap_err();
            assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
        }

        #[test]
        fn a_group_lists_its_leader_and_an_unused_group_lists_nobody() {
            let mut command = Command::new("/bin/sleep");
            command.arg("30");
            before_exec(&mut command, start_new_process_group);
            let mut child = command.spawn().unwrap();
            let group = child.id();
            let members = process_group_pids(group).unwrap();
            assert_eq!(members, [pid_of(&child)]);
            child.kill().unwrap();
            child.wait().unwrap();
            assert!(process_group_pids(group).unwrap().is_empty());
        }
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn the_clock_tick_rate_is_positive() {
        assert!(clock_ticks_per_second().unwrap() > 0);
    }
}
