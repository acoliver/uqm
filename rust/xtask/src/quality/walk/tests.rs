use super::*;
fn fixture(files: &[(&str, &str)], names: Option<&[String]>, defs_only: bool) -> Measurement {
    let root = tempfile::tempdir().unwrap();
    for (path, text) in files {
        let path = root.path().join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }
    let mut walker = Walker::new(root.path().into(), names, defs_only);
    walker.context = Context {
        package: "uqm".into(),
        module: "uqm".into(),
        configs: Configuration::ci(&BTreeSet::new())
            .unwrap()
            .into_iter()
            .map(Rc::new)
            .collect(),
        ..Context::default()
    };
    walker.load(&root.path().join("src/lib.rs"), None).unwrap();
    walker.finish(false).unwrap()
}
#[test]
fn identities_include_modules_impl_types_and_traits() {
    let m = fixture(&[("src/lib.rs", "mod graphics { mod filegfx { fn load() {} } mod sdl { mod png2sdl { fn load() {} } } } struct Ship; trait Race {} impl Race for Ship { fn load() {} }")], None, false);
    let identities: BTreeSet<_> = m
        .findings
        .iter()
        .filter(|f| f.counter == "complexity")
        .map(|f| f.identity.as_str())
        .collect();
    assert!(identities.contains("uqm::graphics::filegfx::load"));
    assert!(identities.contains("uqm::graphics::sdl::png2sdl::load"));
    assert!(identities.contains("uqm::<Ship as Race>::load"));
}
#[test]
fn include_inherits_allow_coverage_and_cfg_union_counts_both_platforms() {
    let m = fixture(&[("src/lib.rs", "#[allow(clippy::all)] mod native { include!(\"abi/ship.rs\"); } #[cfg(unix)] fn posix() {} #[cfg(windows)] fn windows() {} #[cfg(test)] mod tests { unsafe fn excluded() {} }"), ("src/abi/ship.rs", "pub unsafe fn provider() {\n unsafe { run(); }\n}\n")], None, false);
    assert_eq!(m.report("unsafe_fn", None).unwrap(), 1);
    assert_eq!(m.report("abi_includes", None).unwrap(), 1);
    assert!(m
        .findings
        .iter()
        .any(|f| f.counter == "allows" && f.value >= 3));
    assert!(m.findings.iter().any(|f| f.identity == "uqm::posix"));
    assert!(m.findings.iter().any(|f| f.identity == "uqm::windows"));
}
#[test]
fn ident_definitions_exclude_foreign_declarations_reexports_docs_and_strings_but_include_tests() {
    let names = vec!["InitQueue".into()];
    let m = fixture(&[("src/lib.rs", "/// C: InitQueue\nfn InitQueue() {}\nextern \"C\" { fn InitQueue(); }\nuse another::InitQueue;\n#[cfg(test)] mod tests { fn InitQueue() { let text = \"InitQueue\"; } }\n")], Some(&names), true);
    assert_eq!(m.identifiers.iter().map(|f| f.value).sum::<u64>(), 2);
    let m = fixture(
        &[(
            "src/lib.rs",
            "/// C: InitQueue\nfn InitQueue() { let text = \"InitQueue\"; InitQueue(); }\n",
        )],
        Some(&names),
        false,
    );
    assert_eq!(m.identifiers.iter().map(|f| f.value).sum::<u64>(), 2);
}
#[test]
fn active_tokens_on_the_same_line_as_test_items_still_count() {
    let m = fixture(
        &[(
            "src/lib.rs",
            "fn live() {} #[cfg(test)] fn test_only() {}\n",
        )],
        None,
        false,
    );
    assert_eq!(m.report("loc", None).unwrap(), 1);
}

#[test]
fn private_aliases_raw_records_queue_calls_and_hook_aliases_are_measured() {
    let m = fixture(
        &[(
            "src/lib.rs",
            r#"
            use crate::collections::queue as q;
            use elsewhere::ELEMENT as Record;
            pub use elsewhere::STARSHIP;
            fn run(element: *mut Record) {
                q::PutQueue();
                unsafe { if let Some(pre) = (*element).preprocess_func { pre(element); } }
                assert!(unsafe { works() });
            }
            fn invoke_pre(element: *mut Record) { ((*element).preprocess_func.unwrap())(element); }
        "#,
        )],
        None,
        false,
    );
    assert_eq!(m.report("queue_calls", None).unwrap(), 1);
    assert_eq!(m.report("hook_calls", None).unwrap(), 1);
    assert_eq!(m.report("unsafe_blocks", None).unwrap(), 2);
    assert_eq!(m.report("raw_records", None).unwrap(), 3);
}

