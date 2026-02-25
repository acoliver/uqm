// State File Management
// Handles in-memory state file operations for starinfo, randgrpinfo, and defgrpinfo

use std::sync::Mutex;

pub const STARINFO_FILE: usize = 0;
pub const RANDGRPINFO_FILE: usize = 1;
pub const DEFGRPINFO_FILE: usize = 2;

/// Size hints for state files (from C headers)
const STAR_BUFSIZE: usize = 256 * 1024; // 256 KB
const RAND_BUFSIZE: usize = 64 * 1024; // 64 KB
const DEF_BUFSIZE: usize = 64 * 1024; // 64 KB

const NUM_STATE_FILES: usize = 3;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StateFileError {
    InvalidFile,
    FileTooLarge,
    ReadOutOfBounds,
    WriteFailed,
}

impl std::fmt::Display for StateFileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StateFileError::InvalidFile => write!(f, "Invalid state file index"),
            StateFileError::FileTooLarge => write!(f, "File size exceeds maximum"),
            StateFileError::ReadOutOfBounds => write!(f, "Read operation out of bounds"),
            StateFileError::WriteFailed => write!(f, "Write operation failed"),
        }
    }
}

impl std::error::Error for StateFileError {}

/// In-memory state file handler
#[derive(Debug, PartialEq)]
pub struct StateFile {
    name: &'static str,
    size_hint: usize,
    open_count: i32,
    data: Vec<u8>,
    used: usize,
    ptr: usize,
}

impl StateFile {
    /// Create a new state file with the specified name and size hint
    fn new(name: &'static str, size_hint: usize) -> Self {
        StateFile {
            name,
            size_hint,
            open_count: 0,
            data: Vec::with_capacity(size_hint),
            used: 0,
            ptr: 0,
        }
    }

    /// Get the current file length (logical high-water mark of bytes written)
    pub fn length(&self) -> usize {
        self.used
    }

    /// Get current read/write position
    pub fn position(&self) -> usize {
        self.ptr
    }

    /// Read data from the file
    pub fn read(&mut self, buf: &mut [u8]) -> Result<usize, StateFileError> {
        if self.ptr >= self.data.len() {
            return Ok(0); // EOF
        }

        let available = self.data.len() - self.ptr;
        let bytes_to_read = buf.len().min(available);

        if bytes_to_read > 0 {
            buf[..bytes_to_read].copy_from_slice(&self.data[self.ptr..self.ptr + bytes_to_read]);
            self.ptr += bytes_to_read;
        }

        Ok(bytes_to_read)
    }

    /// Write data to the file
    pub fn write(&mut self, buf: &[u8]) -> Result<(), StateFileError> {
        let required_end = self.ptr + buf.len();

        // Grow physical buffer if needed
        if required_end > self.data.len() {
            let new_size = required_end.max(self.data.len() * 3 / 2);
            self.data.resize(new_size, 0);
            if new_size > self.size_hint {
                self.size_hint = new_size;
            }
        }

        self.data[self.ptr..self.ptr + buf.len()].copy_from_slice(buf);
        self.ptr += buf.len();

        // Update logical high-water mark
        if self.ptr > self.used {
            self.used = self.ptr;
        }

        Ok(())
    }

    /// Seek to a position in the file.
    /// Allows seeking past end — no upper-bound clamping.
    /// Negative results are clamped to 0.
    pub fn seek(&mut self, offset: i64, whence: SeekWhence) -> Result<(), StateFileError> {
        let new_pos = match whence {
            SeekWhence::Set => offset,
            SeekWhence::Current => self.ptr as i64 + offset,
            SeekWhence::End => self.used as i64 + offset,
        };

        if new_pos < 0 {
            self.ptr = 0;
        } else {
            self.ptr = new_pos as usize;
        }
        Ok(())
    }

    /// Open the file for reading or writing.
    /// Pre-allocates physical buffer on first open.
    /// Write mode resets the logical size but keeps the physical allocation.
    fn open(&mut self, mode: FileMode) -> Result<(), StateFileError> {
        self.open_count += 1;

        if self.open_count > 1 {
            // Warning in debug builds would go here
        }

        // Pre-allocate on first open (when no physical buffer exists)
        if self.data.is_empty() {
            self.data.resize(self.size_hint, 0);
            self.used = 0;
        }

        match mode {
            FileMode::Read | FileMode::ReadWrite => {
                // Preserve used (logical size)
            }
            FileMode::Write => {
                // Reset logical size but keep physical allocation
                self.used = 0;
            }
        }

        self.ptr = 0;
        Ok(())
    }

