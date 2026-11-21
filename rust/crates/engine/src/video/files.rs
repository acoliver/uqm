//! Video components loaded through the supplied virtual filesystem.
use super::{decoder::DukVideoDecoder, VideoError};
use crate::host::Files;
use std::path::Path;

pub(super) fn read(files: &dyn Files, filename: &str) -> Option<Vec<u8>> {
    let data = files
        .read(Path::new(filename))
        .or_else(|_| files.read(Path::new(&format!("/{filename}"))))
        .ok()?;
    (!data.is_empty()).then_some(data)
}

/// Open the four components of the clip named `filename` (an optional `.duk` suffix is ignored).
pub(super) fn open(files: &dyn Files, filename: &str) -> Result<DukVideoDecoder, VideoError> {
    let basename = filename.trim_end_matches(".duk");
    let component = |extension: &str| {
        read(files, &format!("{basename}.{extension}"))
            .ok_or_else(|| VideoError::IoError(format!("cannot read {basename}.{extension}")))
    };
    let duk = component("duk")?;
    let frm = component("frm")?;
    let hdr = component("hdr")?;
    let tbl = component("tbl")?;
    DukVideoDecoder::open_from_data(&hdr, &tbl, &frm, &duk)
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
        assert!(matches!(
            open(&fixture, "missing.duk"),
            Err(VideoError::IoError(_))
        ));
        assert_eq!(
            *fixture.reads.borrow(),
            vec![PathBuf::from("missing.duk"), PathBuf::from("/missing.duk")]
        );
        fixture.reads.borrow_mut().clear();
        fixture.files.insert("empty.duk".into(), vec![]);
        fixture.files.insert("/empty.duk".into(), vec![1]);
        assert!(matches!(
            open(&fixture, "empty.duk"),
            Err(VideoError::IoError(_))
        ));
        assert_eq!(*fixture.reads.borrow(), vec![PathBuf::from("empty.duk")]);
        fixture.reads.borrow_mut().clear();
        assert!(matches!(
            open(&fixture, "\u{fffd}"),
            Err(VideoError::IoError(_))
        ));
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
            Err(VideoError::BadFile(_))
        ));
    }
}