#[test]
fn explicit_paths_inside_inline_modules_and_manifest_includes_are_followed() {
    let m = fixture(
        &[
            (
                "src/lib.rs",
                r#"mod outer { #[path = "custom.rs"] mod inner; } include!(concat!(env!("CARGO_MANIFEST_DIR"), "/extra.rs"));"#,
            ),
            ("src/outer/custom.rs", "unsafe fn first() {}"),
            ("extra.rs", "unsafe fn second() {}"),
        ],
        None,
        false,
    );
    assert_eq!(m.report("unsafe_fn", None).unwrap(), 2);
}

#[test]
fn cfg_test_expressions_do_not_inflate_complexity_or_unsafe_lines() {
    let m = fixture(
        &[(
            "src/lib.rs",
            "unsafe fn run() {\n#[cfg(test)]\n{ if a { while b { fail(); } } }\nlive();\n}\n",
        )],
        None,
        false,
    );
    let f = m
        .findings
        .iter()
        .find(|f| f.metric == "cyclomatic")
        .unwrap();
    assert_eq!(f.value, 1);
    assert_eq!(m.report("unsafe_fn_lines", None).unwrap(), 3);
}

#[test]
fn stale_macro_registry_is_an_error() {
    let root = tempfile::tempdir().unwrap();
    let walker = Walker::new(root.path().into(), None, false);
    assert!(walker
        .finish(true)
        .unwrap_err()
        .contains("stale quality macro handler"));
}

#[test]
fn macro_handlers_expand_every_registered_definition() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    struct Macros(Vec<syn::ItemMacro>);
    impl<'a> Visit<'a> for Macros {
        fn visit_item_macro(&mut self, m: &'a syn::ItemMacro) {
            if m.ident.is_some() {
                self.0.push(m.clone());
            }
        }
    }
    for (file, name) in macros::HANDLERS {
        let source = syn::parse_file(&fs::read_to_string(root.join(file)).unwrap()).unwrap();
        let mut found = Macros(Vec::new());
        found.visit_file(&source);
        let m = found
            .0
            .iter()
            .find(|m| m.ident.as_ref().unwrap() == name)
            .unwrap();
        let definition = Definition::parse(m.mac.tokens.clone()).unwrap();
        assert!(definition.generates_items(), "{file}::{name}");
        struct Calls<'a> {
            name: &'a str,
            definition: &'a Definition,
            count: usize,
        }
        impl<'a> Visit<'a> for Calls<'_> {
            fn visit_macro(&mut self, m: &'a syn::Macro) {
                if m.path.is_ident(self.name) {
                    let expanded = self
                        .definition
                        .expand(m.tokens.clone())
                        .unwrap_or_else(|e| panic!("{}: {e}", self.name));
                    syn::parse2::<syn::File>(expanded).unwrap();
                    self.count += 1;
                }
            }
        }
        let mut calls = Calls {
            name,
            definition: &definition,
            count: 0,
        };
        calls.visit_file(&source);
        if *name == "assert_c_layout" {
            let expanded = definition
                .expand(quote::quote!(
                    Mirror,
                    Record {
                        first: first,
                        second: second
                    }
                ))
                .unwrap();
            let file: syn::File = syn::parse2(expanded).unwrap();
            assert_eq!(file.items.len(), 1);
            assert!(file.to_token_stream().to_string().contains("unsafe"));
        }
        assert!(
            calls.count > 0 || *name == "assert_c_layout",
            "{file}::{name}"
        );
    }
}

