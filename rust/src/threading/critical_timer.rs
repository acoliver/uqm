//! One-shot critical timers on a kqueue, for macOS waits.
//!
//! `std::thread::sleep` and a plain `kevent` timeout are subject to the
//! kernel's timer coalescing. On GitHub's hosted macOS runners that rounds
//! waits up to roughly 16 ms steps: 2/10/20/40 ms requests woke with median
//! overshoots of 14-16/40-44/75-90/68-85 ms, even at user-interactive QoS
//! and with a second thread spinning. An `EVFILT_TIMER` armed with
//! `NOTE_CRITICAL` asks the kernel for minimal coalescing; the same requests
//! overshot by a median of 0.03-0.05 ms.
//!
//! A timer is registered and waited on in one `kevent` call that blocks
//! with no timeout. The timer has a fixed ident, and registering an ident
//! that is already armed replaces it, so a timer from an earlier wait is
//! re-armed rather than doubled. A timer that fired while nobody was waiting
//! can at worst wake a later wait early, which every caller already treats
//! as "look again".

use std::os::fd::{AsFd as _, AsRawFd as _, BorrowedFd, FromRawFd as _, OwnedFd};
use std::time::Duration;

/// Ident of the timer on [`CriticalTimer`]'s own queue.
const TIMER_IDENT: libc::uintptr_t = 1;

/// A kqueue dedicated to timed waits. Waits take `&self`; one thread waits on
/// a timer at a time, because every wait re-arms the same ident.
#[derive(Debug)]
pub struct CriticalTimer {
    queue: OwnedFd,
}

impl CriticalTimer {
    pub fn new() -> std::io::Result<Self> {
        Ok(Self {
            queue: create_queue()?,
        })
    }

    /// Block for `duration`. An interrupted wait returns early and counts as
    /// a wake.
    pub fn sleep(&self, duration: Duration) -> std::io::Result<()> {
        if duration.is_zero() {
            return Ok(());
        }
        wait_for_event(self.queue.as_fd(), &[timer_change(TIMER_IDENT, duration)]).map(|_| ())
    }
}

/// Create a kqueue that no exec'd child inherits.
///
/// Darwin creates kqueue descriptors without close-on-exec and sets it in a
/// second call, so no child may be forked in between.
pub(crate) fn create_queue() -> std::io::Result<OwnedFd> {
    let _guard = crate::automation::child_session::guard_descriptor_inheritance();
    // SAFETY: kqueue takes no arguments.
    let raw_queue = unsafe { libc::kqueue() };
    if raw_queue < 0 {
        return Err(std::io::Error::last_os_error());
    }
    // SAFETY: kqueue returned a new descriptor that nothing else owns.
    let queue = unsafe { OwnedFd::from_raw_fd(raw_queue) };
    // SAFETY: the queue descriptor is open.
    if unsafe { libc::fcntl(queue.as_raw_fd(), libc::F_SETFD, libc::FD_CLOEXEC) } != 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(queue)
}

/// The change record that arms a one-shot critical timer. The kernel's
/// timer data is in microseconds; a sub-microsecond remainder rounds up so
/// the wait is never shorter than requested.
pub(crate) fn timer_change(ident: libc::uintptr_t, duration: Duration) -> libc::kevent {
    let micros = isize::try_from(duration.as_nanos().div_ceil(1000)).unwrap_or(isize::MAX);
    libc::kevent {
        ident,
        filter: libc::EVFILT_TIMER,
        flags: libc::EV_ADD | libc::EV_ONESHOT,
        fflags: libc::NOTE_CRITICAL | libc::NOTE_USECONDS,
        data: micros,
        udata: std::ptr::null_mut(),
    }
}

