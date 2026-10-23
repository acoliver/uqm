//! Platform string helper from port.c. Other portability functions are supplied
//! by the supported platforms' C runtime rather than first-party adapters.

use std::ffi::c_char;

/// # Safety
/// `text` must point to a writable NUL-terminated string.
#[no_mangle]
pub unsafe extern "C" fn strupr(text: *mut c_char) -> *mut c_char {
    let mut p = text;
    while *p != 0 {
        *p = libc::toupper((*p as u8).into()) as c_char;
        p = p.add(1);
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn uppercases_in_place_and_returns_original_pointer() {
        let mut input = *b"Abc09-z\0";
        let p = input.as_mut_ptr().cast();
        assert_eq!(unsafe { strupr(p) }, p);
        assert_eq!(&input, b"ABC09-Z\0");
        let mut empty = [0u8];
        unsafe {
            strupr(empty.as_mut_ptr().cast());
        }
        assert_eq!(empty, [0]);
    }
}
