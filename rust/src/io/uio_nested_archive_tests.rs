//! A ZIP archive held inside another ZIP archive mounts through the public
//! UIO calls, as the original `zip_mount` did over a handle opened through the
//! mount stack: the nested archive's bytes belong to the mount that reads them
//! and nothing is looked up on the host afterwards.
//!
//! The fixtures are declared in `tmp/issue38-port/nested-zip/fixture-spec.md`.
//! Every archive is written by the `zip` crate into a temporary host
//! directory; every operation is an exported C ABI function.

use std::io::{Cursor, Write};

use serial_test::serial;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

use super::archive_stat_tests::{read_all, Archive};
use super::resolution_tests::{
    cstr, errno_value, list_names, mount_count, mount_stdio, mount_zip, open_dir, read,
    stdio_access, Fx, RDONLY_TOP,
};
use super::*;
use crate::io::zip_reader::SourceProbe;

const PARENT_MARKER: &[u8] = b"parent-crc-marker";
const PAYLOAD_MARKER: &[u8] = b"payload-data-here";

fn options(method: CompressionMethod) -> SimpleFileOptions {
    SimpleFileOptions::default().compression_method(method)
}

fn build_zip(fill: impl FnOnce(&mut ZipWriter<Cursor<Vec<u8>>>)) -> Vec<u8> {
    let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
    fill(&mut zip);
    zip.finish().unwrap().into_inner()
}

fn put(zip: &mut ZipWriter<Cursor<Vec<u8>>>, name: &str, data: &[u8], method: CompressionMethod) {
    zip.start_file(name, options(method)).unwrap();
    zip.write_all(data).unwrap();
}

fn compressible(text: &[u8]) -> Vec<u8> {
    text.iter().copied().cycle().take(4096).collect()
}

fn all_bytes() -> Vec<u8> {
    (0..=255u8).collect()
}

/// The inner archive: deflated, empty, binary, UTF-8 and raw-encoded names,
/// explicit and implicit directories, a property file and a list directory.
fn inner_zip(a_text: &[u8]) -> Vec<u8> {
    let mut bytes = build_zip(|zip| {
        put(
            zip,
            "a.txt",
            &compressible(a_text),
            CompressionMethod::Deflated,
        );
        put(zip, "empty.bin", b"", CompressionMethod::Stored);
        put(zip, "bin.dat", &all_bytes(), CompressionMethod::Stored);
        put(
            zip,
            "uni/\u{e9}\u{4e2d}.txt",
            b"unicode name",
            CompressionMethod::Stored,
        );
        put(zip, "caf_", b"raw encoded name", CompressionMethod::Stored);
        zip.add_directory("sub/", options(CompressionMethod::Stored))
            .unwrap();
        put(zip, "sub/x.txt", b"sub x", CompressionMethod::Stored);
        put(zip, "deep/y/z.txt", b"deep z", CompressionMethod::Deflated);
        put(
            zip,
            "res/game.rmp",
            b"name = STRING:Nested\nvol = INT32:7\n",
            CompressionMethod::Deflated,
        );
        put(zip, "lists/one.mle", b"1", CompressionMethod::Stored);
        put(zip, "lists/two.mle", b"2", CompressionMethod::Stored);
        put(zip, "lists/skip.txt", b"3", CompressionMethod::Stored);
        put(zip, "lists/dir.mle/inside", b"4", CompressionMethod::Stored);
    });
    // The name "caf_" is written as ASCII; the byte 0x82 without the UTF-8
    // flag is CP437 for e-acute, the encoding older archivers recorded.
    let mut at = 0;
    while let Some(found) = bytes[at..].windows(4).position(|w| w == b"caf_") {
        bytes[at + found + 3] = 0x82;
        at += found + 4;
    }
    bytes
}

fn flip_first(bytes: &mut [u8], marker: &[u8]) {
    let at = bytes
        .windows(marker.len())
        .position(|w| w == marker)
        .expect("the marker is in the bytes");
    bytes[at] ^= 0xFF;
}