/// Apply `changes` to `queue` and block, with no timeout, until one event is
/// ready. Returns its filter, or no filter after an interrupted wait.
pub(crate) fn wait_for_event(
    queue: BorrowedFd<'_>,
    changes: &[libc::kevent],
) -> std::io::Result<Option<i16>> {
    let mut event = libc::kevent {
        ident: 0,
        filter: 0,
        flags: 0,
        fflags: 0,
        data: 0,
        udata: std::ptr::null_mut(),
    };
    // SAFETY: `changes` is a valid slice of change records, there is room
    // for exactly one event, and a null timeout blocks until one arrives.
    let ready = unsafe {
        libc::kevent(
            queue.as_raw_fd(),
            changes.as_ptr(),
            changes.len() as libc::c_int,
            &mut event,
            1,
            std::ptr::null(),
        )
    };
    match ready {
        1 if event.flags & libc::EV_ERROR != 0 => {
            Err(std::io::Error::from_raw_os_error(event.data as i32))
        }
        1 => Ok(Some(event.filter)),
        0 => Ok(None),
        _ => {
            let error = std::io::Error::last_os_error();
            if error.kind() == std::io::ErrorKind::Interrupted {
                Ok(None)
            } else {
                Err(error)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn constants_match_the_sdk_header() {
        // <sys/event.h>
        assert_eq!(libc::EVFILT_TIMER, -7);
        assert_eq!(libc::EV_ONESHOT, 0x0010);
        assert_eq!(libc::NOTE_USECONDS, 0x0000_0002);
        assert_eq!(libc::NOTE_CRITICAL, 0x0000_0020);
    }

    #[test]
    fn timer_change_requests_a_critical_one_shot_in_microseconds() {
        let change = timer_change(7, Duration::from_millis(20));
        // `kevent` is packed on Darwin, so fields are copied out before use.
        let (ident, filter, flags, fflags, data) = (
            change.ident,
            change.filter,
            change.flags,
            change.fflags,
            change.data,
        );
        assert_eq!(ident, 7);
        assert_eq!(filter, libc::EVFILT_TIMER);
        assert_eq!(flags, libc::EV_ADD | libc::EV_ONESHOT);
        assert_eq!(fflags, libc::NOTE_CRITICAL | libc::NOTE_USECONDS);
        assert_eq!(data, 20_000);
    }

    #[test]
    fn timer_change_rounds_a_partial_microsecond_up() {
        let rounded = timer_change(1, Duration::from_nanos(1_001)).data;
        let clamped = timer_change(1, Duration::MAX).data;
        assert_eq!(rounded, 2);
        assert_eq!(clamped, isize::MAX);
    }

    #[test]
    fn sleep_waits_at_least_the_duration() {
        let timer = CriticalTimer::new().unwrap();
        let started = Instant::now();
        timer.sleep(Duration::from_millis(20)).unwrap();
        let elapsed = started.elapsed();
        assert!(elapsed >= Duration::from_millis(20), "{elapsed:?}");
        assert!(elapsed < Duration::from_secs(2), "{elapsed:?}");
    }

    #[test]
    fn zero_duration_returns_immediately() {
        let timer = CriticalTimer::new().unwrap();
        let started = Instant::now();
        timer.sleep(Duration::ZERO).unwrap();
        assert!(started.elapsed() < Duration::from_millis(10));
    }

    #[test]
    fn repeated_sleeps_on_one_queue_each_honour_their_duration() {
        let timer = CriticalTimer::new().unwrap();
        for _ in 0..3 {
            let started = Instant::now();
            timer.sleep(Duration::from_millis(15)).unwrap();
            assert!(started.elapsed() >= Duration::from_millis(15));
        }
    }

    #[test]
    fn the_queue_does_not_survive_an_exec() {
        let timer = CriticalTimer::new().unwrap();
        // SAFETY: the queue descriptor is open for the life of `timer`.
        let flags = unsafe { libc::fcntl(timer.queue.as_raw_fd(), libc::F_GETFD) };
        assert!(flags >= 0);
        assert_ne!(flags & libc::FD_CLOEXEC, 0);
    }
}