    /// Close the file.
    /// Decrements open_count unconditionally to match C semantics (can go negative).
    fn close(&mut self) {
        self.ptr = 0;
        self.open_count -= 1;
    }

    /// Delete/reset the file data
    fn delete(&mut self) {
        if self.open_count != 0 {
            // Warning in debug builds would go here
        }
        self.data.clear();
        self.data.shrink_to_fit();
        self.used = 0;
        self.ptr = 0;
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FileMode {
    Read,
    Write,
    ReadWrite,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SeekWhence {
    Set,
    Current,
    End,
}

/// State file manager - handles all state files
pub struct StateFileManager {
    files: [StateFile; NUM_STATE_FILES],
}

impl StateFileManager {
    /// Create a new state file manager
    pub fn new() -> Self {
        StateFileManager {
            files: [
                StateFile::new("STARINFO", STAR_BUFSIZE),
                StateFile::new("RANDGRPINFO", RAND_BUFSIZE),
                StateFile::new("DEFGRPINFO", DEF_BUFSIZE),
            ],
        }
    }

    /// Open a state file
    pub fn open(
        &mut self,
        file_index: usize,
        mode: FileMode,
    ) -> Result<&mut StateFile, StateFileError> {
        if file_index >= NUM_STATE_FILES {
            return Err(StateFileError::InvalidFile);
        }
        self.files[file_index].open(mode)?;
        Ok(&mut self.files[file_index])
    }

    /// Close a state file
    pub fn close(&mut self, file_index: usize) -> Result<(), StateFileError> {
        if file_index >= NUM_STATE_FILES {
            return Err(StateFileError::InvalidFile);
        }
        self.files[file_index].close();
        Ok(())
    }

    /// Delete a state file
    pub fn delete(&mut self, file_index: usize) -> Result<(), StateFileError> {
        if file_index >= NUM_STATE_FILES {
            return Err(StateFileError::InvalidFile);
        }
        self.files[file_index].delete();
        Ok(())
    }

    /// Get direct access to a state file
    pub fn get_file(&self, file_index: usize) -> Option<&StateFile> {
        if file_index >= NUM_STATE_FILES {
            None
        } else {
            Some(&self.files[file_index])
        }
    }

    /// Get mutable access to a state file
    pub fn get_file_mut(&mut self, file_index: usize) -> Option<&mut StateFile> {
        if file_index >= NUM_STATE_FILES {
            None
        } else {
            Some(&mut self.files[file_index])
        }
    }
}

impl Default for StateFileManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Global state file manager (thread-safe)
static GLOBAL_STATE_FILES: Mutex<Option<StateFileManager>> = Mutex::new(None);

/// Initialize the global state file manager
pub fn init_global_state_files() {
    let mut global = GLOBAL_STATE_FILES.lock().unwrap();
    if global.is_none() {
        *global = Some(StateFileManager::new());
    }
}

/// Get access to the global state file manager
pub fn get_global_state_manager<'a>() -> Option<&'a Mutex<Option<StateFileManager>>> {
    Some(&GLOBAL_STATE_FILES)
}

/// Read a 32-bit little-endian value from a state file
pub fn read_u32_le(file: &mut StateFile) -> Result<u32, StateFileError> {
    let mut buf = [0u8; 4];
    file.read(&mut buf)?;
    Ok(u32::from_le_bytes(buf))
}

/// Write a 32-bit little-endian value to a state file
pub fn write_u32_le(file: &mut StateFile, value: u32) -> Result<(), StateFileError> {
    file.write(&value.to_le_bytes())
}

/// Read an array of 32-bit little-endian values from a state file
pub fn read_u32_array(file: &mut StateFile, values: &mut [u32]) -> Result<(), StateFileError> {
    for v in values.iter_mut() {
        *v = read_u32_le(file)?;
    }
    Ok(())
}

/// Write an array of 32-bit little-endian values to a state file
pub fn write_u32_array(file: &mut StateFile, values: &[u32]) -> Result<(), StateFileError> {
    for &v in values {
        write_u32_le(file, v)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_file_new() {
        let file = StateFile::new("TEST", 1024);
        assert_eq!(file.name, "TEST");
        assert_eq!(file.size_hint, 1024);
        assert_eq!(file.data.len(), 0);
        assert_eq!(file.ptr, 0);
    }

    #[test]
    fn test_state_file_write_read() {
        let mut file = StateFile::new("TEST", 1024);
        file.open(FileMode::Write).unwrap();

        let data = b"Hello, World!";
        file.write(data).unwrap();

        assert_eq!(file.length(), data.len());

        file.seek(0, SeekWhence::Set).unwrap();
        let mut buf = vec![0u8; data.len()];
        file.read(&mut buf).unwrap();

        assert_eq!(&buf, data);
    }

    #[test]
    fn test_state_file_append() {
        let mut file = StateFile::new("TEST", 1024);
        file.open(FileMode::Write).unwrap();

        file.write(b"Hello").unwrap();
        file.write(b", ").unwrap();
        file.write(b"World").unwrap();

        assert_eq!(file.length(), 12);

        file.seek(0, SeekWhence::Set).unwrap();
        let mut buf = vec![0u8; 12];
        file.read(&mut buf).unwrap();

        assert_eq!(&buf, b"Hello, World");
    }

    #[test]
    fn test_state_file_seek_current() {
        let mut file = StateFile::new("TEST", 1024);
        file.write(b"HelloWorld").unwrap();
        // ptr is at 10 after write, seek +3 => 13 (no upper clamp)
        file.seek(3, SeekWhence::Current).unwrap();
        assert_eq!(file.position(), 13);
        // 13 - 5 = 8
        file.seek(-5, SeekWhence::Current).unwrap();
        assert_eq!(file.position(), 8);
        // Read from position 8: "ld" (2 bytes available out of 10)
        let mut buf = vec![0u8; 3];
        let bytes_read = file.read(&mut buf).unwrap();
        assert_eq!(bytes_read, 2);
        assert_eq!(&buf[..2], b"ld");
    }

    #[test]
    fn test_state_file_seek_end() {
        let mut file = StateFile::new("TEST", 1024);
        file.write(b"Hello").unwrap();

        file.seek(0, SeekWhence::End).unwrap();
        assert_eq!(file.position(), 5);

        file.write(b"World").unwrap();
        assert_eq!(file.length(), 10);
    }

    #[test]
    fn test_state_file_seek_negative() {
        let mut file = StateFile::new("TEST", 1024);
        file.write(b"Hello").unwrap();

        file.seek(-10, SeekWhence::Set).unwrap();
        assert_eq!(file.position(), 0); // Should clamp to 0

        file.seek(-100, SeekWhence::End).unwrap();
        assert_eq!(file.position(), 0); // Should clamp to 0
    }

    #[test]
    fn test_state_file_write_mode_clears() {
        let mut file = StateFile::new("TEST", 1024);
        file.open(FileMode::Write).unwrap();
        file.write(b"Hello").unwrap();
        file.close();

        file.open(FileMode::Write).unwrap();
        assert_eq!(file.length(), 0);
        assert_eq!(file.position(), 0);
    }

    #[test]
    fn test_state_file_read_preserves() {
        let mut file = StateFile::new("TEST", 1024);
        file.open(FileMode::Write).unwrap();
        file.write(b"Hello").unwrap();
        file.close();

        file.open(FileMode::Read).unwrap();
        assert_eq!(file.length(), 5);
        assert_eq!(file.position(), 0);
    }

    #[test]
    fn test_state_file_open_count() {
        let mut file = StateFile::new("TEST", 1024);
        assert_eq!(file.open_count, 0);

        file.open(FileMode::Read).unwrap();
        assert_eq!(file.open_count, 1);

        file.open(FileMode::Read).unwrap();
        assert_eq!(file.open_count, 2);

        file.close();
        assert_eq!(file.open_count, 1);

        file.close();
        assert_eq!(file.open_count, 0);
    }

    #[test]
    fn test_state_file_delete() {
        let mut file = StateFile::new("TEST", 1024);
        file.write(b"HelloWorld").unwrap();
        assert_eq!(file.length(), 10);

        file.delete();
        assert_eq!(file.length(), 0);
        assert_eq!(file.position(), 0);
    }

    #[test]
    fn test_read_u32_le() {
        let mut file = StateFile::new("TEST", 1024);
        file.write(&0x12345678u32.to_le_bytes()).unwrap();

        file.seek(0, SeekWhence::Set).unwrap();
        let result = read_u32_le(&mut file).unwrap();

        assert_eq!(result, 0x12345678);
    }

    #[test]
    fn test_write_u32_le() {
        let mut file = StateFile::new("TEST", 1024);
        write_u32_le(&mut file, 0xABCDEF00).unwrap();

        file.seek(0, SeekWhence::Set).unwrap();
        let mut buf = [0u8; 4];
        file.read(&mut buf).unwrap();

        assert_eq!(u32::from_le_bytes(buf), 0xABCDEF00);
    }

    #[test]
    fn test_read_u32_array() {
        let mut file = StateFile::new("TEST", 1024);
        let values = [1u32, 2, 3, 4, 5];
        write_u32_array(&mut file, &values).unwrap();

        file.seek(0, SeekWhence::Set).unwrap();
        let mut result = [0u32; 5];
        read_u32_array(&mut file, &mut result).unwrap();

        assert_eq!(result, values);
    }

    #[test]
    fn test_file_manager_new() {
        let manager = StateFileManager::new();
        assert_eq!(manager.files.len(), NUM_STATE_FILES);
        assert_eq!(manager.files[STARINFO_FILE].name, "STARINFO");
        assert_eq!(manager.files[RANDGRPINFO_FILE].name, "RANDGRPINFO");
        assert_eq!(manager.files[DEFGRPINFO_FILE].name, "DEFGRPINFO");
    }

    #[test]
    fn test_file_manager_open_close() {
        let mut manager = StateFileManager::new();

        let file = manager.open(STARINFO_FILE, FileMode::Write).unwrap();
        file.write(b"Test").unwrap();
        manager.close(STARINFO_FILE).unwrap();

        assert_eq!(manager.get_file(STARINFO_FILE).unwrap().length(), 4);
    }

    #[test]
    fn test_file_manager_open_invalid() {
        let mut manager = StateFileManager::new();
        let result = manager.open(99, FileMode::Read);
        assert_eq!(result, Err(StateFileError::InvalidFile));
    }

    #[test]
    fn test_file_manager_delete() {
        let mut manager = StateFileManager::new();

        {
            let file = manager.open(STARINFO_FILE, FileMode::Write).unwrap();
            file.write(b"Test").unwrap();
        }

        manager.delete(STARINFO_FILE).unwrap();
        assert_eq!(manager.get_file(STARINFO_FILE).unwrap().length(), 0);
    }

    #[test]
    fn test_default() {
        let manager: StateFileManager = Default::default();
        assert_eq!(manager.files.len(), NUM_STATE_FILES);
    }

    #[test]
    fn test_large_write_expansion() {
        let mut file = StateFile::new("TEST", 1024);
        // Write more than size_hint
        let large_data = vec![0u8; 4096];
        file.write(&large_data).unwrap();

        assert_eq!(file.length(), 4096);
        assert!(file.data.capacity() >= 4096);
    }

    #[test]
    fn test_read_past_end() {
        let mut file = StateFile::new("TEST", 1024);
        file.write(b"Hello").unwrap();

        file.seek(10, SeekWhence::Set).unwrap();
        let mut buf = [0u8; 10];
        let bytes_read = file.read(&mut buf).unwrap();

        assert_eq!(bytes_read, 0);
    }

    #[test]
    fn test_partial_read() {
        let mut file = StateFile::new("TEST", 1024);
        file.write(b"Hello").unwrap();

        file.seek(3, SeekWhence::Set).unwrap();
        let mut buf = [0u8; 10];
        let bytes_read = file.read(&mut buf).unwrap();

        assert_eq!(bytes_read, 2);
        assert_eq!(&buf[..2], b"lo");
    }
}
