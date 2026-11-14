//! The mount resolver used by both native resource loading and replay.
use super::{
    map_virtual_to_host_confined, normalize_virtual_path_full, resolve_mount_for_path,
    resolve_path, MountInfo, UIO_FSTYPE_ZIP,
};
use crate::host::{File, FileStat, FileStream, Files};
use std::io::{Read, Seek, SeekFrom, Write};
use std::{
    fs, io,
    path::{Path, PathBuf},
};

#[derive(Default)]
pub struct Vfs {
    pub(super) mounts: Vec<MountInfo>,
}

impl File for FileData {
    fn len(&self) -> io::Result<u64> {
        self.len()
    }
}

impl Vfs {
    pub(super) fn open_at(
        &self,
        host: &Path,
        base: &Path,
        path: &Path,
        write: bool,
    ) -> io::Result<FileStream> {
        let virtual_path = normalize_virtual_path_full(base, path);
        if write {
            let target = self.write_path(host, &virtual_path, path)?;
            let file = fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(target)?;
            return Ok(FileStream::new(FileData::File(file)));
        }
        let mut covered = false;
        for mount in self.mounts.iter().filter(|mount| mount.active_in_registry) {
            let Ok(suffix) = virtual_path.strip_prefix(&mount.mount_point) else {
                continue;
            };
            covered = true;
            let opened = if mount.fs_type == UIO_FSTYPE_ZIP {
                mount
                    .zip_index
                    .as_ref()
                    .expect("ZIP mount without index")
                    .open_entry(&suffix.to_string_lossy())
                    .map(FileData::ZipEntry)
            } else {
                fs::File::open(map_virtual_to_host_confined(&mount.mounted_root, suffix))
                    .map(FileData::File)
            };
            match opened {
                Ok(file) => return Ok(FileStream::new(file)),
                Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
                Err(error) => return Err(error),
            }
        }
        if covered {
            return Err(io::Error::from(io::ErrorKind::NotFound));
        }
        fs::File::open(resolve_path(host, path))
            .map(FileData::File)
            .map(FileStream::new)
    }

    fn write_path(&self, host: &Path, virtual_path: &Path, path: &Path) -> io::Result<PathBuf> {
        match resolve_mount_for_path(&self.mounts, virtual_path) {
            Some(resolved)
                if resolved.mount.read_only || resolved.mount.fs_type == UIO_FSTYPE_ZIP =>
            {
                Err(io::Error::from(io::ErrorKind::PermissionDenied))
            }
            Some(resolved) => Ok(resolved.host_path),
            None => Ok(resolve_path(host, path)),
        }
    }

    pub(super) fn remove_at(&self, host: &Path, base: &Path, path: &Path) -> io::Result<()> {
        let virtual_path = normalize_virtual_path_full(base, path);
        fs::remove_file(self.write_path(host, &virtual_path, path)?)
    }

    pub(super) fn stat_at(&self, host: &Path, base: &Path, path: &Path) -> io::Result<FileStat> {
        let virtual_path = normalize_virtual_path_full(base, path);
        let mut covered = false;
        for mount in self.mounts.iter().filter(|mount| mount.active_in_registry) {
            let Ok(suffix) = virtual_path.strip_prefix(&mount.mount_point) else {
                continue;
            };
            covered = true;
            if mount.fs_type == UIO_FSTYPE_ZIP {
                let zip = mount.zip_index.as_ref().expect("ZIP mount without index");
                let name = suffix.to_string_lossy();
                if zip.is_directory(&name) {
                    return Ok(FileStat {
                        len: 0,
                        is_dir: true,
                        is_file: false,
                    });
                }
                if let Some(entry) = zip.get_entry(&name) {
                    return Ok(FileStat {
                        len: entry.uncompressed_size,
                        is_dir: false,
                        is_file: true,
                    });
                }
            } else if let Ok(meta) =
                fs::metadata(map_virtual_to_host_confined(&mount.mounted_root, suffix))
            {
                return Ok(metadata(meta));
            }
        }
        if covered {
            return Err(io::Error::from(io::ErrorKind::NotFound));
        }
        fs::metadata(resolve_path(host, path)).map(metadata)
    }

    pub(super) fn list_at(&self, host: &Path, base: &Path, path: &Path) -> io::Result<Vec<String>> {
        let virtual_path = normalize_virtual_path_full(base, path);
        let mut covered = false;
        let mut seen = std::collections::BTreeSet::new();
        let mut names = Vec::new();
        for mount in self.mounts.iter().filter(|mount| mount.active_in_registry) {
            let Ok(suffix) = virtual_path.strip_prefix(&mount.mount_point) else {
                continue;
            };
            covered = true;
            let entries = if mount.fs_type == UIO_FSTYPE_ZIP {
                mount
                    .zip_index
                    .as_ref()
                    .expect("ZIP mount without index")
                    .list_directory(&suffix.to_string_lossy())
            } else {
                let target = map_virtual_to_host_confined(&mount.mounted_root, suffix);
                match directory_names(&target) {
                    Ok(names) => names,
                    Err(_) => continue,
                }
            };
            for name in entries {
                if seen.insert(name.clone()) {
                    names.push(name);
                }
            }
        }
        if covered {
            Ok(names)
        } else {
            directory_names(&resolve_path(host, path))
        }
    }
}

fn metadata(meta: fs::Metadata) -> FileStat {
    FileStat {
        len: meta.len(),
        is_dir: meta.is_dir(),
        is_file: meta.is_file(),
    }
}
fn directory_names(path: &Path) -> io::Result<Vec<String>> {
    Ok(fs::read_dir(path)?
        .filter_map(|entry| entry.ok()?.file_name().to_str().map(str::to_owned))
        .collect())
}

impl Files for Vfs {
    fn open_read(&self, path: &Path) -> io::Result<FileStream> {
        self.open_at(Path::new("/"), Path::new("/"), path, false)
    }
    fn open_write(&self, path: &Path) -> io::Result<FileStream> {
        self.open_at(Path::new("/"), Path::new("/"), path, true)
    }
    fn remove(&self, path: &Path) -> io::Result<()> {
        self.remove_at(Path::new("/"), Path::new("/"), path)
    }
    fn list(&self, path: &Path) -> io::Result<Vec<String>> {
        self.list_at(Path::new("/"), Path::new("/"), path)
    }
    fn stat(&self, path: &Path) -> io::Result<FileStat> {
        self.stat_at(Path::new("/"), Path::new("/"), path)
    }
}

pub enum FileData {
    File(std::fs::File),
    ZipEntry(crate::io::zip_reader::ZipEntryReader),
}

impl Read for FileData {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            FileData::File(f) => f.read(buf),
            FileData::ZipEntry(z) => z.read(buf),
        }
    }
}

impl Write for FileData {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            FileData::File(f) => f.write(buf),
            FileData::ZipEntry(_) => Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "Cannot write to ZIP entry",
            )),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            FileData::File(f) => f.flush(),
            FileData::ZipEntry(_) => Ok(()),
        }
    }
}

impl Seek for FileData {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        match self {
            FileData::File(f) => f.seek(pos),
            FileData::ZipEntry(z) => z.seek(pos),
        }
    }
}

impl FileData {
    /// Get file size
    pub fn len(&self) -> std::io::Result<u64> {
        match self {
            FileData::File(f) => f.metadata().map(|m| m.len()),
            FileData::ZipEntry(z) => Ok(z.size()),
        }
    }
}
