//! Thin owners around the public UIO C-ABI functions, used the way the C
//! game uses them. Nothing here reaches into UIO internals: mounts, directory
//! handles, listings and streams all go through the exported functions.

use std::ffi::{c_int, c_void, CStr, CString};
use std::path::Path;
use std::ptr;

use crate::io::uio_bridge::{
    uio_DirHandle, uio_DirList_free, uio_MountHandle, uio_Repository, uio_Stream, uio_closeDir,
    uio_closeRepository, uio_fclose, uio_fopen, uio_fread, uio_getDirList, uio_init, uio_mountDir,
    uio_openDir, uio_openDirRelative, uio_openRepository, uio_stat, uio_transplantDir, uio_unInit,
    uio_unmountDir,
};

use super::declared::{FSTYPE_STDIO, FSTYPE_ZIP, MATCH_LITERAL};

pub(super) fn cstr(text: &str) -> CString {
    CString::new(text).expect("fixture text has no NUL")
}

fn last_errno() -> i32 {
    std::io::Error::last_os_error().raw_os_error().unwrap_or(0)
}

/// One test's claim on the global UIO and resource state. Created first in
/// every row; clears both on the way in and on the way out.
pub(super) struct Session;

impl Session {
    pub(super) fn begin() -> Self {
        unsafe {
            crate::resource::ffi_bridge::UninitResourceSystem();
            uio_unInit();
            uio_init();
        }
        Session
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        unsafe {
            crate::resource::ffi_bridge::UninitResourceSystem();
            uio_unInit();
        }
    }
}

/// Handle to a mount; `NONE` is the C `NULL` relative handle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Mount(pub(super) *mut uio_MountHandle);

impl Mount {
    pub(super) const NONE: Mount = Mount(ptr::null_mut());

    pub(super) fn is_null(self) -> bool {
        self.0.is_null()
    }
}

pub(super) struct Repo {
    raw: *mut uio_Repository,
}

impl Repo {
    pub(super) fn open() -> Self {
        let raw = unsafe { uio_openRepository(0) };
        assert!(!raw.is_null());
        Repo { raw }
    }

    /// `uio_mountDir (repo, point, STDIO, NULL, NULL, host, autoMount, flags, relative)`
    pub(super) fn mount_stdio(
        &self,
        point: &str,
        host: &Path,
        flags: c_int,
        relative: Mount,
    ) -> Mount {
        let point = cstr(point);
        let host = cstr(host.to_str().expect("fixture paths are UTF-8"));
        Mount(unsafe {
            uio_mountDir(
                self.raw,
                point.as_ptr(),
                FSTYPE_STDIO,
                ptr::null_mut(),
                ptr::null(),
                host.as_ptr(),
                ptr::null_mut(),
                flags,
                relative.0,
            )
        })
    }

    /// `uio_mountDir (repo, point, ZIP, dir, name, "/", autoMount, flags, relative)`
    pub(super) fn mount_zip(
        &self,
        point: &str,
        dir: &Dir,
        name: &str,
        flags: c_int,
        relative: Mount,
    ) -> Mount {
        let point = cstr(point);
        let name = cstr(name);
        let inside = cstr("/");
        Mount(unsafe {
            uio_mountDir(
                self.raw,
                point.as_ptr(),
                FSTYPE_ZIP,
                dir.raw,
                name.as_ptr(),
                inside.as_ptr(),
                ptr::null_mut(),
                flags,
                relative.0,
            )
        })
    }

    pub(super) fn transplant(
        &self,
        point: &str,
        source: &Dir,
        flags: c_int,
        relative: Mount,
    ) -> Mount {
        let point = cstr(point);
        Mount(unsafe { uio_transplantDir(point.as_ptr(), source.raw, flags, relative.0) })
    }

    pub(super) fn open_dir(&self, path: &str) -> Dir {
        let path = cstr(path);
        let raw = unsafe { uio_openDir(self.raw, path.as_ptr(), 0) };
        assert!(!raw.is_null(), "uio_openDir returned NULL");
        Dir { raw }
    }

