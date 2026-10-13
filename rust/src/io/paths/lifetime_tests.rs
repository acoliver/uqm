//! Who owns each block `io::paths` allocates (`fixture-spec.md` of
//! `path-lifetime`, L1 to L17): every node, name and returned string is a live
//! block until exactly one release, and nothing the caller lent is released.
//! The ledger is `lifetime_probe`, fed by the real `alloc_block`/`free_block`
//! the production functions use.

use std::ffi::{c_char, CStr, CString};
use std::ptr;

use super::c_abi::*;
use super::comp_list::{self, OwnedPathComps, PathComp};
use super::components;
use super::lifetime_probe::{caller_free, Probe};

const NODE_SIZE: usize = std::mem::size_of::<PathComp>();

/// A node and its name block, read while the list is live.
#[derive(Clone, Copy)]
struct Blocks {
    node: *const PathComp,
    name: *const c_char,
    name_len: usize,
}

fn chain(head: *const PathComp) -> Vec<Blocks> {
    let mut out = Vec::new();
    let mut node = head;
    while !node.is_null() {
        // SAFETY: the caller holds a live list.
        unsafe {
            out.push(Blocks {
                node,
                name: (*node).name,
                name_len: (*node).name_len,
            });
            node = (*node).next;
        }
    }
    out
}

/// Every node and name block is live, with its own size.
fn assert_live(probe: &Probe, blocks: &[Blocks]) {
    for b in blocks {
        assert_eq!(probe.size_of_live(b.node), Some(NODE_SIZE), "node block");
        assert_eq!(
            probe.size_of_live(b.name),
            Some(b.name_len + 1),
            "name block"
        );
    }
}

/// Every node and name block was released exactly once and is no longer live.
fn assert_released_once(probe: &Probe, blocks: &[Blocks]) {
    for b in blocks {
        assert_eq!(probe.releases_of(b.node), 1, "node released once");
        assert_eq!(probe.releases_of(b.name), 1, "name released once");
        assert!(!probe.is_live(b.node) && !probe.is_live(b.name));
    }
}

fn cstr(bytes: &[u8]) -> CString {
    CString::new(bytes).unwrap()
}

#[test]
fn owned_lists_hold_two_blocks_per_name_and_drop_releases_each_once() {
    let name_bytes: Vec<Vec<u8>> = (0..1000).map(|n| vec![b'a'; n % 7]).collect();
    for count in (0..=40).chain([1000]) {
        let probe = Probe::start();
        let list = OwnedPathComps::new(name_bytes[..count].iter().map(Vec::as_slice));
        let blocks = chain(list.as_ptr());
        assert_eq!(blocks.len(), count);
        assert_eq!(probe.live_blocks(), 2 * count);
        assert_eq!(probe.allocations(), 2 * count);
        assert_live(&probe, &blocks);
        assert_eq!(
            probe.live_bytes(),
            blocks
                .iter()
                .map(|b| NODE_SIZE + b.name_len + 1)
                .sum::<usize>()
        );
        drop(list);
        assert_released_once(&probe, &blocks);
        assert_eq!(probe.attempted_releases(), 2 * count);
        probe.assert_balanced();
    }
}

#[test]
fn a_new_node_adopts_its_name_and_delete_releases_both() {
    let probe = Probe::start();
    let name = comp_list::alloc_name(b"n");
    assert_eq!(probe.live_blocks(), 1);
    let node = unsafe { uio_path_comp_new(name, 1, ptr::null_mut()) };
    assert_eq!(
        probe.live_blocks(),
        2,
        "the node allocates itself, not a name copy"
    );
    assert_eq!(unsafe { (*node).name }, name);
    let blocks = chain(node);
    assert_live(&probe, &blocks);
    unsafe { uio_path_comp_delete(ptr::null_mut()) };
    assert_eq!(
        probe.attempted_releases(),
        0,
        "deleting nothing releases nothing"
    );
    unsafe { uio_path_comp_delete(node) };
    assert_released_once(&probe, &blocks);
    probe.assert_balanced();
}

#[test]
fn delete_walks_a_long_list_without_recursing() {
    let probe = Probe::start();
    let count = 10_000;
    let head = unsafe { comp_list::from_names((0..count).map(|_| &b"x"[..]), ptr::null_mut()) };
    let blocks = chain(head);
    assert_eq!(probe.live_blocks(), 2 * count);
    unsafe { comp_list::delete(head) };
    assert_released_once(&probe, &blocks);
    assert_eq!(probe.attempted_releases(), 2 * count);
    probe.assert_balanced();
}

