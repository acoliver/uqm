// Directory Operations

use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DirError {
    NotFound,
    PermissionDenied,
    AlreadyExists,
    InvalidPath,
    NotADirectory,
    IoError,
}

impl std::fmt::Display for DirError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DirError::NotFound => write!(f, "Directory not found"),
            DirError::PermissionDenied => write!(f, "Permission denied"),
            DirError::AlreadyExists => write!(f, "Directory already exists"),
            DirError::InvalidPath => write!(f, "Invalid path"),
            DirError::NotADirectory => write!(f, "Not a directory"),
            DirError::IoError => write!(f, "I/O error"),
        }
    }
}

impl std::error::Error for DirError {}

impl From<io::Error> for DirError {
    fn from(err: io::Error) -> Self {
        match err.kind() {
            io::ErrorKind::NotFound => DirError::NotFound,
            io::ErrorKind::PermissionDenied => DirError::PermissionDenied,
            io::ErrorKind::AlreadyExists => DirError::AlreadyExists,
            io::ErrorKind::InvalidInput => DirError::InvalidPath,
            _ => DirError::IoError,
        }
    }
}

#[derive(Debug)]
pub struct DirHandle {
    path: PathBuf,
    read_dir: fs::ReadDir,
}

impl DirHandle {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, DirError> {
        let path = path.as_ref().to_path_buf();

        if !path.exists() {
            return Err(DirError::NotFound);
        }

        if path.is_file() {
            return Err(DirError::NotADirectory);
        }

        let read_dir = fs::read_dir(&path)?;

        Ok(DirHandle { path, read_dir })
    }

