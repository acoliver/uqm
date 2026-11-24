//! The mount table behind `Files`: priority order across directories and
//! archives, mount bookkeeping, retained directories and file operations.

use super::*;
use crate::host::Files;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;
use tempfile::TempDir;

/// A fresh [`Vfs`] with the temporary directories and archives it mounts.
struct Stack {
    vfs: Vfs,
    root: Dir,
    scratch: Vec<TempDir>,
}

impl Stack {
    fn new() -> Self {
        let vfs = Vfs::new();
        let root = vfs.open_dir(Path::new("/"));
        Stack {
            vfs,
            root,
            scratch: Vec::new(),
        }
    }

    fn directory(&mut self, files: &[(&str, &[u8])]) -> TempDir {
        let dir = TempDir::new().unwrap();
        for (name, data) in files {
            std::fs::write(dir.path().join(name), data).unwrap();
        }
        dir
    }

    fn mount_dir(
        &mut self,
        files: &[(&str, &[u8])],
        placement: Placement,
        read_only: bool,
    ) -> (MountId, std::path::PathBuf) {
        let dir = self.directory(files);
        let path = dir.path().to_path_buf();
        let id = self.vfs.mount_dir("/", &path, placement, read_only);
        self.scratch.push(dir);
        (id, path)
    }

    fn mount_zip(&mut self, files: &[(&str, &[u8])], placement: Placement) -> MountId {
        let dir = TempDir::new().unwrap();
        write_zip(&dir.path().join("pack.zip"), files);
        let source = self.vfs.open_host_dir(dir.path());
        let id = self
            .vfs
            .mount_zip("/", &source, Path::new("pack.zip"), placement, true)
            .unwrap();
        self.scratch.push(dir);
        id
    }

    fn read(&self, name: &str) -> Option<Vec<u8>> {
        self.root.read(Path::new(name)).ok()
    }
}

fn write_zip(path: &Path, files: &[(&str, &[u8])]) {
    let mut zip = zip::ZipWriter::new(std::fs::File::create(path).unwrap());
    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    for (name, data) in files {
        zip.start_file(*name, options).unwrap();
        zip.write_all(data).unwrap();
    }
    zip.finish().unwrap();
}

#[test]
fn files_port_roundtrips_and_removes_bytes() {
    let temp = TempDir::new().unwrap();
    let files = crate::host::platform().disk();
    let path = temp.path().join("slot.sav");
    let mut output = files.open_write(&path).unwrap();
    output.write_all(b"\0save\xff\n").unwrap();
    output.flush().unwrap();
    drop(output);
    assert_eq!(files.stat(&path).unwrap().len, 7);
    let mut input = files.open_read(&path).unwrap();
    assert_eq!(input.seek(SeekFrom::End(-2)).unwrap(), 5);
    let mut tail = Vec::new();
    input.read_to_end(&mut tail).unwrap();
    assert_eq!(tail, b"\xff\n");
    assert_eq!(files.list(temp.path()).unwrap(), ["slot.sav"]);
    files.remove(&path).unwrap();
    assert_eq!(
        files.open_read(&path).err().unwrap().kind(),
        std::io::ErrorKind::NotFound
    );
}

#[test]
fn retained_config_writes_ignore_later_content_mounts() {
    let mut stack = Stack::new();
    let (_, config) = stack.mount_dir(&[], Placement::Top, false);
    let files = stack.vfs.open_dir(Path::new("/"));
    let (_, content) = stack.mount_dir(&[], Placement::Top, true);
    let path = Path::new("uqmsave.01");
    let mut output = files.open_write(path).unwrap();
    output.write_all(b"save bytes").unwrap();
    output.finish().unwrap();
    assert_eq!(std::fs::read(config.join(path)).unwrap(), b"save bytes");
    assert!(!content.join(path).exists());
    assert_eq!(files.read(path).unwrap(), b"save bytes");
    files.remove(path).unwrap();
    assert!(!config.join(path).exists());
}

#[test]
#[cfg(target_os = "linux")]
fn files_listing_preserves_lossy_native_names() {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;

    let temp = TempDir::new().unwrap();
    let name = OsString::from_vec(b"ship-\xff.ogg".to_vec());
    std::fs::write(temp.path().join(name), b"sound").unwrap();
    assert_eq!(
        crate::host::platform().disk().list(temp.path()).unwrap(),
        ["ship-\u{fffd}.ogg"]
    );
}

