//! C-callable entry points for the string bank, one per function declared in
//! `sc2/src/libs/resource/stringbank.h`.
//!
//! These carry no export attribute while the C `stringbank.c.o` provider is
//! still the selected one: exporting the same symbols from both would be a
//! duplicate-provider link. Activation gives each function its C symbol with
//! `#[export_name = "..."]` in the same change that retires the C object; the
//! symbol each one takes over is named in its documentation.
//!
//! The C contract has no error channel, so an argument that C would have
//! turned into undefined behaviour (null pointers, a NUL split character, a
//! result array with no slot) is a caller bug. It is logged and the process
//! aborts; nothing unwinds into C. The only tolerated null is the argument to
//! `StringBank_Free`, which the C function documents by testing `if (bank)`.

use std::ffi::{c_char, c_int, c_uint, CStr};
use std::num::{NonZeroU8, NonZeroUsize};
use std::ptr;

use super::StringBank;
use crate::logging::{log_add, LogLevel};

/// Takes over `StringBank_Create`. Allocation failure aborts, as in every
/// other Rust allocation, so the result is never null.
pub extern "C" fn string_bank_create() -> *mut StringBank {
    Box::into_raw(StringBank::create())
}

/// Takes over `StringBank_Free`.
///
/// # Safety
/// `bank` is null or a pointer from [`string_bank_create`] that has not been
/// freed. Every string handed out by the bank is invalid afterwards.
pub unsafe extern "C" fn string_bank_free(bank: *mut StringBank) {
    if !bank.is_null() {
        // SAFETY: per the contract above this is the unique owner pointer.
        drop(unsafe { Box::from_raw(bank) });
    }
}

/// Takes over `StringBank_AddString`.
///
/// # Safety
/// `bank` is a live bank and `s` a NUL-terminated string.
pub unsafe extern "C" fn string_bank_add_string(
    bank: *mut StringBank,
    s: *const c_char,
) -> *const c_char {
    let (bank, s) = unsafe { bank_and_string("StringBank_AddString", bank, s) };
    string_pointer(bank.add_string(s))
}

/// Takes over `StringBank_AddSubstring`.
///
/// # Safety
/// As [`string_bank_add_string`].
pub unsafe extern "C" fn string_bank_add_substring(
    bank: *mut StringBank,
    s: *const c_char,
    n: c_uint,
) -> *const c_char {
    let (bank, s) = unsafe { bank_and_string("StringBank_AddSubstring", bank, s) };
    string_pointer(bank.add_substring(s, n as usize))
}

/// Takes over `StringBank_AddOrFindString`.
///
/// # Safety
/// As [`string_bank_add_string`].
pub unsafe extern "C" fn string_bank_add_or_find_string(
    bank: *mut StringBank,
    s: *const c_char,
) -> *const c_char {
    let (bank, s) = unsafe { bank_and_string("StringBank_AddOrFindString", bank, s) };
    string_pointer(bank.add_or_find_string(s))
}

/// Takes over `StringBank_AddOrFindSubstring`.
///
/// # Safety
/// As [`string_bank_add_string`].
pub unsafe extern "C" fn string_bank_add_or_find_substring(
    bank: *mut StringBank,
    s: *const c_char,
    n: c_uint,
) -> *const c_char {
    let (bank, s) = unsafe { bank_and_string("StringBank_AddOrFindSubstring", bank, s) };
    string_pointer(bank.add_or_find_substring(s, n as usize))
}

/// Takes over `SplitString`. Writes one pointer (null for a piece too long
/// for a chunk) to each of the first `returned count` slots of `result` and
/// returns that count.
///
/// # Safety
/// `s` is a NUL-terminated string, `result` points to at least `n` writable
/// pointer slots and `bank` is a live bank.
pub unsafe extern "C" fn split_string(
    s: *const c_char,
    splitchar: c_char,
    n: c_int,
    result: *mut *const c_char,
    bank: *mut StringBank,
) -> c_int {
    const FUNCTION: &str = "SplitString";
    let (bank, s) = unsafe { bank_and_string(FUNCTION, bank, s) };
    if result.is_null() {
        contract_violation(FUNCTION, "result array is null");
    }
    let (delimiter, capacity) = match split_arguments(splitchar, n) {
        Ok(arguments) => arguments,
        Err(reason) => contract_violation(FUNCTION, reason),
    };
    let mut count = 0;
    for (slot, piece) in bank.split_string(s, delimiter, capacity).enumerate() {
        // SAFETY: the pieces number at most `capacity` (`n`) and the caller
        // provides `n` slots.
        unsafe { result.add(slot).write(string_pointer(piece)) };
        count += 1;
    }
    count
}

/// The split arguments C leaves undefined: a NUL split character would run
/// past the end of the string, and `n < 1` leaves no slot for the final piece
/// that `SplitString` always writes.
fn split_arguments(splitchar: c_char, n: c_int) -> Result<(NonZeroU8, NonZeroUsize), &'static str> {
    let delimiter = NonZeroU8::new(splitchar.to_ne_bytes()[0]).ok_or("split character is NUL")?;
    let capacity = usize::try_from(n)
        .ok()
        .and_then(NonZeroUsize::new)
        .ok_or("result capacity is below one")?;
    Ok((delimiter, capacity))
}

