//! Resource index and resource file calls against real files behind the
//! Rust UIO layer: loading and saving index files, the stream-like file
//! helpers, and loading resource data from a content directory.

use std::ffi::{c_char, c_void, CStr, CString};
use std::ptr;

use serial_test::serial;
use tempfile::TempDir;

use super::*;
use crate::io::uio_bridge::{
    uio_Repository, uio_closeDir, uio_closeRepository, uio_init, uio_mountDir, uio_openDir,
    uio_openRepository, uio_unInit,
};

const UIO_FSTYPE_STDIO: c_int = 1;
const UIO_MOUNT_TOP: c_int = 1 << 2;

/// A temporary directory mounted at the root of a fresh UIO repository.
struct Content {
    files: TempDir,
    repository: *mut uio_Repository,
    dir: *mut c_void,
}

impl Content {
    fn new(files: &[(&str, &[u8])]) -> Self {
        reset_state();
        let temp = TempDir::new().unwrap();
        for (name, data) in files {
            std::fs::write(temp.path().join(name), data).unwrap();
        }
        unsafe {
            uio_init();
            let repository = uio_openRepository(0);
            let root = c("/");
            let host = c(temp.path().to_str().unwrap());
            let mount = uio_mountDir(
                repository,
                root.as_ptr(),
                UIO_FSTYPE_STDIO,
                ptr::null_mut(),
                ptr::null(),
                host.as_ptr(),
                ptr::null_mut(),
                UIO_MOUNT_TOP,
                ptr::null_mut(),
            );
            assert!(!mount.is_null());
            let dir = uio_openDir(repository, root.as_ptr(), 0);
            assert!(!dir.is_null());
            Content {
                files: temp,
                repository,
                dir: dir.cast(),
            }
        }
    }

    fn host(&self, name: &str) -> std::path::PathBuf {
        self.files.path().join(name)
    }
}

impl Drop for Content {
    fn drop(&mut self) {
        unsafe {
            uio_closeDir(self.dir.cast());
            uio_closeRepository(self.repository);
            uio_unInit();
        }
        reset_state();
    }
}

fn reset_state() {
    let mut guard = RESOURCE_STATE.lock().unwrap_or_else(|p| p.into_inner());
    *guard = None;
}

fn c(text: &str) -> CString {
    CString::new(text).unwrap()
}

fn load(content: &Content, name: &str, prefix: Option<&str>) {
    let name = c(name);
    let prefix = prefix.map(c);
    unsafe {
        LoadResourceIndex(
            content.dir,
            name.as_ptr(),
            prefix.as_ref().map_or(ptr::null(), |p| p.as_ptr()),
        )
    };
}

fn get_string(key: &str) -> Option<String> {
    let key = c(key);
    let got = unsafe { res_GetString(key.as_ptr()) };
    if got.is_null() {
        return None;
    }
    Some(
        unsafe { CStr::from_ptr(got) }
            .to_string_lossy()
            .into_owned(),
    )
}

fn int(key: &str) -> c_int {
    unsafe { res_GetInteger(c(key).as_ptr()) }
}

fn has_key(key: &str) -> bool {
    unsafe { res_HasKey(c(key).as_ptr()) != 0 }
}

fn save(content: &Content, name: &str, root: Option<&str>, strip: c_int) {
    let name = c(name);
    let root = root.map(c);
    unsafe {
        SaveResourceIndex(
            content.dir,
            name.as_ptr(),
            root.as_ref().map_or(ptr::null(), |r| r.as_ptr()),
            strip,
        )
    };
}

fn open(content: &Content, name: &str, mode: &str) -> *mut c_void {
    unsafe { res_OpenResFile(content.dir, c(name).as_ptr(), c(mode).as_ptr()) }
}

#[test]
#[serial]
fn loading_an_index_file_defines_typed_values() {
    let content = Content::new(&[(
        "game.rmp",
        b"# comment line\nname = STRING:Zoq-Fot-Pik\nsfxvol = INT32:20\nfullscreen = BOOLEAN:true\nhue = COLOR:rgb(0x10, 0x20, 0x30)\n",
    )]);
    load(&content, "game.rmp", None);

    assert_eq!(get_string("name").as_deref(), Some("Zoq-Fot-Pik"));
    assert_eq!(int("sfxvol"), 20);
    assert_eq!(unsafe { res_GetBoolean(c("fullscreen").as_ptr()) }, 1);
    assert_ne!(unsafe { res_GetColor(c("hue").as_ptr()) }, 0);
    assert!(!has_key("comment"));
}

