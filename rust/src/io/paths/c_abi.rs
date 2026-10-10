//! C-callable entry points, one per function `paths.c` defines for
//! `sc2/src/libs/uio/paths.h`, with the header's exact signatures.
//!
//! None carries an export attribute while the C `paths.c.o` provider is still
//! the selected one: exporting the same symbols from both would be a
//! duplicate-provider link. Activation gives each function its C symbol with
//! `#[export_name = "..."]` in the same change that removes `paths.c` from
//! the build; each function names the symbol it takes over.
//!
//! # Contracts
//!
//! * Strings are raw bytes up to the NUL; no UTF-8 is assumed.
//! * Component pointers (`startComp`/`endComp`) point into the caller's own
//!   buffer. Nothing is copied or freed, and they stay valid exactly as long
//!   as that buffer does.
//! * Everything returned as `char *` or `uio_PathComp *` is from `malloc`
//!   (through `rust_hmalloc`) and is released with the C `uio_free`
//!   (`free`) or `uio_PathComp_delete`.
//! * A null pointer where the C would fault, and the C `assert`s (`dir >
//!   dirEnd`, a component end that is not at a separator, `composePath` of no
//!   components), are caller bugs: logged, then the process aborts.
//!   Nothing unwinds into C. `validPathName` accepts a null `path` with `len`
//!   0, as the C does.
//! * `decomposePath` is the only function that reports an error: -1 with
//!   `errno` set to `ENOENT`, outputs untouched.

use std::ffi::{c_char, c_int, CStr};
use std::ptr;

use super::comp_list::{self, PathComp};
use super::components::{self as comp, Span};
use super::unc;
use crate::io::dirs::c_abi::{contract_violation, set_errno};

/// A non-null pointer argument.
fn required<T>(function: &str, argument: &str, value: *const T) {
    if value.is_null() {
        contract_violation(function, &format!("{argument} is null"));
    }
}

/// The bytes of a NUL-terminated string.
///
/// # Safety
/// `value` is null or a NUL-terminated string that outlives `'a`.
unsafe fn string<'a>(function: &str, argument: &str, value: *const c_char) -> &'a [u8] {
    required(function, argument, value);
    // SAFETY: non-null and NUL-terminated per the contract.
    unsafe { CStr::from_ptr(value) }.to_bytes()
}

/// `[start, end)` as a slice.
///
/// # Safety
/// When `start <= end`, both are in one allocation that outlives `'a`.
unsafe fn between<'a>(function: &str, start: *const c_char, end: *const c_char) -> &'a [u8] {
    if start > end {
        contract_violation(function, "the path start is after the path end");
    }
    if start == end {
        return &[];
    }
    required(function, "dir", start);
    // SAFETY: a non-empty range of one allocation per the contract.
    unsafe { std::slice::from_raw_parts(start.cast(), end.offset_from(start) as usize) }
}

/// Stores `span` of the string at `base` into the two out pointers.
///
/// # Safety
/// The out pointers are writable and `span` lies in the allocation of `base`.
unsafe fn put_span(
    base: *const c_char,
    span: Span,
    start: *mut *const c_char,
    end: *mut *const c_char,
) {
    // SAFETY: per the contract.
    unsafe {
        start.write(base.add(span.start));
        end.write(base.add(span.end));
    }
}

fn out<T>(function: &str, argument: &str, value: *mut T) -> *mut T {
    required(function, argument, value);
    value
}

/// Takes over `getFirstPathComponent`.
///
/// # Safety
/// `dir..dirEnd` is readable memory of one buffer; the out pointers are writable.
pub unsafe extern "C" fn get_first_path_component(
    dir: *const c_char,
    dir_end: *const c_char,
    start_comp: *mut *const c_char,
    end_comp: *mut *const c_char,
) {
    const F: &str = "getFirstPathComponent";
    let (start_comp, end_comp) = (out(F, "startComp", start_comp), out(F, "endComp", end_comp));
    // SAFETY: per the contract.
    unsafe {
        let bytes = between(F, dir, dir_end);
        put_span(dir, comp::first_component(bytes), start_comp, end_comp);
    }
}

