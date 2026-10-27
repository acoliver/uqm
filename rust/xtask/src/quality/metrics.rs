use proc_macro2::{TokenStream, TokenTree};
use quote::ToTokens;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
#[cfg(test)]
use syn::spanned::Spanned;
use syn::visit::{self, Visit};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Finding {
    pub counter: String,
    pub metric: String,
    pub identity: String,
    pub file: String,
    pub package: String,
    pub subsystem: String,
    pub value: u64,
}

impl Finding {
    pub fn in_scope(&self, scope: &str) -> bool {
        self.package == scope
            || self.subsystem == scope
            || self.file == scope
            || self.file.strip_prefix("src/").is_some_and(|f| f == scope)
            || self.identity == scope
            || self.identity.starts_with(&format!("{scope}::"))
            || self.file.starts_with(&format!("{scope}/"))
    }
    pub fn matches_counter(&self, counter: &str) -> bool {
        if [
            "loc",
            "complexity",
            "ffi",
            "unsafe",
            "allows",
            "native-files",
            "layers",
        ]
        .contains(&counter)
        {
            self.counter == counter
        } else {
            self.metric == counter
        }
    }
    pub fn key(&self) -> String {
        format!("{}|{}|{}", self.counter, self.metric, self.identity)
    }
}

#[derive(Debug)]
pub struct AllowCoverage {
    pub item: String,
    pub scope: String,
    pub file: String,
    pub lines: BTreeSet<usize>,
}

#[derive(Default, Debug)]
pub struct Measurement {
    pub findings: Vec<Finding>,
    pub declarations: Vec<Finding>,
    pub allow_coverage: Vec<AllowCoverage>,
    pub identity_renames: BTreeMap<String, String>,
    pub handlers: BTreeSet<(String, String)>,
    pub identifiers: Vec<Finding>,
}

impl Measurement {
    pub fn report(&self, counter: &str, scope: Option<&str>) -> Result<u64, String> {
        if ![
            "loc",
            "complexity",
            "ffi",
            "unsafe",
            "allows",
            "native-files",
            "layers",
        ]
        .contains(&counter)
            && !self.findings.iter().any(|f| f.metric == counter)
        {
            return Err(format!("unknown quality counter {counter}"));
        }
        Ok(self
            .findings
            .iter()
            .filter(|f| f.matches_counter(counter) && scope.is_none_or(|s| f.in_scope(s)))
            .map(|f| f.value)
            .sum())
    }

    pub fn baseline(&self) -> BTreeMap<String, u64> {
        let mut values = BTreeMap::new();
        for finding in &self.findings {
            let limit = limit(&finding.counter, &finding.metric);
            if finding.counter == "loc" || finding.counter == "complexity" {
                if finding.value > limit {
                    values.insert(finding.key(), finding.value);
                }
            } else if finding.value != 0 {
                let identity = if ["ffi", "unsafe"].contains(&finding.counter.as_str()) {
                    format!("{}::{}", finding.package, finding.subsystem)
                } else {
                    finding.identity.clone()
                };
                *values
                    .entry(format!("{}|{}|{identity}", finding.counter, finding.metric))
                    .or_default() += finding.value;
            }
        }
        values
    }
}

pub fn limit(counter: &str, metric: &str) -> u64 {
    match (counter, metric) {
        ("loc", _) => 800,
        ("complexity", "loc") => 80,
        ("complexity", "cyclomatic") => 25,
        ("complexity", "cognitive") => 30,
        _ => 0,
    }
}