fn mid_zip() -> Vec<u8> {
    let leaf = build_zip(|zip| put(zip, "leaf.txt", b"two levels", CompressionMethod::Stored));
    build_zip(|zip| put(zip, "leaf.zip", &leaf, CompressionMethod::Deflated))
}

/// The outer archive holding `inner` as `packs/inner.uqm` next to the
/// archives each refusal case needs.
fn outer_zip(inner: &[u8]) -> Vec<u8> {
    let short = {
        let whole = inner_zip(b"short");
        whole[..whole.len() - 30].to_vec()
    };
    let mut damaged = build_zip(|zip| {
        put(
            zip,
            "payload.bin",
            PAYLOAD_MARKER,
            CompressionMethod::Stored,
        )
    });
    flip_first(&mut damaged, PAYLOAD_MARKER);
    let badparent = build_zip(|zip| put(zip, "p.bin", PARENT_MARKER, CompressionMethod::Stored));
    let mut outer = build_zip(|zip| {
        put(zip, "packs/inner.uqm", inner, CompressionMethod::Deflated);
        put(zip, "packs/mid.zip", &mid_zip(), CompressionMethod::Stored);
        zip.add_directory("packs/dir/", options(CompressionMethod::Stored))
            .unwrap();
        put(zip, "notes.txt", b"notes", CompressionMethod::Stored);
        zip.add_symlink(
            "packs/link",
            "inner.uqm",
            options(CompressionMethod::Stored),
        )
        .unwrap();
        put(
            zip,
            "packs/bogus.zip",
            b"this is not an archive",
            CompressionMethod::Stored,
        );
        put(zip, "packs/short.zip", &short, CompressionMethod::Stored);
        put(zip, "packs/crc.zip", &damaged, CompressionMethod::Stored);
        put(
            zip,
            "packs/badparent.zip",
            &badparent,
            CompressionMethod::Stored,
        );
    });
    flip_first(&mut outer, PARENT_MARKER);
    outer
}

/// One repository whose `/src` is the host directory holding `outer.zip` and
/// whose `/z` is that archive.
struct Stack {
    repo: *mut uio_Repository,
    host: PathBuf,
    outer: *mut uio_MountHandle,
}

impl Stack {
    fn new(fx: &mut Fx, outer: &[u8]) -> Self {
        let repo = fx.repo();
        Self::in_repo(fx, repo, outer)
    }

    fn in_repo(fx: &mut Fx, repo: *mut uio_Repository, outer: &[u8]) -> Self {
        let host = fx.host(&[]);
        std::fs::write(host.join("outer.zip"), outer).unwrap();
        assert!(!mount_stdio(repo, "/src", &host, RDONLY_TOP).is_null());
        let outer = Self::mount_outer(repo);
        Stack { repo, host, outer }
    }

    fn mount_outer(repo: *mut uio_Repository) -> *mut uio_MountHandle {
        let source = open_dir(repo, "/src");
        let outer = mount_zip(repo, "/z", source, "outer.zip", RDONLY_TOP);
        assert!(!outer.is_null(), "errno {}", errno_value());
        outer
    }

    fn nested(
        &self,
        point: &str,
        entry: &str,
        in_path: &str,
        flags: c_int,
        relative: *mut uio_MountHandle,
    ) -> *mut uio_MountHandle {
        let source = open_dir(self.repo, "/z");
        mount_nested(self.repo, point, source, entry, in_path, flags, relative)
    }

    fn nested_inner(&self, point: &str) -> *mut uio_MountHandle {
        let mount = self.nested(point, "packs/inner.uqm", "/", RDONLY_TOP, ptr::null_mut());
        assert!(!mount.is_null(), "errno {}", errno_value());
        mount
    }
}

fn mount_nested(
    repo: *mut uio_Repository,
    point: &str,
    source: *mut uio_DirHandle,
    entry: &str,
    in_path: &str,
    flags: c_int,
    relative: *mut uio_MountHandle,
) -> *mut uio_MountHandle {
    let (point, entry, in_path) = (cstr(point), cstr(entry), cstr(in_path));
    unsafe {
        uio_mountDir(
            repo,
            point.as_ptr(),
            UIO_FSTYPE_ZIP,
            source,
            entry.as_ptr(),
            in_path.as_ptr(),
            ptr::null_mut(),
            flags,
            relative,
        )
    }
}

