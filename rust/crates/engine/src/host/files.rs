//! Virtual paths and owned streams shared by native and replay file clients.
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::Path;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FileStat {
    pub len: u64,
    pub is_file: bool,
    pub is_dir: bool,
}

pub trait File: Read + Write + Seek {
    fn len(&self) -> io::Result<u64>;
    fn is_empty(&self) -> io::Result<bool> {
        self.len().map(|len| len == 0)
    }
}

pub trait Files {
    fn open_read(&self, path: &Path) -> io::Result<FileStream>;
    fn open_write(&self, path: &Path) -> io::Result<FileStream>;
    fn remove(&self, path: &Path) -> io::Result<()>;
    fn list(&self, path: &Path) -> io::Result<Vec<String>>;
    fn stat(&self, path: &Path) -> io::Result<FileStat>;

    fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        let mut bytes = Vec::new();
        self.open_read(path)?.read_to_end(&mut bytes)?;
        Ok(bytes)
    }
}

pub struct FileStream {
    inner: Box<dyn File>,
    position: u64,
}

impl FileStream {
    pub fn new(inner: impl File + 'static) -> Self {
        Self {
            inner: Box::new(inner),
            position: 0,
        }
    }

    pub fn len(&self) -> io::Result<u64> {
        self.inner.len()
    }
    pub fn is_empty(&self) -> io::Result<bool> {
        self.inner.is_empty()
    }
    pub fn position(&self) -> u64 {
        self.position
    }

    pub fn finish(mut self) -> io::Result<()> {
        self.flush()
    }

    /// The loaders split lines at capacity minus one, retaining the newline.
    /// Embedded NUL bytes end the returned text, but not consumption of the line.
    pub fn line(&mut self, capacity: usize) -> io::Result<Option<Vec<u8>>> {
        let mut bytes = Vec::new();
        for _ in 1..capacity {
            let mut byte = [0];
            if self.read(&mut byte)? == 0 {
                break;
            }
            bytes.push(byte[0]);
            if byte[0] == b'\n' {
                break;
            }
        }
        if bytes.is_empty() {
            return Ok(None);
        }
        if let Some(nul) = bytes.iter().position(|&b| b == 0) {
            bytes.truncate(nul);
        }
        Ok(Some(bytes))
    }
}

impl Read for FileStream {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        let count = self.inner.read(bytes)?;
        self.position += count as u64;
        Ok(count)
    }
}
impl Write for FileStream {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let count = self.inner.write(bytes)?;
        self.position = self.inner.stream_position()?;
        Ok(count)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}
impl Seek for FileStream {
    fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
        self.position = self.inner.seek(position)?;
        Ok(self.position)
    }
}
