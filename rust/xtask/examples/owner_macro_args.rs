use quote::ToTokens;
use std::{collections::BTreeMap, fs, path::Path};
use syn::{
    spanned::Spanned,
    visit::{self, Visit},
};
struct Edit<'a> {
    source: &'a str,
    lines: Vec<usize>,
    names: &'a BTreeMap<String, (usize, bool)>,
    own: bool,
    cx: bool,
    edits: Vec<(usize, String)>,
}
impl Edit<'_> {
    fn offset(&self, p: proc_macro2::LineColumn) -> usize {
        let start = self.lines[p.line - 1];
        start
            + self.source[start..]
                .char_indices()
                .nth(p.column)
                .map_or(self.source.len() - start, |(i, _)| i)
    }
}
impl<'a> Visit<'a> for Edit<'_> {
    fn visit_item_fn(&mut self, f: &'a syn::ItemFn) {
        let old = (self.own, self.cx);
        self.own=f.sig.inputs.iter().any(|p|matches!(p,syn::FnArg::Typed(p)if matches!(&*p.pat,syn::Pat::Ident(p)if p.ident=="world")));
        self.cx = f.sig.inputs.to_token_stream().to_string().contains("Cx");
        visit::visit_item_fn(self, f);
        (self.own, self.cx) = old;
    }
    fn visit_expr_closure(&mut self, c: &'a syn::ExprClosure) {
        let old = self.cx;
        if c.inputs
            .iter()
            .any(|p| matches!(p,syn::Pat::Ident(p)if p.ident=="cx"))
        {
            self.cx = true
        }
        visit::visit_expr_closure(self, c);
        self.cx = old;
    }
    fn visit_macro(&mut self, m: &'a syn::Macro) {
        use syn::parse::Parser;
        if let Ok(args) = syn::punctuated::Punctuated::<syn::Expr, syn::Token![,]>::parse_terminated
            .parse2(m.tokens.clone())
        {
            for e in &args {
                self.visit_expr(e)
            }
        }
    }
    fn visit_expr_call(&mut self, e: &'a syn::ExprCall) {
        if let syn::Expr::Path(p) = &*e.func {
            let name = p.path.segments.last().unwrap().ident.to_string();
            if let Some((arity, mutable)) = self.names.get(&name) {
                if e.args.len() + 1 == *arity {
                    let arg = if self.own {
                        "world"
                    } else if self.cx {
                        "cx.world"
                    } else if *mutable {
                        "&mut **crate::engine::World::current()"
                    } else {
                        "&**crate::engine::World::current()"
                    };
                    self.edits.push((
                        self.offset(e.paren_token.span.open().end()),
                        format!("{arg}{}", if e.args.is_empty() { "" } else { ", " }),
                    ));
                }
            }
        }
        visit::visit_expr_call(self, e);
    }
}
fn walk(p: &Path, names: &BTreeMap<String, (usize, bool)>) {
    for entry in fs::read_dir(p).unwrap() {
        let p = entry.unwrap().path();
        if p.is_dir() {
            walk(&p, names)
        } else if p.extension().is_some_and(|x| x == "rs") {
            let source = fs::read_to_string(&p).unwrap();
            let ast = syn::parse_file(&source).unwrap();
            let mut lines = vec![0];
            for (i, b) in source.bytes().enumerate() {
                if b == b'\n' {
                    lines.push(i + 1)
                }
            }
            let mut aliases = names.clone();
            if source.contains("get_game_state as get") {
                aliases.insert("get".into(), names["get_game_state"]);
            }
            if source.contains("set_game_state as set") {
                aliases.insert("set".into(), names["set_game_state"]);
            }
            let mut e = Edit {
                source: &source,
                lines,
                names: &aliases,
                own: false,
                cx: false,
                edits: vec![],
            };
            e.visit_file(&ast);
            e.edits.sort_by_key(|e| e.0);
            let mut output = source.clone();
            for (a, s) in e.edits.into_iter().rev() {
                output.insert_str(a, &s)
            }
            if output != source {
                fs::write(p, output).unwrap()
            }
        }
    }
}
fn main() {
    struct Defs(BTreeMap<String, (usize, bool)>);
    impl<'a> Visit<'a> for Defs {
        fn visit_item_fn(&mut self, f: &'a syn::ItemFn) {
            if let Some(syn::FnArg::Typed(p)) = f.sig.inputs.first() {
                if matches!(&*p.pat,syn::Pat::Ident(p)if p.ident=="world") {
                    self.0.insert(
                        f.sig.ident.to_string(),
                        (
                            f.sig.inputs.len(),
                            p.ty.to_token_stream().to_string().starts_with("& mut"),
                        ),
                    );
                }
            }
        }
    }
    let mut defs = Defs(BTreeMap::new());
    for file in [
        "src/state/ffi.rs",
        "src/state/c_api.rs",
        "src/state/game_state_keys.rs",
    ] {
        defs.visit_file(&syn::parse_file(&fs::read_to_string(file).unwrap()).unwrap());
    }
    walk(Path::new("src"), &defs.0);
    walk(Path::new("tests"), &defs.0);
}
