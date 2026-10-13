//! `uio_stat` and `uio_access` on archive paths, `uio_stat` on host paths,
//! and how they agree with the handle `uio_fstat` describes.
//!
//! One metadata authority answers all of them: the central-directory record
//! the archive index reads when the archive is mounted (see `zip_stat`). The
//! archives are written by the `zip` crate and then patched in their central
//! directories where a foreign archiver's record is needed (the host that
//! made an entry, its raw attributes), and mounted through `uio_mountDir`.
//! Every call is an exported C ABI function.

use std::io::Write;
use std::os::unix::fs::{MetadataExt, PermissionsExt};

use serial_test::serial;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, DateTime};

use super::archive_stat_tests::{
    add_entry, bits, copy, entries, filled_stat, fstat_of, read_all, stat_bytes, Archive, Entry,
    MAY_2020, MAY_2020_UNIX,
};
use super::resolution_tests::{
    cstr, errno_value, mount_stdio, mount_zip, open_dir, Fx, RDONLY_TOP,
};
use super::*;

const SUB_TIME_UNIX: i64 = 1_623_745_812; // 2021-06-15 08:30:12 UTC
const DOS_HOSTS: [u8; 4] = [0, 6, 10, 14];

/// Rewrites the creator host and/or the external attributes of the central
/// directory header that names `name`, the way a foreign archiver would have
/// recorded them.
pub(super) fn patch_central(path: &Path, name: &str, host: Option<u8>, attributes: Option<u32>) {
    const CENTRAL_SIGNATURE: [u8; 4] = [0x50, 0x4b, 0x01, 0x02];
    const HOST_AT: usize = 5;
    const NAME_LENGTH_AT: usize = 28;
    const ATTRIBUTES_AT: usize = 38;
    const NAME_AT: usize = 46;
    let mut bytes = std::fs::read(path).unwrap();
    let mut patched = 0;
    for at in 0..bytes.len().saturating_sub(NAME_AT) {
        if bytes[at..at + 4] != CENTRAL_SIGNATURE {
            continue;
        }
        let length = usize::from(u16::from_le_bytes([
            bytes[at + NAME_LENGTH_AT],
            bytes[at + NAME_LENGTH_AT + 1],
        ]));
        if at + NAME_AT + length > bytes.len()
            || &bytes[at + NAME_AT..at + NAME_AT + length] != name.as_bytes()
        {
            continue;
        }
        if let Some(host) = host {
            bytes[at + HOST_AT] = host;
        }
        if let Some(attributes) = attributes {
            bytes[at + ATTRIBUTES_AT..at + ATTRIBUTES_AT + 4]
                .copy_from_slice(&attributes.to_le_bytes());
        }
        patched += 1;
    }
    assert_eq!(patched, 1, "exactly one central header names {name}");
    std::fs::write(path, bytes).unwrap();
}

fn stored(mode: u32) -> SimpleFileOptions {
    SimpleFileOptions::default()
        .compression_method(CompressionMethod::Stored)
        .unix_permissions(mode)
        .last_modified_time(DateTime::from_date_and_time(2021, 6, 15, 8, 30, 12).unwrap())
}

/// The archive every test but the host ones mounts: the declared entries, an
/// explicit directory `sub/`, `deep/x/leaf.bin` under directories no entry
/// names, entries a DOS/Windows archiver wrote, one with no attributes at
/// all, and the entries no file system view can show.
fn mixed_archive(path: &Path) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let mut zip = zip::ZipWriter::new(std::fs::File::create(path).unwrap());
    for entry in entries() {
        add_entry(&mut zip, &entry);
    }
    zip.add_directory("sub/", stored(0o750)).unwrap();
    for (name, data) in [
        ("sub/inner.bin", &b"inner"[..]),
        ("deep/x/leaf.bin", b"leaf"),
    ] {
        zip.start_file(name, stored(0o644)).unwrap();
        zip.write_all(data).unwrap();
    }
    for host in DOS_HOSTS {
        for name in ["rw.bin", "ro.bin"] {
            zip.start_file(format!("dos{host}-{name}"), stored(0o644))
                .unwrap();
            zip.write_all(b"dos").unwrap();
        }
        for name in ["dir/", "rodir/"] {
            zip.add_directory(format!("dos{host}-{name}"), stored(0o755))
                .unwrap();
        }
    }
    for name in ["unix-none.bin", "fifo", "odd.bin", "typed.bin"] {
        zip.start_file(name, stored(0o644)).unwrap();
        zip.write_all(name.as_bytes()).unwrap();
    }
    zip.add_symlink("link", "bytes.bin", stored(0o777)).unwrap();
    zip.finish().unwrap();

    for host in DOS_HOSTS {
        for (name, attributes) in [
            ("rw.bin", 0x20),
            ("ro.bin", 0x21),
            ("dir/", 0x10),
            ("rodir/", 0x11),
        ] {
            patch_central(
                path,
                &format!("dos{host}-{name}"),
                Some(host),
                Some(attributes),
            );
        }
    }
    patch_central(path, "unix-none.bin", None, Some(0));
    patch_central(path, "fifo", None, Some(0o010644 << 16));
    patch_central(path, "odd.bin", Some(7), None);
    patch_central(path, "typed.bin", Some(0), Some(0x10));
}

