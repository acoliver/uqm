//! Owning wrappers over the UIO handle types the commands use.
//!
//! Each wrapper closes what it owns when dropped, so a command cannot leak a
//! handle on an early return, and each fallible call reads `errno` before
//! anything else can overwrite it.

use std::ffi::{c_int, CStr};
use std::ptr::NonNull;

use super::Errno;
use crate::io::uio_bridge::{
    stat, uio_DirHandle, uio_DirList, uio_DirList_free, uio_Handle, uio_Repository,
    uio_StdioAccessHandle, uio_StdioAccessHandle_getPath, uio_Stream, uio_clearerr, uio_close,
    uio_closeDir, uio_fclose, uio_ferror, uio_fgets, uio_fopen, uio_fputc, uio_fputs, uio_fseek,
    uio_fwrite, uio_getDirList, uio_getStdioAccess, uio_mkdir, uio_open, uio_openDir,
    uio_openDirRelative, uio_read, uio_releaseStdioAccess, uio_rename, uio_rmdir, uio_stat,
    uio_unlink, uio_write,
};

/// A directory handle of a repository.
pub(super) struct Dir(NonNull<uio_DirHandle>);

impl Dir {
    /// Opens `/` of `repository`.
    ///
    /// # Safety
    ///
    /// `repository` is live for as long as the handle is used.
    pub(super) unsafe fn open_root(repository: NonNull<uio_Repository>) -> Result<Self, Errno> {
        // SAFETY: per the contract; the path is a C string.
        let handle = unsafe { uio_openDir(repository.as_ptr(), c"/".as_ptr(), 0) };
        NonNull::new(handle).map(Self).ok_or_else(Errno::last)
    }

    pub(super) fn open_relative(&self, path: &CStr) -> Result<Self, Errno> {
        // SAFETY: `self` is a live handle and `path` a C string.
        let handle = unsafe { uio_openDirRelative(self.0.as_ptr(), path.as_ptr(), 0) };
        NonNull::new(handle).map(Self).ok_or_else(Errno::last)
    }

    pub(super) fn virtual_path(&self) -> &[u8] {
        // SAFETY: the handle is live while `self` is.
        unsafe { self.0.as_ref() }.virtual_path_bytes()
    }

    pub(super) fn open_file(&self, path: &CStr, flags: c_int, mode: c_int) -> Result<File, Errno> {
        // SAFETY: `self` is a live handle and `path` a C string.
        let handle = unsafe { uio_open(self.0.as_ptr(), path.as_ptr(), flags, mode) };
        NonNull::new(handle).map(File).ok_or_else(Errno::last)
    }

    pub(super) fn open_stream(&self, path: &CStr, mode: &CStr) -> Result<Stream, Errno> {
        // SAFETY: `self` is a live handle; both arguments are C strings.
        let stream = unsafe { uio_fopen(self.0.as_ptr(), path.as_ptr(), mode.as_ptr()) };
        NonNull::new(stream).map(Stream).ok_or_else(Errno::last)
    }

    pub(super) fn make_dir(&self, path: &CStr, mode: libc::mode_t) -> Result<(), Errno> {
        // SAFETY: `self` is a live handle and `path` a C string.
        succeeded(unsafe { uio_mkdir(self.0.as_ptr(), path.as_ptr(), mode) })
    }

    pub(super) fn remove_dir(&self, path: &CStr) -> Result<(), Errno> {
        // SAFETY: `self` is a live handle and `path` a C string.
        succeeded(unsafe { uio_rmdir(self.0.as_ptr(), path.as_ptr()) })
    }

    pub(super) fn unlink(&self, path: &CStr) -> Result<(), Errno> {
        // SAFETY: `self` is a live handle and `path` a C string.
        succeeded(unsafe { uio_unlink(self.0.as_ptr(), path.as_ptr()) })
    }

