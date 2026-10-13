//! The fixture world of `fixture-spec.md`: a repository over real temporary
//! directories and a ZIP archive, driven through the public UIO calls.
//!
//! The mount registry is process-wide, so every test that builds a [`World`]
//! is `#[serial]`.

use std::ffi::CString;
use std::fs;
use std::io::Write;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::ptr::{self, NonNull};

use tempfile::TempDir;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, DateTime, ZipWriter};

use super::tokens::make_args;
use super::{CommandStatus, DebugSession, Streams};
use crate::io::uio_bridge::{
    uio_MountHandle, uio_Repository, uio_closeRepository, uio_mountDir, uio_openRepository,
    uio_unmountDir,
};

pub(super) const MOUNT_RDONLY: i32 = 1 << 1;
pub(super) const MOUNT_BOTTOM: i32 = 0 << 2;
pub(super) const MOUNT_TOP: i32 = 1 << 2;
pub(super) const MOUNT_BELOW: i32 = 2 << 2;
const FSTYPE_STDIO: i32 = 1;
const FSTYPE_ZIP: i32 = 2;

pub(super) const RAW_FILE: &[u8] = b"raw\xff\xfe.dat";
pub(super) const RAW_DIR: &[u8] = b"d\xc3\xa9\xff";
/// A mount point that is not UTF-8: virtual names are bytes on every host.
pub(super) const RAW_MOUNT_POINT: &[u8] = b"/rawmnt\xfe\xff";

pub(super) fn big_bytes() -> Vec<u8> {
    (0..200_000usize).map(|i| (i % 251) as u8).collect()
}

fn c_string(bytes: &[u8]) -> CString {
    CString::new(bytes).expect("fixture strings have no NUL")
}

/// What one command printed and returned.
pub(super) struct Outcome {
    pub(super) status: CommandStatus,
    pub(super) out: Vec<u8>,
    pub(super) err: Vec<u8>,
}

impl Outcome {
    pub(super) fn out_text(&self) -> String {
        String::from_utf8_lossy(&self.out).into_owned()
    }

    pub(super) fn err_text(&self) -> String {
        String::from_utf8_lossy(&self.err).into_owned()
    }
}

/// One repository with these mounts, in the order made:
/// `/` host `host` (rw), `/ro` host `ro` (read-only), `/pack` the fixture ZIP
/// (read-only), `/tmp` host `tmp` (rw), `/below` host `below` made BELOW
/// `/ro`, `/bottom` host `bottom` made BOTTOM.
pub(super) struct World {
    repository: *mut uio_Repository,
    pub(super) host: TempDir,
    pub(super) ro: TempDir,
    pub(super) zip_dir: TempDir,
    pub(super) tmp: TempDir,
    pub(super) below: TempDir,
    pub(super) bottom: TempDir,
    /// Host directory shown at the virtual mount point [`RAW_MOUNT_POINT`].
    pub(super) raw_mount: TempDir,
    /// Whether the host file system can hold the names [`RAW_FILE`] and
    /// [`RAW_DIR`]; macOS refuses names that are not UTF-8 (EILSEQ).
    pub(super) raw_host: bool,
    pub(super) ro_mount: *mut uio_MountHandle,
}

impl World {
    pub(super) fn new() -> Self {
        let mut world = Self::unmounted();
        world.fill_host();
        fs::write(world.ro.path().join("fixed.txt"), b"fixed").unwrap();
        fs::write(world.raw_mount.path().join("inner.txt"), b"raw mount").unwrap();
        let archive = world.zip_dir.path().join("pack.zip");
        write_zip(&archive);
        world.mount_all(&archive);
        world
    }

    fn unmounted() -> Self {
        // SAFETY: opening a repository has no preconditions.
        let repository = unsafe { uio_openRepository(0) };
        Self {
            repository,
            host: tempdir(),
            ro: tempdir(),
            zip_dir: tempdir(),
            tmp: tempdir(),
            below: tempdir(),
            bottom: tempdir(),
            raw_mount: tempdir(),
            raw_host: false,
            ro_mount: ptr::null_mut(),
        }
    }

    /// A repository with only `/` mounted, over `host`.
    pub(super) fn bare() -> Self {
        let world = Self::unmounted();
        mount(
            world.repository,
            "/",
            FSTYPE_STDIO,
            world.host.path(),
            MOUNT_TOP,
            ptr::null_mut(),
        );
        world
    }

    /// A repository where nothing is writable: `host` at `/` read-only, and
    /// the fixture archive at `/pack`.
    pub(super) fn read_only() -> Self {
        let mut world = Self::unmounted();
        world.fill_host();
        let archive = world.zip_dir.path().join("pack.zip");
        write_zip(&archive);
        let none = ptr::null_mut();
        let repository = world.repository;
        mount(
            repository,
            "/",
            FSTYPE_STDIO,
            world.host.path(),
            MOUNT_TOP | MOUNT_RDONLY,
            none,
        );
        mount(
            repository,
            "/pack",
            FSTYPE_ZIP,
            &archive,
            MOUNT_TOP | MOUNT_RDONLY,
            none,
        );
        world
    }

