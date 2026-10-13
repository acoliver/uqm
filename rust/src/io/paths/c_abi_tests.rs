//! The C entry points against `fixture-spec.md` sections A to G and I:
//! pointers into the caller's buffer, the list layout and links, the
//! `malloc` ownership of everything returned, and the caller-bug aborts.

use std::ffi::{c_char, CStr, CString};
use std::ptr;

use super::c_abi::*;
use super::comp_list::{self, OwnedPathComps, PathComp};
use crate::resource::abort_child::expect_abort;

type JoinRow = (&'static [u8], &'static [u8], &'static [u8], &'static [u8]);

fn cstr(bytes: &[u8]) -> CString {
    CString::new(bytes).unwrap()
}

/// `(start, end)` offsets of the pair of pointers within `base`.
fn offsets(base: *const c_char, start: *const c_char, end: *const c_char) -> (isize, isize) {
    unsafe { (start.offset_from(base), end.offset_from(base)) }
}

/// Takes ownership of a `malloc` string the entry points returned.
fn take(string: *mut c_char) -> Vec<u8> {
    assert!(!string.is_null());
    let bytes = unsafe { CStr::from_ptr(string) }.to_bytes().to_vec();
    super::lifetime_probe::caller_free(string.cast());
    bytes
}

fn names(head: *const PathComp) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    let mut node = head;
    while !node.is_null() {
        unsafe {
            let name = std::slice::from_raw_parts((*node).name.cast::<u8>(), (*node).name_len);
            assert_eq!(*(*node).name.add((*node).name_len), 0, "NUL terminated");
            assert_eq!(
                CStr::from_ptr((*node).name).to_bytes(),
                name,
                "nameLen is strlen"
            );
            out.push(name.to_vec());
            node = (*node).next;
        }
    }
    out
}

#[test]
fn component_walkers_point_into_the_callers_buffer() {
    let path = cstr(b"a//b");
    let dir = path.as_ptr();
    let end = unsafe { dir.add(4) };
    let (mut start, mut stop) = (ptr::null(), ptr::null());
    unsafe { get_first_path_component(dir, end, &mut start, &mut stop) };
    assert_eq!(offsets(dir, start, stop), (0, 1));
    let mut seen = Vec::new();
    for _ in 0..3 {
        unsafe { get_next_path_component(end, &mut start, &mut stop) };
        seen.push(offsets(dir, start, stop));
    }
    assert_eq!(seen, [(2, 2), (3, 4), (4, 4)]);

    unsafe { get_first_path0_component(dir, &mut start, &mut stop) };
    assert_eq!(offsets(dir, start, stop), (0, 1));
    unsafe { get_next_path0_component(&mut start, &mut stop) };
    assert_eq!(offsets(dir, start, stop), (2, 2));
    unsafe { get_next_path0_component(&mut start, &mut stop) };
    assert_eq!(offsets(dir, start, stop), (3, 4));
    unsafe { get_next_path0_component(&mut start, &mut stop) };
    assert_eq!(offsets(dir, start, stop), (4, 4));

    unsafe { get_last_path_component(dir, end, &mut start, &mut stop) };
    assert_eq!(offsets(dir, start, stop), (3, 4));
    unsafe { get_last_path0_component(dir, &mut start, &mut stop) };
    assert_eq!(offsets(dir, start, stop), (3, 4));
    unsafe { get_previous_path_component(dir, &mut start, &mut stop) };
    assert_eq!(offsets(dir, start, stop), (2, 2));
    unsafe { get_previous_path_component(dir, &mut start, &mut stop) };
    assert_eq!(offsets(dir, start, stop), (0, 1));
    unsafe { get_previous_path_component(dir, &mut start, &mut stop) };
    assert_eq!(offsets(dir, start, stop), (0, 0));
}

#[test]
fn a_dir_end_shorter_than_the_string_bounds_the_walk() {
    let path = cstr(b"a/b");
    let dir = path.as_ptr();
    let (mut start, mut stop) = (ptr::null(), ptr::null());
    let end = unsafe { dir.add(2) };
    unsafe { get_first_path_component(dir, end, &mut start, &mut stop) };
    unsafe { get_next_path_component(end, &mut start, &mut stop) };
    assert_eq!(offsets(dir, start, stop), (2, 2));
}

#[test]
fn joins_return_malloc_strings() {
    let rows: [JoinRow; 4] = [
        (b"", b"x", b"x", b"/x"),
        (b"a/", b"b", b"a/b", b"/a/b"),
        (b"/", b"b", b"/b", b"//b"),
        (b"\xff", b"\x80", b"\xff/\x80", b"/\xff/\x80"),
    ];
    for (first, second, joined, absolute) in rows {
        let (first, second) = (cstr(first), cstr(second));
        let (a, b) = unsafe {
            (
                join_paths(first.as_ptr(), second.as_ptr()),
                join_paths_absolute(first.as_ptr(), second.as_ptr()),
            )
        };
        assert_ne!(a.cast_const(), first.as_ptr(), "never aliases an input");
        assert_eq!(take(a), joined);
        assert_eq!(take(b), absolute);
    }
}