    pub(super) fn unmount(&self, mount: Mount) -> c_int {
        unsafe { uio_unmountDir(mount.0) }
    }

    pub(super) fn close(self) {
        drop(self);
    }
}

impl Drop for Repo {
    fn drop(&mut self) {
        unsafe { uio_closeRepository(self.raw) };
    }
}

pub(super) struct Dir {
    pub(super) raw: *mut uio_DirHandle,
}

impl Dir {
    pub(super) fn as_resource_dir(&self) -> *mut c_void {
        self.raw.cast()
    }

    pub(super) fn open_relative(&self, path: &str) -> Dir {
        let path = cstr(path);
        let raw = unsafe { uio_openDirRelative(self.raw, path.as_ptr(), 0) };
        assert!(!raw.is_null(), "uio_openDirRelative returned NULL");
        Dir { raw }
    }

    /// `uio_getDirList (dir, "", pattern, matchType)` as sorted names.
    pub(super) fn list(&self, pattern: &str, match_type: c_int) -> Vec<String> {
        let empty = cstr("");
        let pattern = cstr(pattern);
        unsafe {
            let list = uio_getDirList(self.raw, empty.as_ptr(), pattern.as_ptr(), match_type);
            assert!(!list.is_null(), "uio_getDirList returned NULL");
            let count = usize::try_from((*list).numNames).expect("count is not negative");
            let names = (0..count)
                .map(|i| {
                    CStr::from_ptr(*(*list).names.add(i))
                        .to_string_lossy()
                        .into_owned()
                })
                .collect();
            uio_DirList_free(list);
            names
        }
    }

    pub(super) fn list_all(&self) -> Vec<String> {
        self.list("", MATCH_LITERAL)
    }

    /// Mode and size from `uio_stat`, or the errno of the failure.
    pub(super) fn stat(&self, path: &str) -> Result<(u32, i64), i32> {
        let path = cstr(path);
        unsafe {
            let mut sb: libc::stat = std::mem::zeroed();
            if uio_stat(self.raw, path.as_ptr(), &mut sb) == 0 {
                Ok((u32::from(sb.st_mode), sb.st_size))
            } else {
                Err(last_errno())
            }
        }
    }

    pub(super) fn is_dir(&self, path: &str) -> bool {
        matches!(self.stat(path), Ok((mode, _)) if mode & u32::from(libc::S_IFMT) == u32::from(libc::S_IFDIR))
    }

    pub(super) fn open_stream(&self, path: &str) -> Result<Stream, i32> {
        let path = cstr(path);
        let mode = cstr("rb");
        let raw = unsafe { uio_fopen(self.raw, path.as_ptr(), mode.as_ptr()) };
        if raw.is_null() {
            Err(last_errno())
        } else {
            Ok(Stream { raw })
        }
    }

    /// The whole file through `uio_fopen`/`uio_fread`, or the errno.
    pub(super) fn read(&self, path: &str) -> Result<Vec<u8>, i32> {
        Ok(self.open_stream(path)?.read_to_end())
    }
}

impl Drop for Dir {
    fn drop(&mut self) {
        unsafe { uio_closeDir(self.raw) };
    }
}

pub(super) struct Stream {
    raw: *mut uio_Stream,
}

impl Stream {
    pub(super) fn read_to_end(&self) -> Vec<u8> {
        let mut out = Vec::new();
        let mut buf = [0u8; 4096];
        loop {
            let n = unsafe { uio_fread(buf.as_mut_ptr().cast(), 1, buf.len(), self.raw) };
            if n == 0 {
                return out;
            }
            out.extend_from_slice(&buf[..n]);
        }
    }

    pub(super) fn read_exact_prefix(&self, len: usize) -> Vec<u8> {
        let mut buf = vec![0u8; len];
        let n = unsafe { uio_fread(buf.as_mut_ptr().cast(), 1, len, self.raw) };
        buf.truncate(n);
        buf
    }
}

impl Drop for Stream {
    fn drop(&mut self) {
        unsafe { uio_fclose(self.raw) };
    }
}
