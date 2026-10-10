//! Every command over the fixture world of `fixture-spec.md`, through the
//! public UIO calls and real temporary directories and a ZIP archive.

use std::fs;
use std::os::unix::fs::MetadataExt;

use serial_test::serial;

use super::world::{
    big_bytes, in_child_with_env, open_descriptors_for, run_bytes, World, RAW_DIR, RAW_FILE,
    RAW_MOUNT_POINT,
};
use super::CommandStatus::{Failure, Success};

fn bytes_of(parts: &[&[u8]]) -> Vec<u8> {
    parts.concat()
}

#[test]
#[serial]
fn wrong_argument_counts_are_usage_errors_that_change_nothing() {
    let world = World::new();
    let mut session = world.session();
    let host_entries = fs::read_dir(world.host.path()).unwrap().count();
    let cases = [
        "cat",
        "cat a b",
        "cd",
        "cd a b",
        "mkdir",
        "mkdir a b",
        "mv",
        "mv a",
        "mv a b c",
        "rm",
        "rm a b",
        "rmdir",
        "rmdir a b",
        "stat",
        "stat a b",
        "writetest",
        "writetest a b",
        "fwritetest",
        "fwritetest a b",
        "exec",
    ];
    for line in cases {
        let outcome = world.run(&mut session, line);
        assert_eq!(outcome.status, Failure, "{line}");
        assert_eq!(
            outcome.err_text(),
            "Invalid number of arguments.\n",
            "{line}"
        );
        assert!(outcome.out.is_empty(), "{line}");
    }
    assert_eq!(session.working_dir(), b"/");
    assert_eq!(
        fs::read_dir(world.host.path()).unwrap().count(),
        host_entries
    );
}

#[test]
#[serial]
fn unknown_commands_are_reported_and_run_nothing() {
    let world = World::new();
    let mut session = world.session();
    for line in ["bogus", "ca", "catx hello.txt", "Cat hello.txt", "exit1"] {
        let outcome = world.run(&mut session, line);
        assert_eq!(outcome.status, Failure, "{line}");
        assert_eq!(outcome.err_text(), "Invalid command.\n", "{line}");
        assert!(outcome.out.is_empty(), "{line}");
    }
    let outcome = run_bytes(&mut session, b"c\xe0t hello.txt");
    assert_eq!(outcome.err_text(), "Invalid command.\n");
    assert!(!session.exit_requested());
}

#[test]
#[serial]
fn exit_mem_and_mount_with_arguments() {
    let world = World::new();
    let mut session = world.session();
    let outcome = world.run(&mut session, "mem anything at all");
    assert_eq!(outcome.status, Success);
    assert_eq!(outcome.out_text(), "Memory debugging not compiled in.\n");
    assert!(outcome.err.is_empty());

    let outcome = world.run(&mut session, "mount extra");
    assert_eq!(outcome.status, Success);
    assert!(outcome.out.is_empty() && outcome.err.is_empty());

    assert!(!session.exit_requested());
    let outcome = world.run(&mut session, "exit now please");
    assert_eq!(outcome.status, Success);
    assert!(session.exit_requested());
    assert!(outcome.out.is_empty() && outcome.err.is_empty());
}

#[test]
#[serial]
fn cat_prints_the_bytes_and_their_count() {
    let world = World::new();
    let mut session = world.session();
    let cases: Vec<(&str, Vec<u8>)> = vec![
        ("cat hello.txt", b"hello\n[6 bytes]\n".to_vec()),
        ("cat empty.bin", b"[0 bytes]\n".to_vec()),
        ("cat /ro/fixed.txt", b"fixed[5 bytes]\n".to_vec()),
        ("cat /pack/dir/b.txt", b"bee[3 bytes]\n".to_vec()),
        ("cat //sub/../sub/in.txt", b"inner[5 bytes]\n".to_vec()),
    ];
    for (line, expected) in cases {
        let outcome = world.run(&mut session, line);
        assert_eq!(outcome.status, Success, "{line}");
        assert_eq!(outcome.out, expected, "{line}");
        assert!(outcome.err.is_empty(), "{line}");
    }

    let all: Vec<u8> = (0..=255u8).collect();
    let outcome = world.run(&mut session, "cat all.bin");
    assert_eq!(outcome.out, bytes_of(&[&all, b"[256 bytes]\n"]));

    // 200000 bytes cross the 64 KiB read size three times.
    let outcome = world.run(&mut session, "cat big.bin");
    assert_eq!(outcome.out, bytes_of(&[&big_bytes(), b"[200000 bytes]\n"]));

    let outcome = world.run(&mut session, "cat /pack/a.txt");
    assert_eq!(outcome.out, bytes_of(&[&[b'a'; 4096], b"[4096 bytes]\n"]));
}