fn sorted(mut names: Vec<String>) -> Vec<String> {
    names.sort();
    names
}

fn size_of_path(repo: *mut uio_Repository, path: &str) -> Result<i64, c_int> {
    let dir = open_dir(repo, "/");
    let path = cstr(path);
    let mut info: stat = unsafe { std::mem::zeroed() };
    let rc = unsafe { uio_stat(dir, path.as_ptr(), &mut info) };
    let errno = errno_value();
    unsafe { uio_closeDir(dir) };
    if rc == 0 {
        Ok(info.st_size)
    } else {
        Err(errno)
    }
}

fn stream_bytes(stream: *mut uio_Stream, length: usize) -> Vec<u8> {
    let mut bytes = vec![0u8; length];
    let got = unsafe { uio_fread(bytes.as_mut_ptr().cast(), 1, length, stream) };
    bytes.truncate(got);
    bytes
}

fn block_bytes(block: *mut uio_FileBlock, length: usize) -> Vec<u8> {
    let mut buffer = ptr::null_mut::<c_char>();
    let got = unsafe { uio_accessFileBlock(block, 0, length, &mut buffer) };
    assert_eq!(got, length as isize, "errno {}", errno_value());
    unsafe { slice::from_raw_parts(buffer.cast::<u8>(), length) }.to_vec()
}

#[test]
#[serial]
fn a_nested_archive_reads_every_kind_of_entry_through_the_public_calls() {
    let mut fx = Fx::new();
    let stack = Stack::new(&mut fx, &outer_zip(&inner_zip(b"first")));
    stack.nested_inner("/n");
    let dir = open_dir(stack.repo, "/n");

    assert_eq!(read(dir, "a.txt").unwrap(), compressible(b"first"));
    assert_eq!(read(dir, "empty.bin").unwrap(), b"");
    assert_eq!(read(dir, "bin.dat").unwrap(), all_bytes());
    assert_eq!(
        read(dir, "uni/\u{e9}\u{4e2d}.txt").unwrap(),
        b"unicode name"
    );
    assert_eq!(read(dir, "caf\u{e9}").unwrap(), b"raw encoded name");

    assert_eq!(size_of_path(stack.repo, "n/a.txt"), Ok(4096));
    assert_eq!(size_of_path(stack.repo, "n/empty.bin"), Ok(0));
    assert_eq!(size_of_path(stack.repo, "n/bin.dat"), Ok(256));
    assert_eq!(
        sorted(list_names(dir, ".")),
        [
            "a.txt",
            "bin.dat",
            "caf\u{e9}",
            "deep",
            "empty.bin",
            "lists",
            "res",
            "sub",
            "uni"
        ]
    );
    assert_eq!(read(dir, "missing.txt"), Err(libc::ENOENT));
}

#[test]
#[serial]
fn a_nested_archive_mounts_an_explicit_or_implied_inner_directory() {
    let mut fx = Fx::new();
    let stack = Stack::new(&mut fx, &outer_zip(&inner_zip(b"first")));
    let explicit = stack.nested("/e", "packs/inner.uqm", "/sub", RDONLY_TOP, ptr::null_mut());
    let implied = stack.nested(
        "/i",
        "packs/inner.uqm",
        "/deep/y/",
        RDONLY_TOP,
        ptr::null_mut(),
    );
    assert!(!explicit.is_null() && !implied.is_null());

    assert_eq!(read(open_dir(stack.repo, "/e"), "x.txt").unwrap(), b"sub x");
    assert_eq!(read(open_dir(stack.repo, "/e"), "a.txt"), Err(libc::ENOENT));
    assert_eq!(
        read(open_dir(stack.repo, "/i"), "z.txt").unwrap(),
        b"deep z"
    );

    let before = mount_count();
    for (in_path, errno) in [("/a.txt", libc::ENOTDIR), ("/none", libc::ENOENT)] {
        let mount = stack.nested(
            "/x",
            "packs/inner.uqm",
            in_path,
            RDONLY_TOP,
            ptr::null_mut(),
        );
        assert!(mount.is_null(), "{in_path}");
        assert_eq!(errno_value(), errno, "{in_path}");
    }
    assert_eq!(mount_count(), before);
}