#[test]
fn listing_an_unmounted_missing_path_is_empty_through_a_directory() {
    let temp = TempDir::new().unwrap();
    let missing = temp.path().join("missing");
    let vfs = Vfs::default();
    assert_eq!(
        vfs.list(&missing).unwrap_err().kind(),
        std::io::ErrorKind::NotFound
    );
    let directory = vfs.open_host_dir(&missing);
    assert!(directory
        .list_matching(Path::new(""), "", Match::Prefix)
        .is_empty());
}

#[test]
fn a_file_missing_from_a_lower_priority_archive_is_read_from_the_directory_above() {
    let mut stack = Stack::new();
    let (top, _) = stack.mount_dir(&[("loose.txt", b"loose data")], Placement::Top, true);
    stack.mount_zip(&[("packed.txt", b"packed data")], Placement::Below(top));

    assert_eq!(stack.read("loose.txt").as_deref(), Some(&b"loose data"[..]));
    assert_eq!(
        stack.read("packed.txt").as_deref(),
        Some(&b"packed data"[..])
    );
}

#[test]
fn a_file_missing_from_the_top_archive_is_read_from_the_archive_below() {
    let mut stack = Stack::new();
    let top = stack.mount_zip(&[("upper.txt", b"upper")], Placement::Top);
    stack.mount_zip(&[("lower.txt", b"lower")], Placement::Below(top));

    assert_eq!(stack.read("upper.txt").as_deref(), Some(&b"upper"[..]));
    assert_eq!(stack.read("lower.txt").as_deref(), Some(&b"lower"[..]));
}

#[test]
fn the_highest_priority_mount_wins_when_several_hold_the_same_file() {
    let mut stack = Stack::new();
    let archive = stack.mount_zip(&[("shared.txt", b"from archive")], Placement::Top);
    stack.mount_dir(
        &[("shared.txt", b"from directory")],
        Placement::Above(archive),
        true,
    );

    assert_eq!(
        stack.read("shared.txt").as_deref(),
        Some(&b"from directory"[..])
    );
}

#[test]
fn a_file_found_in_no_mount_cannot_be_opened() {
    let mut stack = Stack::new();
    let (top, _) = stack.mount_dir(&[("here.txt", b"x")], Placement::Top, true);
    stack.mount_zip(&[("there.txt", b"y")], Placement::Below(top));

    assert_eq!(stack.read("nowhere.txt"), None);
}

fn read_mounted_zip(mount_point: &str, parent: &str) -> Option<Vec<u8>> {
    let host = TempDir::new().unwrap();
    let archive_dir = host.path().join(parent.trim_start_matches('/'));
    std::fs::create_dir_all(&archive_dir).unwrap();
    write_zip(
        &archive_dir.join("packed.ani"),
        &[("packed.ani", b"archive animation contents")],
    );
    let vfs = Vfs::new();
    vfs.mount_dir("/", host.path(), Placement::Bottom, true);
    let dir = vfs.open_dir(Path::new(parent));
    let mount = vfs
        .mount_zip(
            mount_point,
            &dir,
            Path::new("packed.ani"),
            Placement::Top,
            true,
        )
        .unwrap();
    let data = dir.read(Path::new("packed.ani")).ok();
    assert!(vfs.unmount(mount));
    data
}

#[test]
fn root_zip_mount_spellings_read_archive_entries() {
    for point in [".", "/", "", "./"] {
        for parent in [".", "/", ""] {
            assert_eq!(
                read_mounted_zip(point, parent).as_deref(),
                Some(b"archive animation contents".as_slice()),
                "mount={point:?}, parent={parent:?}"
            );
        }
    }
}

#[test]
fn relative_zip_mounts_and_directory_handles_read_archive_entries() {
    for point in ["animation", "/animation", "./animation/"] {
        for parent in ["animation", "/animation", "./animation/"] {
            assert_eq!(
                read_mounted_zip(point, parent).as_deref(),
                Some(b"archive animation contents".as_slice()),
                "mount={point:?}, parent={parent:?}"
            );
        }
    }
}