#[test]
#[serial]
fn loading_with_a_prefix_namespaces_every_key() {
    let content = Content::new(&[("addon.rmp", b"battle = STRING:song.ogg\nvol = INT32:3\n")]);
    load(&content, "addon.rmp", Some("mod.addon."));

    assert_eq!(get_string("mod.addon.battle").as_deref(), Some("song.ogg"));
    assert_eq!(int("mod.addon.vol"), 3);
    assert!(!has_key("battle"));
}

#[test]
#[serial]
fn a_later_index_overrides_earlier_values_for_the_same_key() {
    let content = Content::new(&[
        ("base.rmp", b"music = STRING:base.mod\nkeep = INT32:1\n"),
        ("addon.rmp", b"music = STRING:addon.ogg\n"),
    ]);
    load(&content, "base.rmp", None);
    load(&content, "addon.rmp", None);

    assert_eq!(get_string("music").as_deref(), Some("addon.ogg"));
    assert_eq!(int("keep"), 1);
}

#[test]
#[serial]
fn loading_an_unusable_index_changes_nothing() {
    let content = Content::new(&[
        ("empty.rmp", b""),
        ("binary.rmp", &[0xFF, 0xFE, 0x00, 0x80]),
    ]);
    load(&content, "empty.rmp", None);
    load(&content, "binary.rmp", None);
    load(&content, "missing.rmp", None);
    unsafe { LoadResourceIndex(content.dir, ptr::null(), ptr::null()) };
    assert!(!has_key("anything"));
    assert_eq!(unsafe { CountResourceTypes() }, 5);
}

#[test]
#[serial]
fn saving_writes_sorted_entries_that_load_back_to_the_same_values() {
    let content = Content::new(&[(
        "in.rmp",
        b"b.name = STRING:beta\na.count = INT32:7\nc.flag = BOOLEAN:false\n",
    )]);
    load(&content, "in.rmp", None);

    save(&content, "out.rmp", None, 0);

    let written = std::fs::read_to_string(content.host("out.rmp")).unwrap();
    let lines: Vec<&str> = written.lines().collect();
    assert_eq!(
        lines,
        [
            "a.count = INT32:7",
            "b.name = STRING:beta",
            "c.flag = BOOLEAN:false"
        ]
    );

    reset_state();
    load(&content, "out.rmp", None);
    assert_eq!(int("a.count"), 7);
    assert_eq!(get_string("b.name").as_deref(), Some("beta"));
    assert_eq!(unsafe { res_GetBoolean(c("c.flag").as_ptr()) }, 0);
}

#[test]
#[serial]
fn saving_with_a_root_keeps_only_matching_keys_and_can_strip_the_root() {
    let content = Content::new(&[(
        "in.rmp",
        b"config.sfxvol = INT32:9\nconfig.music = STRING:on\nother.value = INT32:1\n",
    )]);
    load(&content, "in.rmp", None);

    save(&content, "kept.rmp", Some("config."), 0);
    save(&content, "stripped.rmp", Some("config."), 1);

    let kept = std::fs::read_to_string(content.host("kept.rmp")).unwrap();
    assert_eq!(kept, "config.music = STRING:on\nconfig.sfxvol = INT32:9\n");
    let stripped = std::fs::read_to_string(content.host("stripped.rmp")).unwrap();
    assert_eq!(stripped, "music = STRING:on\nsfxvol = INT32:9\n");
}

#[test]
#[serial]
fn saving_skips_entries_whose_type_cannot_be_written_back() {
    let content = Content::new(&[("in.rmp", b"known = INT32:4\nweird = GFXRES:path/to/thing\n")]);
    load(&content, "in.rmp", None);
    save(&content, "out.rmp", None, 0);
    let written = std::fs::read_to_string(content.host("out.rmp")).unwrap();
    assert_eq!(written, "known = INT32:4\n");
}

