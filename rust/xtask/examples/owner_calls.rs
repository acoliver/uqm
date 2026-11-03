use std::{fs, path::Path};
use syn::{
    spanned::Spanned,
    visit::{self, Visit},
};
struct Calls<'a> {
    source: &'a str,
    lines: Vec<usize>,
    edits: Vec<(usize, usize, String)>,
}
impl Calls<'_> {
    fn range(&self, s: proc_macro2::Span) -> (usize, usize) {
        let a = s.start();
        let b = s.end();
        let offset = |p: proc_macro2::LineColumn| {
            let start = self.lines[p.line - 1];
            start
                + self.source[start..]
                    .char_indices()
                    .nth(p.column)
                    .map_or(self.source.len() - start, |(i, _)| i)
        };
        (offset(a), offset(b))
    }
}
impl<'a> Visit<'a> for Calls<'_> {
    fn visit_macro(&mut self, mac: &'a syn::Macro) {
        use syn::parse::Parser;
        if let Ok(args) = syn::punctuated::Punctuated::<syn::Expr, syn::Token![,]>::parse_terminated
            .parse2(mac.tokens.clone())
        {
            for expr in &args {
                self.visit_expr(expr);
            }
        }
    }
    fn visit_expr_call(&mut self, e: &'a syn::ExprCall) {
        if let Some(first) = e.args.first() {
            let (a, b) = self.range(first.span());
            let text = &self.source[a..b];
            if [
                "&mut **crate::engine::World::current()",
                "&**crate::engine::World::current()",
            ]
            .contains(&text)
            {
                let (start, end) = self.range(e.span());
                let mut call = self.source[start..end].to_owned();
                call.replace_range(a - start..b - start, "world");
                self.edits.push((
                    start,
                    end,
                    format!(
                        "crate::engine::World::current().{}(|world| {call})",
                        if text.starts_with("&mut") {
                            "write"
                        } else {
                            "read"
                        }
                    ),
                ));
                return;
            }
        }
        visit::visit_expr_call(self, e)
    }
}
fn walk(path: &Path) {
    for entry in fs::read_dir(path).unwrap() {
        let p = entry.unwrap().path();
        if p.is_dir() {
            walk(&p)
        } else if p.extension().is_some_and(|x| x == "rs") {
            let source = fs::read_to_string(&p).unwrap();
            let syntax = syn::parse_file(&source).unwrap();
            let mut lines = vec![0];
            for (i, b) in source.bytes().enumerate() {
                if b == b'\n' {
                    lines.push(i + 1)
                }
            }
            let mut calls = Calls {
                source: &source,
                lines,
                edits: vec![],
            };
            calls.visit_file(&syntax);
            let mut output = source.clone();
            for (a, b, text) in calls.edits.into_iter().rev() {
                output.replace_range(a..b, &text)
            }
            if output != source {
                fs::write(p, output).unwrap()
            }
        }
    }
}
fn main() {
    walk(Path::new("src"));
    walk(Path::new("tests"))
}
