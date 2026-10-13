//! macOS process activity for the game loop.
//!
//! A process that looks idle to macOS (no frontmost window, little CPU)
//! becomes eligible for App Nap, and the kernel coalesces its timers
//! aggressively: sleeps requested for 13-17 ms were observed waking
//! 72-89 ms later on a hosted runner. A real-time game declares itself to
//! the OS through `-[NSProcessInfo beginActivityWithOptions:reason:]` with
//! the user-interactive option set (`NSActivityUserInitiated |
//! NSActivityLatencyCritical`), which exempts it from both.
//!
//! The Objective-C runtime is called through its C API so no crate is
//! needed. `objc_msgSend` has no fixed prototype, so each call site casts
//! it to the exact signature of the method being sent, as arm64 requires.

use std::ffi::{c_char, c_void};
use std::mem::transmute;
use std::ptr::NonNull;

/// `NSActivityUserInitiated` from Foundation's `NSProcessInfo.h`
/// (`0x00FFFFFF | NSActivityIdleSystemSleepDisabled`, and that bit, 1 << 20,
/// is already inside `0x00FFFFFF`).
pub const NS_ACTIVITY_USER_INITIATED: u64 = 0x00FF_FFFF;

/// `NSActivityLatencyCritical` from `NSProcessInfo.h`.
pub const NS_ACTIVITY_LATENCY_CRITICAL: u64 = 0xFF_0000_0000;

/// The option set the game loop declares: Foundation's
/// `NSActivityUserInteractive`.
pub const GAME_LOOP_OPTIONS: u64 = NS_ACTIVITY_USER_INITIATED | NS_ACTIVITY_LATENCY_CRITICAL;

/// Shown in Activity Monitor and `pmset -g assertions`.
const REASON: &[u8] = b"Ur-Quan Masters game loop\0";

type Id = *mut c_void;
type Sel = *mut c_void;

#[link(name = "objc")]
extern "C" {
    fn objc_getClass(name: *const c_char) -> Id;
    fn sel_registerName(name: *const c_char) -> Sel;
    fn objc_msgSend();
}

#[link(name = "Foundation", kind = "framework")]
extern "C" {}

type MsgSendNoArgs = unsafe extern "C" fn(Id, Sel) -> Id;
type MsgSendCString = unsafe extern "C" fn(Id, Sel, *const c_char) -> Id;
type MsgSendActivity = unsafe extern "C" fn(Id, Sel, u64, Id) -> Id;

/// Why the activity could not be begun.
#[derive(Debug, PartialEq, Eq)]
pub enum ActivityError {
    MissingClass(&'static str),
    MissingSelector(&'static str),
    NilResult(&'static str),
}

impl std::fmt::Display for ActivityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingClass(name) => write!(f, "class {name} not found"),
            Self::MissingSelector(name) => write!(f, "selector {name} not found"),
            Self::NilResult(what) => write!(f, "{what} returned nil"),
        }
    }
}

/// Formats the one-line startup report.
pub fn format_report(result: &Result<NonNull<c_void>, ActivityError>) -> String {
    match result {
        Ok(_) => format!("activity: latency-critical begun options=0x{GAME_LOOP_OPTIONS:x}"),
        Err(err) => format!("activity: unavailable ({err})"),
    }
}

fn class(name: &'static str, c_name: &'static [u8]) -> Result<Id, ActivityError> {
    debug_assert_eq!(c_name.last(), Some(&0));
    // SAFETY: `c_name` is NUL-terminated (asserted above, and all callers
    // pass byte-string literals ending in `\0`).
    let class = unsafe { objc_getClass(c_name.as_ptr().cast()) };
    if class.is_null() {
        Err(ActivityError::MissingClass(name))
    } else {
        Ok(class)
    }
}

fn selector(name: &'static str, c_name: &'static [u8]) -> Result<Sel, ActivityError> {
    debug_assert_eq!(c_name.last(), Some(&0));
    // SAFETY: as in `class`; `sel_registerName` registers unknown names, so
    // a nil result means the runtime itself failed.
    let sel = unsafe { sel_registerName(c_name.as_ptr().cast()) };
    if sel.is_null() {
        Err(ActivityError::MissingSelector(name))
    } else {
        Ok(sel)
    }
}