#[test]
#[serial]
fn cat_takes_raw_byte_paths() {
    let world = World::new();
    let mut session = world.session();
    // A virtual mount point is bytes on every host.
    let raw_dir = &RAW_MOUNT_POINT[1..];
    let outcome = run_bytes(&mut session, &bytes_of(&[b"cat ", raw_dir, b"/inner.txt"]));
    assert_eq!(outcome.status, Success);
    assert_eq!(outcome.out, b"raw mount[9 bytes]\n");
    let outcome = run_bytes(&mut session, &bytes_of(&[b"cd /", raw_dir]));
    assert_eq!(outcome.status, Success);
    assert_eq!(
        world.run(&mut session, "cat inner.txt").out,
        b"raw mount[9 bytes]\n"
    );
    assert_eq!(
        world.run(&mut session, "pwd").out,
        bytes_of(&[b"[]", raw_dir])
    );
    let outcome = world.run(&mut session, "ls");
    assert_eq!(outcome.out, b"inner.txt\n");

    // Names on the host itself, where the host file system can hold them.
    if world.raw_host {
        let mut session = world.session();
        let outcome = run_bytes(&mut session, &bytes_of(&[b"cat ", RAW_FILE]));
        assert_eq!(outcome.out, b"rawname[7 bytes]\n");
        run_bytes(&mut session, &bytes_of(&[b"cd ", RAW_DIR]));
        assert_eq!(world.run(&mut session, "cat x").out, b"x[1 bytes]\n");
    }
}

#[test]
#[serial]
fn cat_failures_name_the_cause_and_print_no_count() {
    let world = World::new();
    let mut session = world.session();
    let outcome = world.run(&mut session, "cat nope");
    assert_eq!(outcome.status, Failure);
    assert_eq!(
        outcome.err_text(),
        "Could not open file: No such file or directory\n"
    );
    assert!(outcome.out.is_empty());

    let outcome = world.run(&mut session, "cat sub");
    assert_eq!(outcome.status, Failure);
    assert!(
        outcome.err_text().starts_with("Could not open file: ")
            || outcome.err_text().starts_with("Could not read from file: "),
        "{}",
        outcome.err_text()
    );
    assert!(!outcome.out_text().contains("bytes]"));
}

#[test]
#[serial]
fn cat_and_writetest_close_what_they_open() {
    let world = World::new();
    let mut session = world.session();
    let hello = world.host.path().join("hello.txt");
    for _ in 0..3 {
        world.run(&mut session, "cat hello.txt");
        world.run(&mut session, "cat sub");
        world.run(&mut session, "cat hello.txt hello.txt");
    }
    assert_eq!(open_descriptors_for(&hello), 0);
    world.run(&mut session, "writetest closed.txt");
    assert_eq!(
        open_descriptors_for(&world.host.path().join("closed.txt")),
        0
    );
    world.run(&mut session, "fwritetest closed2.txt");
    assert_eq!(
        open_descriptors_for(&world.host.path().join("closed2.txt")),
        0
    );
}