/// Takes over `getFirstPath0Component`.
///
/// # Safety
/// `dir` is a NUL-terminated string; the out pointers are writable.
pub unsafe extern "C" fn get_first_path0_component(
    dir: *const c_char,
    start_comp: *mut *const c_char,
    end_comp: *mut *const c_char,
) {
    const F: &str = "getFirstPath0Component";
    let (start_comp, end_comp) = (out(F, "startComp", start_comp), out(F, "endComp", end_comp));
    // SAFETY: per the contract.
    unsafe {
        let bytes = string(F, "dir", dir);
        put_span(dir, comp::first_component(bytes), start_comp, end_comp);
    }
}

/// Takes over `getNextPathComponent`. `*endComp` is where the previous
/// component ended; `[*endComp, dirEnd)` must be readable.
///
/// # Safety
/// `*endComp..dirEnd` is readable memory of one buffer; the out pointers are
/// readable and writable.
pub unsafe extern "C" fn get_next_path_component(
    dir_end: *const c_char,
    start_comp: *mut *const c_char,
    end_comp: *mut *const c_char,
) {
    const F: &str = "getNextPathComponent";
    let (start_comp, end_comp) = (out(F, "startComp", start_comp), out(F, "endComp", end_comp));
    // SAFETY: per the contract.
    unsafe {
        let from = end_comp.read();
        let rest = between(F, from, dir_end);
        put_span(
            from,
            comp::next_component(rest, Span::new(0, 0)),
            start_comp,
            end_comp,
        );
    }
}

/// Takes over `getNextPath0Component`.
///
/// # Safety
/// `*endComp` points into a NUL-terminated string; the out pointers are
/// readable and writable.
pub unsafe extern "C" fn get_next_path0_component(
    start_comp: *mut *const c_char,
    end_comp: *mut *const c_char,
) {
    const F: &str = "getNextPath0Component";
    let (start_comp, end_comp) = (out(F, "startComp", start_comp), out(F, "endComp", end_comp));
    // SAFETY: per the contract.
    unsafe {
        let from = end_comp.read();
        let rest = string(F, "*endComp", from);
        put_span(
            from,
            comp::next_component(rest, Span::new(0, 0)),
            start_comp,
            end_comp,
        );
    }
}

/// Takes over `getLastPathComponent`.
///
/// # Safety
/// `dir..endDir` is readable memory of one buffer; the out pointers are writable.
pub unsafe extern "C" fn get_last_path_component(
    dir: *const c_char,
    end_dir: *const c_char,
    start_comp: *mut *const c_char,
    end_comp: *mut *const c_char,
) {
    const F: &str = "getLastPathComponent";
    let (start_comp, end_comp) = (out(F, "startComp", start_comp), out(F, "endComp", end_comp));
    // SAFETY: per the contract.
    unsafe {
        let bytes = between(F, dir, end_dir);
        put_span(dir, comp::last_component(bytes), start_comp, end_comp);
    }
}

/// Takes over `getLastPath0Component`.
///
/// # Safety
/// `dir` is a NUL-terminated string; the out pointers are writable.
pub unsafe extern "C" fn get_last_path0_component(
    dir: *const c_char,
    start_comp: *mut *const c_char,
    end_comp: *mut *const c_char,
) {
    const F: &str = "getLastPath0Component";
    let (start_comp, end_comp) = (out(F, "startComp", start_comp), out(F, "endComp", end_comp));
    // SAFETY: per the contract.
    unsafe {
        let bytes = string(F, "dir", dir);
        put_span(dir, comp::last_component(bytes), start_comp, end_comp);
    }
}

/// Takes over `getPreviousPathComponent` (and the macro
/// `getPreviousPath0Component`, which is the same function). `*startComp`
/// is where the current component starts; `[dir, *startComp)` is read.
///
/// # Safety
/// `dir..*startComp` is readable memory of one buffer; the out pointers are
/// readable and writable.
pub unsafe extern "C" fn get_previous_path_component(
    dir: *const c_char,
    start_comp: *mut *const c_char,
    end_comp: *mut *const c_char,
) {
    const F: &str = "getPreviousPathComponent";
    let (start_comp, end_comp) = (out(F, "startComp", start_comp), out(F, "endComp", end_comp));
    // SAFETY: per the contract.
    unsafe {
        let bytes = between(F, dir, start_comp.read());
        put_span(
            dir,
            comp::previous_component(bytes, bytes.len()),
            start_comp,
            end_comp,
        );
    }
}

