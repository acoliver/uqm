//! Source correspondence for mechanical owner-loan threading.

use proc_macro2::{Span, TokenStream, TokenTree};
use quote::ToTokens;
use std::collections::{BTreeMap, BTreeSet};
use syn::{
    spanned::Spanned,
    visit::{self, Visit},
};

type Position = (usize, usize);

#[derive(Clone)]
struct Erasure {
    start: Position,
    end: Position,
}
impl Erasure {
    fn new(span: Span) -> Self {
        let a = span.start();
        let b = span.end();
        Self {
            start: (a.line, a.column),
            end: (b.line, b.column),
        }
    }
    fn contains(&self, span: Span) -> bool {
        let a = span.start();
        let b = span.end();
        self.start <= (a.line, a.column) && (b.line, b.column) <= self.end
    }
}

fn owner(ty: &syn::Type) -> bool {
    let syn::Type::Reference(reference) = ty else {
        return false;
    };
    let syn::Type::Path(path) = &*reference.elem else {
        return false;
    };
    if path.qself.is_some() {
        return false;
    }
    let name = path
        .path
        .segments
        .iter()
        .map(|s| s.ident.to_string())
        .collect::<Vec<_>>()
        .join("::");
    [
        "Comm",
        "crate::comm::Comm",
        "crate::comm::state::Comm",
        "Cx",
        "crate::engine::battle::Cx",
        "uqm_engine::battle::Cx",
        "World",
        "crate::engine::World",
        "crate::engine::world::World",
        "uqm_engine::World",
        "Kernel",
        "crate::engine::Kernel",
        "crate::engine::kernel::Kernel",
        "uqm_engine::Kernel",
    ]
    .contains(&name.as_str())
}

fn inserted(old: &[String], new: &[String], allowed: &[bool]) -> Vec<usize> {
    if new.len() <= old.len() {
        return Vec::new();
    }
    let mut extra = new.len() - old.len();
    let mut cursor = 0;
    let mut erased = Vec::new();
    for (index, value) in new.iter().enumerate() {
        if old.get(cursor) == Some(value) {
            cursor += 1;
        } else if extra > 0 && allowed[index] {
            erased.push(index);
            extra -= 1;
        } else if cursor < old.len() {
            cursor += 1;
        } else {
            return Vec::new();
        }
    }
    if extra == 0 && cursor == old.len() {
        erased
    } else {
        Vec::new()
    }
}

fn erase<T: ToTokens + Spanned>(
    args: &syn::punctuated::Punctuated<T, syn::Token![,]>,
    indexes: &[usize],
) -> Vec<Erasure> {
    let pairs: Vec<_> = args.pairs().collect();
    let mut erased = Vec::new();
    for &index in indexes {
        erased.push(Erasure::new(pairs[index].value().span()));
        if let Some(comma) = pairs[index].punct() {
            erased.push(Erasure::new(comma.span()));
        } else if index > 0 {
            if let Some(comma) = pairs[index - 1].punct() {
                erased.push(Erasure::new(comma.span()));
            }
        }
    }
    erased
}

fn has_self(tokens: TokenStream) -> bool {
    tokens.into_iter().any(|token| match token {
        TokenTree::Ident(ident) => ident == "self",
        TokenTree::Group(group) => has_self(group.stream()),
        _ => false,
    })
}

fn parameters(
    old: &syn::ItemFn,
    new: &syn::ItemFn,
) -> (Vec<Erasure>, Vec<Erasure>, BTreeSet<String>) {
    let unused = !has_self(old.block.to_token_stream());
    let removed: Vec<_> = old
        .sig
        .inputs
        .iter()
        .enumerate()
        .filter(|(_, arg)| {
            unused
                && matches!(arg, syn::FnArg::Receiver(_))
                && !new
                    .sig
                    .inputs
                    .iter()
                    .any(|arg| matches!(arg, syn::FnArg::Receiver(_)))
        })
        .map(|(i, _)| i)
        .collect();
    let before: Vec<_> = old
        .sig
        .inputs
        .iter()
        .enumerate()
        .filter(|(i, _)| !removed.contains(i))
        .map(|(_, arg)| arg.to_token_stream().to_string())
        .collect();
    let after: Vec<_> = new
        .sig
        .inputs
        .iter()
        .map(|arg| arg.to_token_stream().to_string())
        .collect();
    let allowed: Vec<_> = new
        .sig
        .inputs
        .iter()
        .map(|arg| matches!(arg, syn::FnArg::Typed(arg) if owner(&arg.ty)))
        .collect();
    let added = inserted(&before, &after, &allowed);
    let names = new
        .sig
        .inputs
        .iter()
        .filter_map(|arg| {
            let syn::FnArg::Typed(arg) = arg else {
                return None;
            };
            let syn::Pat::Ident(name) = &*arg.pat else {
                return None;
            };
            owner(&arg.ty).then(|| name.ident.to_string())
        })
        .collect();
    (
        erase(&old.sig.inputs, &removed),
        erase(&new.sig.inputs, &added),
        names,
    )
}

