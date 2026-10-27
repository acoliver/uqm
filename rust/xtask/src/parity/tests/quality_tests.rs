use super::*;
use quote::ToTokens;
use syn::visit::Visit;

#[derive(Default)]
struct Functions {
    measured: usize,
    violations: Vec<String>,
}

impl Functions {
    fn check(&mut self, name: &syn::Ident, block: &syn::Block) {
        self.measured += 1;
        let loc = crate::quality::metrics::lines(block.to_token_stream()).len() as u64;
        let (cyclomatic, cognitive) = crate::quality::metrics::Complexity::measure(block);
        for (metric, value) in [
            ("loc", loc),
            ("cyclomatic", cyclomatic),
            ("cognitive", cognitive),
        ] {
            if value > crate::quality::metrics::limit("complexity", metric) {
                self.violations.push(format!("{name}: {metric}={value}"));
            }
        }
    }
}

impl<'ast> Visit<'ast> for Functions {
    fn visit_item_fn(&mut self, function: &'ast syn::ItemFn) {
        self.check(&function.sig.ident, &function.block);
    }
    fn visit_impl_item_fn(&mut self, function: &'ast syn::ImplItemFn) {
        self.check(&function.sig.ident, &function.block);
    }
}

fn visit_files(directory: &Path, functions: &mut Functions) {
    for entry in fs::read_dir(directory).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            if path.file_name().unwrap() != "tests" {
                visit_files(&path, functions);
            }
        } else if path.extension().is_some_and(|ext| ext == "rs")
            && path.file_name().unwrap() != "tests.rs"
        {
            let parsed = syn::parse_file(&fs::read_to_string(path).unwrap()).unwrap();
            functions.visit_file(&parsed);
        }
    }
}

#[test]
fn parity_functions_meet_existing_quality_limits() {
    let directory = root().join("rust/xtask/src");
    let mut functions = Functions::default();
    functions.visit_file(
        &syn::parse_file(&fs::read_to_string(directory.join("parity.rs")).unwrap()).unwrap(),
    );
    visit_files(&directory.join("parity"), &mut functions);
    assert!(
        functions.measured > 30,
        "parity functions were not measured"
    );
    assert!(
        functions.violations.is_empty(),
        "{:#?}",
        functions.violations
    );
}