#[test]
fn valid_path_name_takes_a_length_and_a_null_for_none() {
    let path = b"a//b";
    unsafe {
        assert!(valid_path_name(path.as_ptr().cast(), 1));
        assert!(!valid_path_name(path.as_ptr().cast(), 3));
        assert!(valid_path_name(ptr::null(), 0));
    }
}

#[test]
fn decompose_links_and_owns_every_block() {
    let path = cstr(b"/a//b/");
    let (mut head, mut absolute) = (ptr::null_mut(), false);
    assert_eq!(
        unsafe { decompose_path(path.as_ptr(), &mut head, &mut absolute) },
        0
    );
    assert!(absolute);
    assert_eq!(names(head), [b"a".to_vec(), b"b".to_vec()]);
    unsafe {
        let second = (*head).next;
        assert!((*head).up.is_null());
        assert_eq!((*second).up, head);
        assert!((*second).next.is_null());
        assert_ne!((*head).name.cast_const(), path.as_ptr(), "names are copies");
        assert_eq!(uio_count_path_comps(head), 2);
        assert_eq!(uio_last_path_comp(head), second);
        uio_path_comp_delete(head);
    }
    // A null isAbsolute is accepted.
    let (mut head, path) = (ptr::null_mut(), cstr(b"x"));
    assert_eq!(
        unsafe { decompose_path(path.as_ptr(), &mut head, ptr::null_mut()) },
        0
    );
    unsafe { uio_path_comp_delete(head) };
}

#[test]
fn decompose_of_nothing_and_of_the_root() {
    let sentinel = ptr::NonNull::<PathComp>::dangling().as_ptr();
    let (mut head, mut absolute) = (sentinel, false);
    let empty = cstr(b"");
    assert_eq!(
        unsafe { decompose_path(empty.as_ptr(), &mut head, &mut absolute) },
        -1
    );
    assert_eq!(
        std::io::Error::last_os_error().raw_os_error(),
        Some(libc::ENOENT)
    );
    assert_eq!((head, absolute), (sentinel, false), "outputs untouched");
    let root = cstr(b"/");
    assert_eq!(
        unsafe { decompose_path(root.as_ptr(), &mut head, &mut absolute) },
        0
    );
    assert!(head.is_null() && absolute);
}

#[test]
fn compose_reports_the_length_it_wrote() {
    for (absolute, expected) in [(true, &b"/a//b"[..]), (false, b"a//b")] {
        let head = OwnedPathComps::new([&b"a"[..], b"", b"b"]).into_raw();
        let (mut path, mut len) = (ptr::null_mut(), 0);
        unsafe { compose_path(head, absolute, &mut path, &mut len) };
        let bytes = take(path);
        assert_eq!((bytes.as_slice(), len), (expected, expected.len()));
        unsafe { uio_path_comp_delete(head) };
    }
}

#[test]
fn make_path_comps_keeps_empty_components_and_links_up() {
    let up = unsafe { uio_path_comp_new(comp_list::alloc_name(b"top"), 3, ptr::null_mut()) };
    let rows: [(&[u8], &[&[u8]]); 5] = [
        (b"", &[]),
        (b"a/b", &[b"a", b"b"]),
        (b"a/", &[b"a"]),
        (b"a//", &[b"a", b""]),
        (b"/a//b", &[b"", b"a", b"", b"b"]),
    ];
    for (path, expected) in rows {
        let path = cstr(path);
        let head = unsafe { uio_make_path_comps(path.as_ptr(), up) };
        let expected: Vec<Vec<u8>> = expected.iter().map(|n| n.to_vec()).collect();
        assert_eq!(names(head), expected);
        if !head.is_null() {
            unsafe {
                assert_eq!((*head).up, up);
                assert!((*up).next.is_null(), "the up node is not linked forward");
                let mut node = (*head).next;
                let mut previous = head;
                while !node.is_null() {
                    assert_eq!((*node).up, previous);
                    previous = node;
                    node = (*node).next;
                }
            }
        }
        unsafe { uio_path_comp_delete(head) };
    }
    unsafe { uio_path_comp_delete(up) };
}

