//! Namespace collisions must keep the last accepted definition, including kind changes.
use super::{index, settings::Settings};
use crate::host::Files;
use serial_test::serial;
use std::path::Path;

#[test]
#[serial]
fn rmp_kind_replacement_does_not_resurrect_settings_or_binding_values() {
    index::uninit();
    let mount = crate::io::fixture::TempMount::new();
    mount.write("first.rmp", b"config.alwaysgl = BOOLEAN:true\nkeys.1.name = STRING:First\nmenu.select.1 = STRING:key Return\nunknown = SHIP:0\n");
    mount.write("second.rmp", b"config.alwaysgl = GFXRES:art.ani\nkeys.1.name = STRTAB:names.txt\nmenu.select.1 = WEIRD:replacement\n");
    index::load_index(&mount.dir(), c"first.rmp", None);
    let settings = Settings;
    assert_eq!(settings.boolean("config.alwaysgl"), Some(true));
    assert_eq!(settings.string("keys.1.name").as_deref(), Some("First"));
    index::load_index(&mount.dir(), c"second.rmp", None);
    assert_eq!(settings.boolean("config.alwaysgl"), None);
    assert_eq!(settings.string("keys.1.name"), None);
    assert!(settings.has_key("keys.1.name"));
    assert!(settings.has_key("menu.select.1"));
    assert_eq!(index::kind("unknown").as_deref(), Some("SHIP"));
    assert!(index::serialize("config.", true).is_empty());
    assert!(index::serialize("keys.", true).is_empty());
    assert!(index::serialize("menu.", true).is_empty());
    settings.put_string("keys.1.name", "Second");
    assert_eq!(
        index::serialize("keys.", true),
        ["1.name = STRING:Second\n"]
    );
    assert!(settings.remove("keys.1.name"));
    assert!(!settings.has_key("keys.1.name"));
    index::uninit();
}

#[test]
#[serial]
fn sorted_settings_bytes_keep_signed_boolean_color_and_prefix_rules() {
    index::uninit();
    let mount = crate::io::fixture::TempMount::new();
    mount.write("values.rmp", b"z = STRING:tail\na = INT32:-7\nb = BOOLEAN:TRUE\nc = COLOR:rgba(1, 2, 3, 4)\nnot-saved = MUSICRES:song.ogg\ninvalid = no-colon\n");
    index::load_index(&mount.dir(), c"values.rmp", Some(c"config."));
    Settings.save(&mount.dir(), c"uqm.cfg", "config.").unwrap();
    assert_eq!(mount.dir().read(Path::new("uqm.cfg")).unwrap(), b"a = INT32:-7\nb = BOOLEAN:true\nc = COLOR:rgba(0x01, 0x02, 0x03, 0x04)\nz = STRING:tail\n");
    assert_eq!(index::serialize("keys.", true), Vec::<String>::new());
    index::uninit();
}