#[test]
#[serial]
fn an_archive_inside_an_archive_inside_an_archive_mounts() {
    let mut fx = Fx::new();
    let stack = Stack::new(&mut fx, &outer_zip(&inner_zip(b"first")));
    let mid = stack.nested("/m", "packs/mid.zip", "/", RDONLY_TOP, ptr::null_mut());
    assert!(!mid.is_null(), "errno {}", errno_value());
    let leaf = mount_nested(
        stack.repo,
        "/l",
        open_dir(stack.repo, "/m"),
        "leaf.zip",
        "/",
        RDONLY_TOP,
        ptr::null_mut(),
    );
    assert!(!leaf.is_null(), "errno {}", errno_value());
    assert_eq!(
        read(open_dir(stack.repo, "/l"), "leaf.txt").unwrap(),
        b"two levels"
    );
}

#[test]
#[serial]
fn mount_priority_decides_between_a_nested_archive_and_its_siblings() {
    let mut fx = Fx::new();
    let stack = Stack::new(&mut fx, &outer_zip(&inner_zip(b"first")));
    let side = fx.host(&[("a.txt", b"host a"), ("only-host.txt", b"host only")]);
    std::fs::write(
        stack.host.join("side.zip"),
        build_zip(|zip| put(zip, "a.txt", b"side a", CompressionMethod::Stored)),
    )
    .unwrap();

    // Nested on top of a host directory: the nested file wins, the rest falls through.
    let sibling = mount_stdio(stack.repo, "/n", &side, UIO_MOUNT_TOP);
    let top = stack.nested("/n", "packs/inner.uqm", "/", RDONLY_TOP, ptr::null_mut());
    assert!(!top.is_null(), "errno {}", errno_value());
    let dir = open_dir(stack.repo, "/n");
    assert_eq!(read(dir, "a.txt").unwrap(), compressible(b"first"));
    assert_eq!(read(dir, "only-host.txt").unwrap(), b"host only");
    assert_eq!(unsafe { uio_unmountDir(top) }, 0);

    // Nested below the host directory: the host wins on shared names only.
    let below = stack.nested(
        "/n",
        "packs/inner.uqm",
        "/",
        UIO_MOUNT_RDONLY | UIO_MOUNT_BELOW,
        sibling,
    );
    assert!(!below.is_null(), "errno {}", errno_value());
    assert_eq!(read(dir, "a.txt").unwrap(), b"host a");
    assert_eq!(read(dir, "bin.dat").unwrap(), all_bytes());
    assert_eq!(unsafe { uio_unmountDir(below) }, 0);

    // Nested above an archive sibling.
    let side_zip = mount_zip(
        stack.repo,
        "/n",
        open_dir(stack.repo, "/src"),
        "side.zip",
        RDONLY_TOP,
    );
    assert!(!side_zip.is_null());
    let above = stack.nested(
        "/n",
        "packs/inner.uqm",
        "/",
        UIO_MOUNT_RDONLY | UIO_MOUNT_ABOVE,
        side_zip,
    );
    assert!(!above.is_null(), "errno {}", errno_value());
    assert_eq!(read(dir, "a.txt").unwrap(), compressible(b"first"));
    assert_eq!(unsafe { uio_unmountDir(above) }, 0);
    assert_eq!(read(dir, "a.txt").unwrap(), b"side a");
}