    /// The mounts in the order of the module documentation.
    fn mount_all(&mut self, archive: &Path) {
        let repository = self.repository;
        let none = ptr::null_mut();
        mount(
            repository,
            "/",
            FSTYPE_STDIO,
            self.host.path(),
            MOUNT_TOP,
            none,
        );
        self.ro_mount = mount(
            repository,
            "/ro",
            FSTYPE_STDIO,
            self.ro.path(),
            MOUNT_TOP | MOUNT_RDONLY,
            none,
        );
        mount(
            repository,
            "/pack",
            FSTYPE_ZIP,
            archive,
            MOUNT_TOP | MOUNT_RDONLY,
            none,
        );
        mount(
            repository,
            "/tmp",
            FSTYPE_STDIO,
            self.tmp.path(),
            MOUNT_TOP,
            none,
        );
        mount(
            repository,
            "/below",
            FSTYPE_STDIO,
            self.below.path(),
            MOUNT_BELOW,
            self.ro_mount,
        );
        mount(
            repository,
            "/bottom",
            FSTYPE_STDIO,
            self.bottom.path(),
            MOUNT_BOTTOM,
            none,
        );
        mount_bytes(
            repository,
            RAW_MOUNT_POINT,
            self.raw_mount.path(),
            MOUNT_TOP,
        );
    }

    fn fill_host(&mut self) {
        let host = self.host.path().to_path_buf();
        let host = host.as_path();
        fs::write(host.join("hello.txt"), b"hello\n").unwrap();
        fs::write(host.join("empty.bin"), b"").unwrap();
        fs::write(host.join("all.bin"), (0..=255u8).collect::<Vec<_>>()).unwrap();
        fs::write(host.join("big.bin"), big_bytes()).unwrap();
        fs::create_dir(host.join("sub")).unwrap();
        fs::write(host.join("sub/in.txt"), b"inner").unwrap();
        self.raw_host = create_raw_names(host);
        fs::write(host.join("script_exit3.sh"), b"exit 3").unwrap();
        fs::write(host.join("script_kill.sh"), b"kill -9 $$").unwrap();
        fs::set_permissions(host.join("hello.txt"), fs::Permissions::from_mode(0o644)).unwrap();
    }

    pub(super) fn repository(&self) -> NonNull<uio_Repository> {
        NonNull::new(self.repository).expect("a repository was opened")
    }

    pub(super) fn raw_repository(&self) -> *mut uio_Repository {
        self.repository
    }

    pub(super) fn session(&self) -> DebugSession {
        // SAFETY: the repository outlives the session in every test; a
        // `World` is dropped last.
        unsafe { DebugSession::open(self.repository()) }.expect("the root opens")
    }

    pub(super) fn unmount_ro(&mut self) {
        // SAFETY: the handle is live and unmounted once.
        assert_eq!(unsafe { uio_unmountDir(self.ro_mount) }, 0);
        self.ro_mount = ptr::null_mut();
    }

    /// Runs one command line and captures what it wrote.
    pub(super) fn run(&self, session: &mut DebugSession, line: &str) -> Outcome {
        run_bytes(session, line.as_bytes())
    }
}

impl Drop for World {
    fn drop(&mut self) {
        // SAFETY: the repository is live and closed once; its mounts go with it.
        unsafe { uio_closeRepository(self.repository) };
    }
}

/// Runs the command line `line` (raw bytes) on `session`.
pub(super) fn run_bytes(session: &mut DebugSession, line: &[u8]) -> Outcome {
    let argv = make_args(line).expect("fixture lines have no NUL");
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let status = session
        .execute(
            &argv,
            &mut Streams {
                out: &mut out,
                err: &mut err,
            },
        )
        .expect("writing to memory cannot fail");
    Outcome { status, out, err }
}

/// The fixture archive: `a.txt` (deflated, 4096 bytes, DOS time 2020-01-02
/// 03:04:06), `dir/b.txt`, `script_exit5.sh`.
fn write_zip(path: &Path) {
    let time = DateTime::from_date_and_time(2020, 1, 2, 3, 4, 6).unwrap();
    let options = |method| {
        SimpleFileOptions::default()
            .compression_method(method)
            .last_modified_time(time)
    };
    let mut zip = ZipWriter::new(fs::File::create(path).unwrap());
    zip.start_file("a.txt", options(CompressionMethod::Deflated))
        .unwrap();
    zip.write_all(&[b'a'; 4096]).unwrap();
    zip.start_file("dir/b.txt", options(CompressionMethod::Stored))
        .unwrap();
    zip.write_all(b"bee").unwrap();
    zip.start_file("script_exit5.sh", options(CompressionMethod::Stored))
        .unwrap();
    zip.write_all(b"exit 5").unwrap();
    zip.finish().unwrap();
}