#[test]
#[serial]
fn cd_and_pwd_follow_the_virtual_tree() {
    let world = World::new();
    let mut session = world.session();
    assert_eq!(world.run(&mut session, "pwd").out, b"[]");
    assert_eq!(world.run(&mut session, "cd sub").status, Success);
    assert_eq!(world.run(&mut session, "pwd").out, b"[]sub");
    assert_eq!(session.working_dir(), b"/sub");
    assert_eq!(
        world.run(&mut session, "cat in.txt").out,
        b"inner[5 bytes]\n"
    );
    assert_eq!(world.run(&mut session, "cd ..").status, Success);
    assert_eq!(world.run(&mut session, "pwd").out, b"[]");
    assert_eq!(world.run(&mut session, "cd /pack/dir").status, Success);
    assert_eq!(world.run(&mut session, "pwd").out, b"[]pack/dir");
    assert_eq!(world.run(&mut session, "cat b.txt").out, b"bee[3 bytes]\n");
    assert_eq!(world.run(&mut session, "cd //sub/../sub/").status, Success);
    assert_eq!(world.run(&mut session, "pwd").out, b"[]sub");
    assert_eq!(world.run(&mut session, "cd /").status, Success);
    assert_eq!(world.run(&mut session, "pwd").out, b"[]");
}

#[test]
#[serial]
fn a_failed_cd_keeps_the_working_directory() {
    let world = World::new();
    let mut session = world.session();
    world.run(&mut session, "cd sub");
    let outcome = world.run(&mut session, "cd in.txt");
    assert_eq!(outcome.status, Failure);
    assert_eq!(
        outcome.err_text(),
        "Could not access new dir: Not a directory\n"
    );
    let outcome = world.run(&mut session, "cd nope");
    assert_eq!(
        outcome.err_text(),
        "Could not access new dir: No such file or directory\n"
    );
    assert!(outcome.out.is_empty());
    assert_eq!(world.run(&mut session, "pwd").out, b"[]sub");
    assert_eq!(world.run(&mut session, "cat in.txt").status, Success);
}

fn sorted(mut names: Vec<&[u8]>) -> Vec<u8> {
    names.sort_unstable();
    names
        .iter()
        .flat_map(|name| [*name, b"\n"].concat())
        .collect()
}

#[test]
#[serial]
fn ls_lists_the_union_of_mounts_in_byte_order() {
    let world = World::new();
    let mut session = world.session();
    let mut names: Vec<&[u8]> = vec![
        b"all.bin",
        b"below",
        b"big.bin",
        b"bottom",
        b"empty.bin",
        b"hello.txt",
        b"pack",
        b"ro",
        b"script_exit3.sh",
        b"script_kill.sh",
        b"sub",
        b"tmp",
        &RAW_MOUNT_POINT[1..],
    ];
    if world.raw_host {
        names.extend([RAW_DIR, RAW_FILE]);
    }
    let expected = sorted(names);
    for line in ["ls", "ls /", "ls /*", "ls *"] {
        let outcome = world.run(&mut session, line);
        assert_eq!(outcome.status, Success, "{line}");
        assert_eq!(outcome.out, expected, "{line}");
        assert!(outcome.err.is_empty(), "{line}");
    }
}

#[test]
#[serial]
fn ls_directories_literals_and_archives() {
    let world = World::new();
    let mut session = world.session();
    let cases = [
        ("ls sub/", "in.txt\n"),
        ("ls sub/in.txt", "in.txt\n"),
        ("ls /hello.txt", "hello.txt\n"),
        ("ls /pack/", "a.txt\ndir\nscript_exit5.sh\n"),
        ("ls /pack/dir/", "b.txt\n"),
        ("ls /ro/", "fixed.txt\n"),
        ("ls sub/ /ro/", "in.txt\nfixed.txt\n"),
        ("ls sub/nothing", ""),
    ];
    for (line, expected) in cases {
        let outcome = world.run(&mut session, line);
        assert_eq!(outcome.status, Success, "{line}");
        assert_eq!(outcome.out_text(), expected, "{line}");
    }
    world.run(&mut session, "cd sub");
    assert_eq!(world.run(&mut session, "ls").out, b"in.txt\n");
    let outcome = run_bytes(
        &mut session,
        &bytes_of(&[b"ls ", RAW_MOUNT_POINT, b"/inner.txt"]),
    );
    assert_eq!(outcome.out, b"inner.txt\n");
    if world.raw_host {
        let outcome = run_bytes(&mut session, &bytes_of(&[b"ls /", RAW_DIR, b"/"]));
        assert_eq!(outcome.out, b"x\n");
    }
}