#[test]
#[serial]
fn two_repositories_mount_their_own_nested_archives() {
    let mut fx = Fx::new();
    let first = Stack::new(&mut fx, &outer_zip(&inner_zip(b"first")));
    let second = Stack::new(&mut fx, &outer_zip(&inner_zip(b"second")));
    first.nested_inner("/n");
    second.nested_inner("/n");

    assert_eq!(
        read(open_dir(first.repo, "/n"), "a.txt").unwrap(),
        compressible(b"first")
    );
    assert_eq!(
        read(open_dir(second.repo, "/n"), "a.txt").unwrap(),
        compressible(b"second")
    );
    assert_eq!(unsafe { uio_unmountDir(first.outer) }, 0);
    assert_eq!(
        read(open_dir(second.repo, "/n"), "a.txt").unwrap(),
        compressible(b"second")
    );
}

#[test]
#[serial]
fn a_nested_source_that_cannot_be_an_archive_is_refused_with_a_typed_errno() {
    let mut fx = Fx::new();
    let stack = Stack::new(&mut fx, &outer_zip(&inner_zip(b"first")));
    // A real archive sits on the host where the outer archive's lexical path would lead.
    let decoy = fx.host(&[]);
    std::fs::create_dir_all(decoy.join("packs")).unwrap();
    std::fs::write(decoy.join("packs/bogus.zip"), inner_zip(b"decoy")).unwrap();
    assert!(!mount_stdio(
        stack.repo,
        "/z",
        &decoy,
        UIO_MOUNT_RDONLY | UIO_MOUNT_BOTTOM
    )
    .is_null());

    let before = mount_count();
    for (entry, flags, errno) in [
        ("packs/link", RDONLY_TOP, libc::ENOTSUP),
        ("packs/short.zip", RDONLY_TOP, libc::EIO),
        ("packs/bogus.zip", RDONLY_TOP, libc::EIO),
        ("packs/badparent.zip", RDONLY_TOP, libc::EIO),
        ("packs/dir", RDONLY_TOP, libc::EISDIR),
        ("packs", RDONLY_TOP, libc::EISDIR),
        ("packs/nothing.zip", RDONLY_TOP, libc::ENOENT),
        ("packs/inner.uqm", UIO_MOUNT_TOP, libc::EACCES),
        ("packs/badparent.zip", UIO_MOUNT_TOP, libc::EACCES),
    ] {
        super::set_errno(0);
        let mount = stack.nested("/n", entry, "/", flags, ptr::null_mut());
        assert!(mount.is_null(), "{entry}");
        assert_eq!(errno_value(), errno, "{entry}");
    }
    assert_eq!(mount_count(), before, "a refusal leaves no mount behind");
}

#[test]
#[serial]
fn damaged_data_in_a_nested_archive_fails_the_read_not_the_process() {
    let mut fx = Fx::new();
    let stack = Stack::new(&mut fx, &outer_zip(&inner_zip(b"first")));
    let mount = stack.nested("/c", "packs/crc.zip", "/", RDONLY_TOP, ptr::null_mut());
    assert!(!mount.is_null(), "errno {}", errno_value());
    let dir = open_dir(stack.repo, "/c");
    assert_eq!(read(dir, "payload.bin"), Err(libc::EIO));
}

#[test]
#[serial]
fn a_nested_mount_outlives_its_parent_mount_and_the_parent_file() {
    let mut fx = Fx::new();
    let stack = Stack::new(&mut fx, &outer_zip(&inner_zip(b"first")));
    let nested = stack.nested_inner("/n");
    let dir = open_dir(stack.repo, "/n");
    let stream = unsafe { uio_fopen(dir, cstr("a.txt").as_ptr(), c"rb".as_ptr()) };
    let handle = unsafe { uio_open(dir, cstr("bin.dat").as_ptr(), O_RDONLY, 0) };
    let block = unsafe { uio_openFileBlock(handle) };
    assert!(!stream.is_null() && !handle.is_null() && !block.is_null());

    assert_eq!(unsafe { uio_unmountDir(stack.outer) }, 0);
    std::fs::remove_file(stack.host.join("outer.zip")).unwrap();
    assert_eq!(
        size_of_path(stack.repo, "z/packs/inner.uqm"),
        Err(libc::ENOENT)
    );

    assert_eq!(read(dir, "a.txt").unwrap(), compressible(b"first"));
    assert_eq!(size_of_path(stack.repo, "n/bin.dat"), Ok(256));
    assert!(list_names(dir, ".").contains(&"bin.dat".to_string()));

    assert_eq!(unsafe { uio_unmountDir(nested) }, 0);
    assert_eq!(read(dir, "a.txt"), Err(libc::ENOENT));
    assert_eq!(stream_bytes(stream, 4096), compressible(b"first"));
    assert_eq!(block_bytes(block, 256), all_bytes());
    assert_eq!(unsafe { uio_lseek(handle, 0, SEEK_SET) }, 0);
    assert_eq!(read_all(handle), all_bytes());
    unsafe {
        uio_closeFileBlock(block);
        uio_close(handle);
        assert_eq!(uio_fclose(stream), 0);
    }
}

