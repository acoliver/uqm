#[test]
fn content_files_and_sources_withhold_writes_across_children_and_packages() {
    use crate::host::Files;
    use std::io::Write;
    use std::path::Path;

    let mount = crate::io::fixture::TempMount::new();
    mount.write("resource.rmp", b"text = STRTAB:sub/text.txt\n");
    mount.write("sub/text.txt", b"retained");
    let mut zip = zip::ZipWriter::new(std::fs::File::create(mount.path_of("pack.zip")).unwrap());
    zip.start_file("sub/member", zip::write::SimpleFileOptions::default())
        .unwrap();
    zip.write_all(b"packaged").unwrap();
    zip.finish().unwrap();
    let content = mount.content(c"resource.rmp");
    let resolved = content.resolve(c"text", super::AssetKind::Text).unwrap();
    let packaged = content
        .files()
        .with_package("/", Path::new("pack.zip"))
        .unwrap();
    for directory in [
        content.files().clone(),
        resolved.source.directory.clone(),
        content.files().open_relative(Path::new("sub")),
        content.files().open_relative(Path::new("/sub")),
        packaged.clone(),
        packaged.open_relative(Path::new("/sub")),
    ] {
        let error = directory
            .open_write(Path::new("not-created"))
            .err()
            .unwrap();
        assert_eq!(error.kind(), std::io::ErrorKind::PermissionDenied);
        assert_eq!(
            directory.remove(Path::new("text.txt")).unwrap_err().kind(),
            std::io::ErrorKind::PermissionDenied
        );
    }
    mount.vfs().unmount_all();
    let other = crate::io::fixture::TempMount::new();
    other.write("sub/text.txt", b"different run");
    mount
        .vfs()
        .mount_dir("/", &other.path_of(""), crate::io::Placement::Top, false);
    assert_eq!(
        content.files().read(Path::new("sub/text.txt")).unwrap(),
        b"retained"
    );
    assert_eq!(packaged.read(Path::new("sub/member")).unwrap(), b"packaged");
    assert_eq!(
        resolved
            .source
            .directory
            .read(Path::new("sub/text.txt"))
            .unwrap(),
        b"retained"
    );
    assert_eq!(
        std::fs::read(mount.path_of("sub/text.txt")).unwrap(),
        b"retained"
    );
}

use super::*;
use crate::resource::settings::{Bindings, Settings};

fn import(builder: &mut CatalogBuilder, directory: &Dir, name: &CStr, prefix: Option<&CStr>) {
    builder.import_index(
        directory,
        name,
        prefix,
        &mut Settings::default(),
        &mut Bindings::default(),
    );
}

#[test]
fn index_files_define_typed_values_and_file_entries() {
    let mount = crate::io::fixture::TempMount::new();
    mount.write("a.rmp", b"name = STRING:hello\ncount = INT32:-7\nflag = BOOLEAN:TRUE\ntint = COLOR:rgb15(31, 0, 16)\ntext = STRTAB:base/text.txt\nodd = WEIRD:thing\nbroken\n");
    let mut builder = CatalogBuilder::new(mount.dir());
    import(&mut builder, &mount.dir(), c"a.rmp", Some(c"p."));
    assert_eq!(builder.entries["p.name"].entry.descriptor, "hello");
    assert_eq!(builder.entries["p.count"].entry.value, -7i32 as u32);
    assert_eq!(builder.entries["p.flag"].entry.value, 1);
    assert_eq!(builder.entries["p.tint"].entry.value, 0xff00_84ff);
    import(&mut builder, &mount.dir(), c"missing.rmp", None);
    let content = builder.finish();
    assert_eq!(
        content
            .require(c"p.text", AssetKind::Text)
            .unwrap()
            .descriptor,
        "base/text.txt"
    );
    assert_eq!(content.kind("p.odd"), Some("WEIRD"));
    assert!(content.kind("broken").is_none());
    assert!(matches!(
        content.resolve(c"p.text", AssetKind::Binary),
        Err(ResolveError::WrongKind { .. })
    ));
    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(
        || content.require(c"p.text", AssetKind::Binary)
    ))
    .is_err());
    assert!(matches!(
        content.resolve(c"missing", AssetKind::Graphic),
        Err(ResolveError::Missing { .. })
    ));
    assert_eq!(content.0.entries.len(), 6);
}

