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
        // SAFETY: self retains the directory and the strings are terminated for this call.
        let stream = unsafe { uio_fopen(self.0.as_ptr(), name.as_ptr(), mode.as_ptr()) };
        NonNull::new(stream)
            .map(Stream)
            .ok_or_else(io::Error::last_os_error)
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
        // SAFETY: self retains the directory and name is terminated.
        let result = unsafe { uio_unlink(self.0.as_ptr(), name.as_ptr()) };
        if result == 0 {
            Ok(())
        } else {
            Err(io::Error::last_os_error())
        }
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

pub struct Stream(NonNull<uio_Stream>);
impl Stream {
    pub fn len(&self) -> io::Result<u64> {
        let mut stat = std::mem::MaybeUninit::uninit();
        // SAFETY: self exclusively owns the open stream, and fstat initializes stat on success.
        if unsafe { uio_fstat(uio_streamHandle(self.0.as_ptr()), stat.as_mut_ptr()) } == 0 {
            // SAFETY: successful fstat initialized every field.
            Ok(unsafe { stat.assume_init() }.st_size as u64)
        } else {
            Err(io::Error::last_os_error())
        }
    }

    pub fn is_empty(&self) -> io::Result<bool> {
        self.len().map(|size| size == 0)
    }

    pub fn finish(mut self) -> io::Result<()> {
        let flushed = io::Write::flush(&mut self);
        let this = std::mem::ManuallyDrop::new(self);
        // SAFETY: finish consumes the stream, suppressing Drop and closing it exactly once.
        let closed = unsafe { uio_fclose(this.0.as_ptr()) };
        flushed?;
        if closed == 0 {
            Ok(())
        } else {
            Err(io::Error::last_os_error())
        }
    }
}
impl io::Read for Stream {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        // SAFETY: the mutable borrow owns the stream and the complete writable slice.
        let read = unsafe { uio_fread(bytes.as_mut_ptr().cast(), 1, bytes.len(), self.0.as_ptr()) };
        // SAFETY: the stream remains live and exclusively borrowed.
        if read == 0 && unsafe { uio_ferror(self.0.as_ptr()) } != 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(read)
        }
    }
}
impl io::Write for Stream {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        // SAFETY: the mutable borrow owns the stream and the complete readable slice.
        let written = unsafe { uio_fwrite(bytes.as_ptr().cast(), 1, bytes.len(), self.0.as_ptr()) };
        // SAFETY: the stream remains live and exclusively borrowed.
        if written == 0 && unsafe { uio_ferror(self.0.as_ptr()) } != 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(written)
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        // SAFETY: this mutable borrow owns the live stream.
        if unsafe { uio_fflush(self.0.as_ptr()) } == 0 {
            Ok(())
        } else {
            Err(io::Error::last_os_error())
        }
    }
}
impl Drop for Stream {
    fn drop(&mut self) {
        // SAFETY: Stream is the sole release owner; no raw pointer escapes it.
        unsafe { uio_fclose(self.0.as_ptr()) };
    }
}