#[test]
#[serial]
fn open_streams_and_blocks_survive_closing_the_repository() {
    let mut fx = Fx::new();
    let repo = unsafe { uio_openRepository(0) };
    let stack = Stack::in_repo(&mut fx, repo, &outer_zip(&inner_zip(b"first")));
    stack.nested_inner("/n");
    let dir = open_dir(repo, "/n");
    let stream = unsafe { uio_fopen(dir, cstr("a.txt").as_ptr(), c"rb".as_ptr()) };
    let handle = unsafe { uio_open(dir, cstr("bin.dat").as_ptr(), O_RDONLY, 0) };
    let block = unsafe { uio_openFileBlock(handle) };

    unsafe { uio_closeRepository(repo) };
    std::fs::remove_file(stack.host.join("outer.zip")).unwrap();
    assert_eq!(mount_count(), 0);

    assert_eq!(stream_bytes(stream, 4096), compressible(b"first"));
    assert_eq!(block_bytes(block, 256), all_bytes());
    unsafe {
        uio_closeFileBlock(block);
        uio_close(handle);
        assert_eq!(uio_fclose(stream), 0);
    }
}

#[test]
#[serial]
fn a_replaced_parent_gives_new_mounts_new_bytes_and_old_handles_their_old_ones() {
    let mut fx = Fx::new();
    let stack = Stack::new(&mut fx, &outer_zip(&inner_zip(b"first")));
    let old_mount = stack.nested_inner("/n");
    let old_dir = open_dir(stack.repo, "/n");
    let old_handle = unsafe { uio_open(old_dir, cstr("a.txt").as_ptr(), O_RDONLY, 0) };
    assert!(!old_handle.is_null());

    assert_eq!(unsafe { uio_unmountDir(stack.outer) }, 0);
    std::fs::remove_file(stack.host.join("outer.zip")).unwrap();
    std::fs::write(
        stack.host.join("outer.zip"),
        outer_zip(&inner_zip(b"second")),
    )
    .unwrap();
    Stack::mount_outer(stack.repo);

    // The held mount still serves the bytes it was made from; a new one reads the replacement.
    assert_eq!(read(old_dir, "a.txt").unwrap(), compressible(b"first"));
    let renewed = stack.nested_inner("/r");
    assert_ne!(renewed, old_mount);
    assert_eq!(
        read(open_dir(stack.repo, "/r"), "a.txt").unwrap(),
        compressible(b"second")
    );
    assert_eq!(read_all(old_handle), compressible(b"first"));
    unsafe { uio_close(old_handle) };
}

#[test]
#[serial]
fn a_transplant_of_a_nested_directory_shares_the_nested_archive() {
    let mut fx = Fx::new();
    let stack = Stack::new(&mut fx, &outer_zip(&inner_zip(b"first")));
    let nested = stack.nested_inner("/n");
    let source = open_dir(stack.repo, "/n/sub");
    let moved =
        unsafe { uio_transplantDir(cstr("/t").as_ptr(), source, RDONLY_TOP, ptr::null_mut()) };
    assert!(!moved.is_null(), "errno {}", errno_value());

    assert_eq!(unsafe { uio_unmountDir(nested) }, 0);
    assert_eq!(unsafe { uio_unmountDir(stack.outer) }, 0);
    assert_eq!(read(open_dir(stack.repo, "/t"), "x.txt").unwrap(), b"sub x");
}