/// A directory the host will not let this process read.
fn locked_dir(world: &World) {
    use std::os::unix::fs::PermissionsExt;
    let locked = world.host.path().join("sub/locked");
    fs::create_dir(&locked).unwrap();
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o000)).unwrap();
}

#[test]
#[serial]
fn ls_of_a_directory_no_mount_has_lists_nothing() {
    let world = World::new();
    let mut session = world.session();
    let outcome = world.run(&mut session, "ls nope/");
    assert_eq!(outcome.status, Success);
    assert!(outcome.out.is_empty() && outcome.err.is_empty());
}

#[test]
#[serial]
fn ls_failures_go_to_out_and_stop_the_list() {
    // A process with root rights reads any directory.
    if unsafe { libc::geteuid() } == 0 {
        return;
    }
    let world = World::new();
    locked_dir(&world);
    let mut session = world.session();
    let outcome = world.run(&mut session, "ls sub/locked/");
    assert_eq!(outcome.status, Failure);
    assert_eq!(
        outcome.out_text(),
        "Error in uio_getDirList(): Permission denied.\n"
    );
    assert!(outcome.err.is_empty());

    let outcome = world.run(&mut session, "ls sub/ sub/locked/ /ro/");
    assert_eq!(outcome.status, Failure);
    assert_eq!(
        outcome.out_text(),
        "in.txt\nlocked\nError in uio_getDirList(): Permission denied.\n"
    );
}

#[test]
#[serial]
fn mkdir_and_rmdir_change_the_writable_mount() {
    let world = World::new();
    let mut session = world.session();
    assert_eq!(world.run(&mut session, "mkdir made").status, Success);
    assert!(world.host.path().join("made").is_dir());
    assert!(world.run(&mut session, "ls").out_text().contains("made\n"));

    let outcome = world.run(&mut session, "mkdir made");
    assert_eq!(outcome.status, Failure);
    assert_eq!(
        outcome.err_text(),
        "Could not create directory: File exists\n"
    );
    let outcome = world.run(&mut session, "mkdir nope/a/b");
    assert_eq!(
        outcome.err_text(),
        "Could not create directory: No such file or directory\n"
    );

    assert_eq!(world.run(&mut session, "rmdir made").status, Success);
    assert!(!world.host.path().join("made").exists());
    let outcome = world.run(&mut session, "rmdir sub");
    assert_eq!(
        outcome.err_text(),
        "Could not remove directory: Directory not empty\n"
    );
    assert!(world.host.path().join("sub/in.txt").exists());
    let outcome = world.run(&mut session, "rmdir nope");
    assert_eq!(
        outcome.err_text(),
        "Could not remove directory: No such file or directory\n"
    );
}

#[test]
#[serial]
fn changes_below_a_read_only_mount_land_in_the_writable_mount() {
    let world = World::new();
    let mut session = world.session();
    for line in [
        "mkdir /ro/x",
        "writetest /ro/w",
        "fwritetest /ro/f",
        "mkdir /pack/x",
    ] {
        let outcome = world.run(&mut session, line);
        assert_eq!(outcome.status, Success, "{line}: {}", outcome.err_text());
    }
    // The writes went to `/`, below the read-only mounts; they are visible
    // through the stack and the read-only directory is untouched.
    assert!(world.host.path().join("ro/x").is_dir());
    assert_eq!(
        fs::read(world.host.path().join("ro/w")).unwrap(),
        b"Hello world!\n\0"
    );
    assert!(world.host.path().join("ro/f").is_file());
    assert!(world.host.path().join("pack/x").is_dir());
    assert_eq!(fs::read_dir(world.ro.path()).unwrap().count(), 1);
    assert_eq!(
        world.run(&mut session, "ls /ro/").out_text(),
        "f\nfixed.txt\nw\nx\n"
    );
}

