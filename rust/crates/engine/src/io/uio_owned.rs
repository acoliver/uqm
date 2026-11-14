//! Retained directories and exclusive streams over the existing UIO mount resolver.
use super::*;
use std::{ffi::CStr, io, ptr::NonNull};

#[repr(C)]
pub struct uio_DirHandle {
    pub(super) path: PathBuf,
    /// The original virtual path (e.g. "/packages") used for mount resolution.
    /// `path` holds the resolved host path; this holds the pre-resolution virtual path.
    pub(super) virtual_path: PathBuf,
    pub(super) refcount: std::sync::atomic::AtomicI32,
    pub(super) repository: *mut uio_Repository,
    pub(super) root_end: PathBuf,
}

impl uio_DirHandle {
    pub(super) fn retain_reference(&self) {
        let previous = self.refcount.fetch_add(1, Ordering::SeqCst);
        assert!(previous > 0, "retaining a closed UIO directory");
    }
}

#[derive(Debug)]
pub struct Directory(NonNull<uio_DirHandle>);

impl Directory {
    /// # Safety
    /// `directory` must hold a live UIO reference for this call. Retaining it
    /// extends the handle lifetime independently of the caller's reference.
    pub unsafe fn retain(directory: NonNull<uio_DirHandle>) -> Self {
        // SAFETY: the caller supplies a live reference; UIO uses this same refcount on close.
        unsafe { directory.as_ref().retain_reference() };
        Self(directory)
    }

    pub fn open(&self, name: &CStr) -> io::Result<Stream> {
        self.open_mode(name, c"rb")
    }

    pub fn create(&self, name: &CStr) -> io::Result<Stream> {
        self.open_mode(name, c"wb")
    }

    fn open_mode(&self, name: &CStr, mode: &CStr) -> io::Result<Stream> {
        use crate::host::Files;
        let files = self.files();
        let path = Path::new(
            name.to_str()
                .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))?,
        );
        if mode == c"rb" {
            files.open_read(path)
        } else {
            files.open_write(path)
        }
    }

    pub fn files(&self) -> DirectoryFiles {
        // SAFETY: self owns a live reference throughout the snapshot.
        unsafe { directory(self.0.as_ptr()).expect("owned directory") }
    }

    /// Regular, non-hidden entries in the same enumeration order as the native browser.
    pub fn files_with_suffix(&self, suffix: &CStr) -> Vec<std::ffi::CString> {
        use crate::strings::string_table::{
            CaptureStringTable, FreeStringTable, GetStringAddress, GetStringTableCount,
            SetAbsStringTableIndex,
        };
        // SAFETY: the provider owns each returned string until the table is freed below.
        unsafe {
            let table = crate::resource::direct::LoadDirEntryTable(
                self.0.as_ptr(),
                c"".as_ptr(),
                suffix.as_ptr(),
                3,
            );
            if table.is_null() {
                return Vec::new();
            }
            let names = (0..GetStringTableCount(CaptureStringTable(table)))
                .map(|index| {
                    CStr::from_ptr(GetStringAddress(SetAbsStringTableIndex(
                        CaptureStringTable(table),
                        index,
                    )))
                    .to_owned()
                })
                .collect();
            FreeStringTable(table);
            names
        }
    }

    pub fn remove(&self, name: &CStr) -> io::Result<()> {
        use crate::host::Files;
        self.files()
            .remove(Path::new(name.to_str().map_err(|error| {
                io::Error::new(io::ErrorKind::InvalidInput, error)
            })?))
    }
}
impl Clone for Directory {
    fn clone(&self) -> Self {
        // SAFETY: this owner keeps its existing reference live through the increment.
        unsafe { Self::retain(self.0) }
    }
}
impl Drop for Directory {
    fn drop(&mut self) {
        // SAFETY: release exactly the reference acquired by retain or clone.
        unsafe { uio_closeDir(self.0.as_ptr()) };
    }
}

pub use crate::host::FileStream as Stream;

#[derive(Clone, Debug)]
pub struct DirectoryFiles {
    path: PathBuf,
    virtual_path: PathBuf,
}

/// Snapshot a transitional directory at the FFI boundary.
/// # Safety
/// A non-null pointer must be a live directory for this call.
pub unsafe fn directory(pointer: *mut uio_DirHandle) -> io::Result<DirectoryFiles> {
    // SAFETY: caller supplies a live pointer; null is an external ABI input.
    let directory =
        unsafe { pointer.as_ref() }.ok_or_else(|| io::Error::from(io::ErrorKind::InvalidInput))?;
    Ok(DirectoryFiles {
        path: directory.path.clone(),
        virtual_path: directory.virtual_path.clone(),
    })
}

impl crate::host::Files for DirectoryFiles {
    fn open_read(&self, path: &Path) -> io::Result<Stream> {
        get_mount_registry()
            .lock()
            .unwrap()
            .open_at(&self.path, &self.virtual_path, path, false)
    }
    fn open_write(&self, path: &Path) -> io::Result<Stream> {
        get_mount_registry()
            .lock()
            .unwrap()
            .open_at(&self.path, &self.virtual_path, path, true)
    }
    fn remove(&self, path: &Path) -> io::Result<()> {
        get_mount_registry()
            .lock()
            .unwrap()
            .remove_at(&self.path, &self.virtual_path, path)
    }
    fn list(&self, path: &Path) -> io::Result<Vec<String>> {
        get_mount_registry()
            .lock()
            .unwrap()
            .list_at(&self.path, &self.virtual_path, path)
    }
    fn stat(&self, path: &Path) -> io::Result<crate::host::FileStat> {
        get_mount_registry()
            .lock()
            .unwrap()
            .stat_at(&self.path, &self.virtual_path, path)
    }
}

/// # Safety
/// Directory and filename must remain live for this ABI call.
pub unsafe fn read_file(dir: *mut uio_DirHandle, name: *const c_char) -> io::Result<Vec<u8>> {
    use crate::host::Files;
    if name.is_null() {
        return Err(io::Error::from(io::ErrorKind::InvalidInput));
    }
    // SAFETY: caller supplies a terminated string and live directory.
    let path = unsafe { CStr::from_ptr(name) }.to_string_lossy();
    unsafe { directory(dir) }?.read(Path::new(path.as_ref()))
}
