//! Qualification of mount precedence, retained capabilities and ZIP boundaries.
use super::{Dir, Placement, Vfs};
use crate::host::Files;
use std::io::{ErrorKind, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

fn scratch() -> tempfile::TempDir {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../tmp/u01-owners");
    std::fs::create_dir_all(&root).unwrap();
    tempfile::tempdir_in(root).unwrap()
}

fn directory(root: &Path, name: &str, files: &[(&str, &[u8])]) -> PathBuf {
    let path = root.join(name);
    std::fs::create_dir_all(&path).unwrap();
    for (name, bytes) in files {
        let file = path.join(name);
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(file, bytes).unwrap();
    }
    path
}

fn archive(root: &Path, files: &[(&str, &[u8])]) -> PathBuf {
    let path = root.join("package.zip");
    let mut zip = zip::ZipWriter::new(std::fs::File::create(&path).unwrap());
    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    for (name, bytes) in files {
        zip.start_file(*name, options).unwrap();
        zip.write_all(bytes).unwrap();
    }
    zip.finish().unwrap();
    path
}

fn read(dir: &Dir, name: &str) -> Vec<u8> {
    dir.read(Path::new(name)).unwrap()
}

#[test]
fn loose_zip_and_two_shadows_preserve_precedence_and_miss_fallback() {
    let scratch = scratch();
    let base = directory(scratch.path(), "base", &[("shared", b"loose")]);
    let first = directory(scratch.path(), "first", &[("shared", b"first")]);
    let second = directory(scratch.path(), "second", &[("shared", b"second")]);
    archive(
        scratch.path(),
        &[("shared", b"zip"), ("zip-only", b"fallback")],
    );
    let vfs = Vfs::new();
    let source = vfs.open_dir(scratch.path());
    let loose = vfs.mount_dir("/", &base, Placement::Top, true);
    let zip = vfs
        .mount_zip(
            "/",
            &source,
            Path::new("package.zip"),
            Placement::Below(loose),
            true,
        )
        .unwrap();
    let root = vfs.open_dir(Path::new("/"));
    assert_eq!(read(&root, "shared"), b"loose");
    let shadow1 = vfs.mount_dir("/", &first, Placement::Above(loose), true);
    let shadow2 = vfs.mount_dir("/", &second, Placement::Above(shadow1), true);
    assert_eq!(read(&root, "shared"), b"second");
    assert_eq!(read(&root, "zip-only"), b"fallback");
    assert!(vfs.unmount(shadow2));
    assert_eq!(read(&root, "shared"), b"first");
    assert!(vfs.unmount(shadow1));
    assert_eq!(read(&root, "shared"), b"loose");
    assert!(vfs.unmount(loose));
    assert_eq!(read(&root, "shared"), b"zip");
    assert!(vfs.unmount(zip));
}

#[test]
fn equal_positions_prefer_specific_point_then_creation_order() {
    let scratch = scratch();
    let anchor_path = directory(scratch.path(), "anchor", &[]);
    let broad = directory(scratch.path(), "broad", &[("nested/same", b"broad")]);
    let first = directory(scratch.path(), "first", &[("same", b"first")]);
    let second = directory(scratch.path(), "second", &[("same", b"second")]);
    let vfs = Vfs::new();
    let anchor = vfs.mount_dir("/", &anchor_path, Placement::Top, true);
    vfs.mount_dir("/", &broad, Placement::Above(anchor), true);
    let specific = vfs.mount_dir("/nested", &first, Placement::Above(anchor), true);
    vfs.mount_dir("/nested", &second, Placement::Above(anchor), true);
    let root = vfs.open_dir(Path::new("/"));
    assert_eq!(read(&root, "nested/same"), b"first");
    assert!(vfs.unmount(specific));
    assert_eq!(read(&root, "nested/same"), b"second");
}

#[test]
fn listing_deduplicates_in_first_encounter_order_without_sorting() {
    let scratch = scratch();
    let upper = directory(scratch.path(), "upper", &[("z", b""), ("same", b"")]);
    let lower = directory(scratch.path(), "lower", &[("a", b""), ("same", b"")]);
    let vfs = Vfs::new();
    let disk = crate::host::platform().disk();
    let mut expected = disk.list(&upper).unwrap();
    for name in disk.list(&lower).unwrap() {
        if !expected.contains(&name) {
            expected.push(name);
        }
    }
    vfs.mount_dir("/", &lower, Placement::Bottom, true);
    vfs.mount_dir("/", &upper, Placement::Top, true);
    assert_eq!(
        vfs.open_dir(Path::new("/")).list(Path::new("")).unwrap(),
        expected
    );
}

#[test]
fn corrupt_top_archive_does_not_retry_valid_lower_member() {
    let scratch = scratch();
    let lower = directory(scratch.path(), "lower", &[("shared", b"lower")]);
    let archive = archive(scratch.path(), &[("shared", b"stored-payload")]);
    let vfs = Vfs::new();
    let source = vfs.open_dir(scratch.path());
    vfs.mount_dir("/", &lower, Placement::Bottom, true);
    vfs.mount_zip("/", &source, Path::new("package.zip"), Placement::Top, true)
        .unwrap();
    let mut bytes = std::fs::read(&archive).unwrap();
    let position = bytes
        .windows(b"stored-payload".len())
        .position(|w| w == b"stored-payload")
        .unwrap();
    bytes[position] ^= 1;
    std::fs::write(archive, bytes).unwrap();
    assert_eq!(
        vfs.open_dir(Path::new("/"))
            .read(Path::new("shared"))
            .unwrap_err()
            .kind(),
        ErrorKind::InvalidData
    );
    assert_eq!(std::fs::read(lower.join("shared")).unwrap(), b"lower");
}

#[test]
fn retained_directory_can_open_and_read_after_unmount() {
    let scratch = scratch();
    let root = directory(scratch.path(), "root", &[("data", b"retained")]);
    let vfs = Vfs::new();
    let id = vfs.mount_dir("/", &root, Placement::Top, false);
    let dir = vfs.open_dir(Path::new("/"));
    let mut stream = dir.open_read(Path::new("data")).unwrap();
    assert!(vfs.unmount(id));
    drop(vfs);
    assert_eq!(read(&dir, "data"), b"retained");
    let mut bytes = Vec::new();
    stream.read_to_end(&mut bytes).unwrap();
    assert_eq!(bytes, b"retained");
}

#[test]
fn retained_write_and_remove_never_retarget_a_replacement_mount() {
    let scratch = scratch();
    let a = directory(scratch.path(), "a", &[("remove", b"a")]);
    let b = directory(scratch.path(), "b", &[("remove", b"b"), ("write", b"b")]);
    let vfs = Vfs::new();
    vfs.mount_dir("/", &a, Placement::Top, false);
    let retained = vfs.open_dir(Path::new("/"));
    vfs.unmount_all();
    vfs.mount_dir("/", &b, Placement::Top, false);
    retained.write_or_remove(Path::new("write"), b"a").unwrap();
    retained.remove(Path::new("remove")).unwrap();
    assert_eq!(std::fs::read(b.join("write")).unwrap(), b"b");
    assert_eq!(std::fs::read(b.join("remove")).unwrap(), b"b");
    assert_eq!(std::fs::read(a.join("write")).unwrap(), b"a");
    assert!(!a.join("remove").exists());
}

#[test]
fn retained_read_only_directory_cannot_gain_write_permission_after_remount() {
    let scratch = scratch();
    let a = directory(scratch.path(), "a", &[("data", b"a")]);
    let b = directory(scratch.path(), "b", &[("data", b"b")]);
    let vfs = Vfs::new();
    vfs.mount_dir("/", &a, Placement::Top, true);
    let retained = vfs.open_dir(Path::new("/"));
    vfs.unmount_all();
    vfs.mount_dir("/", &b, Placement::Top, false);
    assert_eq!(
        retained.open_write(Path::new("data")).err().unwrap().kind(),
        ErrorKind::PermissionDenied
    );
    assert_eq!(
        retained.remove(Path::new("data")).unwrap_err().kind(),
        ErrorKind::PermissionDenied
    );
    assert_eq!(std::fs::read(a.join("data")).unwrap(), b"a");
    assert_eq!(std::fs::read(b.join("data")).unwrap(), b"b");
}

#[test]
fn removed_mount_ids_cannot_remove_new_mounts() {
    let scratch = scratch();
    let root = directory(scratch.path(), "root", &[]);
    let vfs = Vfs::new();
    let old = vfs.mount_dir("/", &root, Placement::Top, false);
    vfs.unmount_all();
    let new = vfs.mount_dir("/", &root, Placement::Top, false);
    assert_ne!(old, new);
    assert!(!vfs.unmount(old));
    assert!(vfs.unmount(new));
}

#[test]
fn zip_stream_read_past_eof_and_invalid_seek_preserve_position() {
    let scratch = scratch();
    let path = archive(scratch.path(), &[("data", b"0123456789")]);
    let index = super::zip_reader::ZipIndex::new(crate::host::platform().disk(), &path).unwrap();
    let mut stream = index.open_entry("data").unwrap();
    assert_eq!(stream.seek(SeekFrom::Start(50)).unwrap(), 50);
    assert_eq!(stream.read(&mut [0; 4]).unwrap(), 0);
    assert_eq!(
        stream.seek(SeekFrom::Current(-51)).unwrap_err().kind(),
        ErrorKind::InvalidInput
    );
    assert_eq!(stream.position(), 50);
    assert_eq!(stream.seek(SeekFrom::Start(2)).unwrap(), 2);
    assert_eq!(
        stream.seek(SeekFrom::Current(i64::MAX)).unwrap(),
        2 + i64::MAX as u64
    );
    assert_eq!(stream.read(&mut [0; 4]).unwrap(), 0);
    assert_eq!(
        stream.seek(SeekFrom::Current(i64::MAX)).unwrap_err().kind(),
        ErrorKind::InvalidInput
    );
    assert_eq!(stream.position(), 2 + i64::MAX as u64);
    stream.seek(SeekFrom::End(-2)).unwrap();
    let mut bytes = [b'.'; 4];
    assert_eq!(stream.read(&mut bytes).unwrap(), 2);
    assert_eq!(&bytes, b"89..");
}
