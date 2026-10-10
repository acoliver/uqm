//! Permission bits of what the host file system creates for UIO.
//!
//! `uio_open` and `uio_mkdir` take a mode, the host applies the process umask
//! to it, and that is the mode of what is made. The legacy rules
//! (`stdio_open`, `stdio_mkdir`, `uio_fopen`, `uio_makePath`,
//! `copyFilePhysical`): `uio_fopen` opens with 0666; the parent directories a
//! copy-up makes use 0777; the copy of a host file has the source's
//! permission bits (`st_mode & 0777`), the copy of an archive entry 0644;
//! `uio_getStdioAccess` makes its generated directory 0700 and the unpacked
//! file 0600. An existing file or directory keeps its mode.
//!
//! The umask is process-wide, so the test binary never changes its own. The
//! parent test starts a copy of itself (`--exact`, the way
//! `native_isolation` does) whose umask is set between fork and exec, and the
//! child drives the public UIO functions. The parent then reads the bits the
//! real file system holds. Without the environment variable the child test is
//! a no-op.

use std::collections::BTreeMap;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::process::CommandExt;
use std::process::Command;

use serial_test::serial;
use tempfile::TempDir;

use super::resolution_tests::{
    cstr, errno_value, mount_stdio, mount_zip, open_dir, write_zip, Fx, RDONLY_TOP,
};
use super::*;

const CHILD_ROOT: &str = "UIO_MODE_CHILD_ROOT";

fn bits(path: &Path) -> u32 {
    std::fs::metadata(path).unwrap().permissions().mode() & 0o7777
}

