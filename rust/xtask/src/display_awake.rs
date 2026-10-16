//! Keeps the Mac's display awake while the native runner drives real windows.
//!
//! The game only holds a `PreventUserIdleDisplaySleep` assertion while it runs,
//! so the display can idle-sleep between scenarios. A display that went to
//! sleep and woke again reports window bounds that are a scaled transform of
//! the real window, which makes the runner's bounds containment check fail
//! every later scenario. The check is correct; the controller therefore holds
//! its own assertion for the whole of `native run` and `native replay`.
//!
//! Two `caffeinate` invocations are used because `-t` is documented as a
//! timeout for the assertions it is given, and `-u` takes a 5 second timeout by
//! default. The short `-u` call turns a display that is already asleep back on
//! and is awaited. The long-lived `-d -i -w <pid>` holder keeps it on and ends
//! on its own if the controller is killed.

use std::io;
use std::path::Path;
use std::process::{Child, Command};

/// The system tool; it ships with macOS.
pub const CAFFEINATE: &str = "/usr/bin/caffeinate";

/// Seconds the user-activity assertion is held; it only has to wake the display.
const WAKE_SECONDS: u32 = 1;

/// A child process that is killed and reaped when the guard is dropped.
#[derive(Debug)]
pub struct ChildGuard {
    child: Child,
}

impl ChildGuard {
    pub fn spawn(mut command: Command) -> io::Result<Self> {
        command.spawn().map(|child| Self { child })
    }

    pub fn id(&self) -> u32 {
        self.child.id()
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        // The child may already have exited; either way it must be reaped.
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Arguments that wake a sleeping display once.
pub fn wake_arguments() -> Vec<String> {
    vec!["-u".into(), "-t".into(), WAKE_SECONDS.to_string()]
}

/// Arguments for the holder that keeps the display and system awake until
/// `owner_pid` exits.
pub fn holder_arguments(owner_pid: u32) -> Vec<String> {
    vec!["-d".into(), "-i".into(), "-w".into(), owner_pid.to_string()]
}

/// The display-wake assertion for one native run. Holds nothing off macOS.
#[derive(Debug)]
pub struct DisplayAwake {
    // Held only so dropping it ends the assertion.
    _holder: Option<ChildGuard>,
}

#[cfg(test)]
impl DisplayAwake {
    fn is_held(&self) -> bool {
        self._holder.is_some()
    }
}

/// Wake the display and hold it awake until the returned guard is dropped.
pub fn hold_display_awake() -> Result<DisplayAwake, String> {
    hold_with(
        cfg!(target_os = "macos"),
        Path::new(CAFFEINATE),
        std::process::id(),
    )
}

fn hold_with(is_macos: bool, caffeinate: &Path, owner_pid: u32) -> Result<DisplayAwake, String> {
    if !is_macos {
        return Ok(DisplayAwake { _holder: None });
    }
    let unavailable = |error: io::Error| {
        format!(
            "cannot keep the display awake: {} failed: {error}. \
             The native runner needs the display on, because a display that has slept reports \
             window bounds the runner rightly refuses",
            caffeinate.display()
        )
    };
    let woke = Command::new(caffeinate)
        .args(wake_arguments())
        .status()
        .map_err(unavailable)?;
    if !woke.success() {
        return Err(format!(
            "cannot keep the display awake: {} {} exited with {woke}",
            caffeinate.display(),
            wake_arguments().join(" ")
        ));
    }
    let mut command = Command::new(caffeinate);
    command.args(holder_arguments(owner_pid));
    let holder = ChildGuard::spawn(command).map_err(unavailable)?;
    eprintln!(
        "display-wake assertion held: {} {} (pid {})",
        caffeinate.display(),
        holder_arguments(owner_pid).join(" "),
        holder.id()
    );
    Ok(DisplayAwake {
        _holder: Some(holder),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_wake_call_declares_user_activity_with_its_own_short_timeout() {
        assert_eq!(wake_arguments(), ["-u", "-t", "1"]);
    }

    #[test]
    fn the_holder_prevents_display_and_idle_sleep_and_follows_the_owner() {
        assert_eq!(holder_arguments(4242), ["-d", "-i", "-w", "4242"]);
    }

    #[test]
    fn nothing_is_spawned_off_macos() {
        // A tool that cannot run would fail the hold if anything were started.
        let held = hold_with(false, Path::new("/nonexistent/caffeinate"), 1).unwrap();
        assert!(!held.is_held());
    }

    #[test]
    fn a_missing_caffeinate_is_a_clear_error_and_not_a_silent_fallback() {
        let error = hold_with(true, Path::new("/nonexistent/caffeinate"), 1).unwrap_err();
        assert!(
            error.contains("cannot keep the display awake")
                && error.contains("/nonexistent/caffeinate"),
            "{error}"
        );
    }

    #[cfg(unix)]
    #[test]
    fn dropping_the_guard_kills_and_reaps_the_child() {
        let mut command = Command::new("/bin/sleep");
        command.arg("30");
        let guard = ChildGuard::spawn(command).unwrap();
        let pid = guard.id() as libc::pid_t;
        // Signal 0 only probes. A live child, or a zombie, still answers it.
        assert_eq!(unsafe { libc::kill(pid, 0) }, 0);
        drop(guard);
        assert_eq!(unsafe { libc::kill(pid, 0) }, -1);
        assert_eq!(io::Error::last_os_error().raw_os_error(), Some(libc::ESRCH));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn the_holder_registers_a_display_sleep_assertion_with_the_system() {
        let mut command = Command::new(CAFFEINATE);
        command.args(holder_arguments(std::process::id()));
        let guard = ChildGuard::spawn(command).unwrap();
        let expected = format!("pid {}(caffeinate)", guard.id());
        // caffeinate registers its assertions shortly after it starts, so poll
        // pmset until they appear instead of assuming a delay.
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
        let listing = loop {
            let output = Command::new("/usr/bin/pmset")
                .args(["-g", "assertions"])
                .output()
                .unwrap();
            let text = String::from_utf8_lossy(&output.stdout).into_owned();
            let held = text.lines().any(|line| {
                line.contains(&expected) && line.contains("PreventUserIdleDisplaySleep")
            });
            if held || std::time::Instant::now() > deadline {
                break text;
            }
        };
        assert!(
            listing.lines().any(
                |line| line.contains(&expected) && line.contains("PreventUserIdleDisplaySleep")
            ),
            "no display-sleep assertion from {expected}:\n{listing}"
        );
        assert!(
            listing.lines().any(|line| line.contains(&expected)
                && line.contains("PreventUserIdleSystemSleep")),
            "no idle-sleep assertion from {expected}:\n{listing}"
        );
    }
}
