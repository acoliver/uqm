//! Video components loaded through the supplied virtual filesystem.
use super::decoder::DukVideoDecoder;
use crate::host::Files;
use crate::resource::FileError;
use std::path::Path;

pub(super) fn read(files: &dyn Files, filename: &str) -> Result<Vec<u8>, FileError> {
    let data = files
        .read(Path::new(filename))
        .or_else(|_| files.read(Path::new(&format!("/{filename}"))))
        .map_err(|error| FileError::at(filename, error))?;
    if data.is_empty() {
        return Err(FileError::invalid(filename, "empty video component"));
    }
    Ok(data)
}

/// Open the four components of the clip named `filename` (an optional `.duk` suffix is ignored).
pub(super) fn open(files: &dyn Files, filename: &str) -> Result<DukVideoDecoder, FileError> {
    let basename = filename.trim_end_matches(".duk");
    let duk = read(files, &format!("{basename}.duk"))?;
    let frm = read(files, &format!("{basename}.frm"))?;
    let hdr = read(files, &format!("{basename}.hdr"))?;
    let tbl = read(files, &format!("{basename}.tbl"))?;
    DukVideoDecoder::open_from_data(&hdr, &tbl, &frm, &duk)
        .map_err(|error| FileError::at(filename, error))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::host::{FileStat, FileStream};
    use std::{cell::RefCell, collections::BTreeMap, io, path::PathBuf};

    #[derive(Default)]
    struct Fixture {
        files: BTreeMap<PathBuf, Vec<u8>>,
        reads: RefCell<Vec<PathBuf>>,
    }
    impl Files for Fixture {
        fn open_read(&self, _: &Path) -> io::Result<FileStream> {
            panic!("unexpected stream")
        }
        fn open_write(&self, _: &Path) -> io::Result<FileStream> {
            panic!("unexpected write")
        }
        fn remove(&self, _: &Path) -> io::Result<()> {
            panic!("unexpected remove")
        }
        fn list(&self, _: &Path) -> io::Result<Vec<String>> {
            panic!("unexpected listing")
        }
        fn stat(&self, _: &Path) -> io::Result<FileStat> {
            panic!("unexpected stat")
        }
        fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
            self.reads.borrow_mut().push(path.to_owned());
            self.files
                .get(path)
                .cloned()
                .ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))
        }
    }

    #[test]
    fn missing_empty_and_non_utf8_paths_preserve_read_and_error_order() {
        let mut fixture = Fixture::default();
        assert!(open(&fixture, "missing.duk").is_err());
        assert_eq!(
            *fixture.reads.borrow(),
            vec![PathBuf::from("missing.duk"), PathBuf::from("/missing.duk")]
        );
        fixture.reads.borrow_mut().clear();
        fixture.files.insert("empty.duk".into(), vec![]);
        fixture.files.insert("/empty.duk".into(), vec![1]);
        assert!(open(&fixture, "empty.duk").is_err());
        assert_eq!(*fixture.reads.borrow(), vec![PathBuf::from("empty.duk")]);
        fixture.reads.borrow_mut().clear();
        assert!(open(&fixture, "\u{fffd}").is_err());
    }

    #[test]
    fn shipped_components_decode_in_original_order_and_bad_header_keeps_its_error() {
        let mut fixture = Fixture::default();
        let source = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
            .join("../sc2/content/addons/3dovideo/logo");
        for extension in ["duk", "frm", "hdr", "tbl"] {
            let filename = format!("logo.{extension}");
            fixture.files.insert(
                filename.clone().into(),
                std::fs::read(source.join(filename)).unwrap(),
            );
        }
        let mut decoder = open(&fixture, "logo.duk").unwrap();
        assert_eq!(decoder.frame_count(), 231);
        assert!(!decoder.decode_frame(0).unwrap().data.is_empty());
        assert_eq!(
            *fixture.reads.borrow(),
            ["logo.duk", "logo.frm", "logo.hdr", "logo.tbl"].map(PathBuf::from)
        );
        fixture.files.insert("logo.hdr".into(), vec![0]);
        assert!(matches!(
            open(&fixture, "logo.duk"),
            Err(FileError { source, .. }) if source.downcast_ref::<super::super::VideoError>().is_some_and(|e| matches!(e, super::super::VideoError::BadFile(_)))
        ));
    }
}
