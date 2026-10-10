//! How a `TYPE:descriptor` value becomes an entry, traced from the C
//! `newResourceDesc` (resinit.c): the first colon splits type from
//! descriptor; a missing colon or a type nobody registered gives
//! `UNKNOWNRES` holding the whole value; keys are any bytes.

use proptest::prelude::*;

use super::*;
use crate::resource::type_registry::{descriptor_to_int, use_descriptor_as_res};

const REGISTERED: [&str; 3] = ["STRING", "INT32", UNKNOWN_TYPE];

fn dispatch_with_value_types() -> ResourceDispatch {
    let mut dispatch = ResourceDispatch::new();
    dispatch
        .type_registry
        .install("STRING", Some(use_descriptor_as_res), None, None);
    dispatch
        .type_registry
        .install("INT32", Some(descriptor_to_int), None, None);
    dispatch
}

/// What C stores for `value`, written out from the C source rather than
/// from the production code.
fn c_rule(value: &[u8]) -> (&'static str, &[u8]) {
    if let Some(colon) = value.iter().position(|&b| b == b':') {
        if let Some(name) = REGISTERED
            .iter()
            .find(|name| name.as_bytes() == &value[..colon])
        {
            return (name, &value[colon + 1..]);
        }
    }
    (UNKNOWN_TYPE, value)
}

#[test]
fn a_value_without_a_colon_is_unknownres_holding_all_of_it() {
    let mut dispatch = dispatch_with_value_types();
    dispatch.process_resource_desc("k", "plain/path").unwrap();
    let entry = dispatch.entry("k").unwrap();
    assert_eq!(entry.res_type, "UNKNOWNRES");
    assert_eq!(entry.descriptor_bytes(), b"plain/path");
    assert_eq!(unsafe { entry.data.str_ptr }, entry.descriptor.as_ptr());
}

#[test]
fn an_unregistered_type_is_unknownres_holding_all_of_it() {
    let mut dispatch = dispatch_with_value_types();
    dispatch.process_resource_desc("k", "FUTURE:foo").unwrap();
    let entry = dispatch.entry("k").unwrap();
    assert_eq!(entry.res_type, "UNKNOWNRES");
    assert_eq!(entry.descriptor_bytes(), b"FUTURE:foo");
}

#[test]
fn a_registered_type_keeps_its_type_and_the_text_after_the_first_colon() {
    let mut dispatch = dispatch_with_value_types();
    dispatch.process_resource_desc("n", "INT32:7").unwrap();
    dispatch.process_resource_desc("s", "STRING:a:b").unwrap();
    assert_eq!(dispatch.get_resource_type("n"), Some("INT32"));
    assert_eq!(dispatch.get_int_resource("n"), Some(7));
    assert_eq!(dispatch.get_resource_type("s"), Some("STRING"));
    assert_eq!(dispatch.get_string_resource("s"), Some(b"a:b".as_slice()));
}

#[test]
fn a_type_that_is_not_utf8_or_is_very_long_is_unknown_and_does_not_panic() {
    let mut dispatch = dispatch_with_value_types();
    dispatch
        .process_resource_desc("latin", b"\xE9T:foo")
        .unwrap();
    let long_type = "é".repeat(40);
    dispatch
        .process_resource_desc("long", format!("{long_type}:foo"))
        .unwrap();
    assert_eq!(dispatch.get_resource_type("latin"), Some("UNKNOWNRES"));
    assert_eq!(
        dispatch.entry("latin").unwrap().descriptor_bytes(),
        b"\xE9T:foo"
    );
    assert_eq!(dispatch.get_resource_type("long"), Some("UNKNOWNRES"));
    assert_eq!(
        dispatch.entry("long").unwrap().descriptor_bytes(),
        format!("{long_type}:foo").as_bytes()
    );
}

#[test]
fn keys_are_bytes_and_two_byte_strings_are_two_keys() {
    let mut dispatch = dispatch_with_value_types();
    dispatch.process_resource_desc(b"a\xE9", "INT32:1").unwrap();
    dispatch.process_resource_desc(b"a\xE8", "INT32:2").unwrap();
    assert_eq!(dispatch.entries.len(), 2);
    assert_eq!(dispatch.get_int_resource(b"a\xE9"), Some(1));
    assert_eq!(dispatch.get_int_resource(b"a\xE8"), Some(2));
    assert!(dispatch.remove_resource(b"a\xE9"));
    assert!(!dispatch.remove_resource(b"a\xE9"));
    assert_eq!(dispatch.get_int_resource(b"a\xE8"), Some(2));
}

#[test]
fn the_last_writer_wins_even_when_it_changes_the_type() {
    let mut dispatch = dispatch_with_value_types();
    dispatch.process_resource_desc("k", "INT32:1").unwrap();
    dispatch.process_resource_desc("k", "no type here").unwrap();
    assert_eq!(dispatch.entries.len(), 1);
    let entry = dispatch.entry("k").unwrap();
    assert_eq!(entry.res_type, "UNKNOWNRES");
    assert_eq!(entry.descriptor_bytes(), b"no type here");
}

#[test]
fn a_nul_in_the_key_or_the_value_is_refused_and_nothing_is_stored() {
    let mut dispatch = dispatch_with_value_types();
    assert_eq!(
        dispatch.process_resource_desc(b"k\0x", "INT32:1"),
        Err(DescriptorError::InteriorNul)
    );
    assert_eq!(
        dispatch.process_resource_desc("k", b"STRING:a\0b"),
        Err(DescriptorError::InteriorNul)
    );
    assert_eq!(
        dispatch.process_resource_desc("k", b"a\0b:c"),
        Err(DescriptorError::InteriorNul)
    );
    assert!(dispatch.entries.is_empty());
}

#[test]
fn a_type_without_a_load_function_creates_no_entry_and_keeps_the_old_one() {
    let mut dispatch = dispatch_with_value_types();
    dispatch.type_registry.install("NOLOAD", None, None, None);
    dispatch.process_resource_desc("k", "INT32:5").unwrap();
    assert_eq!(
        dispatch.process_resource_desc("k", "NOLOAD:x"),
        Err(DescriptorError::NoLoadFunction {
            type_name: "NOLOAD".to_string()
        })
    );
    assert_eq!(dispatch.get_int_resource("k"), Some(5));
}

fn value_bytes() -> impl Strategy<Value = Vec<u8>> {
    let type_text = prop_oneof![
        Just(b"STRING".to_vec()),
        Just(b"INT32".to_vec()),
        Just(b"UNKNOWNRES".to_vec()),
        Just(b"FUTURE".to_vec()),
        Just(b"string".to_vec()),
        Just(b"STRING ".to_vec()),
        Just(Vec::new()),
        Just(b"\xE9T".to_vec()),
        proptest::collection::vec(1u8..=255, 0..12),
    ];
    let tail = proptest::collection::vec(prop_oneof![1u8..=255, Just(b':')], 0..12);
    (type_text, any::<bool>(), tail).prop_map(|(mut text, colon, tail)| {
        if colon {
            text.push(b':');
        }
        text.extend(tail);
        text
    })
}

proptest! {
    #[test]
    fn any_value_without_nul_is_stored_as_c_stores_it(value in value_bytes()) {
        let mut dispatch = dispatch_with_value_types();
        dispatch.process_resource_desc("k", &value).unwrap();
        let entry = dispatch.entry("k").unwrap();
        let (res_type, descriptor) = c_rule(&value);
        prop_assert_eq!(entry.res_type.as_str(), res_type);
        prop_assert_eq!(entry.descriptor_bytes(), descriptor);
    }
}