/// `uio_stat` of `path` (relative to "/") in `repo`: the struct, or errno.
fn stat_path(repo: *mut uio_Repository, path: &str) -> Result<stat, c_int> {
    let dir = open_dir(repo, "/");
    let path = cstr(path);
    let mut info = filled_stat();
    super::set_errno(0);
    let rc = unsafe { uio_stat(dir, path.as_ptr(), &mut info) };
    let errno = errno_value();
    unsafe { uio_closeDir(dir) };
    if rc == 0 {
        Ok(info)
    } else {
        Err(errno)
    }
}

fn access_path(repo: *mut uio_Repository, path: &str, mode: c_int) -> Result<(), c_int> {
    let dir = open_dir(repo, "/");
    let path = cstr(path);
    super::set_errno(0);
    let rc = unsafe { uio_access(dir, path.as_ptr(), mode) };
    let errno = errno_value();
    unsafe { uio_closeDir(dir) };
    if rc == 0 {
        Ok(())
    } else {
        Err(errno)
    }
}

fn mode_of(info: &stat) -> u32 {
    u32::from(info.st_mode)
}

fn assert_only_recorded_fields(info: &stat, what: &str) {
    assert_eq!(info.st_atime, 0, "{what}: atime");
    assert_eq!(info.st_ctime, 0, "{what}: ctime");
    assert_eq!(info.st_uid, 0, "{what}: uid");
    assert_eq!(info.st_gid, 0, "{what}: gid");
    assert_eq!(info.st_nlink, 0, "{what}: nlink");
}

#[test]
#[serial]
fn a_path_and_the_handle_opened_from_it_describe_an_entry_identically() {
    let archive = Archive::new(mixed_archive);
    for entry in entries() {
        let path = format!("z/{}", entry.name);
        let by_path = stat_path(archive.repo, &path).expect(entry.name);
        let handle = archive.open(entry.name);
        let by_handle = fstat_of(handle);
        assert_eq!(
            stat_bytes(&by_path),
            stat_bytes(&by_handle),
            "{}",
            entry.name
        );
        assert_eq!(by_path.st_size, entry.data.len() as i64, "{}", entry.name);
        assert_eq!(mode_of(&by_path), 0o100000 | entry.mode, "{}", entry.name);
        assert_eq!(by_path.st_mtime, entry.mtime, "{}", entry.name);
        assert_only_recorded_fields(&by_path, entry.name);
        unsafe { uio_close(handle) };
    }
    let first = &entries()[0];
    assert_eq!(first.mtime, MAY_2020_UNIX);
    assert_eq!(first.time, MAY_2020);
}

#[test]
#[serial]
fn directories_report_what_the_archive_records_or_implies() {
    let archive = Archive::new(mixed_archive);
    let sub = stat_path(archive.repo, "z/sub").unwrap();
    assert_eq!(mode_of(&sub), 0o040750);
    assert_eq!(sub.st_mtime, SUB_TIME_UNIX);
    assert_eq!(sub.st_size, 0);
    assert_only_recorded_fields(&sub, "sub");

    for implied in ["z/deep", "z/deep/x", "z"] {
        let info = stat_path(archive.repo, implied).expect(implied);
        assert_eq!(mode_of(&info), 0o040555, "{implied}");
        assert_eq!(info.st_mtime, 0, "{implied}");
        assert_eq!(info.st_size, 0, "{implied}");
        assert_only_recorded_fields(&info, implied);
    }
    let inner = stat_path(archive.repo, "z/sub/inner.bin").unwrap();
    assert_eq!((inner.st_size, mode_of(&inner)), (5, 0o100644));
    assert_eq!(
        stat_path(archive.repo, "z/deep/none").unwrap_err(),
        libc::ENOENT
    );
}

