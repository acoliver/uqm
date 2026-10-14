//! macOS quality-of-service class for the game thread.
//!
//! The frame loop sleeps for tens of milliseconds at a time and expects to
//! wake close to its deadline. At the default QoS class macOS may coalesce
//! those timers and deprioritise the thread, which shows up as sleeps
//! waking many milliseconds late. An interactive game's frame loop belongs
//! at user-interactive QoS however the process was launched.

use std::os::raw::c_int;

/// `QOS_CLASS_USER_INTERACTIVE` from `<sys/qos.h>`.
pub const QOS_CLASS_USER_INTERACTIVE: u32 = 0x21;

// `qos_class_t` is an unsigned int. It is declared as `u32` here rather
// than as the libc enum because the OS may report classes the enum lacks.
extern "C" {
    fn pthread_set_qos_class_self_np(qos_class: u32, relative_priority: c_int) -> c_int;
    fn pthread_get_qos_class_np(
        thread: libc::pthread_t,
        qos_class: *mut u32,
        relative_priority: *mut c_int,
    ) -> c_int;
}

/// Reads the QoS class of the calling thread, or the error code.
pub fn current_class() -> Result<u32, c_int> {
    let mut class: u32 = 0;
    let mut relative_priority: c_int = 0;
    // SAFETY: `pthread_self` is the calling thread and both out pointers
    // refer to live locals.
    let rc = unsafe {
        pthread_get_qos_class_np(libc::pthread_self(), &mut class, &mut relative_priority)
    };
    if rc == 0 {
        Ok(class)
    } else {
        Err(rc)
    }
}

/// Sets the calling thread to user-interactive QoS. Returns the call's
/// result code (0 on success).
pub fn set_user_interactive() -> c_int {
    // SAFETY: plain libSystem call that only affects the calling thread.
    unsafe { pthread_set_qos_class_self_np(QOS_CLASS_USER_INTERACTIVE, 0) }
}

fn describe_class(class: Result<u32, c_int>) -> String {
    match class {
        Ok(class) => format!("0x{class:02x}"),
        Err(rc) => format!("unavailable(rc={rc})"),
    }
}

/// Formats the one-line startup report.
pub fn format_report(
    before: Result<u32, c_int>,
    set_rc: c_int,
    after: Result<u32, c_int>,
) -> String {
    format!(
        "qos: before={} set_rc={} after={}",
        describe_class(before),
        set_rc,
        describe_class(after)
    )
}

/// Declares the calling thread latency-sensitive and logs the outcome once.
/// A failure is reported and the game continues at the inherited class.
pub fn declare_game_thread_interactive() {
    let before = current_class();
    let set_rc = set_user_interactive();
    let after = current_class();
    eprintln!("{}", format_report(before, set_rc, after));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_names_classes_and_return_code() {
        assert_eq!(
            format_report(Ok(0x15), 0, Ok(0x21)),
            "qos: before=0x15 set_rc=0 after=0x21"
        );
    }

    #[test]
    fn report_names_failures() {
        assert_eq!(
            format_report(Err(3), 1, Err(3)),
            "qos: before=unavailable(rc=3) set_rc=1 after=unavailable(rc=3)"
        );
    }

    #[test]
    fn the_calling_thread_reads_back_user_interactive() {
        // A fresh thread so the test does not alter the harness's threads.
        let observed = std::thread::spawn(|| {
            let rc = set_user_interactive();
            (rc, current_class())
        })
        .join()
        .expect("QoS probe thread panicked");

        assert_eq!(observed, (0, Ok(QOS_CLASS_USER_INTERACTIVE)));
    }
}
