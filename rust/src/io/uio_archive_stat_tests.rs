//! `uio_fstat` on handles opened from a ZIP archive, and `copyFile` from an
//! archive entry to a host directory.
//!
//! The original `zip_fstat` fills a zeroed `struct stat` from the entry: the
//! uncompressed size, the mode the archive recorded, and the DOS modification
//! time read as UTC (access and change times, owner and group stay 0 because
//! the archive holds none). The archives here are written by the `zip` crate
//! with declared modes and times and mounted through `uio_mountDir`; every
//! call is the exported C ABI function.

use std::io::{Read, Seek, SeekFrom, Write};
use std::os::unix::fs::{MetadataExt, PermissionsExt};

use serial_test::serial;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, DateTime};

use super::resolution_tests::{
    cstr, errno_value, mount_stdio, mount_zip, open_dir, Fx, RDONLY_TOP,
};
use super::*;

/// One archive entry as written: its bytes, how they are stored, the unix
/// permission bits and the DOS time, with the unix seconds that time is.
pub(super) struct Entry {
    pub(super) name: &'static str,
    pub(super) data: Vec<u8>,
    pub(super) method: CompressionMethod,
    pub(super) mode: u32,
    pub(super) time: (u16, u8, u8, u8, u8, u8),
    pub(super) mtime: i64,
}

pub(super) const MAY_2020: (u16, u8, u8, u8, u8, u8) = (2020, 5, 17, 13, 45, 30);
pub(super) const MAY_2020_UNIX: i64 = 1_589_723_130;

pub(super) fn entries() -> Vec<Entry> {
    vec![
        Entry {
            name: "empty.bin",
            data: Vec::new(),
            method: CompressionMethod::Stored,
            mode: 0o644,
            time: MAY_2020,
            mtime: MAY_2020_UNIX,
        },
        Entry {
            name: "bytes.bin",
            data: (0..=255u8).collect(),
            method: CompressionMethod::Stored,
            mode: 0o640,
            time: (1999, 12, 31, 23, 59, 58),
            mtime: 946_684_798,
        },
        Entry {
            name: "text.txt",
            data: b"the quick brown fox jumps over the lazy dog\n"
                .iter()
                .copied()
                .cycle()
                .take(4096)
                .collect(),
            method: CompressionMethod::Deflated,
            mode: 0o755,
            time: (2024, 2, 29, 0, 0, 2),
            mtime: 1_709_164_802,
        },
        Entry {
            name: "private.bin",
            data: (0..300u32).map(|i| (i * 7 % 251) as u8).collect(),
            method: CompressionMethod::Stored,
            mode: 0o600,
            time: MAY_2020,
            mtime: MAY_2020_UNIX,
        },
        Entry {
            name: "readonly.bin",
            data: [&[0u8, 1, 0, 0xFF, 0xFE][..], &[0x80u8; 28][..]].concat(),
            method: CompressionMethod::Deflated,
            mode: 0o400,
            time: MAY_2020,
            mtime: MAY_2020_UNIX,
        },
    ]
}

pub(super) fn add_entry(zip: &mut zip::ZipWriter<std::fs::File>, entry: &Entry) {
    let (year, month, day, hour, minute, second) = entry.time;
    let options = SimpleFileOptions::default()
        .compression_method(entry.method)
        .unix_permissions(entry.mode)
        .last_modified_time(
            DateTime::from_date_and_time(year, month, day, hour, minute, second).unwrap(),
        );
    zip.start_file(entry.name, options).unwrap();
    zip.write_all(&entry.data).unwrap();
}

fn write_archive(path: &Path, entries: &[Entry]) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let mut zip = zip::ZipWriter::new(std::fs::File::create(path).unwrap());
    for entry in entries {
        add_entry(&mut zip, entry);
    }
    zip.finish().unwrap();
}

/// An archive with an entry whose central directory records no attributes at
/// all (what a foreign archiver writes for data from a pipe), and a symbolic
/// link entry.
fn write_unrecorded_archive(path: &Path) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let mut zip = zip::ZipWriter::new(std::fs::File::create(path).unwrap());
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
    zip.start_file("noattrs.bin", options).unwrap();
    zip.write_all(b"no mode recorded").unwrap();
    zip.add_symlink("link", "noattrs.bin", options).unwrap();
    zip.finish().unwrap();
    super::archive_path_tests::patch_central(path, "noattrs.bin", None, Some(0));
}

