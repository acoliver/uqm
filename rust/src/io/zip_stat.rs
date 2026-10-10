//! What a ZIP central directory records about an entry, as one `stat`.
//!
//! The archive index reads each entry's central-directory record once, when
//! the archive is mounted. `uio_stat` and `uio_access` answer from that
//! record, and an opened handle copies it, so a path and a handle never
//! disagree about an entry.
//!
//! The mode follows the original `zip_makeFileMode`, with its undefined cases
//! given a decision instead of the original's accidents:
//!
//! - Creator host 0 (FAT), 6 (HPFS), 10 (NTFS) and 14 (VFAT) record DOS
//!   attributes: 0x10 marks a directory, 0x01 read-only. A file is
//!   `S_IFREG|0444`, a directory `S_IFDIR|0555`, and both gain `0222` unless
//!   read-only. Attributes that are entirely 0 record nothing, which is
//!   not read-only (the original's "file came from standard input": 0666).
//! - Creator host 3 (Unix) records `st_mode` in the high 16 bits of the
//!   attributes. A Unix entry that records none there (Java, Go and Python
//!   writers leave them 0) carries at most the DOS-compatible low byte, and
//!   the DOS rule reads that. The original skipped such entries because its
//!   mode was 0; they are ordinary files.
//! - A Unix mode without file-type bits is permission bits only; the kind
//!   comes from the entry name (a trailing slash names a directory).
//! - Any other creator host defines no attribute layout, and an entry whose
//!   recorded kind is not a regular file or directory (a symbolic link, a
//!   device, a FIFO), or contradicts its name, has no `stat` to give. Those
//!   are reported as unsupported, never given an invented mode.
//!
//! Directories the archive only implies, and the archive root, record
//! nothing: they are read-only and searchable, with no time.

use std::fmt;
use std::io::{self, Read, Seek, SeekFrom};

use zip::DateTime;

const TYPE_MASK: u32 = 0o170000;
const REGULAR_FILE: u32 = 0o100000;
const DIRECTORY: u32 = 0o040000;

/// The mode of a directory the archive implies and of the archive root.
pub const IMPLIED_DIRECTORY_MODE: u32 = DIRECTORY | 0o555;

const HOST_FAT: u8 = 0;
const HOST_UNIX: u8 = 3;
const HOST_HPFS: u8 = 6;
const HOST_NTFS: u8 = 10;
const HOST_VFAT: u8 = 14;

const DOS_DIRECTORY: u32 = 0x10;
const DOS_READ_ONLY: u32 = 0x01;

const CENTRAL_SIGNATURE: [u8; 4] = [0x50, 0x4b, 0x01, 0x02];
/// Bytes of a central-directory header up to and including the external
/// attributes.
const CENTRAL_PREFIX_LEN: usize = 42;
const CREATOR_HOST_AT: usize = 5;
const EXTERNAL_ATTRIBUTES_AT: usize = 38;

/// What `stat` reports for an archive entry: the length, the mode and the
/// modification time in seconds since the epoch. The archive holds no access
/// or change time and no owner, so those read as 0.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ZipEntryStat {
    pub size: u64,
    /// File type and permission bits, at most 16 bits wide.
    pub mode: u32,
    /// 0 when the entry carries no valid modification time (the `zip` crate
    /// reports a DOS date that is not a calendar date as absent).
    pub mtime: i64,
}

/// An archive object as `stat` sees it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZipObject {
    /// An entry the archive records.
    Recorded(ZipEntryStat),
    /// The archive root or a directory that only its entries' paths imply.
    ImpliedDirectory,
}

impl ZipObject {
    /// Whether `X_OK` holds: every directory can be searched, and a file can
    /// be executed when its recorded mode has an execute bit.
    pub fn is_executable(&self) -> bool {
        match self {
            ZipObject::Recorded(entry) => {
                entry.mode & TYPE_MASK == DIRECTORY || entry.mode & 0o111 != 0
            }
            ZipObject::ImpliedDirectory => true,
        }
    }
}

/// Why an entry has no `stat`: what its record says is not something the
/// file system view can represent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnsupportedEntry {
    /// The creator host defines no attribute layout.
    CreatorHost(u8),
    /// The Unix mode names a kind other than a regular file or directory.
    NotRegularOrDirectory { mode: u32 },
    /// The recorded kind contradicts the entry name.
    KindContradictsName { directory_name: bool },
}