type Position = (usize, usize);
#[derive(Default)]
pub struct Excluded(Vec<(Position, Position)>);
impl Excluded {
    pub fn new(spans: &[proc_macro2::Span]) -> Self {
        let mut spans: Vec<_> = spans
            .iter()
            .map(|s| {
                (
                    (s.start().line, s.start().column),
                    (s.end().line, s.end().column),
                )
            })
            .collect();
        spans.sort_unstable();
        let mut merged: Vec<(Position, Position)> = Vec::new();
        for (start, end) in spans {
            if let Some(last) = merged.last_mut() {
                if start <= last.1 {
                    last.1 = last.1.max(end);
                    continue;
                }
            }
            merged.push((start, end));
        }
        Self(merged)
    }
    fn contains(&self, span: proc_macro2::Span) -> bool {
        let start = (span.start().line, span.start().column);
        let end = (span.end().line, span.end().column);
        let index = self.0.partition_point(|(s, _)| *s <= start);
        index != 0 && end <= self.0[index - 1].1
    }
}
pub fn lines(stream: TokenStream) -> BTreeSet<usize> {
    filtered_lines(stream, &Excluded::default())
}
pub fn filtered_lines(stream: TokenStream, excluded: &Excluded) -> BTreeSet<usize> {
    fn insert(span: proc_macro2::Span, excluded: &Excluded, output: &mut BTreeSet<usize>) {
        if !excluded.contains(span) {
            output.extend(span.start().line..=span.end().line);
        }
    }
    fn collect(stream: TokenStream, excluded: &Excluded, output: &mut BTreeSet<usize>) {
        for token in stream {
            if excluded.contains(token.span()) {
                continue;
            }
            match token {
                TokenTree::Group(g) => {
                    insert(g.span_open(), excluded, output);
                    insert(g.span_close(), excluded, output);
                    collect(g.stream(), excluded, output);
                }
                t => insert(t.span(), excluded, output),
            }
        }
    }
    let mut output = BTreeSet::new();
    collect(stream, excluded, &mut output);
    output.remove(&0);
    output
}

#[cfg(test)]
pub fn effective_lines<T: ToTokens>(value: &T) -> BTreeSet<usize> {
    struct Docs(BTreeSet<usize>);
    impl<'a> Visit<'a> for Docs {
        fn visit_attribute(&mut self, attr: &'a syn::Attribute) {
            if attr.path().is_ident("doc") {
                self.0
                    .extend(attr.span().start().line..=attr.span().end().line);
            }
        }
    }
    let tokens = value.to_token_stream();
    let mut docs = Docs(BTreeSet::new());
    if let Ok(file) = syn::parse2::<syn::File>(tokens.clone()) {
        docs.visit_file(&file);
    }
    lines(tokens).difference(&docs.0).copied().collect()
}

#[derive(Default)]
pub struct Complexity {
    pub cyclomatic: u64,
    pub cognitive: u64,
    depth: u64,
    boolean: Option<String>,
    configs: Vec<std::rc::Rc<super::cfg::Configuration>>,
    errors: Vec<String>,
}

