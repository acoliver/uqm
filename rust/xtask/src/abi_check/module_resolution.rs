use super::Catalog;
use std::collections::BTreeSet;

impl Catalog {
    pub(super) fn absolute(&self, path: &str, scope: usize) -> String {
        let module = &self.scopes[scope].path;
        if let Some(rest) = path.strip_prefix("crate::") {
            format!(
                "{}::{rest}",
                module
                    .split_once("::")
                    .map_or(module.as_str(), |(head, _)| head)
            )
        } else if let Some(rest) = path.strip_prefix("self::") {
            format!("{module}::{rest}")
        } else if let Some(rest) = path.strip_prefix("super::") {
            let parent = self.scopes[scope].parent.unwrap_or(scope);
            self.absolute(&format!("self::{rest}"), parent)
        } else {
            let head = path.split_once("::").map_or(path, |(head, _)| head);
            if let Some(import) = self.scopes[scope].imports.get(head) {
                format!("{import}{}", &path[head.len()..])
            } else if self.modules.contains_key(&format!("{module}::{head}")) {
                format!("{module}::{path}")
            } else {
                path.to_string()
            }
        }
    }

    pub(super) fn module_scope(
        &self,
        path: &str,
        scope: usize,
        seen: &mut BTreeSet<(usize, String)>,
    ) -> Option<usize> {
        let path = self.absolute(path, scope);
        if !seen.insert((scope, path.clone())) {
            return None;
        }
        if let Some(module) = self.modules.get(&path) {
            return Some(*module);
        }
        let (parent, name) = path.rsplit_once("::")?;
        let parent = self.module_scope(parent, scope, seen)?;
        let owner = &self.scopes[parent];
        if let Some(module) = self.modules.get(&format!("{}::{name}", owner.path)) {
            return Some(*module);
        }
        self.module_scope(owner.imports.get(name)?, parent, seen)
    }
}