/// Every file and directory below `root` with its permission bits and bytes.
fn snapshot(root: &Path) -> BTreeMap<String, (u32, Option<Vec<u8>>)> {
    fn walk(dir: &Path, root: &Path, out: &mut BTreeMap<String, (u32, Option<Vec<u8>>)>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            let name = path
                .strip_prefix(root)
                .unwrap()
                .to_string_lossy()
                .into_owned();
            if path.is_dir() {
                out.insert(name, (bits(&path), None));
                walk(&path, root, out);
            } else {
                out.insert(name, (bits(&path), Some(std::fs::read(&path).unwrap())));
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(root, root, &mut out);
    out
}

fn must_open(dir: *mut uio_DirHandle, name: &str, flags: c_int, mode: c_int) {
    let name = cstr(name);
    let handle = unsafe { uio_open(dir, name.as_ptr(), flags, mode) };
    assert!(!handle.is_null(), "{name:?}: errno {}", errno_value());
    unsafe { uio_close(handle) };
}

/// The fixture the parent builds and the child mounts: a writable host, a
/// read-only host with a 0640 file two directories deep, and an archive
/// holding `z/entry.txt` and `z/other.txt`.
fn create_layout(root: &Path) {
    std::fs::create_dir_all(root.join("rw")).unwrap();
    std::fs::write(root.join("rw/pre.txt"), b"pre").unwrap();
    std::fs::set_permissions(root.join("rw/pre.txt"), PermissionsExt::from_mode(0o600)).unwrap();
    std::fs::create_dir_all(root.join("lower/deep/er")).unwrap();
    std::fs::write(root.join("lower/deep/er/file.txt"), b"lower bytes").unwrap();
    std::fs::set_permissions(
        root.join("lower/deep/er/file.txt"),
        PermissionsExt::from_mode(0o640),
    )
    .unwrap();
    write_zip(
        &root.join("zips/pack.zip"),
        &[("z/entry.txt", b"entry bytes"), ("z/other.txt", b"other")],
    );
}

/// Runs inside the child process, under the umask the parent chose.
#[test]
#[serial]
fn child_makes_files_and_directories() {
    let Some(root) = std::env::var_os(CHILD_ROOT) else {
        return;
    };
    let base = PathBuf::from(root);
    let layout_root = base.join("layout");
    let mut fx = Fx::new();
    let repo = fx.repo();
    assert!(!mount_stdio(
        repo,
        "/",
        &layout_root.join("lower"),
        UIO_MOUNT_BOTTOM | UIO_MOUNT_RDONLY
    )
    .is_null());
    assert!(!mount_stdio(repo, "/src", &layout_root.join("zips"), RDONLY_TOP).is_null());
    let src = open_dir(repo, "/src");
    assert!(!mount_zip(
        repo,
        "/",
        src,
        "pack.zip",
        UIO_MOUNT_BOTTOM | UIO_MOUNT_RDONLY
    )
    .is_null());
    assert!(!mount_stdio(repo, "/", &layout_root.join("rw"), UIO_MOUNT_TOP).is_null());
    let dir = open_dir(repo, "/");

    must_open(dir, "new.txt", O_WRONLY | O_CREAT, 0o640);
    must_open(dir, "exec.sh", O_WRONLY | O_CREAT, 0o755);
    must_open(dir, "pre.txt", O_WRONLY | O_CREAT, 0o666);
    let (name, mode) = (cstr("fopened.txt"), cstr("w"));
    let stream = unsafe { uio_fopen(dir, name.as_ptr(), mode.as_ptr()) };
    assert!(!stream.is_null(), "errno {}", errno_value());
    unsafe { uio_fclose(stream) };

    let newdir = cstr("newdir");
    unsafe {
        assert_eq!(uio_mkdir(dir, newdir.as_ptr(), 0o750), 0);
        assert_eq!(uio_mkdir(dir, newdir.as_ptr(), 0o777), -1);
        assert_eq!(errno_value(), libc::EEXIST);
    }

    // Copy-ups: a host file two directories deep, and an archive entry.
    must_open(dir, "deep/er/file.txt", O_WRONLY | O_APPEND, 0);
    must_open(dir, "z/entry.txt", O_WRONLY | O_APPEND, 0);

    // The unpacked copy of an archive entry and its generated directory.
    let scratch = cstr("scratch");
    unsafe { assert_eq!(uio_mkdir(dir, scratch.as_ptr(), 0o755), 0) };
    let temp = open_dir(repo, "/scratch");
    assert!(!temp.is_null());
    let other = cstr("z/other.txt");
    let access = unsafe { uio_getStdioAccess(dir, other.as_ptr(), O_RDONLY, temp) };
    assert!(!access.is_null(), "errno {}", errno_value());
    let unpacked = unsafe {
        PathBuf::from(
            CStr::from_ptr(uio_StdioAccessHandle_getPath(access))
                .to_str()
                .unwrap(),
        )
    };
    std::fs::write(
        base.join("unpacked.txt"),
        format!(
            "{:o} {:o}\n",
            bits(&unpacked),
            bits(unpacked.parent().unwrap())
        ),
    )
    .unwrap();
    unsafe { uio_releaseStdioAccess(access) };
}

fn run_child_under(umask: u32) -> TempDir {
    let work = TempDir::new().unwrap();
    let target = work.path().join("layout");
    create_layout(&target);
    let name = module_path!()
        .split_once("::")
        .map(|(_, rest)| format!("{rest}::child_makes_files_and_directories"))
        .unwrap();
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args(["--exact", &name, "--nocapture", "--test-threads=1"])
        .env(CHILD_ROOT, work.path());
    // SAFETY: only umask(2), which is async-signal-safe, runs between fork and
    // exec, and only in the child.
    unsafe {
        command.pre_exec(move || {
            libc::umask(umask as libc::mode_t);
            Ok(())
        });
    }
    let before = snapshot(&target.join("lower"));
    let zips_before = snapshot(&target.join("zips"));
    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "umask {umask:03o}: stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("1 passed"),
        "the child test did not run: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert_eq!(
        snapshot(&target.join("lower")),
        before,
        "a read-only mount was edited"
    );
    assert_eq!(
        snapshot(&target.join("zips")),
        zips_before,
        "the archive was edited"
    );
    work
}

fn assert_modes_under(umask: u32) {
    let work = run_child_under(umask);
    let rw = work.path().join("layout/rw");
    let masked = |requested: u32| requested & !umask;
    assert_eq!(
        bits(&rw.join("new.txt")),
        masked(0o640),
        "open O_CREAT 0640"
    );
    assert_eq!(
        bits(&rw.join("exec.sh")),
        masked(0o755),
        "open O_CREAT 0755"
    );
    assert_eq!(
        bits(&rw.join("pre.txt")),
        0o600,
        "an existing file keeps its mode"
    );
    assert_eq!(bits(&rw.join("fopened.txt")), masked(0o666), "uio_fopen");
    assert_eq!(
        bits(&rw.join("newdir")),
        masked(0o750),
        "mkdir 0750 (EEXIST kept it)"
    );

    assert_eq!(bits(&rw.join("deep")), masked(0o777), "copy-up parent");
    assert_eq!(bits(&rw.join("deep/er")), masked(0o777), "copy-up parent");
    assert_eq!(
        bits(&rw.join("deep/er/file.txt")),
        masked(0o640),
        "copy of a 0640 file"
    );
    assert_eq!(
        std::fs::read(rw.join("deep/er/file.txt")).unwrap(),
        b"lower bytes"
    );
    assert_eq!(bits(&rw.join("z")), masked(0o777), "archive copy-up parent");
    assert_eq!(
        bits(&rw.join("z/entry.txt")),
        masked(0o644),
        "copy of an archive entry"
    );
    assert_eq!(
        std::fs::read(rw.join("z/entry.txt")).unwrap(),
        b"entry bytes"
    );

    let unpacked = std::fs::read_to_string(work.path().join("unpacked.txt")).unwrap();
    assert_eq!(
        unpacked,
        format!("{:o} {:o}\n", masked(0o600), masked(0o700)),
        "unpacked file and its generated directory"
    );
}

#[test]
#[serial]
fn modes_follow_the_requested_bits_without_a_umask() {
    assert_modes_under(0o000);
}

#[test]
#[serial]
fn modes_follow_the_requested_bits_under_umask_022() {
    assert_modes_under(0o022);
}

#[test]
#[serial]
fn modes_follow_the_requested_bits_under_umask_077() {
    assert_modes_under(0o077);
}

struct Restore(PathBuf);

impl Drop for Restore {
    fn drop(&mut self) {
        let _ = std::fs::set_permissions(&self.0, PermissionsExt::from_mode(0o755));
    }
}

#[test]
#[serial]
fn a_failed_copy_up_leaves_nothing_behind() {
    let mut fx = Fx::new();
    let repo = fx.repo();
    let rw = fx.host(&[("keep.txt", b"keep")]);
    let lower = fx.host(&[("d/f.txt", b"lower")]);
    assert!(!mount_stdio(repo, "/", &lower, UIO_MOUNT_BOTTOM | UIO_MOUNT_RDONLY).is_null());
    assert!(!mount_stdio(repo, "/", &rw, UIO_MOUNT_TOP).is_null());
    let dir = open_dir(repo, "/");
    let lower_before = snapshot(&lower);
    let rw_before = snapshot(&rw);

    let _restore = Restore(rw.clone());
    std::fs::set_permissions(&rw, PermissionsExt::from_mode(0o555)).unwrap();
    assert_eq!(
        std::fs::create_dir(rw.join("probe"))
            .unwrap_err()
            .raw_os_error(),
        Some(libc::EACCES),
        "the fixture must really refuse writes (not running as root)"
    );

    let name = cstr("d/f.txt");
    let handle = unsafe { uio_open(dir, name.as_ptr(), O_WRONLY | O_APPEND, 0) };
    assert!(handle.is_null());
    assert_eq!(errno_value(), libc::EACCES);
    let made = cstr("d2");
    unsafe {
        assert_eq!(uio_mkdir(dir, made.as_ptr(), 0o755), -1);
    }
    assert_eq!(errno_value(), libc::EACCES);

    std::fs::set_permissions(&rw, PermissionsExt::from_mode(0o755)).unwrap();
    assert_eq!(
        snapshot(&rw),
        rw_before,
        "no partial directory or copy remains"
    );
    assert_eq!(
        snapshot(&lower),
        lower_before,
        "the read-only mount is untouched"
    );
}

/// The permission bits a handle reports are the host's, the way `fstat` gives
/// them. `copyFile` (and the original `uio_copyFile`) pass them on as the mode
/// of the copy, so a handle that reported none would create unreadable files
/// now that the mode of a create is honoured.
#[test]
#[serial]
fn fstat_reports_the_permission_bits_of_the_file() {
    let mut fx = Fx::new();
    let repo = fx.repo();
    let host = fx.host(&[("f.txt", b"abc")]);
    std::fs::set_permissions(host.join("f.txt"), PermissionsExt::from_mode(0o640)).unwrap();
    assert!(!mount_stdio(repo, "/", &host, UIO_MOUNT_TOP).is_null());
    let dir = open_dir(repo, "/");
    let name = cstr("f.txt");
    unsafe {
        let handle = uio_open(dir, name.as_ptr(), O_RDONLY, 0);
        assert!(!handle.is_null());
        let mut info: stat = std::mem::zeroed();
        assert_eq!(uio_fstat(handle, &mut info), 0);
        uio_close(handle);
        assert_eq!(
            u32::from(info.st_mode) & 0o170000,
            0o100000,
            "a regular file"
        );
        assert_eq!(u32::from(info.st_mode) & 0o7777, 0o640);
        assert_eq!(info.st_size, 3);
    }
}

/// `copyFile` creates the copy with the source's permission bits, so a
/// read-only source gives a read-only copy and a private one a private copy.
#[test]
#[serial]
fn copy_file_gives_the_copy_the_permission_bits_of_the_source() {
    use crate::io::ffi::{copyFile, uio_DirHandle as FfiDir};
    let mut fx = Fx::new();
    let repo = fx.repo();
    let host = fx.host(&[("private.txt", b"secret"), ("readonly.txt", b"fixed")]);
    std::fs::set_permissions(host.join("private.txt"), PermissionsExt::from_mode(0o600)).unwrap();
    std::fs::set_permissions(host.join("readonly.txt"), PermissionsExt::from_mode(0o400)).unwrap();
    assert!(!mount_stdio(repo, "/", &host, UIO_MOUNT_TOP).is_null());
    let dir = open_dir(repo, "/") as *mut FfiDir;
    for (source, copy, expected_bits, content) in [
        ("private.txt", "private-copy.txt", 0o600, &b"secret"[..]),
        ("readonly.txt", "readonly-copy.txt", 0o400, &b"fixed"[..]),
    ] {
        let (source_c, copy_c) = (cstr(source), cstr(copy));
        assert_eq!(
            unsafe { copyFile(dir, source_c.as_ptr(), dir, copy_c.as_ptr()) },
            0
        );
        assert_eq!(bits(&host.join(copy)), expected_bits, "{copy}");
        assert_eq!(std::fs::read(host.join(copy)).unwrap(), content, "{copy}");
    }
}