#[test]
#[serial]
fn a_repository_with_no_writable_mount_refuses_every_change() {
    let world = World::read_only();
    let mut session = world.session();
    let refused = [
        (
            "mkdir x",
            "Could not create directory: Operation not permitted\n",
        ),
        (
            "mkdir /pack/x",
            "Could not create directory: Operation not permitted\n",
        ),
        (
            "rmdir sub",
            "Could not remove directory: Read-only file system\n",
        ),
        (
            "rmdir /pack/dir",
            "Could not remove directory: Read-only file system\n",
        ),
        (
            "rm hello.txt",
            "Could not remove file: Read-only file system\n",
        ),
        (
            "rm /pack/a.txt",
            "Could not remove file: Read-only file system\n",
        ),
        (
            "writetest w",
            "Could not open file: Operation not permitted\n",
        ),
        (
            "writetest /pack/w",
            "Could not open file: Operation not permitted\n",
        ),
        (
            "fwritetest f",
            "Could not open file: Operation not permitted\n",
        ),
        (
            "mv hello.txt moved.txt",
            "Could not rename: Operation not permitted\n",
        ),
        (
            "mv /pack/a.txt /pack/c.txt",
            "Could not rename: Operation not permitted\n",
        ),
    ];
    for (line, message) in refused {
        let outcome = world.run(&mut session, line);
        assert_eq!(outcome.status, Failure, "{line}");
        assert_eq!(outcome.err_text(), message, "{line}");
        assert!(outcome.out.is_empty(), "{line}");
    }
    assert!(world.host.path().join("hello.txt").is_file());
    assert!(world.host.path().join("sub").is_dir());
    for name in ["x", "w", "f", "moved.txt"] {
        assert!(!world.host.path().join(name).exists(), "{name}");
    }
    // Reading still works.
    assert_eq!(world.run(&mut session, "cat hello.txt").status, Success);
}

#[test]
#[serial]
fn rm_and_mv_act_on_the_host_files() {
    let world = World::new();
    let mut session = world.session();
    assert_eq!(
        world.run(&mut session, "mv hello.txt moved.txt").status,
        Success
    );
    assert!(!world.host.path().join("hello.txt").exists());
    assert_eq!(
        fs::read(world.host.path().join("moved.txt")).unwrap(),
        b"hello\n"
    );
    assert_eq!(world.run(&mut session, "rm moved.txt").status, Success);
    assert!(!world.host.path().join("moved.txt").exists());

    let outcome = world.run(&mut session, "rm moved.txt");
    assert_eq!(
        outcome.err_text(),
        "Could not remove file: No such file or directory\n"
    );
    let outcome = world.run(&mut session, "mv nope x");
    assert_eq!(outcome.status, Failure);
    assert_eq!(
        outcome.err_text(),
        "Could not rename: No such file or directory\n"
    );
    let outcome = world.run(&mut session, "rm sub");
    assert_eq!(outcome.status, Failure);
    assert!(outcome.err_text().starts_with("Could not remove file: "));
    assert!(world.host.path().join("sub/in.txt").exists());
}

#[test]
#[serial]
fn writetest_creates_the_file_once() {
    let world = World::new();
    let mut session = world.session();
    let outcome = world.run(&mut session, "writetest w.txt");
    assert_eq!(outcome.status, Success);
    assert!(outcome.out.is_empty() && outcome.err.is_empty());
    let path = world.host.path().join("w.txt");
    assert_eq!(fs::read(&path).unwrap(), b"Hello world!\n\0");
    assert_eq!(
        fs::metadata(&path).unwrap().mode() & 0o777 & !0o022,
        0o644 & !0o022
    );

    let outcome = world.run(&mut session, "writetest w.txt");
    assert_eq!(outcome.status, Failure);
    assert_eq!(outcome.err_text(), "Could not open file: File exists\n");
    assert_eq!(fs::read(&path).unwrap(), b"Hello world!\n\0");
}

