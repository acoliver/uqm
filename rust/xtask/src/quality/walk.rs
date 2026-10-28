use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::rc::Rc;

use proc_macro2::{Span, TokenStream, TokenTree};
use quote::ToTokens;
use serde::Deserialize;
use syn::{
    spanned::Spanned,
    visit::{self, Visit},
    Attribute, Item, Meta,
};

use super::{
    cfg::{self, Configuration},
    macros::{self, Definition},
    metrics::{self, Finding, Measurement},
};

#[derive(Deserialize)]
struct Metadata {
    packages: Vec<Package>,
    workspace_members: Vec<String>,
}
#[derive(Deserialize)]
struct Package {
    id: String,
    name: String,
    manifest_path: PathBuf,
    targets: Vec<Target>,
    features: BTreeMap<String, serde_json::Value>,
}
#[derive(Deserialize)]
struct Target {
    name: String,
    kind: Vec<String>,
    src_path: PathBuf,
}

#[derive(Clone, Default)]
struct Context {
    file: String,
    path: PathBuf,
    directory: PathBuf,
    module: String,
    lexical_module: String,
    package: String,
    subsystem: String,
    configs: Vec<Rc<Configuration>>,
    imports: BTreeMap<String, String>,
    definitions: BTreeMap<String, Rc<Definition>>,
    hooks: BTreeSet<String>,
    allows: Vec<String>,
    inline_depth: usize,
}

pub struct Walker {
    rust: PathBuf,
    context: Context,
    measurement: Measurement,
    errors: Vec<String>,
    visited: BTreeSet<PathBuf>,
    events: BTreeSet<String>,
    file_tokens: BTreeMap<String, TokenStream>,
    file_metadata: BTreeMap<String, (String, String, String)>,
    line_metrics: Vec<(usize, TokenStream, bool)>,
    excluded: BTreeMap<String, Vec<Span>>,
    allow_lines: BTreeMap<String, BTreeMap<String, BTreeSet<usize>>>,
    allow_items: Vec<(String, String, TokenStream, Vec<String>)>,
    anonymous: BTreeMap<String, usize>,
    names: Option<BTreeSet<String>>,
    defs_only: bool,
    generated: bool,
    queue_functions: BTreeSet<String>,
    handlers: Vec<(String, String)>,
    layer_declarations: Vec<(Finding, String)>,
    layer_definitions: BTreeMap<String, BTreeSet<String>>,
}

