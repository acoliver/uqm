//! macOS-only OS services: the window list and kqueue registration.

use std::ffi::c_void;
use std::io;
use std::os::fd::{AsRawFd as _, BorrowedFd};

pub mod sandbox;

type CfArrayRef = *const c_void;
type CfDictionaryRef = *const c_void;
type CfTypeRef = *const c_void;

#[repr(C)]
#[derive(Clone, Copy)]
struct CGPoint {
    x: f64,
    y: f64,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct CGSize {
    width: f64,
    height: f64,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct CGRect {
    origin: CGPoint,
    size: CGSize,
}

#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    fn CFArrayGetCount(array: CfArrayRef) -> isize;
    fn CFArrayGetValueAtIndex(array: CfArrayRef, index: isize) -> *const c_void;
    fn CFDictionaryGetValue(dictionary: CfDictionaryRef, key: *const c_void) -> *const c_void;
    fn CFNumberGetValue(number: CfTypeRef, number_type: i32, value: *mut c_void) -> u8;
    fn CFBooleanGetValue(boolean: CfTypeRef) -> u8;
    fn CFRelease(value: CfTypeRef);
}

#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    static kCGWindowOwnerPID: *const c_void;
    static kCGWindowNumber: *const c_void;
    static kCGWindowBounds: *const c_void;
    static kCGWindowIsOnscreen: *const c_void;
    static kCGWindowLayer: *const c_void;
    fn CGWindowListCopyWindowInfo(option: u32, relative_to_window: u32) -> CfArrayRef;
    fn CGRectMakeWithDictionaryRepresentation(
        dictionary: CfDictionaryRef,
        bounds: *mut CGRect,
    ) -> u8;
}

const CF_NUMBER_I64: i32 = 4;

/// One window of a process as the window server reports it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Window {
    pub id: u64,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub on_screen: bool,
}

/// The layer-0 windows owned by `pid` that have a positive ID and a bounds
/// rectangle of at least one point in each direction.
///
/// # Errors
///
/// Returns an error when the window server returns no list.
pub fn observe(pid: u32) -> io::Result<Vec<Window>> {
    // SAFETY: CoreGraphics returns an owned immutable CFArray, or null.
    let windows = unsafe { CGWindowListCopyWindowInfo(0, 0) };
    if windows.is_null() {
        return Err(io::Error::other("CGWindowListCopyWindowInfo returned null"));
    }
    let mut matches = Vec::new();
    // SAFETY: windows is a live CFArray.
    let count = unsafe { CFArrayGetCount(windows) };
    for index in 0..count {
        // SAFETY: index is in range; the dictionary is borrowed from windows and
        // stays valid until the matching CFRelease below.
        let dictionary = unsafe { CFArrayGetValueAtIndex(windows, index) };
        if let Some(window) = window_of_process(dictionary, pid) {
            matches.push(window);
        }
    }
    // SAFETY: windows is owned by this call and released exactly once.
    unsafe { CFRelease(windows) };
    Ok(matches)
}

fn window_of_process(dictionary: CfDictionaryRef, pid: u32) -> Option<Window> {
    // SAFETY: dictionary is a live CGWindow dictionary and the keys are the
    // CFString constants CoreGraphics exports.
    let (owner, layer, id) = unsafe {
        (
            dictionary_i64(dictionary, kCGWindowOwnerPID)?,
            dictionary_i64(dictionary, kCGWindowLayer)?,
            dictionary_i64(dictionary, kCGWindowNumber)?,
        )
    };
    if owner != i64::from(pid) || layer != 0 {
        return None;
    }
    let mut bounds = CGRect {
        origin: CGPoint { x: 0.0, y: 0.0 },
        size: CGSize {
            width: 0.0,
            height: 0.0,
        },
    };
    // SAFETY: the dictionary is live, the key is exported by CoreGraphics and
    // bounds is writable CGRect storage.
    let (bounds_value, on_screen) = unsafe {
        let bounds_value = CFDictionaryGetValue(dictionary, kCGWindowBounds);
        let on_screen = CFDictionaryGetValue(dictionary, kCGWindowIsOnscreen);
        let valid = !bounds_value.is_null()
            && CGRectMakeWithDictionaryRepresentation(bounds_value, &raw mut bounds) != 0;
        (
            valid,
            !on_screen.is_null() && CFBooleanGetValue(on_screen) != 0,
        )
    };
    if !bounds_value || id <= 0 || bounds.size.width < 1.0 || bounds.size.height < 1.0 {
        return None;
    }
    Some(Window {
        id: id as u64,
        x: bounds.origin.x,
        y: bounds.origin.y,
        width: bounds.size.width,
        height: bounds.size.height,
        on_screen,
    })
}