/// The C algorithm of `debugCmdFwriteTest` on a byte vector.
fn fwritetest_model() -> (String, Vec<u8>) {
    let mut file = b"0123456789\n0123456789\n".to_vec();
    file[15] = b'A';
    let mut shown = String::new();
    let (mut at, mut number) = (0, 1);
    while at < file.len() {
        let end = (at + 5).min(file.len());
        let chunk = &file[at..end];
        let stop = chunk
            .iter()
            .position(|&b| b == b'\n')
            .map_or(chunk.len(), |i| i + 1);
        shown += &format!("{number}: [{}]\n", String::from_utf8_lossy(&chunk[..stop]));
        at += stop;
        number += 1;
    }
    file.resize(26, 0);
    file.extend_from_slice(&[b'Q'; 2000]);
    file[5] = b'B';
    (shown, file)
}

#[test]
#[serial]
fn fwritetest_runs_the_stream_sequence() {
    let world = World::new();
    let mut session = world.session();
    let (shown, file) = fwritetest_model();
    let outcome = world.run(&mut session, "fwritetest f.txt");
    assert_eq!(outcome.status, Success, "{}", outcome.err_text());
    assert_eq!(outcome.out_text(), shown);
    assert_eq!(fs::read(world.host.path().join("f.txt")).unwrap(), file);
}

#[test]
#[serial]
fn fwritetest_open_failures_are_reported() {
    let world = World::new();
    let mut session = world.session();
    let outcome = world.run(&mut session, "fwritetest nope/f");
    assert_eq!(outcome.status, Failure);
    assert!(outcome.err_text().starts_with("Could not open file: "));
    assert!(outcome.out.is_empty());
}

fn mount_lines(world: &World, session: &mut super::DebugSession) -> Vec<Vec<u8>> {
    let outcome = world.run(session, "mount");
    assert_eq!(outcome.status, Success);
    assert!(outcome.err.is_empty());
    outcome
        .out
        .split(|&byte| byte == b'\n')
        .filter(|line| !line.is_empty())
        .map(<[u8]>::to_vec)
        .collect()
}

fn host_dir(dir: &tempfile::TempDir) -> Vec<u8> {
    dir.path()
        .to_str()
        .unwrap()
        .trim_start_matches('/')
        .as_bytes()
        .to_vec()
}

#[test]
#[serial]
fn mount_lists_the_registry_in_priority_order() {
    let world = World::new();
    let mut session = world.session();
    let lines = mount_lines(&world, &mut session);
    let expected: Vec<Vec<u8>> = vec![
        bytes_of(&[
            b"???:",
            &host_dir(&world.raw_mount),
            b" on ",
            RAW_MOUNT_POINT,
            b" type stdio (rw)",
        ]),
        bytes_of(&[b"???:", &host_dir(&world.tmp), b" on /tmp type stdio (rw)"]),
        bytes_of(&[b"???: on /pack type zip (ro)"]),
        bytes_of(&[b"???:", &host_dir(&world.ro), b" on /ro type stdio (ro)"]),
        bytes_of(&[
            b"???:",
            &host_dir(&world.below),
            b" on /below type stdio (rw)",
        ]),
        bytes_of(&[b"???:", &host_dir(&world.host), b" on / type stdio (rw)"]),
        bytes_of(&[
            b"???:",
            &host_dir(&world.bottom),
            b" on /bottom type stdio (rw)",
        ]),
    ];
    assert_eq!(
        lines
            .iter()
            .map(|l| String::from_utf8_lossy(l).into_owned())
            .collect::<Vec<_>>(),
        expected
            .iter()
            .map(|l| String::from_utf8_lossy(l).into_owned())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        lines, expected,
        "the raw mount point is printed byte for byte"
    );
}

#[test]
#[serial]
fn mount_follows_unmounts_and_separates_repositories() {
    let mut world = World::new();
    let mut session = world.session();
    let other = World::new();
    let mut other_session = other.session();
    let before = mount_lines(&world, &mut session);
    assert_eq!(before.len(), 7);
    let other_before = mount_lines(&other, &mut other_session);
    assert_eq!(other_before.len(), 7);
    let other_host = host_dir(&other.host);
    assert!(before
        .iter()
        .all(|line| !line.windows(other_host.len()).any(|w| w == other_host)));

    world.unmount_ro();
    let after = mount_lines(&world, &mut session);
    assert_eq!(after.len(), 6);
    assert!(after
        .iter()
        .all(|line| !line.windows(8).any(|w| w == b" on /ro ")));
    assert_eq!(mount_lines(&other, &mut other_session), other_before);
    // The unmounted tree is gone from the listing; the session still works.
    assert!(!world.run(&mut session, "ls").out_text().contains("\nro\n"));
}