/// Creates [`RAW_FILE`] and [`RAW_DIR`] on the host. False only on Apple
/// file systems, which refuse a name that is not UTF-8.
fn create_raw_names(host: &Path) -> bool {
    let made = fs::write(host.join(path_from(RAW_FILE)), b"rawname")
        .and_then(|()| fs::create_dir(host.join(path_from(RAW_DIR))))
        .and_then(|()| fs::write(host.join(path_from(RAW_DIR)).join("x"), b"x"));
    match made {
        Ok(()) => true,
        Err(error) => {
            assert_eq!(error.raw_os_error(), Some(libc::EILSEQ), "{error}");
            if cfg!(not(target_vendor = "apple")) {
                panic!("only Apple file systems refuse byte names");
            }
            false
        }
    }
}

fn tempdir() -> TempDir {
    tempfile::Builder::new()
        .prefix("uio-debug-")
        .tempdir()
        .unwrap()
}

fn path_from(bytes: &[u8]) -> PathBuf {
    PathBuf::from(std::ffi::OsStr::from_bytes(bytes))
}

fn mount_bytes(repository: *mut uio_Repository, point: &[u8], host: &Path, flags: i32) {
    mount_at(
        repository,
        point,
        FSTYPE_STDIO,
        host,
        flags,
        ptr::null_mut(),
    );
}

fn mount(
    repository: *mut uio_Repository,
    point: &str,
    fs_type: i32,
    host: &Path,
    flags: i32,
    relative: *mut uio_MountHandle,
) -> *mut uio_MountHandle {
    mount_at(repository, point.as_bytes(), fs_type, host, flags, relative)
}

fn mount_at(
    repository: *mut uio_Repository,
    point: &[u8],
    fs_type: i32,
    host: &Path,
    flags: i32,
    relative: *mut uio_MountHandle,
) -> *mut uio_MountHandle {
    let (point, host) = (c_string(point), c_string(host.as_os_str().as_bytes()));
    // SAFETY: live repository, C strings, no source directory.
    let handle = unsafe {
        uio_mountDir(
            repository,
            point.as_ptr(),
            fs_type,
            ptr::null_mut(),
            ptr::null(),
            host.as_ptr(),
            ptr::null_mut(),
            flags,
            relative,
        )
    };
    assert!(!handle.is_null(), "fixture mount {point:?} failed");
    handle
}

/// How many descriptors of this process refer to `path`. A handle a command
/// leaked shows up here; the path is unique to the test.
pub(super) fn open_descriptors_for(path: &Path) -> usize {
    let wanted = fs::canonicalize(path).unwrap();
    (0..4096)
        .filter(|&fd| descriptor_path(fd).is_some_and(|open| open == wanted))
        .count()
}

#[cfg(target_os = "macos")]
fn descriptor_path(fd: i32) -> Option<PathBuf> {
    let mut buffer = [0 as std::ffi::c_char; libc::PATH_MAX as usize];
    // SAFETY: F_GETPATH writes a NUL-terminated path into a PATH_MAX buffer.
    if unsafe { libc::fcntl(fd, libc::F_GETPATH, buffer.as_mut_ptr()) } == -1 {
        return None;
    }
    // SAFETY: NUL terminated by the successful call.
    let bytes = unsafe { std::ffi::CStr::from_ptr(buffer.as_ptr()) }.to_bytes();
    Some(path_from(bytes))
}

#[cfg(target_os = "linux")]
fn descriptor_path(fd: i32) -> Option<PathBuf> {
    fs::read_link(format!("/proc/self/fd/{fd}")).ok()
}

/// Runs the test `module::name` again in a child process with `environment`
/// set on the child only, and requires that it ran and passed there. In the
/// child, `body` runs; in the parent it never does.
pub(super) fn in_child_with_env(
    module: &str,
    name: &str,
    environment: &[(&str, &str)],
    body: impl FnOnce(),
) {
    const MARKER: &str = "UQM_DEBUG_ENV_CHILD";
    let (_crate_name, in_crate) = module
        .split_once("::")
        .expect("module_path! starts with the crate name");
    let test_path = format!("{in_crate}::{name}");
    if std::env::var(MARKER).as_deref() == Ok(test_path.as_str()) {
        body();
        return;
    }
    let mut command = std::process::Command::new(std::env::current_exe().unwrap());
    command
        .args(["--exact", &test_path, "--nocapture", "--test-threads=1"])
        .env(MARKER, &test_path);
    for (key, value) in environment {
        command.env(key, value);
    }
    let output = command.output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success() && stdout.contains("1 passed"),
        "child {test_path} failed: {:?}\nstdout: {stdout}\nstderr: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
}