/// # Safety
///
/// `dictionary` must be a live CGWindow dictionary and `key` one of its
/// exported CFString keys.
unsafe fn dictionary_i64(dictionary: CfDictionaryRef, key: *const c_void) -> Option<i64> {
    // SAFETY: guaranteed by the caller.
    let value = unsafe { CFDictionaryGetValue(dictionary, key) };
    if value.is_null() {
        return None;
    }
    let mut number = 0_i64;
    // SAFETY: number is correctly aligned and sized for kCFNumberSInt64Type.
    (unsafe { CFNumberGetValue(value, CF_NUMBER_I64, (&raw mut number).cast()) } != 0)
        .then_some(number)
}

/// Apply `changes` to the kqueue `queue` without waiting for any event.
///
/// # Errors
///
/// Returns the OS error from `kevent`, for example `EBADF`.
pub fn register_events(queue: BorrowedFd<'_>, changes: &[libc::kevent]) -> io::Result<()> {
    let count = libc::c_int::try_from(changes.len())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "too many kevent changes"))?;
    // SAFETY: changes is a valid slice of change records, no event list is
    // requested and no timeout is used.
    let registered = unsafe {
        libc::kevent(
            queue.as_raw_fd(),
            changes.as_ptr(),
            count,
            std::ptr::null_mut(),
            0,
            std::ptr::null(),
        )
    };
    if registered != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::fd::AsFd as _;

    #[test]
    fn a_process_without_windows_lists_none() {
        // A host without a window server answers with an error instead of a list.
        match observe(u32::MAX) {
            Ok(windows) => assert!(windows.is_empty()),
            Err(error) => assert!(error.to_string().contains("CGWindowListCopyWindowInfo")),
        }
    }

    #[test]
    fn registering_changes_on_something_that_is_not_a_queue_fails() {
        let (reader, _writer) = std::io::pipe().unwrap();
        let change = libc::kevent {
            ident: 0,
            filter: libc::EVFILT_VNODE,
            flags: libc::EV_ADD,
            fflags: libc::NOTE_WRITE,
            data: 0,
            udata: std::ptr::null_mut(),
        };
        let error = register_events(reader.as_fd(), &[change]).unwrap_err();
        assert!(matches!(
            error.raw_os_error(),
            Some(libc::EBADF) | Some(libc::ENOTTY) | Some(libc::EINVAL)
        ));
    }

    #[test]
    fn a_directory_watch_registers_and_fires_on_a_write() {
        use crate::threading::critical_timer::{create_queue, timer_change, wait_for_event};
        let root = tempfile::tempdir().unwrap();
        let directory = std::fs::File::open(root.path()).unwrap();
        let queue = create_queue().unwrap();
        let watch = libc::kevent {
            ident: directory.as_raw_fd() as libc::uintptr_t,
            filter: libc::EVFILT_VNODE,
            flags: libc::EV_ADD | libc::EV_ENABLE | libc::EV_CLEAR,
            fflags: libc::NOTE_WRITE | libc::NOTE_EXTEND,
            data: 0,
            udata: std::ptr::null_mut(),
        };
        register_events(queue.as_fd(), &[watch]).unwrap();
        std::fs::write(root.path().join("created"), b"x").unwrap();
        let event = wait_for_event(
            queue.as_fd(),
            &[timer_change(7, std::time::Duration::from_secs(5))],
        )
        .unwrap();
        assert_eq!(event, Some(libc::EVFILT_VNODE));
    }
}
