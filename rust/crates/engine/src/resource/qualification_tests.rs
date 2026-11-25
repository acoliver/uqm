//! Namespace collisions must keep the last accepted definition, including kind changes.
use super::{
    settings::{Bindings, Settings},
    CatalogBuilder,
};
use crate::host::Files;
use serial_test::serial;
use std::path::Path;

#[test]
#[serial]
fn rmp_kind_replacement_does_not_resurrect_settings_or_binding_values() {
    let mut settings = Settings::default();
    let mut bindings = Bindings::default();
    let mount = crate::io::fixture::TempMount::new();
    let mut catalog = CatalogBuilder::new(mount.dir());
    mount.write("first.rmp", b"config.alwaysgl = BOOLEAN:true\nkeys.1.name = STRING:First\nmenu.select.1 = STRING:key Return\nunknown = SHIP:0\n");
    mount.write("second.rmp", b"config.alwaysgl = GFXRES:art.ani\nkeys.1.name = STRTAB:names.txt\nmenu.select.1 = WEIRD:replacement\n");
    catalog.import_index(
        &mount.dir(),
        c"first.rmp",
        None,
        &mut settings,
        &mut bindings,
    );
    assert_eq!(settings.boolean("config.alwaysgl"), Some(true));
    assert_eq!(bindings.string("keys.1.name").as_deref(), Some("First"));
    catalog.import_index(
        &mount.dir(),
        c"second.rmp",
        None,
        &mut settings,
        &mut bindings,
    );
    assert_eq!(settings.boolean("config.alwaysgl"), None);
    assert_eq!(bindings.string("keys.1.name"), None);
    assert!(bindings.has_key("keys.1.name"));
    assert!(bindings.has_key("menu.select.1"));
    assert_eq!(catalog.finish().kind("unknown"), Some("SHIP"));
    settings
        .save(&mount.dir(), c"config.cfg", "config.")
        .unwrap();
    assert!(mount
        .dir()
        .read(Path::new("config.cfg"))
        .unwrap()
        .is_empty());
    bindings.save(&mount.dir(), c"keys.cfg", "keys.").unwrap();
    assert!(mount.dir().read(Path::new("keys.cfg")).unwrap().is_empty());
    bindings.save(&mount.dir(), c"menu.cfg", "menu.").unwrap();
    assert!(mount.dir().read(Path::new("menu.cfg")).unwrap().is_empty());
    bindings.put_string("keys.1.name", "Second");
    assert_eq!(
        {
            bindings.save(&mount.dir(), c"flight.cfg", "keys.").unwrap();
            mount.dir().read(Path::new("flight.cfg")).unwrap()
        },
        b"1.name = STRING:Second\n"
    );
    assert!(bindings.remove("keys.1.name"));
    assert!(!bindings.has_key("keys.1.name"));
}

#[test]
#[serial]
fn sorted_settings_bytes_keep_signed_boolean_color_and_prefix_rules() {
    let mut settings = Settings::default();
    let mut bindings = Bindings::default();
    let mount = crate::io::fixture::TempMount::new();
    let mut catalog = CatalogBuilder::new(mount.dir());
    mount.write("values.rmp", b"z = STRING:tail\na = INT32:-7\nb = BOOLEAN:TRUE\nc = COLOR:rgba(1, 2, 3, 4)\nnot-saved = MUSICRES:song.ogg\ninvalid = no-colon\n");
    catalog.import_index(
        &mount.dir(),
        c"values.rmp",
        Some(c"config."),
        &mut settings,
        &mut bindings,
    );
    settings.save(&mount.dir(), c"uqm.cfg", "config.").unwrap();
    assert_eq!(mount.dir().read(Path::new("uqm.cfg")).unwrap(), b"a = INT32:-7\nb = BOOLEAN:true\nc = COLOR:rgba(0x01, 0x02, 0x03, 0x04)\nz = STRING:tail\n");
    bindings.save(&mount.dir(), c"keys.cfg", "keys.").unwrap();
    assert!(mount.dir().read(Path::new("keys.cfg")).unwrap().is_empty());
}

#[test]
#[serial]
fn property_owners_keep_independent_replacements_and_serialized_bytes() {
    let mount = crate::io::fixture::TempMount::new();
    mount.write("a.cfg", b"alwaysgl = BOOLEAN:true\nvolume = INT32:-7\n");
    mount.write("b.cfg", b"alwaysgl = GFXRES:other.ani\nvolume = INT32:19\n");
    let mut first = Settings::default();
    let mut second = Settings::default();
    first.load_index(&mount.dir(), c"a.cfg", Some(c"config."));
    second.load_index(&mount.dir(), c"b.cfg", Some(c"config."));
    assert_eq!(first.boolean("config.alwaysgl"), Some(true));
    assert_eq!(second.boolean("config.alwaysgl"), None);
    assert!(second.has_key("config.alwaysgl"));
    second.put_integer("config.volume", 23);
    assert_eq!(first.integer("config.volume"), Some(-7));
    first.save(&mount.dir(), c"first.cfg", "config.").unwrap();
    second.save(&mount.dir(), c"second.cfg", "config.").unwrap();
    assert_eq!(
        mount.dir().read(Path::new("first.cfg")).unwrap(),
        b"alwaysgl = BOOLEAN:true\nvolume = INT32:-7\n"
    );
    assert_eq!(
        mount.dir().read(Path::new("second.cfg")).unwrap(),
        b"volume = INT32:23\n"
    );
    let mut first_bindings = Bindings::default();
    let mut second_bindings = Bindings::default();
    first_bindings.put_string("keys.1.up.1", "key Up");
    second_bindings.put_string("keys.1.up.1", "key w");
    first_bindings.remove("keys.1.up.1");
    assert_eq!(
        second_bindings.string("keys.1.up.1").as_deref(),
        Some("key w")
    );
    drop(first);
    assert_eq!(second.integer("config.volume"), Some(23));
}