/// Takes over `joinPaths`. The result is a new `malloc` block for the caller
/// to `free`.
///
/// # Safety
/// `first` and `second` are NUL-terminated strings.
pub unsafe extern "C" fn join_paths(first: *const c_char, second: *const c_char) -> *mut c_char {
    // SAFETY: per the contract.
    let joined = unsafe {
        comp::join_paths(
            string("joinPaths", "first", first),
            string("joinPaths", "second", second),
        )
    };
    comp_list::alloc_name(&joined)
}

/// Takes over `joinPathsAbsolute`. The result is a new `malloc` block for the
/// caller to `free`.
///
/// # Safety
/// `first` and `second` are NUL-terminated strings.
pub unsafe extern "C" fn join_paths_absolute(
    first: *const c_char,
    second: *const c_char,
) -> *mut c_char {
    // SAFETY: per the contract.
    let joined = unsafe {
        comp::join_paths_absolute(
            string("joinPathsAbsolute", "first", first),
            string("joinPathsAbsolute", "second", second),
        )
    };
    comp_list::alloc_name(&joined)
}

/// Takes over `validPathName`. `path` may be null only when `len` is 0.
///
/// # Safety
/// `path` points to `len` readable bytes.
pub unsafe extern "C" fn valid_path_name(path: *const c_char, len: usize) -> bool {
    if len == 0 {
        return true;
    }
    required("validPathName", "path", path);
    // SAFETY: `len` readable bytes per the contract.
    comp::valid_path_name(unsafe { std::slice::from_raw_parts(path.cast(), len) })
}

/// Takes over `uio_skipUNCServerShare`.
///
/// # Safety
/// `in_path` is a NUL-terminated string.
pub unsafe extern "C" fn uio_skip_unc_server_share(in_path: *const c_char) -> usize {
    // SAFETY: per the contract.
    unc::skip(unsafe { string("uio_skipUNCServerShare", "inPath", in_path) })
}

/// Takes over `uio_getUNCServerShare`. `*outPath` is a new `malloc` block for
/// the caller to `free`, or null when `inPath` is not a UNC path.
///
/// # Safety
/// `in_path` is a NUL-terminated string; `out_path` is writable; `out_len`
/// is null or writable.
pub unsafe extern "C" fn uio_get_unc_server_share(
    in_path: *const c_char,
    out_path: *mut *mut c_char,
    out_len: *mut usize,
) -> usize {
    const F: &str = "uio_getUNCServerShare";
    let out_path = out(F, "outPath", out_path);
    // SAFETY: per the contract.
    let found = unc::get(unsafe { string(F, "inPath", in_path) });
    let (name, name_len, consumed) = match found {
        Some(found) => (
            comp_list::alloc_name(&found.name),
            found.name.len(),
            found.consumed,
        ),
        None => (ptr::null_mut(), 0, 0),
    };
    // SAFETY: writable per the contract.
    unsafe {
        out_path.write(name);
        if !out_len.is_null() {
            out_len.write(name_len);
        }
    }
    consumed
}

/// Takes over `decomposePath`. Returns 0, or -1 with `errno` `ENOENT` for the
/// empty path. `*pathComp` is a new list for `uio_PathComp_delete`.
/// `isAbsolute` may be null.
///
/// # Safety
/// `path` is a NUL-terminated string; `path_comp` is writable; `is_absolute`
/// is null or writable.
pub unsafe extern "C" fn decompose_path(
    path: *const c_char,
    path_comp: *mut *mut PathComp,
    is_absolute: *mut bool,
) -> c_int {
    const F: &str = "decomposePath";
    let path_comp = out(F, "pathComp", path_comp);
    // SAFETY: per the contract.
    let decomposed = match comp::decompose(unsafe { string(F, "path", path) }) {
        Ok(decomposed) => decomposed,
        Err(comp::EmptyPath) => {
            set_errno(libc::ENOENT);
            return -1;
        }
    };
    // SAFETY: a null `up` has nothing to outlive; outputs writable per the contract.
    unsafe {
        path_comp.write(comp_list::from_names(decomposed.names, ptr::null_mut()));
        if !is_absolute.is_null() {
            is_absolute.write(decomposed.absolute);
        }
    }
    0
}

