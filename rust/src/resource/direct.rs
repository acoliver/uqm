//! Directory entry tables (`LoadDirEntryTable`).
//!
//! Port of `sc2/src/libs/resource/direct.c`. The table lists the regular
//! files of a UIO directory that match a pattern, as a C `STRING_TABLE`
//! (see [`super::string_table`]).
//!
//! Listing, stat and directory handles come from the in-crate Rust UIO
//! (`crate::io::uio_bridge`), so mounts, zip archives, mount precedence and
//! de-duplication behave exactly as they do for every other UIO caller.
//!
//! Selection follows `direct.c`: names starting with `.` are skipped, as are
//! entries that cannot be stat'ed and entries that are not regular files.
//! Names are ordered by raw byte value (`strcmp`). The C UIO listing the
//! original code relied on returned names in that order; the Rust
//! `uio_getDirList` returns them in directory order, so the order is
//! restored here.

use std::ffi::{c_int, CStr, CString};
use std::fmt;
use std::ptr::NonNull;
use std::slice;

use crate::io::uio_bridge::{
    uio_DirHandle, uio_DirList, uio_DirList_free, uio_closeDir, uio_getDirList,
    uio_openDirRelative, uio_stat,
};

use super::string_table::OwnedStringTable;

pub mod c_abi;

#[cfg(test)]
mod tests;

/// `match_MatchType` from `sc2/src/libs/uio/match.h` as compiled for the
/// game (`HAVE_REGEX` defined, `HAVE_GLOB` not).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(i32)]
pub enum MatchType {
    Literal = 0,
    Prefix = 1,
    Suffix = 2,
    Substring = 3,
    Regex = 4,
}

impl MatchType {
    /// Converts the C enum value; `None` for values the C enum lacks.
    pub fn from_c(value: c_int) -> Option<Self> {
        match value {
            0 => Some(Self::Literal),
            1 => Some(Self::Prefix),
            2 => Some(Self::Suffix),
            3 => Some(Self::Substring),
            4 => Some(Self::Regex),
            _ => None,
        }
    }
}

/// The two conditions `direct.c` asserts on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DirEntryError {
    /// `uio_openDirRelative` returned no handle.
    OpenDir,
    /// `uio_getDirList` returned no list.
    ListDir,
}

impl fmt::Display for DirEntryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DirEntryError::OpenDir => f.write_str("uio_openDirRelative failed"),
            DirEntryError::ListDir => f.write_str("uio_getDirList failed"),
        }
    }
}

impl std::error::Error for DirEntryError {}

/// A UIO directory handle that is closed when dropped.
struct OpenDir(NonNull<uio_DirHandle>);

impl OpenDir {
    /// # Safety
    ///
    /// `base` must be a live handle from the UIO.
    unsafe fn open_relative(base: NonNull<uio_DirHandle>, path: &CStr) -> Option<Self> {
        NonNull::new(uio_openDirRelative(base.as_ptr(), path.as_ptr(), 0)).map(Self)
    }
}

impl Drop for OpenDir {
    fn drop(&mut self) {
        // SAFETY: the handle came from uio_openDirRelative and is closed once.
        unsafe {
            uio_closeDir(self.0.as_ptr());
        }
    }
}

/// A UIO directory list that is freed when dropped. Names borrow the list.
struct DirListing(NonNull<uio_DirList>);

impl DirListing {
    fn read(dir: &OpenDir, pattern: &CStr, match_type: MatchType) -> Option<Self> {
        // SAFETY: `dir` is a live handle and both strings are NUL-terminated.
        let list = unsafe {
            uio_getDirList(
                dir.0.as_ptr(),
                c"".as_ptr(),
                pattern.as_ptr(),
                match_type as c_int,
            )
        };
        NonNull::new(list).map(Self)
    }

    fn names(&self) -> Vec<&CStr> {
        // SAFETY: a list returned by uio_getDirList holds `numNames`
        // NUL-terminated names that stay valid until it is freed in Drop.
        unsafe {
            let list = self.0.as_ref();
            let count = usize::try_from(list.numNames).expect("name count is not negative");
            if count == 0 {
                return Vec::new();
            }
            slice::from_raw_parts(list.names, count)
                .iter()
                .map(|&name| CStr::from_ptr(name))
                .collect()
        }
    }
}

impl Drop for DirListing {
    fn drop(&mut self) {
        // SAFETY: the list came from uio_getDirList and is freed once.
        unsafe { uio_DirList_free(self.0.as_ptr()) }
    }
}

/// Whether `direct.c` keeps `name`: not hidden, stat-able, and a regular file.
fn is_listed(dir: &OpenDir, name: &CStr) -> bool {
    if name.to_bytes().first() == Some(&b'.') {
        return false;
    }
    // SAFETY: an all-zero libc::stat is a valid value, and uio_stat only
    // writes through the pointer we pass it.
    let mut info: libc::stat = unsafe { std::mem::zeroed() };
    let status = unsafe { uio_stat(dir.0.as_ptr(), name.as_ptr(), &mut info) };
    status != -1 && info.st_mode & libc::S_IFMT == libc::S_IFREG
}

/// Names of the regular files in `base`/`path` that match `pattern`, in byte
/// order.
///
/// # Safety
///
/// `base` must be a live handle returned by the UIO and must stay valid for
/// the duration of the call.
pub unsafe fn scan_dir_entries(
    base: NonNull<uio_DirHandle>,
    path: &CStr,
    pattern: &CStr,
    match_type: MatchType,
) -> Result<Vec<CString>, DirEntryError> {
    let dir = OpenDir::open_relative(base, path).ok_or(DirEntryError::OpenDir)?;
    let listing = DirListing::read(&dir, pattern, match_type).ok_or(DirEntryError::ListDir)?;
    let mut names: Vec<CString> = listing
        .names()
        .into_iter()
        .filter(|name| is_listed(&dir, name))
        .map(CStr::to_owned)
        .collect();
    names.sort();
    Ok(names)
}

/// Builds the table `LoadDirEntryTable` returns; `None` when no entry
/// qualifies, where C returns a null reference.
///
/// # Safety
///
/// Same contract as [`scan_dir_entries`].
pub unsafe fn load_dir_entry_table(
    base: NonNull<uio_DirHandle>,
    path: &CStr,
    pattern: &CStr,
    match_type: MatchType,
) -> Result<Option<OwnedStringTable>, DirEntryError> {
    let names = scan_dir_entries(base, path, pattern, match_type)?;
    Ok(OwnedStringTable::from_names(&names))
}
