use super::abi::{ChecksumBuffer, ChecksumEntry};
use std::ptr;

/// # Safety
/// `cb` must be writable and not own an existing allocation. `interval` is positive.
#[no_mangle]
pub unsafe extern "C" fn ChecksumBuffer_init(
    cb: *mut ChecksumBuffer,
    delay: usize,
    interval: usize,
) {
    let size = (2 * delay + 2).div_ceil(interval);
    let entries = libc::malloc(size * size_of::<ChecksumEntry>()).cast::<ChecksumEntry>();
    assert!(!entries.is_null());
    (*cb).checksums = entries;
    (*cb).maxSize = size;
    (*cb).interval = interval;
}

/// # Safety
/// `cb` must be initialized; its entries must not be in use.
#[no_mangle]
pub unsafe extern "C" fn ChecksumBuffer_uninit(cb: *mut ChecksumBuffer) {
    libc::free((*cb).checksums.cast());
    (*cb).checksums = ptr::null_mut();
}

unsafe fn entry(cb: *mut ChecksumBuffer, frame: u32) -> *mut ChecksumEntry {
    assert!((frame as usize).is_multiple_of((*cb).interval));
    (*cb)
        .checksums
        .add((frame as usize / (*cb).interval) % (*cb).maxSize)
}

/// # Safety
/// `cb` must be initialized and `frame` must be an interval boundary in its window.
#[no_mangle]
pub unsafe extern "C" fn ChecksumBuffer_addChecksum(
    cb: *mut ChecksumBuffer,
    frame: u32,
    checksum: u32,
) -> bool {
    (*entry(cb, frame)).checksum = checksum;
    true
}

/// # Safety
/// `cb` must contain the checksum for `frame`, and `result` must be writable.
#[no_mangle]
pub unsafe extern "C" fn ChecksumBuffer_getChecksum(
    cb: *mut ChecksumBuffer,
    frame: u32,
    result: *mut u32,
) -> bool {
    *result = (*entry(cb, frame)).checksum;
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interval_rounding_and_cyclic_reuse() {
        unsafe {
            for delay in 0..9 {
                for interval in 1..5 {
                    let mut cb: ChecksumBuffer = std::mem::zeroed();
                    ChecksumBuffer_init(&mut cb, delay, interval);
                    assert_eq!(cb.maxSize, (2 * delay + 2).div_ceil(interval));
                    for frame in (0..100).step_by(interval) {
                        assert!(ChecksumBuffer_addChecksum(
                            &mut cb,
                            frame as u32,
                            frame as u32 ^ 0x1234
                        ));
                        let mut result = 0;
                        assert!(ChecksumBuffer_getChecksum(
                            &mut cb,
                            frame as u32,
                            &mut result
                        ));
                        assert_eq!(result, frame as u32 ^ 0x1234);
                    }
                    ChecksumBuffer_uninit(&mut cb);
                    ChecksumBuffer_uninit(&mut cb);
                    assert!(cb.checksums.is_null());
                }
            }
        }
    }
}