pub fn measure(
    root: &Path,
    names: Option<&[String]>,
    defs_only: bool,
) -> Result<Measurement, String> {
    let rust = root.join("rust");
    let output = Command::new(super::cargo())
        .current_dir(&rust)
        .args(["metadata", "--no-deps", "--locked", "--format-version", "1"])
        .output()
        .map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err(format!(
            "cargo metadata: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    let metadata: Metadata = serde_json::from_slice(&output.stdout).map_err(|e| e.to_string())?;
    let features = metadata
        .packages
        .iter()
        .flat_map(|p| p.features.keys().cloned())
        .collect();
    let configs: Vec<_> = Configuration::ci(&features)?
        .into_iter()
        .map(Rc::new)
        .collect();
    let handlers = if root.join(".git").exists() {
        macros::HANDLERS
            .iter()
            .map(|(f, n)| (f.to_string(), n.to_string()))
            .collect()
    } else {
        macros::archived_registry(&rust)?
    };
    let mut walker = Walker::new(rust, names, defs_only);
    walker.handlers = handlers;
    for package in metadata
        .packages
        .iter()
        .filter(|p| metadata.workspace_members.contains(&p.id))
    {
        walker.context = Context {
            package: package.name.clone(),
            module: package.name.clone(),
            configs: configs.clone(),
            ..Context::default()
        };
        let package_root = package
            .manifest_path
            .parent()
            .ok_or("manifest has no parent")?;
        let mut targets: Vec<_> = package.targets.iter().collect();
        targets.sort_by_key(|t| {
            (
                !t.kind.iter().any(|k| k == "lib" || k == "rlib"),
                t.name.clone(),
            )
        });
        for target in targets {
            if names.is_none() && target.kind.iter().any(|k| k == "test" || k == "bench") {
                continue;
            }
            walker.context.module = if target.kind.iter().any(|k| k == "lib" || k == "rlib") {
                package.name.clone()
            } else {
                format!("{}::target::{}", package.name, target.name)
            };
            walker.context.lexical_module = walker.context.module.clone();
            walker.context.subsystem = if package.name == "uqm" {
                target.name.clone()
            } else {
                package.name.clone()
            };
            walker.load(&target.src_path, Some(package_root))?;
        }
    }
    let mut measurement = walker.finish(names.is_none())?;
    if names.is_none() {
        super::layers::graph(root, &mut measurement)?;
    }
    Ok(measurement)
}

impl Walker {
    fn new(rust: PathBuf, names: Option<&[String]>, defs_only: bool) -> Self {
        let rust = rust.canonicalize().expect("quality source root exists");
        let queue_functions = fs::read_to_string(rust.join("src/collections/queue.rs"))
            .ok()
            .and_then(|text| syn::parse_file(&text).ok())
            .map(|f| {
                f.items
                    .into_iter()
                    .filter_map(|i| match i {
                        Item::Fn(f) => Some(f.sig.ident.to_string()),
                        _ => None,
                    })
                    .collect()
            })
            .unwrap_or_default();
        Self {
            rust,
            context: Context::default(),
            measurement: Measurement::default(),
            errors: Vec::new(),
            visited: BTreeSet::new(),
            events: BTreeSet::new(),
            file_tokens: BTreeMap::new(),
            file_metadata: BTreeMap::new(),
            line_metrics: Vec::new(),
            excluded: BTreeMap::new(),
            allow_lines: BTreeMap::new(),
            allow_items: Vec::new(),
            anonymous: BTreeMap::new(),
            names: names.map(|n| n.iter().cloned().collect()),
            defs_only,
            generated: false,
            queue_functions,
            layer_declarations: Vec::new(),
            layer_definitions: BTreeMap::new(),
            handlers: macros::HANDLERS
                .iter()
                .map(|(f, n)| (f.to_string(), n.to_string()))
                .collect(),
        }
    }

    fn load(&mut self, path: &Path, package_root: Option<&Path>) -> Result<(), String> {
        let path = path
            .canonicalize()
            .map_err(|e| format!("{}: {e}", path.display()))?;
        if !self.visited.insert(path.clone()) {
            return Ok(());
        }
        let text = fs::read_to_string(&path).map_err(|e| e.to_string())?;
        let file = syn::parse_file(&text).map_err(|e| format!("{}: {e}", path.display()))?;
        let saved = self.context.clone();
        self.context.file = path
            .strip_prefix(&self.rust)
            .map_err(|e| e.to_string())?
            .to_string_lossy()
            .replace('\\', "/");
        self.context.path = path.clone();
        self.context.lexical_module = self.context.module.clone();
        self.context.inline_depth = 0;
        let parent = path.parent().ok_or("source has no parent")?;
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or("source has no stem")?;
        self.context.directory = if package_root.is_some() || matches!(stem, "lib" | "main" | "mod")
        {
            parent.into()
        } else {
            parent.join(stem)
        };
        self.context.imports = BTreeMap::new();
        for item in &file.items {
            if let Item::Use(u) = item {
                metrics::use_paths(&u.tree, "", &mut self.context.imports);
            }
        }
        self.file_tokens
            .insert(self.context.file.clone(), file.to_token_stream());
        self.file_metadata.insert(
            self.context.file.clone(),
            (
                self.context.package.clone(),
                self.context.subsystem.clone(),
                self.context.module.clone(),
            ),
        );
        struct Docs(Vec<Span>);
        impl<'a> Visit<'a> for Docs {
            fn visit_attribute(&mut self, a: &'a Attribute) {
                if a.path().is_ident("doc") {
                    self.0.push(a.span());
                }
            }
        }
        let mut docs = Docs(Vec::new());
        docs.visit_file(&file);
        self.excluded
            .entry(self.context.file.clone())
            .or_default()
            .extend(docs.0);
        self.visit_file(&file);
        self.context = saved;
        Ok(())
    }

    fn finish(mut self, require_handlers: bool) -> Result<Measurement, String> {
        if require_handlers {
            for (file, name) in &self.handlers {
                if !self
                    .measurement
                    .handlers
                    .contains(&(file.to_string(), name.to_string()))
                {
                    self.errors
                        .push(format!("stale quality macro handler: {file}::{name}"));
                }
            }
        }
        if !self.errors.is_empty() {
            return Err(self.errors.join("\n"));
        }
        let excluded_by_file: BTreeMap<_, _> = self
            .excluded
            .iter()
            .map(|(file, spans)| (file.clone(), metrics::Excluded::new(spans)))
            .collect();
        let empty = metrics::Excluded::default();
        let mut active_lines = BTreeMap::new();
        for (file, tokens) in &self.file_tokens {
            let excluded = excluded_by_file.get(file).unwrap_or(&empty);
            let lines = metrics::filtered_lines(tokens.clone(), excluded);
            let (package, subsystem, module) = &self.file_metadata[file];
            self.measurement.findings.push(Finding {
                counter: "loc".into(),
                metric: "loc".into(),
                identity: format!("{module}::@file"),
                file: file.clone(),
                package: package.clone(),
                subsystem: subsystem.clone(),
                value: lines.len() as u64,
            });
            active_lines.insert(file.clone(), lines);
        }
        let mut unsafe_lines = BTreeMap::<String, (Finding, BTreeSet<usize>)>::new();
        for (index, tokens, generated) in &self.line_metrics {
            let f = &mut self.measurement.findings[*index];
            let excluded = if *generated {
                &empty
            } else {
                excluded_by_file.get(&f.file).unwrap_or(&empty)
            };
            let lines = metrics::filtered_lines(tokens.clone(), excluded);
            f.value = lines.len() as u64;
            if f.counter == "unsafe" {
                let row = unsafe_lines.entry(f.file.clone()).or_insert_with(|| {
                    let mut row = f.clone();
                    row.metric = "unsafe_lines".into();
                    row.identity = self.file_metadata[&f.file].2.clone();
                    (row, BTreeSet::new())
                });
                row.1.extend(lines);
            }
        }
        for (_, (mut f, lines)) in unsafe_lines {
            f.value = lines.len() as u64;
            self.measurement.findings.push(f);
        }
        self.finish_allow_items(&excluded_by_file);
        self.finish_allows(&active_lines);
        self.measurement.findings.extend(super::layers::symbols(
            &self.layer_declarations,
            &self.layer_definitions,
        ));
        super::layers::initialize_reports(&mut self.measurement);
        Ok(self.measurement)
    }

    fn anonymous_identity(&mut self, kind: &str, legacy: &str) {
        let parent = self.context.module.clone();
        let ordinal = self
            .anonymous
            .entry(format!("{parent}::{kind}"))
            .or_default();
        *ordinal += 1;
        let identity = format!("{parent}::{kind}#{ordinal}");
        let old = self
            .measurement
            .identity_renames
            .iter()
            .filter(|(_, target)| super::layers::prefix(&parent, target))
            .max_by_key(|(_, target)| target.len())
            .map_or_else(
                || parent.clone(),
                |(source, target)| format!("{source}{}", &parent[target.len()..]),
            );
        self.measurement
            .identity_renames
            .insert(format!("{old}::{legacy}"), identity.clone());
        self.context.module = identity;
    }

    fn error(&mut self, error: String) {
        self.errors.push(format!("{}: {error}", self.context.file));
    }
    fn add(&mut self, counter: &str, metric: &str, span: Span, value: u64) {
        if self.names.is_some() {
            return;
        }
        let position = span.start();
        let event = format!(
            "{}|{}|{}|{}:{}|{}",
            self.context.file, counter, metric, position.line, position.column, self.context.module
        );
        if self.events.insert(event) {
            self.measurement.findings.push(self.finding(
                counter,
                metric,
                self.context.module.clone(),
                value,
            ));
        }
    }
    fn line_metric(&mut self, counter: &str, metric: &str, span: Span, tokens: TokenStream) {
        let previous = self.measurement.findings.len();
        self.add(counter, metric, span, 0);
        if self.measurement.findings.len() != previous {
            self.line_metrics.push((previous, tokens, self.generated));
        }
    }
    fn finding(&self, counter: &str, metric: &str, identity: String, value: u64) -> Finding {
        Finding {
            counter: counter.into(),
            metric: metric.into(),
            identity,
            file: self.context.file.clone(),
            package: self.context.package.clone(),
            subsystem: self.context.subsystem.clone(),
            value,
        }
    }

    fn definition(&mut self, name: &syn::Ident) {
        if self.defs_only {
            self.identifier(name);
        }
    }
    fn identifier(&mut self, name: &syn::Ident) {
        if !self
            .names
            .as_ref()
            .is_some_and(|names| names.contains(&name.to_string()))
        {
            return;
        }
        let pos = name.span().start();
        let event = format!(
            "ident|{}|{}:{}|{}|{}",
            self.context.file, pos.line, pos.column, name, self.context.module
        );
        if self.events.insert(event) {
            self.measurement.identifiers.push(self.finding(
                "ident",
                &name.to_string(),
                self.context.module.clone(),
                1,
            ));
        }
    }

    fn enabled(&mut self, attrs: &[Attribute], span: Span) -> bool {
        let evaluated = self
            .context
            .configs
            .iter()
            .filter_map(|c| match cfg::active(attrs, c) {
                Ok(true) => Some(Ok(c.clone())),
                Ok(false) => None,
                Err(e) => Some(Err(e)),
            })
            .collect::<Result<Vec<_>, _>>();
        match evaluated {
            Err(e) => {
                self.error(e);
                false
            }
            Ok(configs) if self.names.is_none() => {
                if configs.is_empty() {
                    self.excluded
                        .entry(self.context.file.clone())
                        .or_default()
                        .push(span);
                    false
                } else {
                    self.context.configs = configs;
                    true
                }
            }
            Ok(_) => true,
        }
    }

    fn attrs(&mut self, attrs: &[Attribute], tokens: TokenStream) {
        if self.names.is_some() {
            return;
        }
        let mut metas = BTreeMap::new();
        for config in &self.context.configs {
            match cfg::attributes(attrs, config) {
                Ok(expanded) => {
                    for meta in expanded {
                        metas.insert(meta.to_token_stream().to_string(), meta);
                    }
                }
                Err(e) => {
                    self.errors.push(e);
                }
            }
        }
        for meta in metas.values() {
            let path = meta.path();
            if path.is_ident("allow") || path.is_ident("expect") {
                if let Meta::List(list) = meta {
                    let parser =
                        syn::punctuated::Punctuated::<Meta, syn::Token![,]>::parse_terminated;
                    match list.parse_args_with(parser) {
                        Ok(lints) => {
                            for lint in lints {
                                if matches!(lint, Meta::NameValue(_)) {
                                    continue;
                                }
                                let lint = lint.to_token_stream().to_string().replace(' ', "");
                                let identity = self.context.module.clone();
                                let f = self.finding("allows", &lint, identity, 0);
                                let key = f.key();
                                if !self.allow_lines.contains_key(&key) {
                                    self.measurement.findings.push(f);
                                }
                                self.allow_lines.entry(key.clone()).or_default();
                                self.context.allows.push(key);
                            }
                        }
                        Err(e) => self.error(e.to_string()),
                    }
                }
            }
            for (name, metric) in [
                ("no_mangle", "no_mangle"),
                ("export_name", "export_name"),
                ("link_name", "link_name"),
            ] {
                if path.is_ident(name) {
                    self.add("ffi", metric, meta.span(), 1);
                }
            }
            if path.is_ident("unsafe") {
                if let Meta::List(list) = meta {
                    if let Ok(nested) = syn::parse2::<Meta>(list.tokens.clone()) {
                        if nested.path().is_ident("no_mangle") {
                            self.add("ffi", "no_mangle", meta.span(), 1);
                        }
                        if nested.path().is_ident("export_name") {
                            self.add("ffi", "export_name", meta.span(), 1);
                        }
                    }
                }
            }
            if path.is_ident("repr") {
                if let Meta::List(list) = meta {
                    if list
                        .tokens
                        .clone()
                        .into_iter()
                        .any(|t| matches!(t, TokenTree::Ident(i) if i == "C"))
                    {
                        self.add("ffi", "repr_c", meta.span(), 1);
                    }
                }
            }
        }
        let lines = metrics::lines(tokens);
        for key in &self.context.allows {
            self.allow_lines
                .entry(key.clone())
                .or_default()
                .entry(self.context.file.clone())
                .or_default()
                .extend(&lines);
        }
    }

    fn function(&mut self, signature: &syn::Signature, block: &syn::Block) {
        self.definition(&signature.ident);
        self.context.hooks.clear();
        if self.names.is_some() {
            return;
        }
        let (cyclomatic, cognitive) =
            match metrics::Complexity::measure_configured(block, &self.context.configs) {
                Ok(values) => values,
                Err(error) => {
                    self.error(error);
                    return;
                }
            };
        self.line_metric(
            "complexity",
            "loc",
            signature.span(),
            block.to_token_stream(),
        );
        self.add("complexity", "cyclomatic", signature.span(), cyclomatic);
        self.add("complexity", "cognitive", signature.span(), cognitive);
        if signature.unsafety.is_some() {
            self.add("unsafe", "unsafe_fn", signature.span(), 1);
            self.line_metric(
                "unsafe",
                "unsafe_fn_lines",
                signature.span(),
                block.to_token_stream(),
            );
        }
        if signature
            .abi
            .as_ref()
            .is_some_and(|a| a.name.as_ref().is_none_or(|n| n.value() == "C"))
        {
            self.add("ffi", "extern_c_fn", signature.span(), 1);
        }
        for input in &signature.inputs {
            if let syn::FnArg::Typed(arg) = input {
                if matches!(*arg.ty, syn::Type::Ptr(_)) {
                    self.add("ffi", "raw_pointer_parameters", arg.span(), 1);
                }
            }
        }
    }

    fn expand(&mut self, mac: &syn::Macro) {
        let name = mac
            .path
            .segments
            .last()
            .map(|s| s.ident.to_string())
            .unwrap_or_default();
        if name == "include" {
            let result = self.include_path(mac.tokens.clone()).and_then(|literal| {
                let path = self
                    .context
                    .path
                    .parent()
                    .ok_or("include has no parent")?
                    .join(&literal);
                if literal.contains("abi/") {
                    self.add("ffi", "abi_includes", mac.span(), 1);
                }
                self.load(&path, None)
            });
            if let Err(e) = result {
                self.error(e);
            }
        } else if let Some(definition) = self.context.definitions.get(&name).cloned() {
            let expanded = definition
                .expand(mac.tokens.clone())
                .and_then(|t| syn::parse2::<syn::File>(t).map_err(|e| e.to_string()));
            match expanded {
                Ok(file) => {
                    let previous = self.generated;
                    let module = self.context.module.clone();
                    self.macro_identity(&file, mac);
                    self.generated = true;
                    for item in &file.items {
                        self.visit_item(item);
                    }
                    self.generated = previous;
                    self.context.module = module;
                }
                Err(e) => self.error(format!("{name}!: {e}")),
            }
        } else if name == "assert_c_layout" {
            // This exported macro is invoked with a path, outside its defining module.
            let path = self.rust.join("src/c_bindings.rs");
            let result = fs::read_to_string(path)
                .map_err(|e| e.to_string())
                .and_then(|s| syn::parse_file(&s).map_err(|e| e.to_string()))
                .and_then(|f| {
                    f.items
                        .into_iter()
                        .find_map(|item| match item {
                            Item::Macro(m)
                                if m.ident.as_ref().is_some_and(|n| n == "assert_c_layout") =>
                            {
                                Some(m.mac.tokens)
                            }
                            _ => None,
                        })
                        .ok_or("assert_c_layout macro missing".into())
                })
                .and_then(Definition::parse);
            match result {
                Ok(d) => {
                    self.context.definitions.insert(name, Rc::new(d));
                    self.expand(mac);
                }
                Err(e) => self.error(e),
            }
        }
    }

    fn macro_identity(&mut self, file: &syn::File, mac: &syn::Macro) {
        if file
            .items
            .iter()
            .any(|i| matches!(i, Item::Const(c) if c.ident == "_"))
        {
            self.anonymous_identity(
                "macro",
                &format!(
                    "macro@{}:{}",
                    mac.span().start().line,
                    mac.span().start().column
                ),
            );
        }
    }

    fn include_path(&self, tokens: TokenStream) -> Result<String, String> {
        let expr = syn::parse2::<syn::Expr>(tokens).map_err(|e| e.to_string())?;
        match expr {
            syn::Expr::Lit(lit) => match lit.lit {
                syn::Lit::Str(s) => Ok(s.value()),
                _ => Err("include requires a string".into()),
            },
            syn::Expr::Macro(m) if m.mac.path.is_ident("concat") => {
                use syn::parse::Parser;
                let args =
                    syn::punctuated::Punctuated::<syn::Expr, syn::Token![,]>::parse_terminated
                        .parse2(m.mac.tokens)
                        .map_err(|e| e.to_string())?;
                args.iter()
                    .map(|arg| self.include_path(arg.to_token_stream()))
                    .collect::<Result<Vec<_>, _>>()
                    .map(|parts| parts.concat())
            }
            syn::Expr::Macro(m) if m.mac.path.is_ident("env") => {
                let key = syn::parse2::<syn::LitStr>(m.mac.tokens).map_err(|e| e.to_string())?;
                if key.value() != "CARGO_MANIFEST_DIR" {
                    return Err(format!("unsupported include env {}", key.value()));
                }
                let manifest_dir = if self.context.package == "uqm-xtask" {
                    self.rust.join("xtask")
                } else {
                    self.rust.clone()
                };
                Ok(manifest_dir.to_string_lossy().into())
            }
            _ => Err("unsupported include path expression".into()),
        }
    }

    fn resolve(&self, path: &syn::Path) -> String {
        let text = path
            .segments
            .iter()
            .map(|s| s.ident.to_string())
            .collect::<Vec<_>>()
            .join("::");
        let head = text.split("::").next().unwrap_or_default();
        self.context
            .imports
            .get(head)
            .map(|alias| format!("{alias}{}", &text[head.len()..]))
            .unwrap_or(text)
    }
    fn layer_path(&mut self, target: &str, span: Span, direct_bridge: bool) {
        let target = super::layers::absolute(&self.context.lexical_module, target);
        let target = target.as_str();
        let event = format!(
            "layer|{}|{}:{}|{target}",
            self.context.file,
            span.start().line,
            span.start().column
        );
        if self.names.is_none() && self.events.insert(event) {
            let source = self.finding("layers", "", self.context.module.clone(), 1);
            self.measurement
                .findings
                .extend(super::layers::reference_with_bridge(
                    &source,
                    target,
                    direct_bridge,
                ));
        }
    }
    fn exported_symbol(&mut self, attrs: &[Attribute], ident: &syn::Ident) {
        let exported = attrs.iter().any(|a| a.path().is_ident("no_mangle"));
        let symbol =
            symbol_name(attrs, "export_name").or_else(|| exported.then(|| ident.to_string()));
        if let Some(symbol) = symbol {
            if let Some(module) = self.context.module.strip_prefix("uqm::") {
                self.layer_definitions
                    .entry(symbol)
                    .or_default()
                    .insert(format!("crate::{module}"));
            }
        }
    }
    fn hook(&self, expr: &syn::Expr) -> bool {
        match expr {
            syn::Expr::Field(f) => {
                matches!(&f.member, syn::Member::Named(n) if metrics::HOOK_FIELDS.contains(&n.to_string().as_str()))
            }
            syn::Expr::MethodCall(m) => self.hook(&m.receiver),
            syn::Expr::Paren(p) => self.hook(&p.expr),
            syn::Expr::Path(p) => p
                .path
                .get_ident()
                .is_some_and(|i| self.context.hooks.contains(&i.to_string())),
            _ => false,
        }
    }
}

mod allowances;
#[cfg(test)]
mod tests;
mod visitor;

fn symbol_name(attrs: &[Attribute], key: &str) -> Option<String> {
    attrs.iter().find_map(|a| {
        if a.path().is_ident(key) {
            if let Meta::NameValue(v) = &a.meta {
                if let syn::Expr::Lit(l) = &v.value {
                    if let syn::Lit::Str(s) = &l.lit {
                        return Some(s.value());
                    }
                }
            }
        }
        None
    })
}