#[test]
#[serial]
fn stdio_access_unpacks_a_nested_entry() {
    let mut fx = Fx::new();
    let stack = Stack::new(&mut fx, &outer_zip(&inner_zip(b"first")));
    stack.nested_inner("/n");
    let temp = fx.host(&[]);
    assert!(!mount_stdio(stack.repo, "/tmp", &temp, UIO_MOUNT_TOP).is_null());

    let (access, path) = stdio_access(
        open_dir(stack.repo, "/n"),
        "bin.dat",
        open_dir(stack.repo, "/tmp"),
    )
    .unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), all_bytes());
    unsafe { uio_releaseStdioAccess(access) };
    assert!(!path.exists());
}

#[test]
#[serial]
fn archive_fixtures_still_mount_from_a_plain_host_archive() {
    let archive = Archive::new(|path| {
        std::fs::write(path, inner_zip(b"first")).unwrap();
    });
    let dir = open_dir(archive.repo, "/z");
    assert_eq!(read(dir, "a.txt").unwrap(), compressible(b"first"));
}

/// A weak view of the source the archive mount `mount` reads, taken from the
/// registry. It keeps nothing alive, so it shows which owner does.
fn source_of(mount: *mut uio_MountHandle) -> SourceProbe {
    get_mount_registry()
        .lock()
        .unwrap()
        .iter()
        .find(|info| info.handle_ptr == mount_handle_key(mount))
        .and_then(|info| info.zip_index.as_ref())
        .expect("an archive mount holds its index")
        .source_probe()
}

#[test]
#[serial]
fn each_mount_owns_its_own_source_and_nothing_else() {
    let mut fx = Fx::new();
    let stack = Stack::new(&mut fx, &outer_zip(&inner_zip(b"first")));
    let outer_source = source_of(stack.outer);
    assert!(matches!(outer_source, SourceProbe::Host(_)));
    let nested = stack.nested_inner("/n");
    let nested_source = source_of(nested);
    assert!(matches!(nested_source, SourceProbe::Entry(_)));
    assert!(outer_source.is_alive() && nested_source.is_alive());

    // The nested mount holds its own bytes, not the parent's descriptor.
    assert_eq!(unsafe { uio_unmountDir(stack.outer) }, 0);
    assert!(!outer_source.is_alive(), "the parent is not retained");
    assert!(nested_source.is_alive());
    assert_eq!(
        read(open_dir(stack.repo, "/n"), "a.txt").unwrap(),
        compressible(b"first")
    );

    assert_eq!(unsafe { uio_unmountDir(nested) }, 0);
    assert!(!nested_source.is_alive(), "unmounting releases the archive");
}

#[test]
#[serial]
fn mounting_and_unmounting_repeatedly_leaves_nothing_alive() {
    let mut fx = Fx::new();
    let stack = Stack::new(&mut fx, &outer_zip(&inner_zip(b"first")));
    for round in 0..3 {
        let nested = stack.nested_inner("/n");
        let source = source_of(nested);
        assert_eq!(
            read(open_dir(stack.repo, "/n"), "bin.dat").unwrap(),
            all_bytes(),
            "round {round}"
        );
        assert_eq!(unsafe { uio_unmountDir(nested) }, 0);
        assert!(!source.is_alive(), "round {round}");
    }
    let outer_source = source_of(stack.outer);
    assert_eq!(unsafe { uio_unmountDir(stack.outer) }, 0);
    assert!(!outer_source.is_alive());
}

#[test]
#[serial]
fn closing_the_repository_releases_every_nested_source() {
    let mut fx = Fx::new();
    let repo = unsafe { uio_openRepository(0) };
    let stack = Stack::in_repo(&mut fx, repo, &outer_zip(&inner_zip(b"first")));
    let nested = stack.nested_inner("/n");
    let mid = stack.nested("/m", "packs/mid.zip", "/", RDONLY_TOP, ptr::null_mut());
    let leaf = mount_nested(
        repo,
        "/l",
        open_dir(repo, "/m"),
        "leaf.zip",
        "/",
        RDONLY_TOP,
        ptr::null_mut(),
    );
    let sources = [stack.outer, nested, mid, leaf].map(source_of);
    assert!(sources.iter().all(SourceProbe::is_alive));

    unsafe { uio_closeRepository(repo) };
    assert!(sources.iter().all(|source| !source.is_alive()));
}