struct Call<'a> {
    name: String,
    args: &'a syn::punctuated::Punctuated<syn::Expr, syn::Token![,]>,
}
#[derive(Default)]
struct Calls<'a>(Vec<Call<'a>>);
impl<'a> Visit<'a> for Calls<'a> {
    fn visit_expr_call(&mut self, call: &'a syn::ExprCall) {
        self.0.push(Call {
            name: call.func.to_token_stream().to_string(),
            args: &call.args,
        });
        visit::visit_expr_call(self, call);
    }
    fn visit_expr_method_call(&mut self, call: &'a syn::ExprMethodCall) {
        self.0.push(Call {
            name: format!("{}::{}", call.receiver.to_token_stream(), call.method),
            args: &call.args,
        });
        visit::visit_expr_method_call(self, call);
    }
}

fn loan(expression: &syn::Expr, names: &BTreeSet<String>) -> bool {
    match expression {
        syn::Expr::Path(path) => path
            .path
            .get_ident()
            .is_some_and(|name| names.contains(&name.to_string())),
        syn::Expr::Reference(reference) => loan(&reference.expr, names),
        _ => false,
    }
}

fn arguments(old: &syn::Block, new: &syn::Block, names: &BTreeSet<String>) -> Vec<Erasure> {
    let mut before = Calls::default();
    before.visit_block(old);
    let mut after = Calls::default();
    after.visit_block(new);
    let mut cursor = 0;
    let mut erased = Vec::new();
    for current in after.0 {
        let Some(offset) = before.0[cursor..]
            .iter()
            .position(|call| call.name == current.name)
        else {
            continue;
        };
        cursor += offset;
        let prior = &before.0[cursor];
        cursor += 1;
        let old: Vec<_> = prior
            .args
            .iter()
            .map(|arg| arg.to_token_stream().to_string())
            .collect();
        let new: Vec<_> = current
            .args
            .iter()
            .map(|arg| arg.to_token_stream().to_string())
            .collect();
        let allowed: Vec<_> = current.args.iter().map(|arg| loan(arg, names)).collect();
        erased.extend(erase(current.args, &inserted(&old, &new, &allowed)));
    }
    erased
}

fn rows(tokens: TokenStream, erased: &[Erasure], result: &mut BTreeMap<usize, Vec<String>>) {
    fn add(
        result: &mut BTreeMap<usize, Vec<String>>,
        erased: &[Erasure],
        value: String,
        span: Span,
    ) {
        if !erased.iter().any(|range| range.contains(span)) {
            for line in span.start().line..=span.end().line {
                result.entry(line).or_default().push(value.clone());
            }
        }
    }
    for token in tokens {
        match token {
            TokenTree::Group(group) => {
                let (open, close) = match group.delimiter() {
                    proc_macro2::Delimiter::Parenthesis => ("(", ")"),
                    proc_macro2::Delimiter::Bracket => ("[", "]"),
                    proc_macro2::Delimiter::Brace => ("{", "}"),
                    proc_macro2::Delimiter::None => ("", ""),
                };
                add(result, erased, open.into(), group.span_open());
                rows(group.stream(), erased, result);
                if !erased
                    .iter()
                    .any(|range| range.contains(group.span_close()))
                {
                    result
                        .entry(group.span_close().start().line)
                        .or_default()
                        .push(close.into());
                }
            }
            token => add(result, erased, token.to_string(), token.span()),
        }
    }
}