#[test]
fn new_nodes_start_unlinked_and_delete_frees_names_once() {
    unsafe {
        assert_eq!(uio_count_path_comps(ptr::null()), 0);
        assert!(uio_last_path_comp(ptr::null_mut()).is_null());
        uio_path_comp_delete(ptr::null_mut());
        let node = uio_path_comp_new(comp_list::alloc_name(b"n"), 1, ptr::null_mut());
        assert!((*node).next.is_null() && (*node).up.is_null());
        assert_eq!(uio_last_path_comp(node), node);
        // Deleting from the middle of a list leaves the nodes before it alone.
        let list = uio_make_path_comps(c"a/b/c".as_ptr(), ptr::null_mut());
        let second = (*list).next;
        (*list).next = ptr::null_mut();
        uio_path_comp_delete(second);
        assert_eq!(names(list), [b"a".to_vec()]);
        uio_path_comp_delete(list);
        uio_path_comp_delete(node);
    }
}

#[test]
fn owned_list_drops_forward_and_in_reverse_creation_order() {
    let lists: Vec<OwnedPathComps> = (0..8)
        .map(|n| OwnedPathComps::new((0..n).map(|_| &b"x"[..])))
        .collect();
    assert_eq!(
        lists.iter().map(OwnedPathComps::len).collect::<Vec<_>>(),
        (0..8).collect::<Vec<_>>()
    );
    let mut forward = lists;
    forward.drain(..4);
    drop(forward);
    let reverse: Vec<OwnedPathComps> = (0..4)
        .map(|_| OwnedPathComps::new([&b"y"[..], b"z"]))
        .collect();
    for list in reverse.into_iter().rev() {
        assert_eq!(list.names().collect::<Vec<_>>(), [&b"y"[..], b"z"]);
    }
}

#[test]
fn unc_entry_points() {
    let path = cstr(br"\\srv/share//x");
    unsafe {
        assert_eq!(uio_skip_unc_server_share(c"\\\\srv\\share\\x".as_ptr()), 11);
        let (mut name, mut len) = (ptr::null_mut(), 99);
        assert_eq!(
            uio_get_unc_server_share(path.as_ptr(), &mut name, &mut len),
            13
        );
        assert_eq!((len, take(name).as_slice()), (11, &br"\\srv\share"[..]));
        let plain = cstr(b"/a");
        assert_eq!(
            uio_get_unc_server_share(plain.as_ptr(), &mut name, &mut len),
            0
        );
        assert!(name.is_null());
        assert_eq!(len, 0);
        // outLen may be null.
        assert_eq!(
            uio_get_unc_server_share(plain.as_ptr(), &mut name, ptr::null_mut()),
            0
        );
    }
}

#[test]
fn printing_writes_the_name_and_the_up_chain() {
    let list = unsafe { uio_make_path_comps(c"a/b/c".as_ptr(), ptr::null_mut()) };
    unsafe {
        let last = uio_last_path_comp(list);
        let mut file = temp_file();
        let stream = libc::fdopen(
            libc::dup(std::os::fd::AsRawFd::as_raw_fd(&file)),
            c"w+".as_ptr(),
        );
        uio_print_path_to_comp(stream, last);
        uio_print_path_comp(stream, list);
        uio_print_path_to_comp(stream, ptr::null());
        libc::fflush(stream);
        libc::fclose(stream);
        let mut text = String::new();
        std::io::Seek::rewind(&mut file).unwrap();
        std::io::Read::read_to_string(&mut file, &mut text).unwrap();
        assert_eq!(text, "/a/b/ca");
        uio_path_comp_delete(list);
    }
}

fn temp_file() -> std::fs::File {
    let path = std::env::temp_dir().join(format!(
        "paths-print-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .open(&path)
        .unwrap();
    std::fs::remove_file(&path).unwrap();
    file
}

#[test]
fn compose_without_components_aborts() {
    expect_abort(
        module_path!(),
        "compose_without_components_aborts",
        "composePath() FATAL: caller bug: pathComp is null",
        || {
            let (mut path, mut len) = (ptr::null_mut(), 0);
            unsafe { compose_path(ptr::null(), true, &mut path, &mut len) };
        },
    );
}

#[test]
fn a_start_after_the_end_aborts() {
    expect_abort(
        module_path!(),
        "a_start_after_the_end_aborts",
        "getFirstPathComponent() FATAL: caller bug: the path start is after the path end",
        || {
            let path = cstr(b"ab");
            let dir = path.as_ptr();
            let (mut start, mut end) = (ptr::null(), ptr::null());
            unsafe { get_first_path_component(dir.wrapping_add(2), dir, &mut start, &mut end) };
        },
    );
}

#[test]
fn a_null_path_to_join_aborts() {
    expect_abort(
        module_path!(),
        "a_null_path_to_join_aborts",
        "joinPaths() FATAL: caller bug: second is null",
        || {
            unsafe { join_paths(c"a".as_ptr(), ptr::null()) };
        },
    );
}