#[test]
fn layers_follow_aliases_relative_imports_and_foreign_symbol_names() {
    let measured = fixture(
        &[(
            "src/lib.rs",
            r#"
        mod mainloop { #[no_mangle] static mut CONTENT: u8 = 0; }
        mod graphics {
            use crate::mainloop::CONTENT as state;
            fn read() { let _ = state; }
            extern "C" { #[link_name = "CONTENT"] static content: u8; }
            #[cfg(test)] fn excluded() { crate::mainloop::test_only(); }
        }
        mod comm { struct Mode; mod races { fn run() { let _ = super::Mode; } } }
    "#,
        )],
        None,
        false,
    );
    assert_eq!(measured.report("planned.uqm-engine", None).unwrap(), 3);
    assert_eq!(measured.report("planned.uqm-content", None).unwrap(), 1);
}

fn record_transfer_fixture(record: &str) -> Measurement {
    fixture(
        &[
            (
                "src/lib.rs",
                "mod battle {} mod engine { pub mod battle { pub mod record; } }",
            ),
            ("src/engine/battle/record.rs", record),
        ],
        None,
        false,
    )
}

#[test]
fn record_field_transfer_survives_disappearing_raw_record_debt() {
    use super::super::baseline::{check_limits, Baseline};
    let record = "#[repr(C)] struct element { hTarget: HELEMENT, value: u8 }";
    let base = fixture(
        &[("src/lib.rs", &format!("mod battle {{ {record} }}"))],
        None,
        false,
    );
    let current = record_transfer_fixture(record);
    let target = "uqm::engine::battle::record::element::hTarget";
    assert!(base
        .findings
        .iter()
        .any(|f| f.identity == "uqm::battle::element::hTarget" && f.metric == "raw_records"));
    assert!(!current
        .findings
        .iter()
        .any(|f| f.identity == target && f.metric == "raw_records"));
    assert!(current.declarations.iter().any(|f| f.identity == target));
    let mut baseline = Baseline {
        entries: base.baseline(),
        ..Default::default()
    };
    baseline
        .add_move(
            "uqm::battle::element",
            "uqm::engine::battle::record::element",
            &current,
            &base,
        )
        .unwrap();
    check_limits(
        &current.baseline(),
        &baseline.entries,
        &BTreeMap::new(),
        true,
    )
    .unwrap();
    assert_eq!(baseline.entries["ffi|raw_records|uqm::engine"], 1);
    assert_eq!(baseline.entries["ffi|repr_c|uqm::engine"], 1);
    assert!(!baseline.entries.contains_key("ffi|raw_records|uqm::battle"));
    assert!(baseline
        .add_move(
            "uqm::battle::element",
            "uqm::engine::another",
            &current,
            &base
        )
        .is_err());
    let mut growth = current.baseline();
    growth.insert("ffi|repr_c|uqm::engine".into(), 2);
    assert!(check_limits(&growth, &baseline.entries, &BTreeMap::new(), true).is_err());
    let retained = fixture(
        &[
            ("src/lib.rs", &format!("mod battle {{ {record} }} mod engine {{ pub mod battle {{ pub mod record; }} }}")),
            ("src/engine/battle/record.rs", record),
        ],
        None,
        false,
    );
    assert!(
        super::super::transfers::validate(&retained, &base, &baseline.moved)
            .unwrap_err()
            .contains("still exists")
    );
}

#[test]
fn record_transfer_rejects_removed_fields_even_without_metric_findings() {
    use super::super::baseline::Baseline;
    let base = fixture(
        &[(
            "src/lib.rs",
            "mod battle { #[repr(C)] struct element { hTarget: HELEMENT, value: u8 } }",
        )],
        None,
        false,
    );
    for (record, missing) in [
        ("#[repr(C)] struct element { value: u8 }", "hTarget"),
        ("#[repr(C)] struct element { hTarget: HELEMENT }", "value"),
    ] {
        let current = record_transfer_fixture(record);
        let mut baseline = Baseline {
            entries: base.baseline(),
            ..Default::default()
        };
        let error = baseline
            .add_move(
                "uqm::battle::element",
                "uqm::engine::battle::record::element",
                &current,
                &base,
            )
            .unwrap_err();
        assert!(
            error.contains(&format!("element::{missing} is absent")),
            "{error}"
        );
        assert!(baseline.moved.is_empty());
        assert_eq!(baseline.entries, base.baseline());
    }
}

#[test]
fn declaration_move_keeps_the_source_subsystem_live() {
    let base = fixture(
        &[(
            "src/lib.rs",
            "mod battle { #[no_mangle] static mut MOVED: u8 = 0; static mut KEPT: u8 = 0; }",
        )],
        None,
        false,
    );
    let current = fixture(&[("src/lib.rs", "mod battle { static mut KEPT: u8 = 0; } mod engine { #[no_mangle] static mut MOVED: u8 = 0; }")], None, false);
    let mut baseline = super::super::baseline::Baseline {
        entries: base.baseline(),
        ..Default::default()
    };
    baseline
        .add_move("uqm::battle::MOVED", "uqm::engine::MOVED", &current, &base)
        .unwrap();
    super::super::baseline::validate_transfers(&current, &base, &baseline.moved).unwrap();
}

#[test]
fn declaration_transfers_conserve_each_metric_and_cannot_reuse_credit() {
    use super::super::baseline::{check_limits, Baseline};
    let declarations = "#[no_mangle] static mut MOVED: u8 = 0; #[repr(C)] struct ELEMENT { value: u8 } #[no_mangle] extern \"C\" fn exported(value: *mut ELEMENT) {}";
    let before = format!("mod battle {{ {declarations} static mut KEPT: u8 = 0; }}");
    let after =
        format!("mod battle {{ static mut KEPT: u8 = 0; }} mod engine {{ {declarations} }}");
    let base = fixture(&[("src/lib.rs", &before)], None, false);
    let current = fixture(&[("src/lib.rs", &after)], None, false);
    let mut baseline = Baseline {
        entries: base.baseline(),
        ..Default::default()
    };
    for name in ["MOVED", "ELEMENT", "exported"] {
        baseline
            .add_move(
                &format!("uqm::battle::{name}"),
                &format!("uqm::engine::{name}"),
                &current,
                &base,
            )
            .unwrap();
    }
    check_limits(
        &current.baseline(),
        &baseline.entries,
        &BTreeMap::new(),
        true,
    )
    .unwrap();
    assert_eq!(baseline.entries["ffi|static_mut|uqm::battle"], 1);
    assert_eq!(baseline.entries["ffi|static_mut|uqm::engine"], 1);
    for metric in [
        "static_mut",
        "repr_c",
        "no_mangle",
        "extern_c_fn",
        "raw_pointer_parameters",
        "raw_records",
    ] {
        let total = |entries: &BTreeMap<String, u64>| {
            entries
                .iter()
                .filter(|(k, _)| k.starts_with(&format!("ffi|{metric}|")))
                .map(|(_, v)| v)
                .sum::<u64>()
        };
        assert_eq!(
            total(&base.baseline()),
            total(&baseline.entries),
            "{metric}"
        );
    }
    assert!(baseline
        .add_move(
            "uqm::battle::MOVED",
            "uqm::engine::another",
            &current,
            &base
        )
        .is_err());
    let mut growth = current.baseline();
    *growth.get_mut("ffi|static_mut|uqm::engine").unwrap() += 1;
    assert!(check_limits(&growth, &baseline.entries, &BTreeMap::new(), true).is_err());
    let retained = fixture(
        &[("src/lib.rs", &format!("{after} mod duplicate {{}}"))],
        None,
        false,
    );
    let next =
        super::super::transfers::limits(&baseline.entries, &retained, &current, &baseline.moved)
            .unwrap();
    assert_eq!(
        next, baseline.entries,
        "already landed moves do not credit twice"
    );
    super::super::transfers::retire(&mut baseline.moved, &current);
    assert!(baseline.moved.is_empty());
}

#[test]
fn a_source_declaration_cannot_remain_even_when_its_ffi_debt_disappears() {
    use super::super::baseline::Baseline;
    let base = fixture(
        &[("src/lib.rs", "mod battle { #[repr(C)] struct Record; }")],
        None,
        false,
    );
    let current = fixture(
        &[(
            "src/lib.rs",
            "mod battle { struct Record; } mod engine { #[repr(C)] struct Record; }",
        )],
        None,
        false,
    );
    let mut baseline = Baseline {
        entries: base.baseline(),
        ..Default::default()
    };
    assert!(baseline
        .add_move(
            "uqm::battle::Record",
            "uqm::engine::Record",
            &current,
            &base
        )
        .unwrap_err()
        .contains("still exists"));
    assert!(baseline.moved.is_empty());
    assert_eq!(baseline.entries, base.baseline());
}

#[test]
fn inherited_allow_declaration_transfer_conserves_lines_and_rejects_growth() {
    use super::super::baseline::{check_limits, Baseline};
    let before = "#[allow(non_snake_case)]\nmod old {\n    #[repr(C)]\n    struct RECORD {\n        CamelField: u8,\n    }\n    struct Kept;\n}\n#[allow(non_snake_case)]\nmod new {\n}\n";
    let after = "#[allow(non_snake_case)]\nmod old {\n    struct Kept;\n}\n#[allow(non_snake_case)]\nmod new {\n    #[repr(C)]\n    struct RECORD {\n        CamelField: u8,\n    }\n}\n";
    let base = fixture(&[("src/lib.rs", before)], None, false);
    let current = fixture(&[("src/lib.rs", after)], None, false);
    let mut baseline = Baseline {
        entries: base.baseline(),
        ..Default::default()
    };
    baseline
        .add_move("uqm::old::RECORD", "uqm::new::RECORD", &current, &base)
        .unwrap();
    assert_eq!(baseline.entries["allows|non_snake_case|uqm::old"], 4);
    assert_eq!(baseline.entries["allows|non_snake_case|uqm::new"], 7);
    assert_eq!(
        baseline.entries.values().sum::<u64>(),
        base.baseline().values().sum::<u64>()
    );
    check_limits(
        &current.baseline(),
        &baseline.entries,
        &BTreeMap::new(),
        true,
    )
    .unwrap();
    let mut grown = current.baseline();
    *grown.get_mut("allows|non_snake_case|uqm::new").unwrap() += 1;
    assert!(check_limits(&grown, &baseline.entries, &BTreeMap::new(), true).is_err());
}

#[test]
fn anonymous_item_identities_ignore_line_and_column_changes() {
    let source = "mod graphics {\n#[allow(clippy::identity_op)] const _: () = { let _ = 1 + 0; };\nconst _: () = ();\n}\n";
    let base = fixture(&[("src/lib.rs", source)], None, false);
    let shifted = fixture(
        &[(
            "src/lib.rs",
            &format!(
                "struct MovedAway;\n\n{}",
                source.replace("const _", "    const _")
            ),
        )],
        None,
        false,
    );
    let ids = |m: &Measurement| {
        m.declarations
            .iter()
            .filter(|f| f.identity.contains("::_"))
            .map(|f| f.identity.clone())
            .collect::<BTreeSet<_>>()
    };
    assert_eq!(ids(&base), ids(&shifted));
    assert_eq!(ids(&base).len(), 2);
    assert!(ids(&base).iter().all(|id| !id.contains('@')));
}

#[test]
fn inherited_allow_transfers_deduplicate_shared_lines_and_include_coverage() {
    use super::super::baseline::{check_limits, Baseline};
    let measure = |text: &str| {
        fixture(
            &[
                ("src/lib.rs", text),
                (
                    "src/records.rs",
                    "struct A; struct B;\n#[cfg(test)] struct TestOnly;\n/// docs\nstruct Kept;\n",
                ),
            ],
            None,
            false,
        )
    };
    let base = measure("#[allow(non_snake_case)] mod old { include!(\"records.rs\"); }\n#[allow(non_snake_case)] mod new {}\n");
    let current = fixture(&[("src/lib.rs", "#[allow(non_snake_case)] mod old { struct Kept; }\n#[allow(non_snake_case)] mod new { struct A; struct B; }\n")], None, false);
    let mut baseline = Baseline {
        entries: base.baseline(),
        ..Default::default()
    };
    for name in ["A", "B"] {
        baseline
            .add_move(
                &format!("uqm::old::{name}"),
                &format!("uqm::new::{name}"),
                &current,
                &base,
            )
            .unwrap();
    }
    assert_eq!(baseline.entries["allows|non_snake_case|uqm::old"], 2);
    assert_eq!(baseline.entries["allows|non_snake_case|uqm::new"], 2);
    check_limits(
        &current.baseline(),
        &baseline.entries,
        &BTreeMap::new(),
        true,
    )
    .unwrap();
    let split = fixture(&[("src/lib.rs", "#[allow(non_snake_case)] mod old { struct Kept; }\n#[allow(non_snake_case)] mod new { struct A; }\n#[allow(non_snake_case)] mod other { struct B; }\n")], None, false);
    let mut baseline = Baseline {
        entries: base.baseline(),
        ..Default::default()
    };
    baseline
        .add_move("uqm::old::A", "uqm::new::A", &split, &base)
        .unwrap();
    assert!(baseline
        .add_move("uqm::old::B", "uqm::other::B", &split, &base)
        .unwrap_err()
        .contains("credited to both"));
}

#[test]
fn anonymous_macro_identities_and_baseline_rekey_are_stable() {
    use super::super::baseline::Baseline;
    let macros = "macro_rules! assert_c_layout { ($name:ident) => { const _: () = { #[allow(clippy::identity_op)] fn check() { let _ = 1 + 0; } }; }; }";
    let source =
        "include!(\"c_bindings.rs\");\nassert_c_layout!(First);\nassert_c_layout!(Second);\n";
    let base = fixture(
        &[("src/lib.rs", source), ("src/c_bindings.rs", macros)],
        None,
        false,
    );
    let shifted = fixture(
        &[
            ("src/lib.rs", &format!("struct Named;\n\n{source}")),
            ("src/c_bindings.rs", macros),
        ],
        None,
        false,
    );
    assert_eq!(base.baseline(), shifted.baseline());
    let mut baseline = Baseline::default();
    for (old, new) in &base.identity_renames {
        baseline
            .entries
            .insert(format!("allows|lint|{old}::child"), 7);
        assert!(!new.contains('@'));
    }
    let count = baseline.entries.len();
    baseline.rekey(&base.identity_renames).unwrap();
    let entries = baseline.entries.clone();
    baseline.rekey(&base.identity_renames).unwrap();
    assert_eq!(baseline.entries, entries);
    assert_eq!(baseline.entries.len(), count);
    assert!(baseline.entries.keys().all(|k| !k.contains('@')));
    assert!(baseline.entries.values().all(|v| *v == 7));
}

#[test]
fn planned_bridge_calls_are_budgeted_without_hiding_alias_or_other_upward_edges() {
    let direct = fixture(
        &[(
            "src/lib.rs",
            "mod pickship { fn run() { crate::app::bridge::enter(); } }",
        )],
        None,
        false,
    );
    assert_eq!(direct.report("bridge_calls", None).unwrap(), 1);
    assert_eq!(direct.report("module_edges", None).unwrap(), 0);
    assert_eq!(direct.report("planned.uqm-modes", None).unwrap(), 0);
    let alias = fixture(
        &[(
            "src/lib.rs",
            "mod pickship { use crate::app::bridge as handoff; fn run() { handoff::enter(); } }",
        )],
        None,
        false,
    );
    assert_eq!(alias.report("bridge_calls", None).unwrap(), 1);
    assert!(alias.report("module_edges", None).unwrap() > 0);
    let reexport = fixture(
        &[(
            "src/lib.rs",
            "mod pickship { pub use crate::app::bridge::enter; fn run() { enter(); } }",
        )],
        None,
        false,
    );
    assert_eq!(reexport.report("bridge_calls", None).unwrap(), 1);
    assert!(reexport.report("module_edges", None).unwrap() > 0);
    let address = fixture(
        &[(
            "src/lib.rs",
            "mod pickship { fn run() { let handoff = crate::app::bridge::enter; handoff(); } }",
        )],
        None,
        false,
    );
    assert_eq!(address.report("bridge_calls", None).unwrap(), 0);
    assert_eq!(address.report("module_edges", None).unwrap(), 1);
    assert_eq!(address.report("planned.uqm-modes", None).unwrap(), 1);
    let other = fixture(
        &[(
            "src/lib.rs",
            "mod pickship { fn run() { crate::app::other::enter(); } }",
        )],
        None,
        false,
    );
    assert_eq!(other.report("bridge_calls", None).unwrap(), 0);
    assert_eq!(other.report("module_edges", None).unwrap(), 1);
    assert_eq!(other.report("planned.uqm-modes", None).unwrap(), 1);
}