pub(crate) fn moved_rows(before: &str, current: &str) -> Result<BTreeSet<usize>, String> {
    let old: syn::ItemFn =
        syn::parse_str(before).map_err(|e| format!("base coverage function: {e}"))?;
    let new: syn::ItemFn =
        syn::parse_str(current).map_err(|e| format!("target coverage function: {e}"))?;
    let (old_erased, mut new_erased, names) = parameters(&old, &new);
    new_erased.extend(arguments(&old.block, &new.block, &names));
    let mut old_rows = BTreeMap::new();
    rows(old.to_token_stream(), &old_erased, &mut old_rows);
    let mut new_rows = BTreeMap::new();
    rows(new.to_token_stream(), &new_erased, &mut new_rows);
    if old_rows.values().flatten().eq(new_rows.values().flatten()) {
        let mut original = BTreeMap::new();
        rows(new.to_token_stream(), &[], &mut original);
        return Ok(original.into_keys().collect());
    }
    let mut available: BTreeMap<Vec<String>, usize> = BTreeMap::new();
    for row in old_rows.into_values() {
        *available.entry(row).or_default() += 1;
    }
    let mut moved = BTreeSet::new();
    for (line, tokens) in new_rows {
        if let Some(count) = available.get_mut(&tokens) {
            if *count > 0 {
                *count -= 1;
                moved.insert(line);
            }
        }
    }
    Ok(moved)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inserted_session_parameter_and_argument_are_mechanical() -> Result<(), String> {
        let before = "fn talk(cx: &mut Cx<'_>) {\n    run(cx, 7);\n}";
        let current = "fn talk(comm: &mut Comm, cx: &mut Cx<'_>) {\n    run(comm, cx, 7);\n}";
        assert_eq!(moved_rows(before, current)?, [1, 2, 3].into());
        Ok(())
    }

    #[test]
    fn changed_expression_or_call_on_threaded_line_is_new() -> Result<(), String> {
        let before = "fn talk(cx: &mut Cx<'_>) {\n    run(cx, 7);\n}";
        for body in [
            "run(comm, cx, 8)",
            "other(comm, cx, 7)",
            "run(comm, different, 7)",
        ] {
            let current = format!("fn talk(comm: &mut Comm, cx: &mut Cx<'_>) {{\n    {body};\n}}");
            assert!(
                !moved_rows(before, &current)?.contains(&2),
                "accepted {body}"
            );
        }
        Ok(())
    }

    #[test]
    fn non_owner_insertion_and_argument_removal_are_new() -> Result<(), String> {
        let before = "fn talk(cx: &mut Cx<'_>) {\n    run(cx, 7);\n}";
        for current in [
            "fn talk(other: &mut Other, cx: &mut Cx<'_>) {\n    run(other, cx, 7);\n}",
            "fn talk(cx: &mut Cx<'_>) {\n    run(7);\n}",
            "fn talk(cx: &mut Cx<'_>) {\n    run(cx, 7, arbitrary);\n}",
        ] {
            assert!(!moved_rows(before, current)?.contains(&2));
        }
        Ok(())
    }

    #[test]
    fn unused_receiver_removal_is_mechanical_but_used_receiver_is_not() -> Result<(), String> {
        let before = "fn talk(&self, cx: &mut Cx<'_>) {\n    run(cx, 7);\n}";
        let current = "fn talk(comm: &mut Comm, cx: &mut Cx<'_>) {\n    run(comm, cx, 7);\n}";
        assert!(moved_rows(before, current)?.contains(&1));
        let used = "fn talk(&self, cx: &mut Cx<'_>) {\n    self.run(cx, 7);\n}";
        assert!(!moved_rows(used, current)?.contains(&1));
        Ok(())
    }

    #[test]
    fn borrowing_owner_is_allowed_but_computing_an_argument_is_not() -> Result<(), String> {
        let before = "fn talk(cx: &mut Cx<'_>) {\n    run(cx);\n}";
        let current = "fn talk(comm: &mut Comm, cx: &mut Cx<'_>) {\n    run(&mut comm, cx);\n}";
        assert!(moved_rows(before, current)?.contains(&2));
        let computed = current.replace("&mut comm, cx", "comm.acquire(), cx");
        assert!(!moved_rows(before, &computed)?.contains(&2));
        Ok(())
    }

    #[test]
    fn owner_only_wrapping_changes_are_moved_not_fabricated_coverage() -> Result<(), String> {
        let before = "fn talk(cx: &mut Cx<'_>) { run(cx, 7); }";
        let current =
            "fn talk(\n comm: &mut Comm,\n cx: &mut Cx<'_>\n) {\n run(\n comm,\n cx,\n 7\n );\n}";
        assert!(moved_rows(before, current)?.contains(&6));
        Ok(())
    }
}