    pub(super) fn rename(&self, from: &CStr, to: &CStr) -> Result<(), Errno> {
        // SAFETY: `self` is a live handle; both paths are C strings.
        succeeded(unsafe {
            uio_rename(self.0.as_ptr(), from.as_ptr(), self.0.as_ptr(), to.as_ptr())
        })
    }

    pub(super) fn stat(&self, path: &CStr) -> Result<stat, Errno> {
        // SAFETY: an all-zero `stat` is a valid value for the call to fill.
        let mut buffer: stat = unsafe { std::mem::zeroed() };
        // SAFETY: `self` is a live handle, `path` a C string, the buffer writable.
        let result = unsafe { uio_stat(self.0.as_ptr(), path.as_ptr(), &mut buffer) };
        succeeded(result).map(|()| buffer)
    }

    pub(super) fn list(
        &self,
        path: &CStr,
        pattern: &CStr,
        match_type: c_int,
    ) -> Result<DirListing, Errno> {
        // SAFETY: `self` is a live handle; both arguments are C strings.
        let list =
            unsafe { uio_getDirList(self.0.as_ptr(), path.as_ptr(), pattern.as_ptr(), match_type) };
        NonNull::new(list).map(DirListing).ok_or_else(Errno::last)
    }

    /// The path of `path` for a program: the host file itself, or a copy
    /// under `temp_dir` for an archive entry.
    pub(super) fn stdio_access(&self, path: &CStr, temp_dir: &Dir) -> Result<StdioAccess, Errno> {
        // SAFETY: both handles are live and `path` is a C string.
        let access = unsafe {
            uio_getStdioAccess(
                self.0.as_ptr(),
                path.as_ptr(),
                libc::O_RDONLY,
                temp_dir.0.as_ptr(),
            )
        };
        NonNull::new(access)
            .map(StdioAccess)
            .ok_or_else(Errno::last)
    }
}

impl Drop for Dir {
    fn drop(&mut self) {
        // SAFETY: the handle came from a UIO open call and is closed once.
        unsafe {
            uio_closeDir(self.0.as_ptr());
        }
    }
}

/// A file opened with `uio_open`.
pub(super) struct File(NonNull<uio_Handle>);

impl File {
    /// Reads up to `buffer.len()` bytes; 0 is the end of the file.
    pub(super) fn read(&self, buffer: &mut [u8]) -> Result<usize, Errno> {
        // SAFETY: the handle is live and the buffer writable for its length.
        let count = unsafe { uio_read(self.0.as_ptr(), buffer.as_mut_ptr(), buffer.len()) };
        usize::try_from(count).map_err(|_| Errno::last())
    }

    /// Writes all of `bytes`; the count `uio_write` reports when it fails
    /// short is not told apart from the failure, as in the C.
    pub(super) fn write(&self, bytes: &[u8]) -> Result<(), Errno> {
        // SAFETY: the handle is live and the buffer readable for its length.
        let count = unsafe { uio_write(self.0.as_ptr(), bytes.as_ptr(), bytes.len()) };
        if count == -1 {
            Err(Errno::last())
        } else {
            Ok(())
        }
    }
}

impl Drop for File {
    fn drop(&mut self) {
        // SAFETY: the handle came from `uio_open` and is closed once.
        unsafe {
            uio_close(self.0.as_ptr());
        }
    }
}

/// A stream opened with `uio_fopen`.
pub(super) struct Stream(NonNull<uio_Stream>);

impl Stream {
    /// `uio_fwrite` of `bytes` as items of `item_size` bytes; all of them
    /// must be written.
    pub(super) fn write_items(&self, bytes: &[u8], item_size: usize) -> Result<(), Errno> {
        let items = bytes.len() / item_size;
        // SAFETY: the stream is live and the buffer readable for its length.
        let written =
            unsafe { uio_fwrite(bytes.as_ptr().cast(), item_size, items, self.0.as_ptr()) };
        if written == items {
            Ok(())
        } else {
            Err(Errno::last())
        }
    }