/// Takes over `composePath`. `*path` is a new `malloc` block for the caller
/// to `free`; `*pathLen` is its string length.
///
/// # Safety
/// `path_comp` is the head of a well-formed list; `path` and `path_len` are
/// writable.
pub unsafe extern "C" fn compose_path(
    path_comp: *const PathComp,
    absolute: bool,
    path: *mut *mut c_char,
    path_len: *mut usize,
) {
    const F: &str = "composePath";
    required(F, "pathComp", path_comp);
    let (path, path_len) = (out(F, "path", path), out(F, "pathLen", path_len));
    let mut names = Vec::new();
    let mut node = path_comp;
    while !node.is_null() {
        // SAFETY: live nodes with `name_len` readable name bytes per the contract.
        unsafe {
            names.push(std::slice::from_raw_parts(
                (*node).name.cast(),
                (*node).name_len,
            ));
            node = (*node).next;
        }
    }
    let composed = comp::compose(&names, absolute);
    // SAFETY: writable per the contract.
    unsafe {
        path.write(comp_list::alloc_name(&composed));
        path_len.write(composed.len());
    }
}

/// Takes over `uio_PathComp_new`. The node owns `name`.
///
/// # Safety
/// `name` is a `malloc` block holding a NUL-terminated string of `name_len`
/// bytes that nothing else frees; `up_comp` is null or a live node.
pub unsafe extern "C" fn uio_path_comp_new(
    name: *mut c_char,
    name_len: usize,
    up_comp: *mut PathComp,
) -> *mut PathComp {
    // SAFETY: per the contract.
    unsafe { comp_list::new(name, name_len, up_comp) }
}

/// Takes over `uio_PathComp_delete`.
///
/// # Safety
/// `path_comp` is null or the head of a list of `malloc` nodes and names; none
/// is used afterwards.
pub unsafe extern "C" fn uio_path_comp_delete(path_comp: *mut PathComp) {
    // SAFETY: per the contract.
    unsafe { comp_list::delete(path_comp) }
}

/// Takes over `uio_countPathComps`.
///
/// # Safety
/// `comp` is null or the head of a well-formed list.
pub unsafe extern "C" fn uio_count_path_comps(comp: *const PathComp) -> c_int {
    // SAFETY: per the contract.
    let count = unsafe { comp_list::count(comp) };
    c_int::try_from(count).expect("a path has fewer than i32::MAX components")
}

/// Takes over `uio_lastPathComp`.
///
/// # Safety
/// `comp` is null or the head of a well-formed list.
pub unsafe extern "C" fn uio_last_path_comp(comp: *mut PathComp) -> *mut PathComp {
    // SAFETY: per the contract.
    unsafe { comp_list::last(comp) }
}

/// Takes over `uio_makePathComps`. Unlike `decomposePath` it keeps empty
/// components. The first node's `up` is `up_comp`, which the new list does
/// not own and does not link back to.
///
/// # Safety
/// `path` is a NUL-terminated string; `up_comp` is null or a live node that
/// outlives the list.
pub unsafe extern "C" fn uio_make_path_comps(
    path: *const c_char,
    up_comp: *mut PathComp,
) -> *mut PathComp {
    // SAFETY: per the contract.
    unsafe {
        comp_list::from_names(
            comp::components(string("uio_makePathComps", "path", path)),
            up_comp,
        )
    }
}

/// Takes over `uio_printPathComp`: `fprintf(outStream, "%s", comp->name)`.
///
/// # Safety
/// `out_stream` is an open `FILE *`; `comp` is a live node.
pub unsafe extern "C" fn uio_print_path_comp(out_stream: *mut libc::FILE, comp: *const PathComp) {
    required("uio_printPathComp", "comp", comp);
    // SAFETY: per the contract; the name is NUL terminated. A failed write is
    // ignored, as the C does.
    unsafe {
        libc::fprintf(out_stream, c"%s".as_ptr(), (*comp).name);
    }
}

/// Takes over `uio_printPathToComp`: `/a/b/c` for the chain a, b, c reached
/// through `up`; nothing for null.
///
/// # Safety
/// `out_stream` is an open `FILE *`; `comp` is null or a live node whose `up`
/// chain is well formed.
pub unsafe extern "C" fn uio_print_path_to_comp(
    out_stream: *mut libc::FILE,
    comp: *const PathComp,
) {
    let mut chain = Vec::new();
    let mut node = comp;
    while !node.is_null() {
        chain.push(node);
        // SAFETY: live nodes per the contract.
        node = unsafe { (*node).up };
    }
    for node in chain.into_iter().rev() {
        // SAFETY: per the contract.
        unsafe {
            libc::fprintf(out_stream, c"/".as_ptr());
            uio_print_path_comp(out_stream, node);
        }
    }
}