#[test]
#[serial]
fn a_transplanted_directory_keeps_the_nested_source_until_it_is_gone() {
    let mut fx = Fx::new();
    let stack = Stack::new(&mut fx, &outer_zip(&inner_zip(b"first")));
    let nested = stack.nested_inner("/n");
    let source = source_of(nested);
    let moved = unsafe {
        uio_transplantDir(
            cstr("/t").as_ptr(),
            open_dir(stack.repo, "/n/sub"),
            RDONLY_TOP,
            ptr::null_mut(),
        )
    };
    assert!(!moved.is_null());

    assert_eq!(unsafe { uio_unmountDir(nested) }, 0);
    assert!(source.is_alive(), "the transplanted mount still reads it");
    assert_eq!(unsafe { uio_unmountDir(moved) }, 0);
    assert!(!source.is_alive());
}

#[test]
#[serial]
fn a_host_archive_stays_readable_through_the_descriptor_it_was_mounted_with() {
    let mut fx = Fx::new();
    let stack = Stack::new(&mut fx, &outer_zip(&inner_zip(b"first")));
    std::fs::remove_file(stack.host.join("outer.zip")).unwrap();

    // The path is gone; the mount reads the archive it opened.
    assert_eq!(
        read(open_dir(stack.repo, "/z"), "notes.txt").unwrap(),
        b"notes"
    );
    let nested = stack.nested_inner("/n");
    assert!(!nested.is_null());
    assert_eq!(
        read(open_dir(stack.repo, "/n"), "a.txt").unwrap(),
        compressible(b"first")
    );
}

/// The resource layer reads a nested archive through the same mounts as any
/// other: the index loader and the directory scanner are the game's own calls.
mod resource_layer {
    use std::ptr::NonNull;

    use super::*;
    use crate::resource::direct::{load_dir_entry_table, scan_dir_entries, MatchType};
    use crate::resource::ffi_bridge::{
        res_GetInteger, res_GetString, res_HasKey, LoadResourceIndex, UninitResourceSystem,
    };

    #[test]
    #[serial]
    fn a_resource_index_loads_from_a_nested_archive() {
        unsafe { UninitResourceSystem() };
        let mut fx = Fx::new();
        let stack = Stack::new(&mut fx, &outer_zip(&inner_zip(b"first")));
        stack.nested_inner("/n");
        let dir = open_dir(stack.repo, "/n/res");

        unsafe { LoadResourceIndex(dir.cast(), c"game.rmp".as_ptr(), c"nested.".as_ptr()) };
        let name = unsafe { res_GetString(c"nested.name".as_ptr()) };
        assert!(!name.is_null());
        assert_eq!(unsafe { CStr::from_ptr(name) }, c"Nested");
        assert_eq!(unsafe { res_GetInteger(c"nested.vol".as_ptr()) }, 7);
        assert_eq!(unsafe { res_HasKey(c"vol".as_ptr()) }, 0);
        unsafe { UninitResourceSystem() };
    }

    #[test]
    #[serial]
    fn a_directory_scan_lists_the_regular_entries_of_a_nested_archive() {
        let mut fx = Fx::new();
        let stack = Stack::new(&mut fx, &outer_zip(&inner_zip(b"first")));
        stack.nested_inner("/n");
        let base = NonNull::new(open_dir(stack.repo, "/n")).unwrap();

        let names =
            unsafe { scan_dir_entries(base, c"lists", c".mle", MatchType::Suffix) }.unwrap();
        assert_eq!(names, [c"one.mle".to_owned(), c"two.mle".to_owned()]);
        let table = unsafe { load_dir_entry_table(base, c"lists", c".mle", MatchType::Suffix) }
            .unwrap()
            .expect("two entries make a table");
        assert_eq!(table.len(), 2);
    }
}