#[test]
fn deleting_from_the_middle_releases_only_the_tail() {
    let probe = Probe::start();
    let list = unsafe { uio_make_path_comps(c"a/b/c".as_ptr(), ptr::null_mut()) };
    let all = chain(list);
    assert_eq!(all.len(), 3);
    unsafe {
        let second = (*list).next;
        (*list).next = ptr::null_mut();
        uio_path_comp_delete(second);
    }
    assert_live(&probe, &all[..1]);
    assert_released_once(&probe, &all[1..]);
    assert_eq!(
        probe.live_blocks(),
        2,
        "only the first node and its name remain"
    );
    unsafe { uio_path_comp_delete(list) };
    assert_released_once(&probe, &all);
    probe.assert_balanced();
}

#[test]
fn make_path_comps_never_owns_the_up_node() {
    let probe = Probe::start();
    let up = unsafe { uio_path_comp_new(comp_list::alloc_name(b"top"), 3, ptr::null_mut()) };
    let up_blocks = chain(up);
    for (path, expected_blocks) in [(c"/a//b", 8), (c"", 0), (c"a/", 2)] {
        let head = unsafe { uio_make_path_comps(path.as_ptr(), up) };
        let blocks = chain(head);
        assert_eq!(probe.live_blocks(), 2 + expected_blocks);
        assert_live(&probe, &blocks);
        unsafe { uio_path_comp_delete(head) };
        assert_released_once(&probe, &blocks);
        assert_live(&probe, &up_blocks);
        assert_eq!(probe.releases_of(up), 0, "up is borrowed");
        assert_eq!(probe.live_blocks(), 2);
    }
    unsafe { uio_path_comp_delete(up) };
    assert_released_once(&probe, &up_blocks);
    probe.assert_balanced();
}

#[test]
fn decompose_owns_two_blocks_per_name_and_nothing_on_failure() {
    let probe = Probe::start();
    let path = cstr(b"/a//b/");
    let (mut head, mut absolute) = (ptr::null_mut(), false);
    assert_eq!(
        unsafe { decompose_path(path.as_ptr(), &mut head, &mut absolute) },
        0
    );
    let blocks = chain(head);
    assert_eq!(blocks.len(), 2);
    assert_eq!(probe.live_blocks(), 4);
    assert_live(&probe, &blocks);
    assert_eq!(
        path.as_bytes(),
        b"/a//b/",
        "the input is read, not consumed"
    );
    unsafe { uio_path_comp_delete(head) };
    assert_released_once(&probe, &blocks);
    probe.assert_balanced();

    let sentinel = ptr::NonNull::<PathComp>::dangling().as_ptr();
    let mut head = sentinel;
    let before = probe.allocations();
    assert_eq!(
        unsafe { decompose_path(c"".as_ptr(), &mut head, &mut absolute) },
        -1
    );
    assert_eq!(
        (head, probe.allocations(), probe.live_blocks()),
        (sentinel, before, 0)
    );
    assert_eq!(
        unsafe { decompose_path(c"/".as_ptr(), &mut head, &mut absolute) },
        0
    );
    assert!(head.is_null());
    assert_eq!((probe.allocations(), probe.live_blocks()), (before, 0));
}

#[test]
fn returned_strings_are_single_blocks_the_caller_frees_once() {
    let probe = Probe::start();
    let (first, second) = (cstr(b"a/"), cstr(b"b"));
    let joined = unsafe { join_paths(first.as_ptr(), second.as_ptr()) };
    let absolute = unsafe { join_paths_absolute(first.as_ptr(), second.as_ptr()) };
    assert_eq!(probe.live_blocks(), 2);
    assert_eq!(probe.size_of_live(joined), Some(b"a/b".len() + 1));
    assert_eq!(probe.size_of_live(absolute), Some(b"/a/b".len() + 1));
    assert_eq!(
        (first.as_bytes(), second.as_bytes()),
        (&b"a/"[..], &b"b"[..])
    );
    caller_free(joined.cast());
    caller_free(absolute.cast());
    assert_eq!(
        probe.attempted_releases(),
        2,
        "only the returned blocks were released"
    );
    probe.assert_balanced();

    let list = OwnedPathComps::new([&b"a"[..], b"", b"b"]);
    let blocks = chain(list.as_ptr());
    for absolute in [true, false] {
        let (mut path, mut len) = (ptr::null_mut(), 0);
        unsafe { compose_path(list.as_ptr(), absolute, &mut path, &mut len) };
        assert_eq!(
            probe.live_blocks(),
            7,
            "six list blocks and the composed string"
        );
        assert_eq!(probe.size_of_live(path), Some(len + 1));
        assert_live(&probe, &blocks);
        caller_free(path.cast());
        assert_live(&probe, &blocks);
        assert_eq!(probe.live_blocks(), 6);
    }
    drop(list);
    probe.assert_balanced();
}

