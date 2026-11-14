use super::*;
use crate::host::Files;
use std::io::{Read, Seek, SeekFrom, Write};

#[test]
fn files_port_roundtrips_and_removes_bytes() {
    let root = concat!(env!("UQM_RUST_ROOT"), "/../tmp");
    std::fs::create_dir_all(root).unwrap();
    let temp = tempfile::tempdir_in(root).unwrap();
    let files = Vfs::default();
    let path = temp.path().join("slot.sav");
    let mut output = files.open_write(&path).unwrap();
    output.write_all(b"\0save\xff\n").unwrap();
    output.flush().unwrap();
    drop(output);
    assert_eq!(files.stat(&path).unwrap().len, 7);
    let mut input = files.open_read(&path).unwrap();
    assert_eq!(input.seek(SeekFrom::End(-2)).unwrap(), 5);
    let mut tail = Vec::new();
    input.read_to_end(&mut tail).unwrap();
    assert_eq!(tail, b"\xff\n");
    assert_eq!(files.list(temp.path()).unwrap(), ["slot.sav"]);
    files.remove(&path).unwrap();
    assert_eq!(
        files.open_read(&path).err().unwrap().kind(),
        std::io::ErrorKind::NotFound
    );
}
