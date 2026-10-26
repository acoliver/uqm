use super::*;

fn item_attrs(item: &Item) -> &[Attribute] {
    match item {
        Item::Const(i) => &i.attrs,
        Item::Enum(i) => &i.attrs,
        Item::ExternCrate(i) => &i.attrs,
        Item::Fn(i) => &i.attrs,
        Item::ForeignMod(i) => &i.attrs,
        Item::Impl(i) => &i.attrs,
        Item::Macro(i) => &i.attrs,
        Item::Mod(i) => &i.attrs,
        Item::Static(i) => &i.attrs,
        Item::Struct(i) => &i.attrs,
        Item::Trait(i) => &i.attrs,
        Item::TraitAlias(i) => &i.attrs,
        Item::Type(i) => &i.attrs,
        Item::Union(i) => &i.attrs,
        Item::Use(i) => &i.attrs,
        _ => &[],
    }
}

fn item_name(item: &Item) -> Option<String> {
    Some(match item {
        Item::Const(i) => i.ident.to_string(),
        Item::Enum(i) => i.ident.to_string(),
        Item::Fn(i) => i.sig.ident.to_string(),
        Item::Mod(i) => i.ident.to_string(),
        Item::Static(i) => i.ident.to_string(),
        Item::Struct(i) => i.ident.to_string(),
        Item::Trait(i) => i.ident.to_string(),
        Item::TraitAlias(i) => i.ident.to_string(),
        Item::Type(i) => i.ident.to_string(),
        Item::Union(i) => i.ident.to_string(),
        Item::Impl(i) => format!(
            "<{}{}>",
            i.self_ty.to_token_stream().to_string().replace(' ', ""),
            i.trait_
                .as_ref()
                .map(|(_, t, _)| format!(
                    " as {}",
                    t.to_token_stream().to_string().replace(' ', "")
                ))
                .unwrap_or_default()
        ),
        _ => return None,
    })
}

