//! Video components loaded through the supplied virtual filesystem.
use super::{decoder::DukVideoDecoder, VideoError};
use crate::host::Files;
use std::{ffi::CStr, path::Path};

pub(super) fn read(files: &dyn Files, filename: &str) -> Option<Vec<u8>> {
    let data = files
        .read(Path::new(filename))
        .or_else(|_| files.read(Path::new(&format!("/{filename}"))))
        .ok()?;
    (!data.is_empty()).then_some(data)
}

pub(super) fn open(files: &dyn Files, filename: &CStr) -> Result<DukVideoDecoder, i32> {
    let basename = filename.to_str().map_err(|_| -1)?.trim_end_matches(".duk");
    let duk = read(files, &format!("{basename}.duk")).ok_or(-1)?;
    let frm = read(files, &format!("{basename}.frm")).ok_or(-1)?;
    let hdr = read(files, &format!("{basename}.hdr")).ok_or(-1)?;
    let tbl = read(files, &format!("{basename}.tbl")).ok_or(-1)?;
    DukVideoDecoder::open_from_data(&hdr, &tbl, &frm, &duk).map_err(|error| match error {
        VideoError::BadFile(_) => -2,
        VideoError::Eof => -5,
        VideoError::OutOfBuffer => -4,
        VideoError::NotInitialized | VideoError::IoError(_) => -1,
        VideoError::BadArg(_) => -3,
    })
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
        assert_eq!(open(&fixture, c"missing.duk").err(), Some(-1));
        assert_eq!(
            *fixture.reads.borrow(),
            vec![PathBuf::from("missing.duk"), PathBuf::from("/missing.duk")]
        );
        fixture.reads.borrow_mut().clear();
        fixture.files.insert("empty.duk".into(), vec![]);
        fixture.files.insert("/empty.duk".into(), vec![1]);
        assert_eq!(open(&fixture, c"empty.duk").err(), Some(-1));
        assert_eq!(*fixture.reads.borrow(), vec![PathBuf::from("empty.duk")]);
        fixture.reads.borrow_mut().clear();
        assert_eq!(open(&fixture, c"\xff").err(), Some(-1));
        assert!(fixture.reads.borrow().is_empty());
    }

    #[test]
    fn shipped_components_decode_in_original_order_and_bad_header_keeps_its_error() {
        let mut fixture = Fixture::default();
        let source = Path::new(env!("UQM_RUST_ROOT")).join("../sc2/content/addons/3dovideo/logo");
        for extension in ["duk", "frm", "hdr", "tbl"] {
            let filename = format!("logo.{extension}");
            fixture.files.insert(
                filename.clone().into(),
                std::fs::read(source.join(filename)).unwrap(),
            );
        }
        let mut decoder = open(&fixture, c"logo.duk").unwrap();
        assert_eq!(decoder.frame_count(), 231);
        assert!(!decoder.decode_frame(0).unwrap().data.is_empty());
        assert_eq!(
            *fixture.reads.borrow(),
            ["logo.duk", "logo.frm", "logo.hdr", "logo.tbl"].map(PathBuf::from)
        );
        fixture.files.insert("logo.hdr".into(), vec![0]);
        assert_eq!(open(&fixture, c"logo.duk").err(), Some(-2));
    }
}