/// Begins a process-wide latency-critical activity and returns the retained
/// activity token. The token is deliberately never ended: the activity must
/// last as long as the process.
pub fn begin_latency_critical() -> Result<NonNull<c_void>, ActivityError> {
    let ns_process_info = class("NSProcessInfo", b"NSProcessInfo\0")?;
    let ns_string = class("NSString", b"NSString\0")?;
    let process_info_sel = selector("processInfo", b"processInfo\0")?;
    let string_sel = selector("stringWithUTF8String:", b"stringWithUTF8String:\0")?;
    let begin_sel = selector(
        "beginActivityWithOptions:reason:",
        b"beginActivityWithOptions:reason:\0",
    )?;
    let retain_sel = selector("retain", b"retain\0")?;

    // SAFETY: `objc_msgSend` is cast to the exact signature of each method:
    // `+processInfo` (no args), `+stringWithUTF8String:` (one C string),
    // `-beginActivityWithOptions:reason:` (NSActivityOptions = NSUInteger,
    // NSString *) and `-retain` (no args), all returning an object pointer.
    // `REASON` is NUL-terminated and outlives the call, and NSString copies
    // its contents.
    let activity = unsafe {
        let send_no_args: MsgSendNoArgs = transmute(objc_msgSend as unsafe extern "C" fn());
        let send_c_string: MsgSendCString = transmute(objc_msgSend as unsafe extern "C" fn());
        let send_activity: MsgSendActivity = transmute(objc_msgSend as unsafe extern "C" fn());

        let process_info = send_no_args(ns_process_info, process_info_sel);
        if process_info.is_null() {
            return Err(ActivityError::NilResult("+[NSProcessInfo processInfo]"));
        }
        let reason = send_c_string(ns_string, string_sel, REASON.as_ptr().cast());
        if reason.is_null() {
            return Err(ActivityError::NilResult(
                "+[NSString stringWithUTF8String:]",
            ));
        }
        let activity = send_activity(process_info, begin_sel, GAME_LOOP_OPTIONS, reason);
        if activity.is_null() {
            return Err(ActivityError::NilResult(
                "-[NSProcessInfo beginActivityWithOptions:reason:]",
            ));
        }
        // The returned object is autoreleased; this process has no pool on
        // the calling thread, so take our own reference to keep it alive.
        send_no_args(activity, retain_sel)
    };
    NonNull::new(activity).ok_or(ActivityError::NilResult("-[NSObject retain]"))
}

/// Declares the game process latency-critical and logs the outcome once.
/// An unavailable API is reported and the game continues without it.
pub fn declare_game_process_latency_critical() {
    eprintln!("{}", format_report(&begin_latency_critical()));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn options_match_the_foundation_header_values() {
        assert_eq!(NS_ACTIVITY_USER_INITIATED, 0x00FF_FFFF);
        assert_eq!(NS_ACTIVITY_LATENCY_CRITICAL, 0xFF_0000_0000);
        assert_eq!(GAME_LOOP_OPTIONS, 0xFF_00FF_FFFF);
    }

    #[test]
    fn report_names_the_options_when_begun() {
        let token = NonNull::<c_void>::dangling();
        assert_eq!(
            format_report(&Ok(token)),
            "activity: latency-critical begun options=0xff00ffffff"
        );
    }

    #[test]
    fn report_names_the_reason_when_unavailable() {
        assert_eq!(
            format_report(&Err(ActivityError::MissingClass("NSProcessInfo"))),
            "activity: unavailable (class NSProcessInfo not found)"
        );
        assert_eq!(
            format_report(&Err(ActivityError::NilResult("-[NSObject retain]"))),
            "activity: unavailable (-[NSObject retain] returned nil)"
        );
    }

    #[test]
    fn beginning_the_activity_yields_a_token() {
        assert!(begin_latency_critical().is_ok());
    }
}
