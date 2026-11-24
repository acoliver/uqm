use super::*;
use std::sync::Weak;

struct ObservedDisk {
    disk: Disk,
    table: Mutex<Weak<Mutex<Arc<MountTable>>>>,
    denied: PathBuf,
    finish_failure: PathBuf,
    reads: Mutex<Vec<PathBuf>>,
}

impl ObservedDisk {
    fn new(denied: PathBuf, finish_failure: PathBuf) -> Arc<Self> {
        Arc::new(Self {
            disk: crate::host::platform().disk(),
            table: Mutex::new(Weak::new()),
            denied,
            finish_failure,
            reads: Mutex::new(Vec::new()),
        })
    }

    fn assert_unlocked(&self) {
        let table = self.table.lock().upgrade().unwrap();
        assert!(
            table.try_lock().is_some(),
            "mount table locked during host IO"
        );
    }
}

impl Files for ObservedDisk {
    fn open_read(&self, path: &Path) -> io::Result<FileStream> {
        self.assert_unlocked();
        self.reads.lock().push(path.to_path_buf());
        if path == self.denied {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "fixture denied read",
            ));
        }
        self.disk.open_read(path)
    }
    fn open_write(&self, path: &Path) -> io::Result<FileStream> {
        self.assert_unlocked();
        let stream = self.disk.open_write(path)?;
        Ok(if path == self.finish_failure {
            FileStream::new(FinishFailure(stream))
        } else {
            stream
        })
    }
    fn remove(&self, path: &Path) -> io::Result<()> {
        self.assert_unlocked();
        self.disk.remove(path)
    }
    fn list(&self, path: &Path) -> io::Result<Vec<String>> {
        self.assert_unlocked();
        self.disk.list(path)
    }
    fn stat(&self, path: &Path) -> io::Result<FileStat> {
        self.assert_unlocked();
        self.disk.stat(path)
    }
}

struct FinishFailure(FileStream);
impl File for FinishFailure {
    fn len(&self) -> io::Result<u64> {
        self.0.len()
    }
}
impl io::Read for FinishFailure {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        self.0.read(bytes)
    }
}
impl io::Seek for FinishFailure {
    fn seek(&mut self, position: io::SeekFrom) -> io::Result<u64> {
        self.0.seek(position)
    }
}
impl Write for FinishFailure {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.write(bytes)
    }
    fn flush(&mut self) -> io::Result<()> {
        Err(io::Error::other("fixture finish failure"))
    }
}

fn scratch() -> tempfile::TempDir {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../tmp/u01-owners");
    std::fs::create_dir_all(&root).unwrap();
    tempfile::tempdir_in(root.canonicalize().unwrap()).unwrap()
}

#[test]
fn non_missing_open_errors_keep_their_cause_and_do_not_try_lower_mounts() {
    let scratch = scratch();
    let upper = scratch.path().join("upper");
    let lower = scratch.path().join("lower");
    std::fs::create_dir(&upper).unwrap();
    std::fs::create_dir(&lower).unwrap();
    std::fs::write(lower.join("data"), b"lower").unwrap();
    let disk = ObservedDisk::new(upper.join("data"), scratch.path().join("unused"));
    let vfs = Vfs::with_disk(disk.clone());
    *disk.table.lock() = Arc::downgrade(&vfs.table);
    vfs.mount_dir("/", &lower, Placement::Bottom, true);
    vfs.mount_dir("/", &upper, Placement::Top, true);
    let files = vfs.open_dir(Path::new("/")).snapshot();
    let error = files.open_read(Path::new("data")).err().unwrap();
    assert_eq!(error.kind(), io::ErrorKind::PermissionDenied);
    assert_eq!(error.to_string(), "fixture denied read");
    assert_eq!(*disk.reads.lock(), [upper.join("data")]);
    assert!(files.stat(Path::new("data")).unwrap().is_file);
    assert_eq!(files.list(Path::new("")).unwrap(), ["data"]);
}

#[test]
fn finish_failure_cleanup_stays_with_the_retained_write_owner() {
    let scratch = scratch();
    let a = scratch.path().join("a");
    let b = scratch.path().join("b");
    std::fs::create_dir(&a).unwrap();
    std::fs::create_dir(&b).unwrap();
    std::fs::write(b.join("slot"), b"untouched").unwrap();
    let disk = ObservedDisk::new(scratch.path().join("unused"), a.join("slot"));
    let vfs = Vfs::with_disk(disk.clone());
    *disk.table.lock() = Arc::downgrade(&vfs.table);
    vfs.mount_dir("/", &a, Placement::Top, false);
    let files = vfs.open_dir(Path::new("/"));
    vfs.unmount_all();
    vfs.mount_dir("/", &b, Placement::Top, false);
    let error = files
        .write_or_remove(Path::new("slot"), b"partial")
        .unwrap_err();
    assert_eq!(error.to_string(), "fixture finish failure");
    assert!(!a.join("slot").exists());
    assert_eq!(std::fs::read(b.join("slot")).unwrap(), b"untouched");
}

#[test]
fn host_io_and_zip_decompression_do_not_hold_the_mount_table_lock() {
    let scratch = scratch();
    std::fs::write(scratch.path().join("data"), b"host").unwrap();
    let mut archive =
        zip::ZipWriter::new(std::fs::File::create(scratch.path().join("data.zip")).unwrap());
    archive
        .start_file("packed", zip::write::SimpleFileOptions::default())
        .unwrap();
    archive.write_all(b"zip").unwrap();
    archive.finish().unwrap();
    let disk = ObservedDisk::new(scratch.path().join("unused"), scratch.path().join("unused"));
    let vfs = Vfs::with_disk(disk.clone());
    *disk.table.lock() = Arc::downgrade(&vfs.table);
    vfs.mount_dir("/", scratch.path(), Placement::Top, false);
    let source = vfs.open_host_dir(scratch.path());
    vfs.mount_zip("/", &source, Path::new("data.zip"), Placement::Top, true)
        .unwrap();
    let files = vfs.open_dir(Path::new("/"));
    assert_eq!(files.read(Path::new("packed")).unwrap(), b"zip");
    assert_eq!(files.read(Path::new("data")).unwrap(), b"host");
    assert_eq!(files.stat(Path::new("data")).unwrap().len, 4);
    assert!(files
        .list(Path::new(""))
        .unwrap()
        .contains(&"data".to_owned()));
    source.write_or_remove(Path::new("new"), b"write").unwrap();
    source.remove(Path::new("new")).unwrap();
}