    pub(super) fn put_str(&self, text: &CStr) -> Result<(), Errno> {
        // SAFETY: the stream is live and `text` a C string.
        let result = unsafe { uio_fputs(text.as_ptr(), self.0.as_ptr()) };
        if result == libc::EOF {
            Err(Errno::last())
        } else {
            Ok(())
        }
    }

    pub(super) fn put_char(&self, byte: u8) -> Result<(), Errno> {
        // SAFETY: the stream is live.
        let result = unsafe { uio_fputc(c_int::from(byte), self.0.as_ptr()) };
        if result == c_int::from(byte) {
            Ok(())
        } else {
            Err(Errno::last())
        }
    }

    pub(super) fn seek(&self, offset: libc::c_long, whence: c_int) -> Result<(), Errno> {
        // SAFETY: the stream is live.
        succeeded(unsafe { uio_fseek(self.0.as_ptr(), offset, whence) })
    }

    /// `uio_fgets` into `buffer`: the bytes read (without the NUL), or `None`
    /// at the end of the stream or on an error.
    pub(super) fn get_line(&self, buffer: &mut [u8]) -> Option<usize> {
        let size = c_int::try_from(buffer.len()).expect("the buffer is small");
        // SAFETY: the stream is live and the buffer writable for `size` bytes.
        let line = unsafe { uio_fgets(buffer.as_mut_ptr().cast(), size, self.0.as_ptr()) };
        if line.is_null() {
            return None;
        }
        // SAFETY: `uio_fgets` NUL-terminated the buffer it filled.
        Some(
            unsafe { CStr::from_ptr(buffer.as_ptr().cast()) }
                .to_bytes()
                .len(),
        )
    }

    pub(super) fn has_error(&self) -> bool {
        // SAFETY: the stream is live.
        unsafe { uio_ferror(self.0.as_ptr()) != 0 }
    }

    pub(super) fn clear_error(&self) {
        // SAFETY: the stream is live.
        unsafe { uio_clearerr(self.0.as_ptr()) }
    }
}

impl Drop for Stream {
    fn drop(&mut self) {
        // SAFETY: the stream came from `uio_fopen` and is closed once.
        unsafe {
            uio_fclose(self.0.as_ptr());
        }
    }
}

/// A directory list from `uio_getDirList`.
pub(super) struct DirListing(NonNull<uio_DirList>);

impl DirListing {
    pub(super) fn names(&self) -> Vec<&CStr> {
        // SAFETY: a list from `uio_getDirList` holds `numNames` NUL-terminated
        // names that stay valid until it is freed in Drop.
        unsafe {
            let list = self.0.as_ref();
            let count = usize::try_from(list.numNames).expect("a name count is not negative");
            if count == 0 {
                return Vec::new();
            }
            std::slice::from_raw_parts(list.names, count)
                .iter()
                .map(|&name| CStr::from_ptr(name))
                .collect()
        }
    }
}

impl Drop for DirListing {
    fn drop(&mut self) {
        // SAFETY: the list came from `uio_getDirList` and is freed once.
        unsafe { uio_DirList_free(self.0.as_ptr()) }
    }
}

/// A host path for a program, from `uio_getStdioAccess`. Releasing it
/// removes the copy made for an archive entry.
pub(super) struct StdioAccess(NonNull<uio_StdioAccessHandle>);

impl StdioAccess {
    pub(super) fn path(&self) -> &CStr {
        // SAFETY: the path stays valid until the handle is released in Drop.
        unsafe { CStr::from_ptr(uio_StdioAccessHandle_getPath(self.0.as_ptr())) }
    }
}

impl Drop for StdioAccess {
    fn drop(&mut self) {
        // SAFETY: the handle came from `uio_getStdioAccess` and is released once.
        unsafe { uio_releaseStdioAccess(self.0.as_ptr()) }
    }
}

fn succeeded(result: c_int) -> Result<(), Errno> {
    if result == -1 {
        Err(Errno::last())
    } else {
        Ok(())
    }
}