/// A repository holding the archive at "/z" (read-only, as every real caller
/// mounts one) and a read-write host directory at "/out".
pub(super) struct Archive {
    _fx: Fx,
    pub(super) repo: *mut uio_Repository,
    pub(super) zip_path: PathBuf,
    pub(super) zip_mount: *mut uio_MountHandle,
    pub(super) out: PathBuf,
}

impl Archive {
    pub(super) fn new(write: impl FnOnce(&Path)) -> Self {
        Self::with_out_flags(write, UIO_MOUNT_TOP)
    }

    fn with_out_flags(write: impl FnOnce(&Path), out_flags: c_int) -> Self {
        let mut fx = Fx::new();
        let repo = fx.repo();
        let zips = fx.host(&[]);
        let zip_path = zips.join("pack.zip");
        write(&zip_path);
        let out = fx.host(&[]);
        assert!(!mount_stdio(repo, "/src", &zips, RDONLY_TOP).is_null());
        let source = open_dir(repo, "/src");
        let zip_mount = mount_zip(repo, "/z", source, "pack.zip", RDONLY_TOP);
        assert!(!zip_mount.is_null(), "errno {}", errno_value());
        assert!(!mount_stdio(repo, "/out", &out, out_flags).is_null());
        Archive {
            _fx: fx,
            repo,
            zip_path,
            zip_mount,
            out,
        }
    }

    pub(super) fn open(&self, name: &str) -> *mut uio_Handle {
        let dir = open_dir(self.repo, "/z");
        let name = cstr(name);
        let handle = unsafe { uio_open(dir, name.as_ptr(), O_RDONLY, 0) };
        assert!(!handle.is_null(), "{name:?}: errno {}", errno_value());
        handle
    }
}

const FILLER: u8 = 0xAB;

/// A `stat` every byte of which the caller has set to a recognisable value.
pub(super) fn filled_stat() -> stat {
    let mut info: stat = unsafe { std::mem::zeroed() };
    unsafe { ptr::write_bytes(&mut info as *mut stat as *mut u8, FILLER, size_of::<stat>()) };
    info
}

pub(super) fn stat_bytes(info: &stat) -> Vec<u8> {
    unsafe { slice::from_raw_parts(info as *const stat as *const u8, size_of::<stat>()) }.to_vec()
}

pub(super) fn fstat_of(handle: *mut uio_Handle) -> stat {
    let mut info = filled_stat();
    let rc = unsafe { uio_fstat(handle, &mut info) };
    assert_eq!(rc, 0, "errno {}", errno_value());
    info
}

pub(super) fn read_all(handle: *mut uio_Handle) -> Vec<u8> {
    let mut bytes = Vec::new();
    unsafe { (*handle).lock().unwrap().read_to_end(&mut bytes).unwrap() };
    bytes
}

#[test]
#[serial]
fn zip_entry_fstat_reports_length_mode_and_time() {
    let archive = Archive::new(|path| write_archive(path, &entries()));
    for entry in entries() {
        let handle = archive.open(entry.name);
        let info = fstat_of(handle);
        assert_eq!(
            info.st_size,
            entry.data.len() as i64,
            "{}: size",
            entry.name
        );
        assert_eq!(
            u32::from(info.st_mode) & 0o170000,
            0o100000,
            "{}: regular file",
            entry.name
        );
        assert_eq!(
            u32::from(info.st_mode) & 0o7777,
            entry.mode,
            "{}: permission bits",
            entry.name
        );
        assert_eq!(info.st_mtime, entry.mtime, "{}: mtime", entry.name);
        assert_eq!(
            info.st_atime, 0,
            "{}: the archive holds no atime",
            entry.name
        );
        assert_eq!(
            info.st_ctime, 0,
            "{}: the archive holds no ctime",
            entry.name
        );
        assert_eq!(info.st_uid, 0, "{}: uid", entry.name);
        assert_eq!(info.st_gid, 0, "{}: gid", entry.name);
        unsafe { uio_close(handle) };
    }
}

#[test]
#[serial]
fn zip_entry_fstat_agrees_with_the_bytes_read() {
    let archive = Archive::new(|path| write_archive(path, &entries()));
    for entry in entries() {
        let handle = archive.open(entry.name);
        let size = fstat_of(handle).st_size;
        let end = (*unsafe { &*handle })
            .lock()
            .unwrap()
            .seek(SeekFrom::End(0));
        assert_eq!(end.unwrap(), size as u64, "{}: end of entry", entry.name);
        (*unsafe { &*handle })
            .lock()
            .unwrap()
            .seek(SeekFrom::Start(0))
            .unwrap();
        let bytes = read_all(handle);
        assert_eq!(bytes.len() as i64, size, "{}: bytes read", entry.name);
        assert_eq!(bytes, entry.data, "{}: content", entry.name);
        unsafe { uio_close(handle) };
    }
}