#[test]
fn puts_replace_entries_and_serialize_only_values() {
    let mut entries = HashMap::new();
    for (key, value) in [
        ("cfg.b", "BOOLEAN:false"),
        ("cfg.a", "INT32:12"),
        ("cfg.c", "COLOR:rgba(1, 2, 3, 4)"),
        ("cfg.d", "STRING:text"),
        ("cfg.e", "STRTAB:file.txt"),
        ("other", "INT32:1"),
        ("cfg.a", "STRING:replaced"),
    ] {
        define(&mut entries, key, value);
    }
    assert_eq!(
        serialize_entries(&entries, "cfg.", true),
        [
            "a = STRING:replaced\n",
            "b = BOOLEAN:false\n",
            "c = COLOR:rgba(0x01, 0x02, 0x03, 0x04)\n",
            "d = STRING:text\n"
        ]
    );
    assert_eq!(
        serialize_entries(&entries, "oth", false),
        ["other = INT32:1\n"]
    );
    let mut settings = Settings::default();
    settings.put_string("cfg.a", "replacement");
    assert!(settings.remove("cfg.a"));
    assert!(!settings.remove("cfg.a"));
    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(
        || settings.put_string("bad\0", "x")
    ))
    .is_err());
}

#[test]
fn independent_catalogs_and_repaired_revisions_retain_their_files_and_sources() {
    let first = crate::io::fixture::TempMount::new();
    let second = crate::io::fixture::TempMount::new();
    first.write("base.rmp", b"same = STRTAB:payload\n");
    first.write("payload", b"first");
    second.write("base.rmp", b"same = STRTAB:payload\n");
    second.write("payload", b"second");
    let a = first.content(c"base.rmp");
    let b = second.content(c"base.rmp");
    assert_ne!(a.revision(), b.revision());
    let resolved = a.resolve(c"same", AssetKind::Text).unwrap();
    assert_eq!(resolved.source.index.as_c_str(), c"base.rmp");
    assert_eq!(
        resolved
            .source
            .directory
            .read(Path::new("payload"))
            .unwrap(),
        b"first"
    );
    assert_eq!(resolved.revision, a.revision());
    let mut opened = resolved
        .files
        .open_read(Path::new(resolved.descriptor))
        .unwrap();
    let mut revised = a.revise();
    assert!(revised.remove("same"));
    assert!(!revised.remove("same"));
    let removed = revised.finish();
    assert!(removed.resolve(c"same", AssetKind::Text).is_err());
    assert!(a.resolve(c"same", AssetKind::Text).is_ok());
    assert_eq!(b.files().read(Path::new("payload")).unwrap(), b"second");
    let mut repaired = removed.revise();
    import(&mut repaired, &first.dir(), c"base.rmp", None);
    assert!(repaired.finish().resolve(c"same", AssetKind::Text).is_ok());
    drop(a);
    let mut bytes = Vec::new();
    opened.read_to_end(&mut bytes).unwrap();
    assert_eq!(bytes, b"first");
}

#[test]
fn sibling_revisions_with_equal_edit_counts_have_distinct_identities() {
    let mount = crate::io::fixture::TempMount::new();
    mount.write("base.rmp", b"same = STRTAB:base\n");
    mount.write("left.rmp", b"same = STRTAB:left\n");
    mount.write("right.rmp", b"same = STRTAB:right\n");
    let base = mount.content(c"base.rmp");
    let mut left = base.revise();
    let mut right = base.revise();
    import(&mut left, &mount.dir(), c"left.rmp", None);
    import(&mut right, &mount.dir(), c"right.rmp", None);
    let left = left.finish();
    let right = right.finish();
    assert_ne!(base.revision(), left.revision());
    assert_ne!(left.revision(), right.revision());
    assert!(Arc::ptr_eq(
        &base.revision().session,
        &left.revision().session
    ));
    assert_eq!(
        left.require(c"same", AssetKind::Text).unwrap().descriptor,
        "left"
    );
    assert_eq!(
        right.require(c"same", AssetKind::Text).unwrap().descriptor,
        "right"
    );
    assert_eq!(
        base.require(c"same", AssetKind::Text).unwrap().descriptor,
        "base"
    );
}