fn expected_ctime(seconds: i64) -> String {
    const DAYS: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
    const MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    // SAFETY: both pointers are valid for the call.
    let mut parts: libc::tm = unsafe { std::mem::zeroed() };
    let time = seconds as libc::time_t;
    assert!(!unsafe { libc::gmtime_r(&time, &mut parts) }.is_null());
    format!(
        "{} {} {:2} {:02}:{:02}:{:02} {}\n",
        DAYS[parts.tm_wday as usize],
        MONTHS[parts.tm_mon as usize],
        parts.tm_mday,
        parts.tm_hour,
        parts.tm_min,
        parts.tm_sec,
        parts.tm_year + 1900
    )
}

#[test]
#[serial]
fn stat_reports_the_host_file() {
    in_child_with_env(
        module_path!(),
        "stat_reports_the_host_file",
        &[("TZ", "UTC")],
        || {
            let world = World::new();
            let mut session = world.session();
            let meta = fs::metadata(world.host.path().join("hello.txt")).unwrap();
            let outcome = world.run(&mut session, "stat hello.txt");
            assert_eq!(outcome.status, Success, "{}", outcome.err_text());
            let expected = format!(
                "size 6 bytes\nuid {}  gid {}  mode 0644\nlast access:        {}last modification:  {}last status change: {}",
                meta.uid(),
                meta.gid(),
                expected_ctime(meta.atime()),
                expected_ctime(meta.mtime()),
                expected_ctime(meta.ctime()),
            );
            assert_eq!(outcome.out_text(), expected);
            assert!(outcome.err.is_empty());

            let outcome = world.run(&mut session, "stat sub");
            let sub = fs::metadata(world.host.path().join("sub")).unwrap();
            assert!(outcome
                .out_text()
                .starts_with(&format!("size {} bytes\n", sub.size())));
            assert!(outcome
                .out_text()
                .contains(&format!("mode 0{:o}\n", sub.mode() & 0o7777)));
        },
    );
}

#[test]
#[serial]
fn stat_reports_the_archive_entry_time() {
    in_child_with_env(
        module_path!(),
        "stat_reports_the_archive_entry_time",
        &[("TZ", "UTC")],
        || {
            let world = World::new();
            let mut session = world.session();
            let outcome = world.run(&mut session, "stat /pack/a.txt");
            assert_eq!(outcome.status, Success, "{}", outcome.err_text());
            let text = outcome.out_text();
            assert!(text.starts_with("size 4096 bytes\n"), "{text}");
            assert!(
                text.contains("last modification:  Thu Jan  2 03:04:06 2020\n"),
                "{text}"
            );
        },
    );
}

#[test]
#[serial]
fn stat_failures_name_the_cause() {
    let world = World::new();
    let mut session = world.session();
    let outcome = world.run(&mut session, "stat nope");
    assert_eq!(outcome.status, Failure);
    assert_eq!(
        outcome.err_text(),
        "Could not stat file: No such file or directory\n"
    );
    assert!(outcome.out.is_empty());
}

fn script_path(printed: &str, program: &str) -> std::path::PathBuf {
    let rest = printed
        .strip_prefix(&format!("Executing: {program} "))
        .unwrap_or_else(|| panic!("unexpected announcement {printed:?}"));
    let line = rest.lines().next().unwrap();
    fs::canonicalize(line).unwrap_or_else(|_| line.into())
}

