use super::*;

impl Walker {
    pub(super) fn projection_key(&self, path: &str) -> String {
        let absolute = super::super::layers::absolute(&self.context.lexical_module, path);
        absolute
            .strip_prefix(&format!("{}::", self.context.package))
            .map_or_else(|| absolute.clone(), |tail| format!("crate::{tail}"))
    }

    pub(super) fn resolve(&self, path: &syn::Path) -> String {
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
    pub(super) fn layer_path(&mut self, target: &str, span: Span, direct_bridge: bool) {
        let target = super::super::layers::absolute(&self.context.lexical_module, target);
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
                .extend(super::super::layers::reference_in_phase(
                    &source,
                    target,
                    self.context.platform_split,
                    direct_bridge,
                ));
        }
    }

    pub(super) fn finish_owner_projections(&mut self) {
        for (candidates, finding) in std::mem::take(&mut self.owner_projection_calls) {
            if candidates
                .iter()
                .any(|target| self.owner_projectors.contains(target))
            {
                self.measurement.findings.push(finding);
            }
        }
    }

    pub(super) fn record_owner_call(&mut self, resolved: &str, span: Span) {
        if self.context.file != "src/app/bridge.rs"
            && resolved.contains("engine::")
            && ["Kernel::current", "World::current", "WorldData::current"]
                .iter()
                .any(|suffix| resolved.ends_with(suffix))
        {
            self.add("ffi", "owner_projections", span, 1);
        } else if self.names.is_none() && self.context.file != "src/app/bridge.rs" {
            let key = format!(
                "owner-call|{}|{}:{}|{}",
                self.context.file,
                span.start().line,
                span.start().column,
                self.context.module
            );
            if self.events.insert(key) {
                let mut candidates = vec![self.projection_key(resolved)];
                if !resolved.contains("::") {
                    candidates.push(
                        self.projection_key(&format!(
                            "{}::{resolved}",
                            self.context.lexical_module
                        )),
                    );
                    for import in self
                        .context
                        .imports
                        .values()
                        .filter_map(|path| path.strip_suffix("::*"))
                    {
                        candidates.push(self.projection_key(&format!("{import}::{resolved}")));
                    }
                }
                self.owner_projection_calls.push((
                    candidates,
                    self.finding("ffi", "owner_projections", self.context.module.clone(), 1),
                ));
            }
        }
    }
}
