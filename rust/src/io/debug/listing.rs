//! The text of `uio_printMounts` and `uio_DirHandle_print`.

use std::ffi::c_int;

use crate::io::uio_bridge::MountListing;

const FSTYPE_STDIO: c_int = 1;
const FSTYPE_ZIP: c_int = 2;

/// The name `defaultfs.c` registered the file system under.
fn file_system_name(fs_type: c_int) -> &'static [u8] {
    match fs_type {
        FSTYPE_STDIO => b"stdio",
        FSTYPE_ZIP => b"zip",
        other => unreachable!("a mount has the unknown file system {other}"),
    }
}

/// One line per mount, in the order given (highest priority first):
/// `???:<directory> on <mount point> type <file system> (ro|rw)`.
pub fn format_mounts(mounts: &[MountListing]) -> Vec<u8> {
    let mut text = Vec::new();
    for mount in mounts {
        text.extend_from_slice(b"???:");
        text.extend_from_slice(&mount.source_dir);
        text.extend_from_slice(b" on ");
        text.extend_from_slice(&mount.mount_point);
        text.extend_from_slice(b" type ");
        text.extend_from_slice(file_system_name(mount.fs_type));
        text.extend_from_slice(if mount.read_only {
            b" (ro)\n"
        } else {
            b" (rw)\n"
        });
    }
    text
}

/// `[]` and the virtual path without its leading slash, no newline: the
/// `[root]rest` form with the empty root every debug handle has.
pub fn format_dir_handle(virtual_path: &[u8]) -> Vec<u8> {
    let rest = virtual_path.strip_prefix(b"/").unwrap_or(virtual_path);
    let mut text = Vec::with_capacity(rest.len() + 2);
    text.extend_from_slice(b"[]");
    text.extend_from_slice(rest);
    text
}