impl fmt::Display for UnsupportedEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UnsupportedEntry::CreatorHost(host) => {
                write!(
                    f,
                    "ZIP entry made by host {host} has no known attribute layout"
                )
            }
            UnsupportedEntry::NotRegularOrDirectory { mode } => {
                write!(
                    f,
                    "ZIP entry mode {mode:o} is neither a regular file nor a directory"
                )
            }
            UnsupportedEntry::KindContradictsName { directory_name } => write!(
                f,
                "ZIP entry recorded as a {} is named as a {}",
                if *directory_name { "file" } else { "directory" },
                if *directory_name { "directory" } else { "file" },
            ),
        }
    }
}

impl std::error::Error for UnsupportedEntry {}

impl From<UnsupportedEntry> for io::Error {
    fn from(unsupported: UnsupportedEntry) -> Self {
        io::Error::new(io::ErrorKind::Unsupported, unsupported)
    }
}

/// The part of an entry's central-directory header the `zip` crate does not
/// expose: the host that made it and the raw external attributes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CentralRecord {
    host: u8,
    external_attributes: u32,
}

impl CentralRecord {
    pub fn new(host: u8, external_attributes: u32) -> Self {
        CentralRecord {
            host,
            external_attributes,
        }
    }

    /// Reads the header at `offset` of `archive`. The `zip` crate has parsed
    /// the same header, so anything else there is a damaged archive.
    pub fn read(archive: &mut (impl Read + Seek), offset: u64) -> io::Result<Self> {
        let mut header = [0u8; CENTRAL_PREFIX_LEN];
        archive.seek(SeekFrom::Start(offset))?;
        archive.read_exact(&mut header)?;
        if header[..4] != CENTRAL_SIGNATURE {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("no ZIP central directory header at offset {offset}"),
            ));
        }
        let at = EXTERNAL_ATTRIBUTES_AT;
        Ok(CentralRecord::new(
            header[CREATOR_HOST_AT],
            u32::from_le_bytes([header[at], header[at + 1], header[at + 2], header[at + 3]]),
        ))
    }

    /// The `stat` of the entry named `directory_name`-wise, `size` long and
    /// last modified at `modified`.
    pub fn stat(
        self,
        directory_name: bool,
        size: u64,
        modified: Option<DateTime>,
    ) -> Result<ZipEntryStat, UnsupportedEntry> {
        let mode = match self.host {
            HOST_UNIX if self.external_attributes >> 16 != 0 => {
                unix_mode(self.external_attributes >> 16, directory_name)?
            }
            HOST_UNIX | HOST_FAT | HOST_HPFS | HOST_NTFS | HOST_VFAT => {
                dos_mode(self.external_attributes, directory_name)?
            }
            other => return Err(UnsupportedEntry::CreatorHost(other)),
        };
        Ok(ZipEntryStat {
            size,
            mode,
            mtime: modified.map_or(0, unix_seconds),
        })
    }
}

fn unix_mode(mode: u32, directory_name: bool) -> Result<u32, UnsupportedEntry> {
    let kind = if directory_name {
        DIRECTORY
    } else {
        REGULAR_FILE
    };
    match mode & TYPE_MASK {
        0 => Ok(mode | kind),
        recorded if recorded == kind => Ok(mode),
        REGULAR_FILE | DIRECTORY => Err(UnsupportedEntry::KindContradictsName { directory_name }),
        _ => Err(UnsupportedEntry::NotRegularOrDirectory { mode }),
    }
}

/// A directory name is a directory whatever the attributes say; a directory
/// attribute on a file name contradicts it.
fn dos_mode(attributes: u32, directory_name: bool) -> Result<u32, UnsupportedEntry> {
    if attributes & DOS_DIRECTORY != 0 && !directory_name {
        return Err(UnsupportedEntry::KindContradictsName { directory_name });
    }
    let kind_and_reads = if directory_name {
        DIRECTORY | 0o555
    } else {
        REGULAR_FILE | 0o444
    };
    let writes = if attributes & DOS_READ_ONLY != 0 {
        0
    } else {
        0o222
    };
    Ok(kind_and_reads | writes)
}