#[test]
fn unmounting_removes_the_files_it_provided() {
    let mut stack = Stack::new();
    let (top, _) = stack.mount_dir(&[("a.txt", b"a")], Placement::Top, true);
    let zip = stack.mount_zip(&[("b.txt", b"b")], Placement::Below(top));
    assert!(stack.read("a.txt").is_some() && stack.read("b.txt").is_some());

    assert!(stack.vfs.unmount(zip));

    assert!(stack.read("a.txt").is_some());
    assert_eq!(stack.read("b.txt"), None);
    assert!(!stack.vfs.unmount(zip), "a removed mount is already gone");
}

#[test]
fn a_transplanted_directory_exposes_its_files_at_the_new_mount_point() {
    let mut stack = Stack::new();
    let (_, host) = stack.mount_dir(&[("a.txt", b"transplanted")], Placement::Top, true);
    let source = stack.vfs.open_host_dir(&host);
    stack.vfs.transplant("/copy", &source, Placement::Top);
    assert_eq!(
        stack.read("copy/a.txt").as_deref(),
        Some(&b"transplanted"[..])
    );
}

#[test]
fn files_read_and_stat_the_underlying_file() {
    let mut stack = Stack::new();
    stack.mount_dir(&[("data.bin", b"0123456789")], Placement::Top, false);
    let files = &stack.root;
    let mut input = files.open_read(Path::new("data.bin")).unwrap();
    let mut buf = [0; 4];
    assert_eq!(input.read(&mut buf).unwrap(), 4);
    assert_eq!(&buf, b"0123");
    assert_eq!(input.read(&mut buf).unwrap(), 4);
    assert_eq!(&buf, b"4567");
    assert_eq!(input.read(&mut buf).unwrap(), 2);
    assert_eq!(input.read(&mut buf).unwrap(), 0);
    let info = files.stat(Path::new("data.bin")).unwrap();
    assert_eq!(info.len, 10);
    assert!(info.is_file && !info.is_dir);
}

#[test]
fn writing_through_files_changes_the_file_on_disk() {
    let mut stack = Stack::new();
    let (_, host) = stack.mount_dir(&[], Placement::Top, false);
    let mut output = stack.root.open_write(Path::new("new.bin")).unwrap();
    output.write_all(b"payload").unwrap();
    output.finish().unwrap();
    assert_eq!(std::fs::read(host.join("new.bin")).unwrap(), b"payload");
}

#[test]
fn unlink_removes_a_file_from_a_writable_mount() {
    let mut stack = Stack::new();
    let (_, host) = stack.mount_dir(
        &[("gone.txt", b"x"), ("kept.txt", b"y")],
        Placement::Top,
        false,
    );
    stack.root.remove(Path::new("gone.txt")).unwrap();
    assert!(!host.join("gone.txt").exists());
    assert!(host.join("kept.txt").exists());
    assert_eq!(stack.read("gone.txt"), None);
}

#[test]
fn files_refuse_read_only_writes_and_removals() {
    let mut stack = Stack::new();
    let (_, host) = stack.mount_dir(&[("locked.txt", b"x")], Placement::Top, true);
    let files = &stack.root;
    assert_eq!(
        files.remove(Path::new("locked.txt")).unwrap_err().kind(),
        std::io::ErrorKind::PermissionDenied
    );
    assert!(host.join("locked.txt").exists());
    assert_eq!(
        files
            .open_write(Path::new("locked.txt"))
            .err()
            .unwrap()
            .kind(),
        std::io::ErrorKind::PermissionDenied
    );
    assert_eq!(
        files.remove(Path::new("missing.txt")).unwrap_err().kind(),
        std::io::ErrorKind::PermissionDenied
    );
}

#[test]
fn files_stat_preserves_host_and_archive_size_and_kind() {
    let mut stack = Stack::new();
    let (_, root) = stack.mount_dir(&[("five.txt", b"12345")], Placement::Top, false);
    std::fs::create_dir(root.join("folder")).unwrap();
    stack.mount_zip(&[("data/blob.bin", b"0123456789")], Placement::Bottom);
    let files = &stack.root;
    let file = files.stat(Path::new("five.txt")).unwrap();
    assert_eq!(file.len, 5);
    assert!(file.is_file && !file.is_dir);
    let directory = files.stat(Path::new("folder")).unwrap();
    assert!(directory.is_dir && !directory.is_file);
    let archived = files.stat(Path::new("data/blob.bin")).unwrap();
    assert_eq!(archived.len, 10);
    assert!(archived.is_file && !archived.is_dir);
    let directory = files.stat(Path::new("data")).unwrap();
    assert_eq!(directory.len, 0);
    assert!(directory.is_dir && !directory.is_file);
    assert_eq!(
        files.stat(Path::new("data/none.bin")).unwrap_err().kind(),
        std::io::ErrorKind::NotFound
    );
}