#[test]
#[serial]
fn dos_and_windows_archives_get_the_legacy_modes_from_every_dos_host() {
    let archive = Archive::new(mixed_archive);
    for host in DOS_HOSTS {
        for (name, mode) in [
            ("rw.bin", 0o100666),
            ("ro.bin", 0o100444),
            ("dir", 0o040777),
            ("rodir", 0o040555),
        ] {
            let path = format!("z/dos{host}-{name}");
            let info = stat_path(archive.repo, &path).unwrap_or_else(|e| panic!("{path}: {e}"));
            assert_eq!(mode_of(&info), mode, "{path}");
            assert_eq!(info.st_mtime, SUB_TIME_UNIX, "{path}");
            if name.ends_with(".bin") {
                let handle = archive.open(&format!("dos{host}-{name}"));
                assert_eq!(stat_bytes(&fstat_of(handle)), stat_bytes(&info), "{path}");
                unsafe { uio_close(handle) };
            }
        }
    }
}

#[test]
#[serial]
fn an_entry_with_no_attributes_at_all_is_an_ordinary_writable_file() {
    let archive = Archive::new(mixed_archive);
    let info = stat_path(archive.repo, "z/unix-none.bin").unwrap();
    assert_eq!(mode_of(&info), 0o100666);
    let handle = archive.open("unix-none.bin");
    assert_eq!(stat_bytes(&fstat_of(handle)), stat_bytes(&info));
    unsafe { uio_close(handle) };
}

#[test]
#[serial]
fn entries_no_file_system_view_can_show_have_no_stat_and_no_access() {
    let archive = Archive::new(mixed_archive);
    for name in ["link", "fifo", "odd.bin", "typed.bin"] {
        let path = format!("z/{name}");
        assert_eq!(
            stat_path(archive.repo, &path).unwrap_err(),
            libc::ENOTSUP,
            "{name}"
        );
        for mode in [F_OK, R_OK, X_OK] {
            assert_eq!(
                access_path(archive.repo, &path, mode).unwrap_err(),
                libc::ENOTSUP,
                "{name} mode {mode}"
            );
        }
        let handle = archive.open(name);
        let mut info = filled_stat();
        let before = stat_bytes(&info);
        assert_eq!(unsafe { uio_fstat(handle, &mut info) }, -1, "{name}");
        assert_eq!(stat_bytes(&info), before, "{name}: the caller's struct");
        unsafe { uio_close(handle) };
    }
    // A link is data like any other entry: its bytes, never its target.
    let handle = archive.open("link");
    assert_eq!(read_all(handle), b"bytes.bin");
    unsafe { uio_close(handle) };
}

#[test]
#[serial]
fn an_open_handle_keeps_its_entry_while_the_path_follows_the_mount_stack() {
    let archive = Archive::new(mixed_archive);
    let handle = archive.open("bytes.bin");
    let before = stat_bytes(&fstat_of(handle));
    assert_eq!(
        before,
        stat_bytes(&stat_path(archive.repo, "z/bytes.bin").unwrap())
    );

    assert_eq!(unsafe { uio_unmountDir(archive.zip_mount) }, 0);
    std::fs::remove_file(&archive.zip_path).unwrap();
    assert_eq!(
        stat_path(archive.repo, "z/bytes.bin").unwrap_err(),
        libc::ENOENT
    );
    assert_eq!(
        stat_bytes(&fstat_of(handle)),
        before,
        "the handle still has its entry"
    );

    let replacement = [Entry {
        name: "bytes.bin",
        data: b"new".to_vec(),
        method: CompressionMethod::Stored,
        mode: 0o700,
        time: MAY_2020,
        mtime: MAY_2020_UNIX,
    }];
    let mut zip = zip::ZipWriter::new(std::fs::File::create(&archive.zip_path).unwrap());
    add_entry(&mut zip, &replacement[0]);
    zip.finish().unwrap();
    let source = open_dir(archive.repo, "/src");
    assert!(!mount_zip(archive.repo, "/z", source, "pack.zip", RDONLY_TOP).is_null());

    let renewed = stat_path(archive.repo, "z/bytes.bin").unwrap();
    assert_eq!((renewed.st_size, mode_of(&renewed)), (3, 0o100700));
    assert_eq!(
        stat_bytes(&fstat_of(handle)),
        before,
        "and not the replacement's"
    );
    unsafe { uio_close(handle) };
}