#[test]
#[serial]
fn saving_with_no_file_name_does_nothing() {
    let content = Content::new(&[]);
    unsafe { SaveResourceIndex(content.dir, ptr::null(), ptr::null(), 0) };
    assert_eq!(std::fs::read_dir(content.files.path()).unwrap().count(), 0);
}

#[test]
#[serial]
fn opening_a_directory_yields_the_sentinel_and_missing_files_yield_null() {
    let content = Content::new(&[("data.bin", b"abc")]);
    std::fs::create_dir(content.host("sub")).unwrap();

    assert_eq!(open(&content, "sub", "rb"), STREAM_SENTINEL);
    assert!(open(&content, "missing.bin", "rb").is_null());
    unsafe {
        assert!(res_OpenResFile(content.dir, ptr::null(), c("rb").as_ptr()).is_null());
        assert!(res_OpenResFile(content.dir, c("data.bin").as_ptr(), ptr::null()).is_null());
    }
}

#[test]
#[serial]
fn file_helpers_read_seek_and_measure_a_resource_file() {
    let content = Content::new(&[("data.bin", b"0123456789")]);
    let fp = open(&content, "data.bin", "rb");
    assert!(!fp.is_null() && fp != STREAM_SENTINEL);
    unsafe {
        assert_eq!(LengthResFile(fp), 10);
        assert_eq!(TellResFile(fp), 0, "measuring does not move the position");

        let mut buf = [0u8; 4];
        assert_eq!(ReadResFile(buf.as_mut_ptr() as *mut c_void, 1, 4, fp), 4);
        assert_eq!(&buf, b"0123");
        assert_eq!(TellResFile(fp), 4);
        assert_eq!(LengthResFile(fp), 10);
        assert_eq!(TellResFile(fp), 4, "position survives a length query");

        assert_eq!(GetResFileChar(fp), b'4' as c_int);
        assert_eq!(SeekResFile(fp, 8, 0), 0);
        assert_eq!(GetResFileChar(fp), b'8' as c_int);
        assert_eq!(SeekResFile(fp, -1, 2), 0);
        assert_eq!(GetResFileChar(fp), b'9' as c_int);
        assert_eq!(GetResFileChar(fp), -1);
        assert_eq!(res_CloseResFile(fp), 1);
    }
}

#[test]
#[serial]
fn file_helpers_write_characters_and_blocks() {
    let content = Content::new(&[]);
    let fp = open(&content, "out.txt", "wb");
    assert!(!fp.is_null());
    unsafe {
        assert_eq!(PutResFileChar(b'A' as c_char, fp), b'A' as c_int);
        assert_eq!(PutResFileNewline(fp), b'\n' as c_int);
        let block = b"block";
        assert_eq!(
            WriteResFile(block.as_ptr() as *const c_void, 1, block.len(), fp),
            5
        );
        assert_eq!(res_CloseResFile(fp), 1);
    }
    assert_eq!(std::fs::read(content.host("out.txt")).unwrap(), b"A\nblock");
}

#[test]
#[serial]
fn file_helpers_treat_null_and_directory_streams_as_empty() {
    let mut buf = [0u8; 4];
    let block = [1u8; 4];
    unsafe {
        for fp in [ptr::null_mut(), STREAM_SENTINEL] {
            assert_eq!(ReadResFile(buf.as_mut_ptr() as *mut c_void, 1, 4, fp), 0);
            assert_eq!(WriteResFile(block.as_ptr() as *const c_void, 1, 4, fp), 0);
            assert_eq!(GetResFileChar(fp), -1);
            assert_eq!(PutResFileChar(b'x' as c_char, fp), -1);
            assert_eq!(PutResFileNewline(fp), -1);
            assert_eq!(SeekResFile(fp, 0, 0), -1);
            assert_eq!(TellResFile(fp), -1);
            assert_eq!(res_CloseResFile(fp), 1);
        }
        assert_eq!(LengthResFile(ptr::null_mut()), 0);
        assert_eq!(LengthResFile(STREAM_SENTINEL), 1);
    }
}

