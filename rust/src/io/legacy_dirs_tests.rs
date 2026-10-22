use super::*;
use std::ffi::{CStr, CString};

fn expand(src: &[u8], limit: usize, flags: i32) -> Result<Vec<u8>, i32> {
    expand_bytes(
        src,
        limit,
        flags,
        |name| match name {
            b"HOME" => Some(b"/legacy-home".to_vec()),
            b"F3_VALUE" => Some(b"value/../path".to_vec()),
            _ => None,
        },
        || Some(b"/legacy-home".to_vec()),
        || Ok(b"/".to_vec()),
    )
}

#[test]
fn path_expansion_matches_original_c_cases() {
    // Captured with cwd=/, HOME=/legacy-home and F3_VALUE=value/../path.
    let cases: Vec<(String, i32, usize, String, bool)> =
        serde_json::from_str(include_str!("../../tests/fixtures/dirs-expansion.json")).unwrap();
    for (src, flags, limit, expected, failed) in cases {
        let result = expand(src.as_bytes(), limit, flags);
        if failed {
            let error = match expected.as_str() {
                "EINVAL" => libc::EINVAL,
                "ENAMETOOLONG" => libc::ENAMETOOLONG,
                "ERANGE" => libc::ERANGE,
                other => panic!("unexpected C errno {other}"),
            };
            assert_eq!(result, Err(error), "{src:?}, flags={flags}, limit={limit}");
        } else {
            assert_eq!(
                result,
                Ok(expected.into_bytes()),
                "{src:?}, flags={flags}, limit={limit}"
            );
        }
    }
}

#[test]
fn expansion_boundaries_home_failures_and_nonrecursive_environment() {
    assert_eq!(expand(b"abc", 4, 0), Ok(b"abc".to_vec()));
    assert_eq!(expand(b"abc", 3, 0), Err(libc::ENAMETOOLONG));
    assert_eq!(expand(b"", 0, 0), Err(libc::ENAMETOOLONG));
    assert_eq!(
        expand_bytes(b"~/save", 128, EP_HOME, |_| None, || None, || Ok(vec![])),
        Err(libc::ENOENT)
    );
    assert_eq!(
        expand_bytes(
            b"$A/end",
            128,
            EP_ENVVARS,
            |_| Some(b"$B".to_vec()),
            || None,
            || Ok(vec![])
        ),
        Ok(b"$B/end".to_vec())
    );
    assert_eq!(
        expand_bytes(
            b"x",
            4,
            EP_ABSOLUTE,
            |_| None,
            || None,
            || Ok(b"/long".to_vec())
        ),
        Err(libc::ERANGE)
    );
    assert_eq!(
        expand_bytes(
            b"~/x",
            128,
            EP_HOME | EP_ABSOLUTE,
            |_| None,
            || Some(b"relative-home".to_vec()),
            || Ok(b"/cwd".to_vec())
        ),
        Ok(b"/cwd/relative-home/x".to_vec())
    );
}

#[test]
#[serial_test::serial]
fn ffi_expansion_and_home_pointer() {
    unsafe {
        let home = getHomeDir();
        assert!(!home.is_null());
        let home_bytes = CStr::from_ptr(home).to_bytes();
        let mut result = vec![0i8; home_bytes.len() + 16];
        assert_eq!(
            expandPath(
                result.as_mut_ptr(),
                result.len(),
                c"~/save".as_ptr(),
                EP_HOME
            ),
            0
        );
        assert_eq!(
            CStr::from_ptr(result.as_ptr()).to_bytes(),
            [home_bytes, b"/save"].concat()
        );
        assert_eq!(expandPath(result.as_mut_ptr(), 1, c"xx".as_ptr(), 0), -1);
        assert_eq!(
            std::io::Error::last_os_error().raw_os_error(),
            Some(libc::ENAMETOOLONG)
        );
        assert_eq!(
            expandPath(
                result.as_mut_ptr(),
                result.len(),
                c"${broken".as_ptr(),
                EP_ENVVARS
            ),
            -1
        );
        assert_eq!(
            std::io::Error::last_os_error().raw_os_error(),
            Some(libc::EINVAL)
        );
    }
}

#[test]
fn mkdirhier_preserves_existing_entries_and_reports_intermediate_files() {
    std::fs::create_dir_all(concat!(env!("CARGO_MANIFEST_DIR"), "/../tmp")).unwrap();
    let temp = tempfile::Builder::new()
        .prefix("dirs-")
        .tempdir_in(concat!(env!("CARGO_MANIFEST_DIR"), "/../tmp"))
        .unwrap();
    let path = CString::new(format!("{}/a//b/c/", temp.path().display())).unwrap();
    unsafe {
        assert_eq!(mkdirhier(path.as_ptr()), 0);
        assert!(temp.path().join("a/b/c").is_dir());
        assert_eq!(mkdirhier(path.as_ptr()), 0);
        assert_eq!(mkdirhier(c"".as_ptr()), 0);
        assert_eq!(mkdirhier(c"/".as_ptr()), 0);
        std::fs::write(temp.path().join("file"), b"x").unwrap();
        let file = CString::new(format!("{}/file", temp.path().display())).unwrap();
        assert_eq!(mkdirhier(file.as_ptr()), 0);
        let nested = CString::new(format!("{}/file/child", temp.path().display())).unwrap();
        assert_eq!(mkdirhier(nested.as_ptr()), -1);
        assert_eq!(
            std::io::Error::last_os_error().raw_os_error(),
            Some(libc::ENOTDIR)
        );
        assert_eq!(createDirectory(file.as_ptr(), 0o777), -1);
        assert_eq!(
            std::io::Error::last_os_error().raw_os_error(),
            Some(libc::EEXIST)
        );
    }
}