#[test]
fn unc_server_share_allocates_only_when_found() {
    let probe = Probe::start();
    let (mut out, mut len) = (ptr::null_mut(), 0);
    let found =
        unsafe { uio_get_unc_server_share(c"\\\\srv/share//x".as_ptr(), &mut out, &mut len) };
    assert!(found > 0 && !out.is_null());
    assert_eq!(probe.live_blocks(), 1);
    assert_eq!(probe.size_of_live(out), Some(len + 1));
    caller_free(out.cast());
    probe.assert_balanced();

    let mut out = ptr::NonNull::<c_char>::dangling().as_ptr();
    let before = probe.allocations();
    let found = unsafe { uio_get_unc_server_share(c"/plain".as_ptr(), &mut out, &mut len) };
    assert_eq!((found, out.is_null(), len), (0, true, 0));
    assert_eq!(
        (probe.allocations(), probe.attempted_releases()),
        (before, 1)
    );
}

#[test]
fn into_raw_and_from_raw_move_the_single_owner() {
    let probe = Probe::start();
    let raw = OwnedPathComps::new([&b"a"[..], b"b"]).into_raw();
    assert_eq!(
        probe.attempted_releases(),
        0,
        "into_raw does not drop the list"
    );
    let blocks = chain(raw);
    assert_live(&probe, &blocks);
    unsafe { comp_list::delete(raw) };
    assert_released_once(&probe, &blocks);
    probe.assert_balanced();

    let list = OwnedPathComps::new([&b"c"[..], b"d", b"e"]);
    let blocks = chain(list.as_ptr());
    let before = probe.attempted_releases();
    let adopted = unsafe { OwnedPathComps::from_raw(list.into_raw()) };
    assert_eq!(
        probe.attempted_releases(),
        before,
        "from_raw takes over without releasing"
    );
    drop(adopted);
    assert_released_once(&probe, &blocks);
    probe.assert_balanced();
}

#[test]
fn pure_functions_allocate_and_release_nothing() {
    let probe = Probe::start();
    let buffer = cstr(b"a//b/c");
    let (dir, end) = (buffer.as_ptr(), unsafe { buffer.as_ptr().add(6) });
    let (mut start, mut stop) = (ptr::null(), ptr::null());
    unsafe {
        get_first_path_component(dir, end, &mut start, &mut stop);
        get_next_path_component(end, &mut start, &mut stop);
        get_last_path_component(dir, end, &mut start, &mut stop);
        get_previous_path_component(dir, &mut start, &mut stop);
        get_first_path0_component(dir, &mut start, &mut stop);
        get_next_path0_component(&mut start, &mut stop);
        get_last_path0_component(dir, &mut start, &mut stop);
        assert!(valid_path_name(dir, 1));
        assert_eq!(uio_skip_unc_server_share(dir), 0);
        assert_eq!(uio_count_path_comps(ptr::null()), 0);
        assert!(uio_last_path_comp(ptr::null_mut()).is_null());
    }
    assert!(components::valid_path_name(b"a/b"));
    assert_eq!(
        (
            probe.allocations(),
            probe.attempted_releases(),
            probe.live_blocks()
        ),
        (0, 0, 0)
    );
    assert_eq!(buffer.as_bytes(), b"a//b/c");
}

#[test]
fn a_long_name_is_one_block_released_once() {
    let probe = Probe::start();
    let long = vec![b'z'; 4096];
    let list = OwnedPathComps::new([long.as_slice()]);
    let blocks = chain(list.as_ptr());
    assert_eq!(probe.size_of_live(blocks[0].name), Some(4097));
    assert_eq!(unsafe { CStr::from_ptr(blocks[0].name) }.to_bytes(), long);
    drop(list);
    assert_released_once(&probe, &blocks);
    probe.assert_balanced();
}

/// Negative controls: the ledger refuses what a broken owner would do.
#[test]
#[should_panic(expected = "not a live block")]
fn releasing_a_name_that_two_nodes_share_is_caught() {
    let _probe = Probe::start();
    let shared = comp_list::alloc_name(b"s");
    unsafe {
        let first = comp_list::new(shared, 1, ptr::null_mut());
        let second = comp_list::new(shared, 1, ptr::null_mut());
        comp_list::delete(first);
        comp_list::delete(second);
    }
}

#[test]
#[should_panic(expected = "not a live block")]
fn releasing_a_buffer_the_caller_lent_is_caught() {
    let _probe = Probe::start();
    let lent = cstr(b"lent");
    unsafe {
        let node = comp_list::new(lent.as_ptr().cast_mut(), 4, ptr::null_mut());
        comp_list::delete(node);
    }
}