#[test]
#[serial]
fn the_first_mount_in_priority_order_decides_and_repositories_stay_apart() {
    let archive = Archive::new(mixed_archive);
    let over = Entry {
        name: "sub",
        data: b"a file".to_vec(),
        method: CompressionMethod::Stored,
        mode: 0o600,
        time: MAY_2020,
        mtime: MAY_2020_UNIX,
    };
    let write_over = |path: &Path| {
        let mut zip = zip::ZipWriter::new(std::fs::File::create(path).unwrap());
        add_entry(&mut zip, &over);
        zip.start_file("shared.bin", stored(0o644)).unwrap();
        zip.write_all(b"shared!").unwrap();
        zip.finish().unwrap();
    };
    write_over(&archive.zip_path.with_file_name("over.zip"));
    let source = open_dir(archive.repo, "/src");
    let top = mount_zip(archive.repo, "/z", source, "over.zip", RDONLY_TOP);
    assert!(!top.is_null());

    let sub = stat_path(archive.repo, "z/sub").unwrap();
    assert_eq!(
        (sub.st_size, mode_of(&sub)),
        (6, 0o100600),
        "the file covers the directory"
    );
    assert_eq!(stat_path(archive.repo, "z/shared.bin").unwrap().st_size, 7);
    assert_eq!(unsafe { uio_unmountDir(top) }, 0);
    assert_eq!(
        mode_of(&stat_path(archive.repo, "z/sub").unwrap()),
        0o040750
    );
    assert_eq!(
        stat_path(archive.repo, "z/shared.bin").unwrap_err(),
        libc::ENOENT
    );

    // A second repository holding only the covering archive; the first does
    // not see it and it does not see the first's.
    let host = tempfile::TempDir::new().unwrap();
    write_over(&host.path().join("over.zip"));
    let other = unsafe { uio_openRepository(0) };
    assert!(!mount_stdio(other, "/src", host.path(), RDONLY_TOP).is_null());
    let source = open_dir(other, "/src");
    assert!(!mount_zip(other, "/z", source, "over.zip", RDONLY_TOP).is_null());
    assert_eq!(mode_of(&stat_path(other, "z/sub").unwrap()), 0o100600);
    assert_eq!(stat_path(other, "z/bytes.bin").unwrap_err(), libc::ENOENT);
    assert_eq!(
        mode_of(&stat_path(archive.repo, "z/sub").unwrap()),
        0o040750
    );
    unsafe { uio_closeRepository(other) };
}

#[test]
#[serial]
fn access_follows_the_recorded_metadata_and_changes_nothing() {
    let archive = Archive::new(mixed_archive);
    let archive_bytes = std::fs::read(&archive.zip_path).unwrap();
    for path in [
        "z/empty.bin",
        "z/sub",
        "z/sub/inner.bin",
        "z/deep",
        "z/deep/x",
        "z",
    ] {
        assert_eq!(access_path(archive.repo, path, F_OK), Ok(()), "{path} F_OK");
        assert_eq!(access_path(archive.repo, path, R_OK), Ok(()), "{path} R_OK");
    }
    for directory in ["z", "z/sub", "z/deep", "z/deep/x", "z/dos0-rodir"] {
        assert_eq!(
            access_path(archive.repo, directory, X_OK),
            Ok(()),
            "{directory}"
        );
    }
    assert_eq!(
        access_path(archive.repo, "z/text.txt", X_OK),
        Ok(()),
        "0755"
    );
    for file in ["z/bytes.bin", "z/dos0-rw.bin", "z/unix-none.bin"] {
        assert_eq!(
            access_path(archive.repo, file, X_OK),
            Err(libc::EACCES),
            "{file}"
        );
    }
    assert_eq!(
        access_path(archive.repo, "z/nothing", F_OK),
        Err(libc::ENOENT)
    );
    for path in ["z/text.txt", "z/sub", "z"] {
        assert_eq!(
            access_path(archive.repo, path, W_OK),
            Err(libc::EACCES),
            "{path} W_OK"
        );
    }
    assert!(
        std::fs::read_dir(&archive.out).unwrap().next().is_none(),
        "no probe copies anything up"
    );
    assert_eq!(std::fs::read(&archive.zip_path).unwrap(), archive_bytes);
}

#[test]
#[serial]
fn write_access_to_an_archive_entry_reaches_the_writable_mount_above_without_copying() {
    let archive = Archive::new(mixed_archive);
    let above = tempfile::TempDir::new().unwrap();
    assert!(!mount_stdio(archive.repo, "/z", above.path(), UIO_MOUNT_TOP).is_null());
    assert_eq!(access_path(archive.repo, "z/text.txt", W_OK), Ok(()));
    assert!(std::fs::read_dir(above.path()).unwrap().next().is_none());
    assert_eq!(access_path(archive.repo, "z/text.txt", X_OK), Ok(()));
}