/// The DOS date and time of an entry as seconds since the epoch, read as UTC
/// the way the original `dosToUnixTime` does (the format carries no zone).
fn unix_seconds(time: DateTime) -> i64 {
    // Days from 1970-01-01 to the civil date (proleptic Gregorian).
    let (year, month, day) = (
        i64::from(time.year()),
        i64::from(time.month()),
        i64::from(time.day()),
    );
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let year_of_era = year - era * 400;
    let day_of_year = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    let days = era * 146_097 + day_of_era - 719_468;
    days * 86_400
        + i64::from(time.hour()) * 3_600
        + i64::from(time.minute()) * 60
        + i64::from(time.second())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stat_of(host: u8, attributes: u32, directory_name: bool) -> Result<u32, UnsupportedEntry> {
        CentralRecord::new(host, attributes)
            .stat(directory_name, 0, None)
            .map(|stat| stat.mode)
    }

    #[test]
    fn dos_attributes_give_the_legacy_modes_for_every_dos_host() {
        for host in [HOST_FAT, HOST_HPFS, HOST_NTFS, HOST_VFAT, HOST_UNIX] {
            assert_eq!(stat_of(host, 0x20, false), Ok(0o100666), "host {host}");
            assert_eq!(stat_of(host, 0x21, false), Ok(0o100444), "host {host}");
            assert_eq!(stat_of(host, 0, false), Ok(0o100666), "host {host}");
            assert_eq!(stat_of(host, 0x10, true), Ok(0o040777), "host {host}");
            assert_eq!(stat_of(host, 0x11, true), Ok(0o040555), "host {host}");
            assert_eq!(stat_of(host, 0, true), Ok(0o040777), "host {host}");
        }
    }

    #[test]
    fn a_directory_name_stays_a_directory_without_the_dos_bit() {
        assert_eq!(stat_of(HOST_FAT, 0x20, true), Ok(0o040777));
        assert_eq!(
            stat_of(HOST_FAT, 0x10, false),
            Err(UnsupportedEntry::KindContradictsName {
                directory_name: false
            })
        );
    }

    #[test]
    fn unix_modes_are_taken_as_recorded() {
        assert_eq!(stat_of(HOST_UNIX, 0o100640 << 16, false), Ok(0o100640));
        assert_eq!(stat_of(HOST_UNIX, 0o040750 << 16, true), Ok(0o040750));
        assert_eq!(stat_of(HOST_UNIX, 0o4755 << 16, false), Ok(0o104755));
        assert_eq!(stat_of(HOST_UNIX, 0o640 << 16, false), Ok(0o100640));
        assert_eq!(stat_of(HOST_UNIX, 0o750 << 16, true), Ok(0o040750));
    }

    #[test]
    fn unix_kinds_other_than_file_and_directory_are_unsupported() {
        for kind in [0o120000u32, 0o010000, 0o020000, 0o060000, 0o140000] {
            let mode = kind | 0o644;
            assert_eq!(
                stat_of(HOST_UNIX, mode << 16, false),
                Err(UnsupportedEntry::NotRegularOrDirectory { mode })
            );
        }
        assert_eq!(
            stat_of(HOST_UNIX, 0o100644 << 16, true),
            Err(UnsupportedEntry::KindContradictsName {
                directory_name: true
            })
        );
        assert_eq!(
            stat_of(HOST_UNIX, 0o040755 << 16, false),
            Err(UnsupportedEntry::KindContradictsName {
                directory_name: false
            })
        );
    }

    #[test]
    fn a_host_without_an_attribute_layout_is_named_in_the_error() {
        for host in [1u8, 2, 4, 5, 7, 19, 255] {
            assert_eq!(
                stat_of(host, 0x20, false),
                Err(UnsupportedEntry::CreatorHost(host))
            );
        }
    }

    #[test]
    fn a_missing_dos_time_reads_as_no_time() {
        let record = CentralRecord::new(HOST_UNIX, 0o100644 << 16);
        let valid = DateTime::from_date_and_time(2020, 5, 17, 13, 45, 30).unwrap();
        assert_eq!(
            record.stat(false, 1, Some(valid)).unwrap().mtime,
            1_589_723_130
        );
        assert_eq!(record.stat(false, 1, None).unwrap().mtime, 0);
    }
}