#[test]
#[serial]
fn fstat_on_an_open_entry_survives_unmount_and_host_removal() {
    let archive = Archive::new(|path| write_archive(path, &entries()));
    let entry = &entries()[1];
    let handle = archive.open(entry.name);
    let before = stat_bytes(&fstat_of(handle));

    assert_eq!(unsafe { uio_unmountDir(archive.zip_mount) }, 0);
    std::fs::remove_file(&archive.zip_path).unwrap();

    let after = fstat_of(handle);
    assert_eq!(stat_bytes(&after), before);
    assert_eq!(after.st_size, 256);
    assert_eq!(u32::from(after.st_mode) & 0o7777, 0o640);
    assert_eq!(after.st_mtime, 946_684_798);
    assert_eq!(read_all(handle), entry.data);
    unsafe { uio_close(handle) };
}

#[test]
#[serial]
fn file_block_length_agrees_with_fstat() {
    for entry in entries() {
        let archive = Archive::new(|path| write_archive(path, &entries()));
        let handle = archive.open(entry.name);
        let size = fstat_of(handle).st_size as usize;
        assert_eq!(size, entry.data.len(), "{}", entry.name);

        // The block keeps the entry readable after the caller closed its
        // handle and the archive left the stack and the host.
        assert_eq!(unsafe { uio_unmountDir(archive.zip_mount) }, 0);
        std::fs::remove_file(&archive.zip_path).unwrap();
        unsafe {
            let block = uio_openFileBlock(handle);
            assert!(!block.is_null(), "{}: errno {}", entry.name, errno_value());
            uio_close(handle);

            if size > 0 {
                let mut buffer = ptr::null_mut::<c_char>();
                let got = uio_accessFileBlock(block, 0, size, &mut buffer);
                assert_eq!(got, size as isize, "{}: whole block", entry.name);
                assert_eq!(
                    slice::from_raw_parts(buffer as *const u8, size),
                    &entry.data[..],
                    "{}: block bytes",
                    entry.name
                );
            }
            let mut beyond = ptr::dangling_mut::<c_char>();
            assert_eq!(
                uio_accessFileBlock(block, size as off_t, 1, &mut beyond),
                0,
                "{}: nothing past the length fstat reports",
                entry.name
            );
            assert_eq!(uio_closeFileBlock(block), 0);
        }
    }
}

#[test]
#[serial]
fn an_entry_that_records_no_attributes_is_an_ordinary_writable_file() {
    let archive = Archive::new(write_unrecorded_archive);
    let handle = archive.open("noattrs.bin");
    let info = fstat_of(handle);
    assert_eq!(u32::from(info.st_mode), 0o100666);
    assert_eq!(info.st_size, 16);
    assert_eq!(read_all(handle), b"no mode recorded");
    unsafe { uio_close(handle) };
}

#[test]
#[serial]
fn a_symbolic_link_entry_has_no_stat_and_is_not_followed() {
    let archive = Archive::new(write_unrecorded_archive);
    let handle = archive.open("link");
    let mut info = filled_stat();
    let before = stat_bytes(&info);
    super::set_errno(0);
    assert_eq!(unsafe { uio_fstat(handle, &mut info) }, -1);
    assert_eq!(errno_value(), libc::ENOTSUP);
    assert_eq!(stat_bytes(&info), before, "the caller's struct");
    assert_eq!(read_all(handle), b"noattrs.bin", "the link's own bytes");
    unsafe { uio_close(handle) };
}

#[test]
#[serial]
fn fstat_arguments_and_host_files_keep_their_contract() {
    let archive = Archive::new(|path| write_archive(path, &entries()));
    let handle = archive.open("bytes.bin");
    let mut info = filled_stat();
    unsafe {
        assert_eq!(uio_fstat(ptr::null_mut(), &mut info), -1);
        assert_eq!(uio_fstat(handle, ptr::null_mut()), -1);
    }
    assert_eq!(fstat_of(handle).st_size, 256, "the handle is still usable");
    unsafe { uio_close(handle) };

    let mut fx = Fx::new();
    let repo = fx.repo();
    let host = fx.host(&[("host.bin", b"0123456789")]);
    std::fs::set_permissions(host.join("host.bin"), PermissionsExt::from_mode(0o640)).unwrap();
    assert!(!mount_stdio(repo, "/", &host, UIO_MOUNT_TOP).is_null());
    let dir = open_dir(repo, "/");
    let name = cstr("host.bin");
    unsafe {
        let handle = uio_open(dir, name.as_ptr(), O_RDONLY, 0);
        assert!(!handle.is_null());
        let mut info = filled_stat();
        assert_eq!(uio_fstat(handle, &mut info), 0);
        uio_close(handle);
        let real = std::fs::metadata(host.join("host.bin")).unwrap();
        assert_eq!(info.st_size as u64, real.len());
        assert_eq!(u32::from(info.st_mode), real.permissions().mode());
        assert_eq!(info.st_ino, real.ino());
        assert_eq!(info.st_mtime, real.mtime());
        assert_eq!(info.st_uid, real.uid());
        assert_eq!(info.st_nlink as u64, real.nlink());
    }
}