    pub fn next_entry(&mut self) -> Option<io::Result<DirEntry>> {
        self.read_dir.next().map(|result| {
            result.map(|entry| DirEntry {
                path: self.path.clone(),
                entry,
            })
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

pub struct DirEntry {
    #[allow(dead_code)]
    path: PathBuf,
    entry: fs::DirEntry,
}

impl DirEntry {
    pub fn file_name(&self) -> String {
        self.entry.file_name().to_string_lossy().to_string()
    }

    pub fn full_path(&self) -> PathBuf {
        self.entry.path()
    }

    pub fn is_file(&self) -> bool {
        self.entry
            .file_type()
            .map(|ft| ft.is_file())
            .unwrap_or(false)
    }

    pub fn is_dir(&self) -> bool {
        self.entry
            .file_type()
            .map(|ft| ft.is_dir())
            .unwrap_or(false)
    }

    pub fn metadata(&self) -> io::Result<fs::Metadata> {
        self.entry.metadata()
    }
}

pub fn create_directory<P: AsRef<Path>>(path: P) -> Result<(), DirError> {
    let path = path.as_ref();
    fs::create_dir(path)?;
    Ok(())
}

pub fn create_directory_all<P: AsRef<Path>>(path: P) -> Result<(), DirError> {
    let path = path.as_ref();
    fs::create_dir_all(path)?;
    Ok(())
}

pub fn remove_directory<P: AsRef<Path>>(path: P) -> Result<(), DirError> {
    let path = path.as_ref();
    if !path.exists() {
        return Err(DirError::NotFound);
    }
    if !path.is_dir() {
        return Err(DirError::NotADirectory);
    }
    fs::remove_dir(path)?;
    Ok(())
}

pub fn remove_directory_all<P: AsRef<Path>>(path: P) -> Result<(), DirError> {
    let path = path.as_ref();
    if !path.exists() {
        return Err(DirError::NotFound);
    }
    fs::remove_dir_all(path)?;
    Ok(())
}

pub fn directory_exists<P: AsRef<Path>>(path: P) -> bool {
    let path = path.as_ref();
    path.exists() && path.is_dir()
}

pub fn list_directory<P: AsRef<Path>>(path: P) -> Result<Vec<String>, DirError> {
    let path = path.as_ref();
    let mut entries = Vec::new();

    for entry in fs::read_dir(path)? {
        let entry = entry?;
        entries.push(entry.file_name().to_string_lossy().to_string());
    }

    Ok(entries)
}

pub fn list_files<P: AsRef<Path>>(path: P) -> Result<Vec<String>, DirError> {
    let path = path.as_ref();
    let mut entries = Vec::new();
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        if entry.path().is_file() {
            entries.push(entry.file_name().to_string_lossy().to_string());
        }
    }

    Ok(entries)
}

pub fn list_subdirs<P: AsRef<Path>>(path: P) -> Result<Vec<String>, DirError> {
    let path = path.as_ref();
    let mut entries = Vec::new();
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        if let Ok(ft) = entry.file_type() {
            if ft.is_dir() {
                entries.push(entry.file_name().to_string_lossy().to_string());
            }
        }
    }

    Ok(entries)
}

pub fn current_dir() -> Result<PathBuf, DirError> {
    env::current_dir().map_err(|e| e.into())
}

pub fn set_current_dir<P: AsRef<Path>>(path: P) -> Result<(), DirError> {
    env::set_current_dir(path).map_err(|e| e.into())
}

pub fn is_empty<P: AsRef<Path>>(path: P) -> Result<bool, DirError> {
    let path = path.as_ref();
    if !path.exists() {
        return Ok(true);
    }

    if path.is_dir() {
        let entries = fs::read_dir(path)?;
        Ok(entries.count() == 0)
    } else {
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static TEST_COUNTER: AtomicUsize = AtomicUsize::new(0);

    /// Get a unique test directory for the current test
    fn get_test_dir() -> PathBuf {
        let counter = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        let mut dir = env::temp_dir();
        dir.push(format!(
            "uqm_dirs_test_{:08}_{}",
            std::process::id(),
            counter
        ));
        dir
    }

    /// Setup: create the test directory
    fn setup() -> PathBuf {
        let test_dir = get_test_dir();
        fs::create_dir_all(&test_dir)
            .unwrap_or_else(|e| panic!("Failed to create test directory {:?}: {:?}", test_dir, e));
        test_dir
    }

    /// Cleanup: remove the test directory
    fn cleanup(test_dir: &Path) {
        if test_dir.exists() {
            let _ = fs::remove_dir_all(test_dir);
        }
    }

    #[test]
    fn test_directory_exists() {
        let test_dir = setup();
        let existing_dir = test_dir.join("existing");
        let non_existing = test_dir.join("non_existing");

        assert!(!directory_exists(&existing_dir));
        fs::create_dir(&existing_dir).unwrap();
        assert!(directory_exists(&existing_dir));
        assert!(!directory_exists(&non_existing));
        cleanup(&test_dir);
    }

    #[test]
    fn test_create_directory() {
        let test_dir = setup();
        let new_dir = test_dir.join("new_dir");

        assert!(!directory_exists(&new_dir));
        let result = create_directory(&new_dir);
        assert!(result.is_ok());
        assert!(directory_exists(&new_dir));
        cleanup(&test_dir);
    }

    #[test]
    fn test_create_directory_all() {
        let test_dir = setup();
        let nested_dir = test_dir.join("parent").join("child").join("grandchild");

        assert!(!directory_exists(&nested_dir));
        let result = create_directory_all(&nested_dir);
        assert!(result.is_ok());
        assert!(directory_exists(&nested_dir));
        cleanup(&test_dir);
    }

    #[test]
    fn test_remove_directory() {
        let test_dir = setup();
        let to_remove = test_dir.join("to_remove");

        fs::create_dir(&to_remove).unwrap();
        assert!(directory_exists(&to_remove));
        let result = remove_directory(&to_remove);
        assert!(result.is_ok());
        assert!(!directory_exists(&to_remove));
        cleanup(&test_dir);
    }

    #[test]
    fn test_remove_directory_not_found() {
        let test_dir = setup();
        let non_existing = test_dir.join("non_existing");
        let result = remove_directory(&non_existing);
        assert_eq!(result, Err(DirError::NotFound));
        cleanup(&test_dir);
    }

    #[test]
    fn test_remove_directory_all() {
        let test_dir = setup();
        let to_remove = test_dir.join("to_remove");
        let file1 = to_remove.join("file1.txt");
        let subdir = to_remove.join("subdir");
        let file2 = subdir.join("file2.txt");

        fs::create_dir_all(&subdir).unwrap();
        fs::write(&file1, "content").unwrap();
        fs::write(&file2, "content").unwrap();

        assert!(directory_exists(&to_remove));
        let result = remove_directory_all(&to_remove);
        assert!(result.is_ok());
        assert!(!directory_exists(&to_remove));
        cleanup(&test_dir);
    }

    #[test]
    fn test_list_directory() {
        let test_dir = setup();
        let list_dir = test_dir.join("list_test");
        fs::create_dir(&list_dir).unwrap();

        fs::write(list_dir.join("file1.txt"), "content1").unwrap();
        fs::write(list_dir.join("file2.txt"), "content2").unwrap();
        fs::create_dir(list_dir.join("subdir")).unwrap();
        let entries = list_directory(&list_dir).unwrap();
        assert_eq!(entries.len(), 3);
        assert!(entries.contains(&String::from("file1.txt")));
        assert!(entries.contains(&String::from("file2.txt")));
        assert!(entries.contains(&String::from("subdir")));
        cleanup(&test_dir);
    }

    #[test]
    fn test_list_files() {
        let test_dir = setup();
        let list_dir = test_dir.join("list_files_test");
        fs::create_dir(&list_dir).unwrap();

        fs::write(list_dir.join("file1.txt"), "content1").unwrap();
        fs::write(list_dir.join("file2.txt"), "content2").unwrap();
        fs::create_dir(list_dir.join("subdir")).unwrap();
        let files = list_files(&list_dir).unwrap();
        assert_eq!(files.len(), 2);
        assert!(files.contains(&String::from("file1.txt")));
        assert!(files.contains(&String::from("file2.txt")));
        cleanup(&test_dir);
    }

    #[test]
    fn test_list_subdirs() {
        let test_dir = setup();
        let list_dir = test_dir.join("list_subdirs_test");
        fs::create_dir(&list_dir).unwrap();

        fs::write(list_dir.join("file.txt"), "content").unwrap();
        fs::create_dir(list_dir.join("subdir1")).unwrap();
        fs::create_dir(list_dir.join("subdir2")).unwrap();
        let subdirs = list_subdirs(&list_dir).unwrap();
        assert_eq!(subdirs.len(), 2);
        assert!(subdirs.contains(&String::from("subdir1")));
        assert!(subdirs.contains(&String::from("subdir2")));
        cleanup(&test_dir);
    }

    #[test]
    fn test_dir_handle() {
        let test_dir = setup();
        let handle_dir = test_dir.join("handle_test");
        fs::create_dir(&handle_dir).unwrap();

        fs::write(handle_dir.join("file1.txt"), "").unwrap();
        fs::write(handle_dir.join("file2.txt"), "").unwrap();

        let mut handle = DirHandle::open(&handle_dir).unwrap();
        let mut entries = Vec::new();

        while let Some(Ok(entry)) = handle.next_entry() {
            entries.push(entry.file_name());
        }

        assert_eq!(entries.len(), 2);
        cleanup(&test_dir);
    }

    #[test]
    fn test_dir_handle_open_nonexistent() {
        let test_dir = setup();
        let non_existing = test_dir.join("non_existing");

        let result = DirHandle::open(&non_existing);
        assert!(matches!(result, Err(DirError::NotFound)));
        cleanup(&test_dir);
    }

    #[test]
    fn test_dir_handle_open_file() {
        let test_dir = setup();
        let file_path = test_dir.join("test.txt");
        fs::write(&file_path, "content").unwrap();

        let result = DirHandle::open(&file_path);
        assert!(matches!(result, Err(DirError::NotADirectory)));
        cleanup(&test_dir);
    }

    #[test]
    fn test_is_empty() {
        let test_dir = setup();
        let empty_dir = test_dir.join("empty");
        let non_empty_dir = test_dir.join("non_empty");
        let non_existing = test_dir.join("non_existing");

        assert!(!directory_exists(&empty_dir));
        assert!(is_empty(&empty_dir).unwrap());

        fs::create_dir(&empty_dir).unwrap();
        fs::create_dir(&non_empty_dir).unwrap();
        fs::write(non_empty_dir.join("file.txt"), "content").unwrap();

        assert!(is_empty(&empty_dir).unwrap());
        assert!(!is_empty(&non_empty_dir).unwrap());
        assert!(is_empty(&non_existing).unwrap());
        cleanup(&test_dir);
    }

    #[test]
    fn test_dir_entry() {
        let test_dir = setup();
        let entry_dir = test_dir.join("entry_test");
        fs::create_dir(&entry_dir).unwrap();

        fs::write(entry_dir.join("test_file.txt"), "content").unwrap();
        fs::create_dir(entry_dir.join("test_dir")).unwrap();
        let mut handle = DirHandle::open(&entry_dir).unwrap();

        while let Some(Ok(entry)) = handle.next_entry() {
            if entry.file_name() == "test_file.txt" {
                assert!(entry.is_file());
                assert!(!entry.is_dir());
            } else if entry.file_name() == "test_dir" {
                assert!(!entry.is_file());
                assert!(entry.is_dir());
            }
        }
        cleanup(&test_dir);
    }

    #[test]
    fn test_current_dir() {
        let result = current_dir();
        assert!(result.is_ok());
        assert!(result.unwrap().is_absolute());
    }

    #[test]
    fn test_set_current_dir() {
        let original = current_dir().unwrap();
        let test_dir = setup();

        let result = set_current_dir(&test_dir);
        assert!(result.is_ok());
        let current = current_dir().unwrap();
        let test_canonical = test_dir.canonicalize().unwrap_or_else(|_| test_dir.clone());
        let current_canonical = current.canonicalize().unwrap_or_else(|_| current.clone());
        assert_eq!(current_canonical, test_canonical);

        let _ = set_current_dir(&original);
        cleanup(&test_dir);
    }

    #[test]
    fn test_dir_error_display() {
        let err = DirError::NotFound;
        let display = format!("{}", err);
        assert!(display.contains("not found"));
    }
}
