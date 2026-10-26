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
