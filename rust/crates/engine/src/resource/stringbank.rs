//! Stable, chunk-backed strings for the legacy resource and menu APIs.

use std::ffi::{c_char, c_int, c_uint, CStr};
use std::ptr;

const CHUNK_SIZE: usize = 1024 - size_of::<*mut StringBank>() - size_of::<c_int>();

#[repr(C)]
pub struct StringBank {
    data: [u8; CHUNK_SIZE],
    len: c_int,
    next: *mut StringBank,
}

unsafe fn add_chunk(mut bank: *mut StringBank) -> *mut StringBank {
    let chunk = Box::into_raw(Box::new(StringBank {
        data: [0; CHUNK_SIZE],
        len: 0,
        next: ptr::null_mut(),
    }));
    if !bank.is_null() {
        while !(*bank).next.is_null() {
            bank = (*bank).next;
        }
        (*bank).next = chunk;
    }
    chunk
}

#[no_mangle]
pub extern "C" fn StringBank_Create() -> *mut StringBank {
    unsafe { add_chunk(ptr::null_mut()) }
}

/// # Safety
/// `bank` must be null or a live bank returned by `StringBank_Create`.
#[no_mangle]
pub unsafe extern "C" fn StringBank_Free(mut bank: *mut StringBank) {
    while !bank.is_null() {
        let chunk = Box::from_raw(bank);
        bank = chunk.next;
    }
}

unsafe fn add(bank: *mut StringBank, bytes: &[u8], find: bool) -> *const c_char {
    let len = bytes.len() + 1;
    if len > CHUNK_SIZE {
        return ptr::null();
    }
    let mut chunk = bank;
    if find {
        while !chunk.is_null() {
            let mut i = 0;
            while i < (*chunk).len as usize {
                let entry = (*chunk).data.as_ptr().add(i).cast();
                let string = CStr::from_ptr(entry);
                if string.to_bytes() == bytes {
                    return entry;
                }
                i += string.to_bytes_with_nul().len();
            }
            chunk = (*chunk).next;
        }
    }
    chunk = bank;
    while !chunk.is_null() {
        // The original bank leaves a byte free, even on an exact fit.
        if len < CHUNK_SIZE - (*chunk).len as usize {
            break;
        }
        chunk = (*chunk).next;
    }
    if chunk.is_null() {
        chunk = add_chunk(bank);
    }
    let result = (*chunk).data.as_mut_ptr().add((*chunk).len as usize);
    ptr::copy_nonoverlapping(bytes.as_ptr(), result, bytes.len());
    *result.add(bytes.len()) = 0;
    (*chunk).len += len as c_int;
    result.cast()
}

/// # Safety
/// `bank` is live and `string` is a NUL-terminated string.
#[no_mangle]
pub unsafe extern "C" fn StringBank_AddString(
    bank: *mut StringBank,
    string: *const c_char,
) -> *const c_char {
    add(bank, CStr::from_ptr(string).to_bytes(), false)
}

/// # Safety
/// `bank` is live and `string` is a NUL-terminated string.
#[no_mangle]
pub unsafe extern "C" fn StringBank_AddOrFindString(
    bank: *mut StringBank,
    string: *const c_char,
) -> *const c_char {
    add(bank, CStr::from_ptr(string).to_bytes(), true)
}

/// # Safety
/// `bank` is live and `string` is a NUL-terminated string.
#[no_mangle]
pub unsafe extern "C" fn StringBank_AddSubstring(
    bank: *mut StringBank,
    string: *const c_char,
    n: c_uint,
) -> *const c_char {
    let bytes = CStr::from_ptr(string).to_bytes();
    add(bank, &bytes[..bytes.len().min(n as usize)], false)
}

/// # Safety
/// `bank` is live and `string` is a NUL-terminated string.
#[no_mangle]
pub unsafe extern "C" fn StringBank_AddOrFindSubstring(
    bank: *mut StringBank,
    string: *const c_char,
    n: c_uint,
) -> *const c_char {
    let bytes = CStr::from_ptr(string).to_bytes();
    add(bank, &bytes[..bytes.len().min(n as usize)], true)
}

/// # Safety
/// `s` is NUL-terminated, `bank` is live and `result` has space for at least
/// `max(n, 1)` string pointers, as required by the C API.
#[no_mangle]
pub unsafe extern "C" fn SplitString(
    s: *const c_char,
    splitchar: c_char,
    n: c_int,
    result: *mut *const c_char,
    bank: *mut StringBank,
) -> c_int {
    let mut rest = CStr::from_ptr(s).to_bytes();
    let mut i = 0;
    while i < n - 1 {
        let Some(pos) = rest.iter().position(|&c| c == splitchar as u8) else {
            break;
        };
        *result.add(i as usize) = add(bank, &rest[..pos], true);
        rest = &rest[pos + 1..];
        i += 1;
    }
    *result.add(i as usize) = add(bank, rest, true);
    i + 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duplicates_substrings_and_pointer_stability() {
        unsafe {
            let bank = StringBank_Create();
            let a = StringBank_AddString(bank, c"alpha".as_ptr());
            assert_ne!(a, StringBank_AddString(bank, c"alpha".as_ptr()));
            assert_eq!(a, StringBank_AddOrFindString(bank, c"alpha".as_ptr()));
            assert_eq!(
                a,
                StringBank_AddOrFindSubstring(bank, c"alphabet".as_ptr(), 5)
            );
            assert_eq!(
                CStr::from_ptr(StringBank_AddSubstring(bank, c"x".as_ptr(), 50)),
                c"x"
            );
            for _ in 0..1000 {
                StringBank_AddString(bank, c"fill the bank".as_ptr());
            }
            assert_eq!(CStr::from_ptr(a), c"alpha");
            StringBank_Free(bank);
        }
    }

    #[test]
    fn chunk_boundary_and_oversized_strings() {
        unsafe {
            let bank = StringBank_Create();
            let exact = std::ffi::CString::new(vec![b'x'; CHUNK_SIZE - 1]).unwrap();
            let value = StringBank_AddString(bank, exact.as_ptr());
            assert_eq!((*bank).len, 0);
            assert!(!(*bank).next.is_null());
            assert_eq!(CStr::from_ptr(value), exact.as_c_str());
            let large = std::ffi::CString::new(vec![b'x'; CHUNK_SIZE]).unwrap();
            assert!(StringBank_AddString(bank, large.as_ptr()).is_null());
            assert!(StringBank_AddSubstring(bank, large.as_ptr(), CHUNK_SIZE as u32).is_null());
            StringBank_Free(bank);
        }
    }

    #[test]
    fn splitting_keeps_empty_fields_and_unsplit_tail() {
        unsafe {
            let bank = StringBank_Create();
            let mut result = [ptr::null(); 5];
            let count = SplitString(
                c"a,,a,b,c".as_ptr(),
                b',' as c_char,
                4,
                result.as_mut_ptr(),
                bank,
            );
            assert_eq!(count, 4);
            assert_eq!(result[0], result[2]);
            assert_eq!(CStr::from_ptr(result[1]), c"");
            assert_eq!(CStr::from_ptr(result[3]), c"b,c");
            assert_eq!(
                SplitString(c"a,".as_ptr(), b',' as c_char, 5, result.as_mut_ptr(), bank),
                2
            );
            assert_eq!(CStr::from_ptr(result[1]), c"");
            StringBank_Free(bank);
            StringBank_Free(ptr::null_mut());
        }
    }
}