#[test]
fn an_archive_entry_copies_to_a_writable_directory_through_the_same_files_port() {
    let mut stack = Stack::new();
    stack.mount_zip(&[("packed.txt", b"from the archive")], Placement::Top);
    let (_, root) = stack.mount_dir(&[], Placement::Top, false);
    let files = &stack.root;
    let mut input = files.open_read(Path::new("packed.txt")).unwrap();
    let mut output = files.open_write(Path::new("out.txt")).unwrap();
    std::io::copy(&mut input, &mut output).unwrap();
    output.finish().unwrap();
    assert_eq!(
        std::fs::read(root.join("out.txt")).unwrap(),
        b"from the archive"
    );
    assert_eq!(
        files.read(Path::new("packed.txt")).unwrap(),
        b"from the archive"
    );
}

#[test]
fn directory_entries_are_regular_visible_files_in_listing_order() {
    let scratch = TempDir::new().unwrap();
    for name in ["zeta.mle", "Alpha.mle", ".hidden.mle", "other.txt"] {
        std::fs::write(scratch.path().join(name), b"data").unwrap();
    }
    std::fs::create_dir(scratch.path().join("folder.mle")).unwrap();
    let vfs = Vfs::new();
    vfs.mount_dir("/", scratch.path(), Placement::Top, false);
    let root = vfs.open_dir(Path::new("/"));
    let names = root.files_with_suffix(c".mle");
    assert_eq!(names.len(), 2);
    assert!(names.contains(&c"Alpha.mle".to_owned()));
    assert!(names.contains(&c"zeta.mle".to_owned()));
}

#[test]
fn a_retained_directory_and_its_streams_outlive_the_mount() {
    let scratch = TempDir::new().unwrap();
    std::fs::write(scratch.path().join("fleet.mle"), b"retained").unwrap();
    let vfs = Vfs::new();
    let mount = vfs.mount_dir("/", scratch.path(), Placement::Top, false);
    let directory = vfs.open_dir(Path::new("/"));
    let mut input = directory.open(c"fleet.mle").unwrap();
    assert!(vfs.unmount(mount));
    drop(vfs);
    let mut bytes = Vec::new();
    input.read_to_end(&mut bytes).unwrap();
    assert_eq!(bytes, b"retained");
}

#[test]
fn a_retained_directory_creates_clones_and_removes_files() {
    let scratch = TempDir::new().unwrap();
    let directory = Vfs::new().open_host_dir(scratch.path());
    let clone = directory.clone();
    let mut output = directory.create(c"fleet.mle").unwrap();
    drop(directory);
    output.write_all(b"legacy team bytes").unwrap();
    output.finish().unwrap();
    let mut input = clone.open(c"fleet.mle").unwrap();
    assert_eq!(input.len().unwrap(), 17);
    let mut bytes = Vec::new();
    input.read_to_end(&mut bytes).unwrap();
    assert_eq!(bytes, b"legacy team bytes");
    assert_eq!(input.read(&mut [0; 1]).unwrap(), 0);
}

#[test]
fn directory_operations_preserve_errors_and_truncated_reads() {
    let scratch = TempDir::new().unwrap();
    let directory = Vfs::new().open_host_dir(scratch.path());
    assert!(directory.open(c"missing.mle").is_err());
    let mut output = directory.create(c"short.mle").unwrap();
    output.write_all(b"abc").unwrap();
    output.finish().unwrap();
    let mut input = directory.open(c"short.mle").unwrap();
    assert_eq!(
        input.read_exact(&mut [0; 4]).unwrap_err().kind(),
        std::io::ErrorKind::UnexpectedEof
    );
    directory.remove_named(c"short.mle").unwrap();
    assert!(directory.open(c"short.mle").is_err());
}