#[test]
#[serial]
fn copy_file_gives_the_copy_the_mode_the_archive_defines() {
    let archive = Archive::new(mixed_archive);
    for (name, expected) in [
        ("private.bin", 0o600),
        ("dos0-ro.bin", 0o444),
        ("dos14-ro.bin", 0o444),
    ] {
        assert_eq!(copy(&archive, name, &format!("c-{name}")), 0, "{name}");
        assert_eq!(
            bits(&archive.out.join(format!("c-{name}"))),
            expected,
            "{name}"
        );
    }
    for name in ["dos10-rw.bin", "unix-none.bin"] {
        assert_eq!(copy(&archive, name, &format!("c-{name}")), 0, "{name}");
        let mode = bits(&archive.out.join(format!("c-{name}")));
        assert_eq!(
            mode & 0o600,
            0o600,
            "{name}: 0666 under the umask: {mode:o}"
        );
    }
    for name in ["link", "fifo", "odd.bin", "typed.bin"] {
        assert_eq!(copy(&archive, name, &format!("c-{name}")), -1, "{name}");
        assert!(!archive.out.join(format!("c-{name}")).exists(), "{name}");
    }
}

#[test]
#[serial]
fn a_damaged_archive_does_not_mount() {
    let mut fx = Fx::new();
    let repo = fx.repo();
    let zips = fx.host(&[]);
    let path = zips.join("pack.zip");
    mixed_archive(&path);
    assert!(!mount_stdio(repo, "/src", &zips, RDONLY_TOP).is_null());
    let source = open_dir(repo, "/src");

    let length = std::fs::metadata(&path).unwrap().len();
    std::fs::OpenOptions::new()
        .write(true)
        .open(&path)
        .unwrap()
        .set_len(length - 30)
        .unwrap();
    super::set_errno(0);
    assert!(mount_zip(repo, "/z", source, "pack.zip", RDONLY_TOP).is_null());
    assert_ne!(
        errno_value(),
        0,
        "a typed errno, not a panic across the ABI"
    );
}

#[test]
#[serial]
fn host_paths_report_the_real_metadata_and_agree_with_fstat() {
    let mut fx = Fx::new();
    let repo = fx.repo();
    let host = fx.host(&[("private.txt", b"abc"), ("frozen.txt", b"ice")]);
    std::fs::create_dir(host.join("folder")).unwrap();
    std::fs::set_permissions(host.join("private.txt"), PermissionsExt::from_mode(0o640)).unwrap();
    std::fs::set_permissions(host.join("frozen.txt"), PermissionsExt::from_mode(0o444)).unwrap();
    assert!(!mount_stdio(repo, "/", &host, UIO_MOUNT_TOP).is_null());

    for (name, mode) in [
        ("private.txt", 0o100640),
        ("frozen.txt", 0o100444),
        ("folder", 0o040755),
    ] {
        let real = std::fs::metadata(host.join(name)).unwrap();
        let info = stat_path(repo, name).expect(name);
        assert_eq!(mode_of(&info), real.mode(), "{name}: the host's own mode");
        let declared = if name == "folder" { real.mode() } else { mode };
        assert_eq!(
            mode_of(&info),
            declared,
            "{name}: neither collapsed to 0444 nor 0666"
        );
        assert_eq!(info.st_size as u64, real.len(), "{name}");
        assert_eq!(info.st_ino, real.ino(), "{name}");
        assert_eq!(info.st_mtime, real.mtime(), "{name}");
        assert_eq!(info.st_uid, real.uid(), "{name}");
        assert_eq!(info.st_gid, real.gid(), "{name}");
        assert_eq!(info.st_nlink as u64, real.nlink(), "{name}");

        // No byte of the caller's struct survives that the host did not set.
        let dir = open_dir(repo, "/");
        let path = cstr(name);
        let mut zeroed: stat = unsafe { std::mem::zeroed() };
        assert_eq!(unsafe { uio_stat(dir, path.as_ptr(), &mut zeroed) }, 0);
        assert_eq!(stat_bytes(&info), stat_bytes(&zeroed), "{name}");
    }

    let dir = open_dir(repo, "/");
    let name = cstr("private.txt");
    let handle = unsafe { uio_open(dir, name.as_ptr(), O_RDONLY, 0) };
    assert!(!handle.is_null());
    assert_eq!(
        stat_bytes(&fstat_of(handle)),
        stat_bytes(&stat_path(repo, "private.txt").unwrap())
    );
    unsafe { uio_close(handle) };
}