pub(super) fn bits(path: &Path) -> u32 {
    std::fs::metadata(path).unwrap().permissions().mode() & 0o7777
}

pub(super) fn copy(archive: &Archive, from: &str, to: &str) -> c_int {
    use crate::io::ffi::{copyFile, uio_DirHandle as FfiDir};
    let source = open_dir(archive.repo, "/z") as *mut FfiDir;
    let target = open_dir(archive.repo, "/out") as *mut FfiDir;
    let (from, to) = (cstr(from), cstr(to));
    unsafe { copyFile(source, from.as_ptr(), target, to.as_ptr()) }
}

#[test]
#[serial]
fn copy_file_from_an_archive_entry_to_a_writable_directory() {
    let archive = Archive::new(|path| write_archive(path, &entries()));
    for entry in entries() {
        let copy_name = format!("copy-of-{}", entry.name);
        assert_eq!(copy(&archive, entry.name, &copy_name), 0, "{}", entry.name);
        let copied = archive.out.join(&copy_name);
        assert_eq!(
            std::fs::read(&copied).unwrap(),
            entry.data,
            "{}",
            entry.name
        );
        assert!(std::fs::metadata(&copied).unwrap().is_file());
        let mode = bits(&copied);
        assert_ne!(mode, 0, "{}: a copy nobody can read", entry.name);
        if entry.mode & 0o077 == 0 {
            assert_eq!(
                mode, entry.mode,
                "{}: owner-only bits survive any umask",
                entry.name
            );
        }
    }

    // The destination is created exclusively: a second copy to the same name
    // is refused and leaves the first byte for byte.
    assert_eq!(copy(&archive, "bytes.bin", "twice.bin"), 0);
    assert_eq!(copy(&archive, "text.txt", "twice.bin"), -1);
    assert_eq!(
        std::fs::read(archive.out.join("twice.bin")).unwrap(),
        entries()[1].data
    );

    // Teardown: both mounts and the archive are gone, the copies stay.
    assert_eq!(unsafe { uio_unmountDir(archive.zip_mount) }, 0);
    std::fs::remove_file(&archive.zip_path).unwrap();
    assert_eq!(
        std::fs::read(archive.out.join("copy-of-private.bin")).unwrap(),
        entries()[3].data
    );
    assert_eq!(
        std::fs::read(archive.out.join("twice.bin")).unwrap(),
        entries()[1].data
    );
}

#[test]
#[serial]
fn copy_file_to_a_read_only_destination_is_refused() {
    let archive = Archive::with_out_flags(|path| write_archive(path, &entries()), RDONLY_TOP);
    assert_eq!(copy(&archive, "private.bin", "copy.bin"), -1);
    assert!(
        std::fs::read_dir(&archive.out).unwrap().next().is_none(),
        "no file appears in a read-only destination"
    );
    let handle = archive.open("private.bin");
    assert_eq!(read_all(handle), entries()[3].data, "the source is intact");
    unsafe { uio_close(handle) };
}

#[test]
#[serial]
fn copy_file_of_an_entry_without_attributes_gets_the_defined_mode_and_a_link_creates_nothing() {
    let archive = Archive::new(write_unrecorded_archive);
    assert_eq!(copy(&archive, "noattrs.bin", "copy.bin"), 0);
    let copied = archive.out.join("copy.bin");
    assert_eq!(std::fs::read(&copied).unwrap(), b"no mode recorded");
    let mode = bits(&copied);
    assert_eq!(
        mode & 0o600,
        0o600,
        "0666 under the umask keeps the owner's bits: {mode:o}"
    );
    assert_eq!(copy(&archive, "link", "link-copy"), -1);
    assert!(!archive.out.join("link-copy").exists());
}