#[test]
#[serial]
fn deleting_a_resource_file_removes_it_and_reports_failure_for_a_missing_one() {
    let content = Content::new(&[("old.dat", b"x"), ("keep.dat", b"y")]);
    unsafe {
        assert_eq!(DeleteResFile(content.dir, c("old.dat").as_ptr()), 1);
        assert_eq!(DeleteResFile(content.dir, c("old.dat").as_ptr()), 0);
        assert_eq!(DeleteResFile(content.dir, ptr::null()), 0);
    }
    assert!(!content.host("old.dat").exists());
    assert!(content.host("keep.dat").exists());
}

unsafe extern "C" fn record_length(fp: *mut c_void, length: u32) -> *mut c_void {
    let mut first = [0u8; 1];
    if ReadResFile(first.as_mut_ptr() as *mut c_void, 1, 1, fp) != 1 {
        return ptr::null_mut();
    }
    let seen_name = if _cur_resfile_name.is_null() {
        0
    } else {
        CStr::from_ptr(_cur_resfile_name).to_bytes().len()
    };
    ((length as usize) * 1000 + (first[0] as usize) * 10 + seen_name.min(9)) as *mut c_void
}

#[test]
#[serial]
fn loading_a_resource_from_a_path_hands_the_open_file_to_the_loader() {
    let content = Content::new(&[("res.bin", b"Zabcd"), ("empty.bin", b"")]);
    unsafe {
        contentDir = content.dir.cast();
        let loaded = LoadResourceFromPath(c("res.bin").as_ptr(), Some(record_length));
        assert_eq!(
            loaded as usize,
            5 * 1000 + (b'Z' as usize) * 10 + 7,
            "length 5, first byte 'Z', current name length 7"
        );
        assert!(_cur_resfile_name.is_null(), "name is cleared after loading");

        assert!(LoadResourceFromPath(c("empty.bin").as_ptr(), Some(record_length)).is_null());
        assert!(LoadResourceFromPath(c("missing.bin").as_ptr(), Some(record_length)).is_null());
        assert!(LoadResourceFromPath(ptr::null(), Some(record_length)).is_null());
        assert!(LoadResourceFromPath(c("res.bin").as_ptr(), None).is_null());
        contentDir = ptr::null_mut();
    }
}

#[test]
#[serial]
fn resource_data_is_read_after_the_uncompressed_marker() {
    let mut file = 0xFFFF_FFFFu32.to_ne_bytes().to_vec();
    file.extend_from_slice(b"payload");
    let content = Content::new(&[("data.bin", &file)]);
    let fp = open(&content, "data.bin", "rb");
    unsafe {
        let data = GetResourceData(fp, file.len() as u32);
        assert!(!data.is_null());
        assert_eq!(std::slice::from_raw_parts(data as *const u8, 7), b"payload");
        assert_eq!(FreeResourceData(data), 1);
        assert_eq!(FreeResourceData(ptr::null_mut()), 1);
        res_CloseResFile(fp);
    }
}

#[test]
#[serial]
fn resource_data_refuses_compressed_short_and_empty_requests() {
    let mut compressed = 0x0000_0042u32.to_ne_bytes().to_vec();
    compressed.extend_from_slice(b"packed");
    let mut truncated = 0xFFFF_FFFFu32.to_ne_bytes().to_vec();
    truncated.extend_from_slice(b"abc");
    let content = Content::new(&[
        ("packed.bin", &compressed),
        ("short.bin", &truncated),
        ("tiny.bin", &0xFFFF_FFFFu32.to_ne_bytes()),
    ]);
    unsafe {
        let fp = open(&content, "packed.bin", "rb");
        assert!(GetResourceData(fp, compressed.len() as u32).is_null());
        res_CloseResFile(fp);

        let fp = open(&content, "short.bin", "rb");
        assert!(
            GetResourceData(fp, 100).is_null(),
            "file holds fewer bytes than claimed"
        );
        res_CloseResFile(fp);

        let fp = open(&content, "tiny.bin", "rb");
        assert!(
            GetResourceData(fp, 4).is_null(),
            "no payload after the marker"
        );
        res_CloseResFile(fp);

        let fp = open(&content, "tiny.bin", "rb");
        assert!(GetResourceData(fp, 0).is_null());
        res_CloseResFile(fp);
        assert!(GetResourceData(ptr::null_mut(), 8).is_null());
        assert!(GetResourceData(STREAM_SENTINEL, 8).is_null());
    }
}