impl<'a> Visit<'a> for Walker {
    fn visit_file(&mut self, file: &'a syn::File) {
        if file.attrs.iter().any(|a| {
            a.path().is_ident("cfg")
                && a.meta
                    .to_token_stream()
                    .to_string()
                    .replace(' ', "")
                    .contains("not(test)")
        }) {
            self.add("allows", "cfg_not_test_modules", file.span(), 1);
        }
        let saved = self.context.clone();
        if self.enabled(&file.attrs, file.span()) {
            self.attrs(&file.attrs, file.to_token_stream());
            visit::visit_file(self, file);
        }
        self.context = saved;
    }
    fn visit_item(&mut self, item: &'a Item) {
        let saved = self.context.clone();
        if !self.enabled(item_attrs(item), item.span()) {
            self.context = saved;
            return;
        }
        if let Some(name) = item_name(item) {
            self.context.module = format!("{}::{name}", self.context.module);
            if name == "_" {
                self.context.module.push_str(&format!(
                    "@{}:{}",
                    item.span().start().line,
                    item.span().start().column
                ));
            }
        }
        self.attrs(item_attrs(item), item.to_token_stream());
        match item {
            Item::Enum(i) => self.definition(&i.ident),
            Item::Struct(i) => self.definition(&i.ident),
            Item::Union(i) => self.definition(&i.ident),
            Item::Type(i) => self.definition(&i.ident),
            Item::Trait(i) => self.definition(&i.ident),
            Item::TraitAlias(i) => self.definition(&i.ident),
            Item::Const(i) => self.definition(&i.ident),
            Item::Static(i) => self.definition(&i.ident),
            Item::Mod(i) => self.definition(&i.ident),
            _ => {}
        }
        visit::visit_item(self, item);
        // Macro definitions and use aliases have lexical scope, unlike item names.
        let definitions = self.context.definitions.clone();
        let imports = self.context.imports.clone();
        self.context = saved;
        if matches!(item, Item::Macro(m) if m.ident.is_some()) {
            self.context.definitions = definitions;
        }
        if matches!(item, Item::Use(_)) {
            self.context.imports = imports;
        }
    }
    fn visit_item_mod(&mut self, item: &'a syn::ItemMod) {
        if item.attrs.iter().any(|a| {
            a.path().is_ident("cfg")
                && a.meta
                    .to_token_stream()
                    .to_string()
                    .replace(' ', "")
                    .contains("not(test)")
        }) {
            self.add("allows", "cfg_not_test_modules", item.span(), 1);
        }
        if self.context.package == "uqm" && self.context.module.split("::").count() == 2 {
            self.context.subsystem = item.ident.to_string();
        }
        if let Some((_, items)) = &item.content {
            let directory = self.context.directory.clone();
            self.context.inline_depth += 1;
            self.context.directory.push(item.ident.to_string());
            for item in items {
                self.visit_item(item);
            }
            self.context.directory = directory;
        } else {
            let explicit = item.attrs.iter().find_map(|a| {
                if a.path().is_ident("path") {
                    match &a.meta {
                        Meta::NameValue(v) => match &v.value {
                            syn::Expr::Lit(l) => match &l.lit {
                                syn::Lit::Str(s) => Some(s.value()),
                                _ => None,
                            },
                            _ => None,
                        },
                        _ => None,
                    }
                } else {
                    None
                }
            });
            let path = if let Some(path) = explicit {
                if self.context.inline_depth == 0 {
                    self.context.path.parent().unwrap().join(path)
                } else {
                    self.context.directory.join(path)
                }
            } else {
                let direct = self.context.directory.join(format!("{}.rs", item.ident));
                let nested = self
                    .context
                    .directory
                    .join(item.ident.to_string())
                    .join("mod.rs");
                if direct.exists() && nested.exists() {
                    self.error(format!("ambiguous module {}", item.ident));
                    return;
                }
                if direct.exists() {
                    direct
                } else {
                    nested
                }
            };
            if let Err(e) = self.load(&path, None) {
                self.error(e);
            }
        }
        if self.names.is_some() && !self.defs_only {
            self.identifier(&item.ident);
        }
    }
    fn visit_item_fn(&mut self, item: &'a syn::ItemFn) {
        visit::visit_item_fn(self, item);
        self.function(&item.sig, &item.block);
    }
    fn visit_impl_item(&mut self, item: &'a syn::ImplItem) {
        let saved = self.context.clone();
        let attrs = match item {
            syn::ImplItem::Fn(i) => &i.attrs,
            syn::ImplItem::Const(i) => &i.attrs,
            syn::ImplItem::Type(i) => &i.attrs,
            syn::ImplItem::Macro(i) => &i.attrs,
            _ => {
                visit::visit_impl_item(self, item);
                return;
            }
        };
        if self.enabled(attrs, item.span()) {
            let ident = match item {
                syn::ImplItem::Fn(i) => Some(&i.sig.ident),
                syn::ImplItem::Const(i) => Some(&i.ident),
                syn::ImplItem::Type(i) => Some(&i.ident),
                _ => None,
            };
            if let Some(ident) = ident {
                self.context.module.push_str(&format!("::{ident}"));
            }
            self.attrs(attrs, item.to_token_stream());
            match item {
                syn::ImplItem::Fn(i) => self.function(&i.sig, &i.block),
                syn::ImplItem::Const(i) => self.definition(&i.ident),
                syn::ImplItem::Type(i) => self.definition(&i.ident),
                _ => {}
            }
            visit::visit_impl_item(self, item);
        }
        self.context = saved;
    }
    fn visit_trait_item(&mut self, item: &'a syn::TraitItem) {
        let saved = self.context.clone();
        let attrs = match item {
            syn::TraitItem::Fn(i) => &i.attrs,
            syn::TraitItem::Const(i) => &i.attrs,
            syn::TraitItem::Type(i) => &i.attrs,
            syn::TraitItem::Macro(i) => &i.attrs,
            _ => {
                visit::visit_trait_item(self, item);
                return;
            }
        };
        if self.enabled(attrs, item.span()) {
            self.attrs(attrs, item.to_token_stream());
            match item {
                syn::TraitItem::Fn(i) => {
                    self.context.module.push_str(&format!("::{}", i.sig.ident));
                    if let Some(block) = &i.default {
                        self.function(&i.sig, block);
                    }
                }
                syn::TraitItem::Const(i) if i.default.is_some() => self.definition(&i.ident),
                syn::TraitItem::Type(i) if i.default.is_some() => self.definition(&i.ident),
                _ => {}
            }
            visit::visit_trait_item(self, item);
        }
        self.context = saved;
    }
    fn visit_item_static(&mut self, item: &'a syn::ItemStatic) {
        if matches!(item.mutability, syn::StaticMutability::Mut(_)) {
            self.add("ffi", "static_mut", item.span(), 1);
        }
        visit::visit_item_static(self, item);
    }
    fn visit_item_impl(&mut self, item: &'a syn::ItemImpl) {
        if item.unsafety.is_some() {
            self.add("unsafe", "unsafe_impl", item.span(), 1);
            self.line_metric(
                "unsafe",
                "unsafe_impl_lines",
                item.span(),
                item.to_token_stream(),
            );
        }
        visit::visit_item_impl(self, item);
    }
    fn visit_foreign_item(&mut self, item: &'a syn::ForeignItem) {
        let saved = self.context.clone();
        let attrs: &[Attribute] = match item {
            syn::ForeignItem::Fn(i) => &i.attrs,
            syn::ForeignItem::Static(i) => &i.attrs,
            syn::ForeignItem::Type(i) => &i.attrs,
            syn::ForeignItem::Macro(i) => &i.attrs,
            _ => &[],
        };
        if self.enabled(attrs, item.span()) {
            self.attrs(attrs, item.to_token_stream());
            self.add("ffi", "foreign_items", item.span(), 1);
            visit::visit_foreign_item(self, item);
        }
        self.context = saved;
    }
    fn visit_variant(&mut self, variant: &'a syn::Variant) {
        let saved = self.context.clone();
        if self.enabled(&variant.attrs, variant.span()) {
            self.context
                .module
                .push_str(&format!("::{}", variant.ident));
            self.attrs(&variant.attrs, variant.to_token_stream());
            visit::visit_variant(self, variant);
        }
        self.context = saved;
    }
    fn visit_arm(&mut self, arm: &'a syn::Arm) {
        let saved = self.context.clone();
        if self.enabled(&arm.attrs, arm.span()) {
            self.attrs(&arm.attrs, arm.to_token_stream());
            visit::visit_arm(self, arm);
        }
        self.context = saved;
    }
    fn visit_stmt_macro(&mut self, stmt: &'a syn::StmtMacro) {
        let saved = self.context.clone();
        if self.enabled(&stmt.attrs, stmt.span()) {
            self.attrs(&stmt.attrs, stmt.to_token_stream());
            let name = stmt
                .mac
                .path
                .segments
                .last()
                .map(|s| s.ident.to_string())
                .unwrap_or_default();
            if self.context.definitions.contains_key(&name)
                || name == "include"
                || name == "assert_c_layout"
            {
                self.expand(&stmt.mac);
            } else {
                visit::visit_stmt_macro(self, stmt);
            }
        }
        self.context = saved;
    }
    fn visit_field(&mut self, field: &'a syn::Field) {
        let saved = self.context.clone();
        if self.enabled(&field.attrs, field.span()) {
            if let Some(name) = &field.ident {
                self.context.module.push_str(&format!("::{name}"));
                self.definition(name);
            }
            self.attrs(&field.attrs, field.to_token_stream());
            visit::visit_field(self, field);
        }
        self.context = saved;
    }
    fn visit_item_use(&mut self, item: &'a syn::ItemUse) {
        metrics::use_paths(&item.tree, "", &mut self.context.imports);
        // Pure re-exports carry no raw-record debt.
        if (self.names.is_some() && !self.defs_only)
            || (self.names.is_none() && matches!(item.vis, syn::Visibility::Inherited))
        {
            visit::visit_item_use(self, item);
        }
    }
    fn visit_item_macro(&mut self, item: &'a syn::ItemMacro) {
        if let Some(name) = &item.ident {
            self.definition(name);
            let parsed = Definition::parse(item.mac.tokens.clone());
            match parsed {
                Ok(definition) => {
                    let file = self
                        .context
                        .file
                        .strip_prefix("src/")
                        .map(|f| format!("src/{f}"))
                        .unwrap_or_else(|| self.context.file.clone());
                    let registered = self
                        .handlers
                        .iter()
                        .any(|(f, n)| f == &file && n == &name.to_string());
                    if registered {
                        self.measurement.handlers.insert((file, name.to_string()));
                        self.context
                            .definitions
                            .insert(name.to_string(), Rc::new(definition));
                    } else if definition.generates_items() {
                        self.error(format!("unsupported item-generating macro {name}!"));
                    }
                }
                Err(e) => self.error(format!("macro {name}!: {e}")),
            }
            if self.names.is_some() && !self.defs_only {
                self.identifier(name);
                self.tokens(item.mac.tokens.clone());
            }
        } else if self.names.is_none() || self.defs_only {
            self.expand(&item.mac);
        } else {
            visit::visit_item_macro(self, item);
            if item.mac.path.is_ident("include") {
                self.expand(&item.mac);
            }
        }
    }
    fn visit_macro(&mut self, mac: &'a syn::Macro) {
        if self.names.is_some() {
            visit::visit_macro(self, mac);
            return;
        }
        use syn::parse::Parser;
        if let Ok(expressions) =
            syn::punctuated::Punctuated::<syn::Expr, syn::Token![,]>::parse_terminated
                .parse2(mac.tokens.clone())
        {
            for expr in &expressions {
                self.visit_expr(expr);
            }
        } else {
            self.tokens(mac.tokens.clone());
        }
    }
    fn visit_block(&mut self, block: &'a syn::Block) {
        let hooks = self.context.hooks.clone();
        visit::visit_block(self, block);
        self.context.hooks = hooks;
    }
    fn visit_expr(&mut self, expr: &'a syn::Expr) {
        // Expressions can carry cfg, including the platform-specific block in a function.
        let attrs = cfg::expr_attributes(expr);
        if attrs.is_empty() {
            visit::visit_expr(self, expr);
            return;
        }
        let saved = self.context.configs.clone();
        if self.enabled(attrs, expr.span()) {
            self.attrs(attrs, TokenStream::new());
            visit::visit_expr(self, expr);
        }
        self.context.configs = saved;
    }
    fn visit_expr_unsafe(&mut self, expr: &'a syn::ExprUnsafe) {
        self.add("unsafe", "unsafe_blocks", expr.span(), 1);
        self.line_metric(
            "unsafe",
            "unsafe_block_lines",
            expr.span(),
            expr.block.to_token_stream(),
        );
        visit::visit_expr_unsafe(self, expr);
    }
    fn visit_local(&mut self, local: &'a syn::Local) {
        let saved = self.context.configs.clone();
        if self.enabled(&local.attrs, local.span()) {
            self.attrs(&local.attrs, local.to_token_stream());
            if let Some(init) = &local.init {
                if self.hook(&init.expr) {
                    self.bind_hook(&local.pat);
                }
            }
            visit::visit_local(self, local);
        }
        self.context.configs = saved;
    }
    fn visit_expr_let(&mut self, expr: &'a syn::ExprLet) {
        if self.hook(&expr.expr) {
            self.bind_hook(&expr.pat);
        }
        visit::visit_expr_let(self, expr);
    }
    fn visit_expr_call(&mut self, expr: &'a syn::ExprCall) {
        if self.hook(&expr.func)
            && !self
                .context
                .module
                .rsplit("::")
                .next()
                .unwrap_or_default()
                .starts_with("invoke_")
        {
            self.add("ffi", "hook_calls", expr.span(), 1);
        }
        if let syn::Expr::Path(path) = &*expr.func {
            let resolved = self.resolve(&path.path);
            let queue_glob = self
                .context
                .imports
                .values()
                .any(|p| p.ends_with("collections::queue::*"))
                && path
                    .path
                    .get_ident()
                    .is_some_and(|n| self.queue_functions.contains(&n.to_string()));
            if (resolved.contains("collections::queue::") || queue_glob)
                && self.context.file != "src/collections/queue.rs"
                && !self.context.module.contains("<LinkQueue<")
                && !self.context.module.contains("<ElementTable")
            {
                self.add("ffi", "queue_calls", expr.span(), 1);
            }
            if resolved.contains("app::bridge::") {
                self.add("ffi", "bridge_calls", expr.span(), 1);
            }
            if resolved.ends_with("Hook::Legacy") {
                self.add("ffi", "legacy_hooks", expr.span(), 1);
            }
        }
        visit::visit_expr_call(self, expr);
    }
    fn visit_path(&mut self, path: &'a syn::Path) {
        if !self.record_module()
            && !path
                .segments
                .iter()
                .any(|s| metrics::RAW_RECORDS.contains(&s.ident.to_string().as_str()))
            && self
                .resolve(path)
                .split("::")
                .any(|s| metrics::RAW_RECORDS.contains(&s))
        {
            self.add("ffi", "raw_records", path.span(), 1);
        }
        visit::visit_path(self, path);
    }
    fn visit_attribute(&mut self, attr: &'a Attribute) {
        if !attr.path().is_ident("doc") && !self.defs_only {
            visit::visit_attribute(self, attr);
        }
    }
    fn visit_ident(&mut self, ident: &'a syn::Ident) {
        if self.names.is_none()
            && !self.record_module()
            && metrics::RAW_RECORDS.contains(&ident.to_string().as_str())
        {
            self.add("ffi", "raw_records", ident.span(), 1);
        }
        if !self.defs_only && !self.generated {
            self.identifier(ident);
        }
    }
    fn visit_token_stream(&mut self, tokens: &'a TokenStream) {
        self.tokens(tokens.clone());
    }
}

impl Walker {
    fn tokens(&mut self, tokens: TokenStream) {
        if self.defs_only || self.generated {
            return;
        }
        for token in tokens {
            match token {
                TokenTree::Group(g) => self.tokens(g.stream()),
                TokenTree::Ident(i) => {
                    if self.names.is_none()
                        && !self.record_module()
                        && metrics::RAW_RECORDS.contains(&i.to_string().as_str())
                    {
                        self.add("ffi", "raw_records", i.span(), 1);
                    } else {
                        self.identifier(&i);
                    }
                }
                _ => {}
            }
        }
    }
}

impl Walker {
    fn record_module(&self) -> bool {
        self.context.file == "src/engine/battle/record.rs"
            || self.context.file.ends_with("/engine/src/battle/record.rs")
    }
    fn bind_hook(&mut self, pat: &syn::Pat) {
        match pat {
            syn::Pat::Ident(p) => {
                self.context.hooks.insert(p.ident.to_string());
            }
            syn::Pat::TupleStruct(p) => {
                for p in &p.elems {
                    self.bind_hook(p);
                }
            }
            _ => {}
        }
    }
}
