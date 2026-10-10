//! Tokenizer, command table, text formats and errno text: the parts that
//! need no repository.

use super::commands::{find_command, format_stat, list_query, DEBUG_COMMANDS};
use super::listing::{format_dir_handle, format_mounts};
use super::tokens::{make_args, EmbeddedNul};
use super::{CommandStatus, Errno};
use crate::io::name_filter::{MATCH_LITERAL, MATCH_PREFIX};
use crate::io::uio_bridge::MountListing;

fn words(line: &[u8]) -> Vec<Vec<u8>> {
    make_args(line)
        .unwrap()
        .into_iter()
        .map(|word| word.into_bytes())
        .collect()
}

#[test]
fn blank_lines_have_no_arguments() {
    for line in [&b""[..], b"   \t \r\n", b"\x0b\x0c"] {
        assert!(words(line).is_empty(), "{line:?}");
    }
}

#[test]
fn whitespace_is_the_c_isspace_set() {
    assert_eq!(
        words(b"  cat \t a\x0bb\x0cc \r\n"),
        vec![b"cat".to_vec(), b"a".to_vec(), b"b".to_vec(), b"c".to_vec()]
    );
}

#[test]
fn quotes_and_backslashes_are_ordinary_bytes() {
    assert_eq!(
        words(b"cat \"a b\" 'x' c\\ d"),
        vec![
            b"cat".to_vec(),
            b"\"a".to_vec(),
            b"b\"".to_vec(),
            b"'x'".to_vec(),
            b"c\\".to_vec(),
            b"d".to_vec(),
        ]
    );
}

#[test]
fn high_and_non_utf8_bytes_are_word_bytes() {
    assert_eq!(
        words(b"cat raw\xff\xfe \xa0 \xc3"),
        vec![
            b"cat".to_vec(),
            b"raw\xff\xfe".to_vec(),
            b"\xa0".to_vec(),
            b"\xc3".to_vec()
        ]
    );
}

#[test]
fn a_nul_byte_anywhere_rejects_the_line() {
    for line in [&b"cat a\0b"[..], b"\0", b"cat \0 b", b"cat a \0"] {
        assert_eq!(make_args(line).unwrap_err(), EmbeddedNul, "{line:?}");
    }
}

#[test]
fn many_arguments_are_all_kept() {
    let line = b"x ".repeat(500);
    assert_eq!(words(&line).len(), 500);
}

#[test]
fn the_table_is_the_c_table() {
    let names: Vec<&str> = DEBUG_COMMANDS.iter().map(|command| command.name).collect();
    assert_eq!(
        names,
        [
            "cat",
            "cd",
            "exec",
            "exit",
            "fwritetest",
            "ls",
            "mem",
            "mkdir",
            "mount",
            "mv",
            "pwd",
            "rm",
            "rmdir",
            "stat",
            "writetest"
        ]
    );
    let mut sorted = names.clone();
    sorted.sort_unstable();
    assert_eq!(names, sorted);
}

#[test]
fn lookup_is_exact_and_case_sensitive() {
    for name in DEBUG_COMMANDS.iter().map(|command| command.name) {
        let found = find_command(name.as_bytes()).expect(name);
        assert_eq!(found.name, name);
    }
    for name in [
        &b""[..],
        b"ca",
        b"catx",
        b"Cat",
        b"CAT",
        b"c\xe0t",
        b"cat ",
        b"bogus",
    ] {
        assert!(find_command(name).is_none(), "{name:?}");
    }
}

#[test]
fn command_status_codes_are_the_c_return_values() {
    assert_eq!(CommandStatus::Success.code(), 0);
    assert_eq!(CommandStatus::Failure.code(), 1);
}

#[test]
fn ls_arguments_split_like_list_one_dir() {
    let query = |arg: &[u8]| {
        let q = list_query(arg);
        (q.path.to_vec(), q.pattern.to_vec(), q.match_type)
    };
    let all = |path: &[u8]| (path.to_vec(), Vec::new(), MATCH_PREFIX);
    assert_eq!(query(b""), all(b""));
    assert_eq!(query(b"*"), all(b""));
    assert_eq!(query(b"/"), all(b"/"));
    assert_eq!(query(b"sub/"), all(b"sub/"));
    assert_eq!(query(b"a/b/*"), all(b"a/b"));
    assert_eq!(
        query(b"name"),
        (b"".to_vec(), b"name".to_vec(), MATCH_LITERAL)
    );
    assert_eq!(
        query(b"/name"),
        (b"/".to_vec(), b"name".to_vec(), MATCH_LITERAL)
    );
    assert_eq!(
        query(b"a/b/name"),
        (b"a/b".to_vec(), b"name".to_vec(), MATCH_LITERAL)
    );
    assert_eq!(
        query(b"a*"),
        (b"".to_vec(), b"a*".to_vec(), MATCH_LITERAL),
        "only a lone * is the wildcard"
    );
}

fn mount(fs_type: i32, point: &[u8], source: &[u8], read_only: bool) -> MountListing {
    MountListing {
        fs_type,
        mount_point: point.to_vec(),
        source_dir: source.to_vec(),
        read_only,
    }
}

#[test]
fn mount_lines_have_the_c_format_in_the_given_order() {
    let mounts = [
        mount(1, b"/", b"home/x/content", true),
        mount(2, b"/zip", b"", true),
        mount(1, b"/tmp", b"raw\xff", false),
    ];
    assert_eq!(
        format_mounts(&mounts),
        b"???:home/x/content on / type stdio (ro)\n???: on /zip type zip (ro)\n???:raw\xff on /tmp type stdio (rw)\n"
    );
    assert!(format_mounts(&[]).is_empty());
}

#[test]
fn dir_handles_print_an_empty_root_and_the_rest() {
    assert_eq!(format_dir_handle(b"/"), b"[]");
    assert_eq!(format_dir_handle(b"/a/b\xff"), b"[]a/b\xff");
}

#[test]
fn errno_text_is_strerror() {
    assert_eq!(Errno(libc::ENOENT).message(), b"No such file or directory");
    assert_eq!(Errno(libc::ENOTDIR).message(), b"Not a directory");
    assert!(!Errno(100_000).message().is_empty());
}

#[test]
fn a_time_ctime_cannot_show_is_eoverflow() {
    // SAFETY: an all-zero `stat` is a valid value.
    let mut info: libc::stat = unsafe { std::mem::zeroed() };
    info.st_mtime = libc::time_t::MAX;
    assert_eq!(format_stat(&info).unwrap_err(), Errno(libc::EOVERFLOW));
}