impl Complexity {
    #[cfg(test)]
    pub fn measure(block: &syn::Block) -> (u64, u64) {
        let mut visitor = Self {
            cyclomatic: 1,
            ..Self::default()
        };
        visitor.visit_block(block);
        (visitor.cyclomatic, visitor.cognitive)
    }
    pub fn measure_configured(
        block: &syn::Block,
        configs: &[std::rc::Rc<super::cfg::Configuration>],
    ) -> Result<(u64, u64), String> {
        let mut visitor = Self {
            cyclomatic: 1,
            configs: configs.to_vec(),
            ..Self::default()
        };
        visitor.visit_block(block);
        if visitor.errors.is_empty() {
            Ok((visitor.cyclomatic, visitor.cognitive))
        } else {
            Err(visitor.errors.join("\n"))
        }
    }
    fn active(&mut self, attrs: &[syn::Attribute]) -> bool {
        if self.configs.is_empty() || attrs.is_empty() {
            return true;
        }
        let result = self
            .configs
            .iter()
            .map(|c| super::cfg::active(attrs, c).map(|active| (c, active)))
            .collect::<Result<Vec<_>, _>>();
        match result {
            Ok(values) => {
                self.configs = values
                    .into_iter()
                    .filter(|(_, active)| *active)
                    .map(|(c, _)| c.clone())
                    .collect();
                !self.configs.is_empty()
            }
            Err(e) => {
                self.errors.push(e);
                false
            }
        }
    }
    fn branch(&mut self) {
        self.cyclomatic += 1;
        self.cognitive += 1 + self.depth;
    }
}
impl<'a> Visit<'a> for Complexity {
    fn visit_item(&mut self, _: &'a syn::Item) {}
    fn visit_local(&mut self, local: &'a syn::Local) {
        let saved = self.configs.clone();
        if self.active(&local.attrs) {
            visit::visit_local(self, local);
        }
        self.configs = saved;
    }
    fn visit_expr(&mut self, expr: &'a syn::Expr) {
        let saved = self.configs.clone();
        if !self.active(super::cfg::expr_attributes(expr)) {
            self.configs = saved;
            return;
        }
        match expr {
            syn::Expr::If(e) => {
                self.branch();
                self.visit_expr(&e.cond);
                self.depth += 1;
                self.visit_block(&e.then_branch);
                self.depth -= 1;
                if let Some((_, other)) = &e.else_branch {
                    self.cognitive += 1;
                    if matches!(**other, syn::Expr::If(_)) {
                        self.visit_expr(other);
                    } else {
                        self.depth += 1;
                        self.visit_expr(other);
                        self.depth -= 1;
                    }
                }
            }
            syn::Expr::Match(e) => {
                self.cognitive += 1 + self.depth;
                self.visit_expr(&e.expr);
                self.depth += 1;
                for arm in &e.arms {
                    let arm_configs = self.configs.clone();
                    if !self.active(&arm.attrs) {
                        self.configs = arm_configs;
                        continue;
                    }
                    self.cyclomatic += 1;
                    if let Some((_, guard)) = &arm.guard {
                        self.branch();
                        self.visit_expr(guard);
                    }
                    self.visit_expr(&arm.body);
                    self.configs = arm_configs;
                }
                self.depth -= 1;
            }
            syn::Expr::ForLoop(_) | syn::Expr::While(_) | syn::Expr::Loop(_) => {
                self.branch();
                self.depth += 1;
                visit::visit_expr(self, expr);
                self.depth -= 1;
            }
            syn::Expr::Binary(e) if matches!(e.op, syn::BinOp::And(_) | syn::BinOp::Or(_)) => {
                self.cyclomatic += 1;
                let op = e.op.to_token_stream().to_string();
                let previous = self.boolean.replace(op.clone());
                if previous.as_ref() != Some(&op) {
                    self.cognitive += 1;
                }
                self.visit_expr(&e.left);
                self.visit_expr(&e.right);
                self.boolean = previous;
            }
            syn::Expr::Try(_) => {
                self.cyclomatic += 1;
                visit::visit_expr(self, expr);
            }
            syn::Expr::Break(e) if e.label.is_some() => {
                self.cognitive += 1;
                visit::visit_expr(self, expr);
            }
            syn::Expr::Continue(e) if e.label.is_some() => {
                self.cognitive += 1;
                visit::visit_expr(self, expr);
            }
            syn::Expr::Closure(_) => {
                self.depth += 1;
                visit::visit_expr(self, expr);
                self.depth -= 1;
            }
            _ => visit::visit_expr(self, expr),
        }
        self.configs = saved;
    }
}

pub fn use_paths(tree: &syn::UseTree, prefix: &str, result: &mut BTreeMap<String, String>) {
    match tree {
        syn::UseTree::Path(p) => use_paths(&p.tree, &format!("{prefix}{}::", p.ident), result),
        syn::UseTree::Name(n) => {
            result.insert(n.ident.to_string(), format!("{prefix}{}", n.ident));
        }
        syn::UseTree::Rename(n) => {
            result.insert(n.rename.to_string(), format!("{prefix}{}", n.ident));
        }
        syn::UseTree::Group(g) => {
            for item in &g.items {
                use_paths(item, prefix, result);
            }
        }
        syn::UseTree::Glob(_) => {
            result.insert(format!("{prefix}*"), format!("{prefix}*"));
        }
    }
}

pub const RAW_RECORDS: &[&str] = &[
    "ELEMENT",
    "HELEMENT",
    "STARSHIP",
    "HSTARSHIP",
    "RACE_DESC",
    "RaceDescPtr",
    "DisplayArray",
    "disp_q",
    "race_q",
];
pub const HOOK_FIELDS: &[&str] = &[
    "preprocess_func",
    "postprocess_func",
    "collision_func",
    "death_func",
];

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn effective_loc_excludes_comments_and_docs() {
        let source =
            "/// documentation\nfn run() {\n // comment\n let text = \"// not a comment\";\n}\n";
        let file = syn::parse_file(source).unwrap();
        assert_eq!(effective_lines(&file), [2, 4, 5].into());
    }
    #[test]
    fn nested_branches_have_distinct_cognitive_and_cyclomatic_scores() {
        let block = syn::parse_quote!({
            if a {
                while b {
                    if c && d {
                        run();
                    }
                }
            }
        });
        assert_eq!(Complexity::measure(&block), (5, 7));
    }
}