/// # Safety
/// `bank` is a live bank and `s` a NUL-terminated string, or either is null.
unsafe fn bank_and_string<'a>(
    function: &str,
    bank: *mut StringBank,
    s: *const c_char,
) -> (&'a StringBank, &'a CStr) {
    if bank.is_null() {
        contract_violation(function, "bank is null");
    }
    if s.is_null() {
        contract_violation(function, "string is null");
    }
    // SAFETY: non-null, and live and terminated per the callers' contracts.
    unsafe { (&*bank, CStr::from_ptr(s)) }
}

fn string_pointer(string: Option<&CStr>) -> *const c_char {
    string.map_or(ptr::null(), CStr::as_ptr)
}

fn contract_violation(function: &str, reason: &str) -> ! {
    // SAFETY: `log_add` only requires a valid message.
    unsafe { log_add(LogLevel::Fatal, &format!("{function}() FATAL: {reason}.")) };
    std::process::abort();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    fn text(pointer: *const c_char) -> Vec<u8> {
        assert!(!pointer.is_null());
        unsafe { CStr::from_ptr(pointer) }.to_bytes().to_vec()
    }

    #[test]
    fn create_add_find_free_round_trip() {
        let bank = string_bank_create();
        let abc = CString::new("abc").unwrap();
        unsafe {
            let first = string_bank_add_string(bank, abc.as_ptr());
            let again = string_bank_add_string(bank, abc.as_ptr());
            let found = string_bank_add_or_find_string(bank, abc.as_ptr());
            assert_eq!(text(first), b"abc");
            assert_ne!(first, again);
            assert_eq!(found, first);
            assert_ne!(first, abc.as_ptr(), "the bank keeps its own copy");
            string_bank_free(bank);
        }
    }

    #[test]
    fn free_accepts_null() {
        unsafe { string_bank_free(ptr::null_mut()) };
    }

    #[test]
    fn substring_entry_points_forward_the_byte_count() {
        let bank = string_bank_create();
        let word = CString::new("abcdef").unwrap();
        unsafe {
            let added = string_bank_add_substring(bank, word.as_ptr(), 3);
            let found = string_bank_add_or_find_substring(bank, word.as_ptr(), 3);
            let whole = string_bank_add_substring(bank, word.as_ptr(), c_uint::MAX);
            assert_eq!(text(added), b"abc");
            assert_eq!(found, added);
            assert_eq!(text(whole), b"abcdef");
            string_bank_free(bank);
        }
    }

    #[test]
    fn oversized_strings_return_null() {
        let bank = string_bank_create();
        let long = CString::new(vec![b'x'; super::super::STRBANK_CHUNK_SIZE]).unwrap();
        unsafe {
            assert!(string_bank_add_string(bank, long.as_ptr()).is_null());
            assert!(string_bank_add_or_find_string(bank, long.as_ptr()).is_null());
            assert!(string_bank_add_substring(bank, long.as_ptr(), c_uint::MAX).is_null());
            string_bank_free(bank);
        }
    }

    #[test]
    fn split_string_fills_the_result_slots_and_returns_the_count() {
        let bank = string_bank_create();
        let line = CString::new("one\ntwo\nthree").unwrap();
        let mut slots = [ptr::null::<c_char>(); 5];
        let count =
            unsafe { split_string(line.as_ptr(), b'\n' as c_char, 5, slots.as_mut_ptr(), bank) };
        assert_eq!(count, 3);
        assert_eq!(text(slots[0]), b"one");
        assert_eq!(text(slots[1]), b"two");
        assert_eq!(text(slots[2]), b"three");
        assert!(slots[3].is_null() && slots[4].is_null());
        unsafe { string_bank_free(bank) };
    }

    #[test]
    fn split_string_writes_null_for_a_piece_too_long_for_a_chunk() {
        let bank = string_bank_create();
        let mut raw = vec![b'y'; super::super::STRBANK_CHUNK_SIZE];
        raw.extend_from_slice(b"\nok");
        let line = CString::new(raw).unwrap();
        let mut slots = [ptr::null::<c_char>(); 2];
        let count =
            unsafe { split_string(line.as_ptr(), b'\n' as c_char, 2, slots.as_mut_ptr(), bank) };
        assert_eq!(count, 2);
        assert!(slots[0].is_null());
        assert_eq!(text(slots[1]), b"ok");
        unsafe { string_bank_free(bank) };
    }

    #[test]
    fn split_arguments_reject_what_c_leaves_undefined() {
        assert!(split_arguments(0, 1).is_err());
        assert!(split_arguments(b'\n' as c_char, 0).is_err());
        assert!(split_arguments(b'\n' as c_char, -3).is_err());
        let (delimiter, capacity) = split_arguments(b'\n' as c_char, 100).unwrap();
        assert_eq!((delimiter.get(), capacity.get()), (b'\n', 100));
    }

    #[test]
    fn split_arguments_treat_a_high_split_character_as_its_byte_value() {
        let (delimiter, _) = split_arguments(0xC3u8 as c_char, 2).unwrap();
        assert_eq!(delimiter.get(), 0xC3);
    }
}