#[test]
#[serial]
fn exec_runs_a_program_with_host_paths() {
    let world = World::new();
    let mut session = world.session();
    let outcome = world.run(&mut session, "exec /bin/sh script_exit3.sh");
    assert_eq!(outcome.status, Success);
    assert!(outcome.out.is_empty());
    let text = outcome.err_text();
    assert!(text.ends_with("\nExit status: 3\n"), "{text}");
    assert_eq!(
        script_path(&text, "/bin/sh"),
        fs::canonicalize(world.host.path().join("script_exit3.sh")).unwrap()
    );

    let outcome = world.run(&mut session, "exec /bin/sh script_kill.sh");
    assert_eq!(outcome.status, Success);
    assert!(outcome.err_text().ends_with("\nTerminated on signal 9.\n"));

    // Words that name no file are passed as typed.
    let outcome = world.run(&mut session, "exec /bin/sh -c true");
    assert_eq!(
        outcome.err_text(),
        "Executing: /bin/sh -c true\nExit status: 0\n"
    );
}

#[test]
#[serial]
fn exec_hands_archive_entries_over_as_temporary_files() {
    let world = World::new();
    let mut session = world.session();
    let outcome = world.run(&mut session, "exec /bin/sh /pack/script_exit5.sh");
    assert_eq!(outcome.status, Success);
    let text = outcome.err_text();
    assert!(
        text.ends_with("\nExit status: 5\n"),
        "the copy must exist while the program runs: {text}"
    );
    let used = script_path(&text, "/bin/sh");
    let tmp = fs::canonicalize(world.tmp.path()).unwrap();
    assert!(
        used.starts_with(&tmp) || used.starts_with(world.tmp.path()),
        "{used:?} is not under {tmp:?}"
    );
    assert_eq!(
        fs::read_dir(world.tmp.path()).unwrap().count(),
        0,
        "the copy is removed"
    );
}

#[test]
#[serial]
fn exec_failures_are_reported() {
    let world = World::new();
    let mut session = world.session();
    let outcome = world.run(&mut session, "exec /no/such/program");
    assert_eq!(outcome.status, Success);
    assert_eq!(
        outcome.err_text(),
        "Executing: /no/such/program\nError: execvp() failed: No such file or directory.\nExit status: 1\n"
    );

    // An argument that cannot be given a host path stops the command, and the
    // copies made for the arguments before it are released.
    let outcome = world.run(&mut session, "exec /bin/true /pack/a.txt sub");
    assert_eq!(outcome.status, Failure);
    assert!(
        outcome
            .err_text()
            .starts_with("Cannot execute: Cannot get stdio access to sub: "),
        "{}",
        outcome.err_text()
    );
    assert!(!outcome.err_text().contains("Executing:"));
    assert_eq!(fs::read_dir(world.tmp.path()).unwrap().count(), 0);
}

#[test]
#[serial]
fn exec_needs_the_temporary_directory() {
    // Without `/tmp` in the virtual tree there is nowhere to put copies.
    let bare = World::bare();
    let mut session = bare.session();
    let outcome = bare.run(&mut session, "exec /bin/true");
    assert_eq!(outcome.status, Failure);
    assert_eq!(
        outcome.err_text(),
        "Could not open temp dir: No such file or directory.\n"
    );
    assert!(outcome.out.is_empty());
}

#[test]
#[serial]
fn status_and_error_stream_agree_for_every_command() {
    let world = World::new();
    let mut session = world.session();
    let lines = [
        "cat hello.txt",
        "cat nope",
        "cd sub",
        "cd nope",
        "ls",
        "ls nope/",
        "mem",
        "mount",
        "mkdir okdir",
        "mkdir okdir",
        "mv okdir moved",
        "mv nope x",
        "pwd",
        "rm hello.txt",
        "rm hello.txt",
        "rmdir moved",
        "rmdir moved",
        "stat sub",
        "stat nope",
        "writetest wt",
        "writetest wt",
        "fwritetest ft",
        "fwritetest /ro/ft",
        "bogus",
    ];
    for line in lines {
        let outcome = world.run(&mut session, line);
        let failed = outcome.status == Failure;
        let reports_on_err = !outcome.err.is_empty();
        // `ls` reports on the output stream; every other command on `err`.
        let expected = if line.starts_with("ls") {
            outcome.out_text().starts_with("Error in uio_getDirList")
        } else {
            reports_on_err
        };
        assert_eq!(
            failed,
            expected,
            "{line}: {:?} {:?}",
            outcome.out_text(),
            outcome.err_text()
        );
    }
}
