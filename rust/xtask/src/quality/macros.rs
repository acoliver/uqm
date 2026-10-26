//! Reviewed item macros. Expansion uses their current transcriber, not a second
//! copy of the generated code, so changes to a macro remain measurable.
use std::collections::{BTreeMap, BTreeSet};

use proc_macro2::{Delimiter, Group, TokenStream, TokenTree};
use quote::{quote, ToTokens};
use syn::parse::{discouraged::Speculative, ParseStream, Parser};

pub const HANDLERS: &[(&str, &str)] = &[
    ("src/game_init/kernel_abi.rs", "global"),
    ("src/mainloop/options_abi.rs", "global"),
    ("src/graphics/gfx_common_abi.rs", "screen_global"),
    ("src/mainloop/state_abi.rs", "accessor"),
    ("src/mainloop/main2_abi.rs", "operation"),
    ("src/mainloop/restart_menu/menu_abi.rs", "operation"),
    ("src/ships/native_adapters.rs", "operation"),
    ("src/collections/hash_table.rs", "define_hash_table"),
    ("src/planets/generate/home_exports.rs", "energy_hook"),
    ("src/planets/generate/late_exports.rs", "moons_hook"),
    ("src/planets/generate/late_exports.rs", "story_energy"),
    ("src/planets/generate/late_exports.rs", "story_pickup"),
    ("src/front_end/setupmenu.rs", "menu_handler"),
    ("src/comm/races/comandr.rs", "callbacks"),
    ("src/state/schema.rs", "game_state_fields"),
    ("src/c_bindings.rs", "assert_c_layout"),
];

pub fn archived_registry(rust: &std::path::Path) -> Result<Vec<(String, String)>, String> {
    let path = rust.join("xtask/src/quality/macros.rs");
    if !path.exists() {
        return Ok(HANDLERS
            .iter()
            .map(|(f, n)| (f.to_string(), n.to_string()))
            .collect());
    }
    let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let source = syn::parse_file(&text).map_err(|e| e.to_string())?;
    let expr = source
        .items
        .into_iter()
        .find_map(|item| match item {
            syn::Item::Const(c) if c.ident == "HANDLERS" => Some(c.expr),
            _ => None,
        })
        .ok_or("base macro registry missing HANDLERS")?;
    let syn::Expr::Reference(reference) = *expr else {
        return Err("invalid base macro registry".into());
    };
    let syn::Expr::Array(array) = *reference.expr else {
        return Err("invalid base macro registry".into());
    };
    array
        .elems
        .into_iter()
        .map(|entry| {
            let syn::Expr::Tuple(tuple) = entry else {
                return Err("invalid base macro handler".into());
            };
            let values = tuple
                .elems
                .into_iter()
                .map(|expr| match expr {
                    syn::Expr::Lit(lit) => match lit.lit {
                        syn::Lit::Str(s) => Ok(s.value()),
                        _ => Err("base handler needs strings".to_string()),
                    },
                    _ => Err("base handler needs strings".into()),
                })
                .collect::<Result<Vec<_>, _>>()?;
            if values.len() != 2 {
                return Err("base handler needs file and name".into());
            }
            Ok((values[0].clone(), values[1].clone()))
        })
        .collect()
}

#[derive(Clone)]
enum Pattern {
    Token(TokenTree),
    Group(Delimiter, Vec<Pattern>),
    Capture(String, String),
    Repeat(Vec<Pattern>, Option<TokenTree>, char),
}

#[derive(Clone)]
pub struct Definition {
    arms: Vec<(Vec<Pattern>, TokenStream)>,
}

fn punct(token: &TokenTree, value: char) -> bool {
    matches!(token, TokenTree::Punct(p) if p.as_char() == value)
}

fn pattern(stream: TokenStream) -> Result<Vec<Pattern>, String> {
    let tokens: Vec<_> = stream.into_iter().collect();
    let mut result = Vec::new();
    let mut i = 0;
    while i < tokens.len() {
        if punct(&tokens[i], '$') {
            match tokens.get(i + 1) {
                Some(TokenTree::Group(group)) => {
                    let (separator, op_index) = if tokens
                        .get(i + 2)
                        .is_some_and(|t| ['*', '+', '?'].iter().any(|c| punct(t, *c)))
                    {
                        (None, i + 2)
                    } else {
                        (tokens.get(i + 2).cloned(), i + 3)
                    };
                    let op = match tokens.get(op_index) {
                        Some(TokenTree::Punct(p)) if ['*', '+', '?'].contains(&p.as_char()) => {
                            p.as_char()
                        }
                        _ => return Err("unsupported macro repetition".into()),
                    };
                    result.push(Pattern::Repeat(pattern(group.stream())?, separator, op));
                    i = op_index + 1;
                }
                Some(TokenTree::Ident(name))
                    if tokens.get(i + 2).is_some_and(|t| punct(t, ':')) =>
                {
                    let Some(TokenTree::Ident(kind)) = tokens.get(i + 3) else {
                        return Err("missing macro fragment kind".into());
                    };
                    result.push(Pattern::Capture(name.to_string(), kind.to_string()));
                    i += 4;
                }
                _ => return Err("unsupported macro matcher".into()),
            }
        } else {
            result.push(match &tokens[i] {
                TokenTree::Group(g) => Pattern::Group(g.delimiter(), pattern(g.stream())?),
                t => Pattern::Token(t.clone()),
            });
            i += 1;
        }
    }
    Ok(result)
}

impl Definition {
    pub fn parse(stream: TokenStream) -> Result<Self, String> {
        let parser = |input: ParseStream<'_>| {
            let mut arms = Vec::new();
            while !input.is_empty() {
                let matcher: Group = input.parse()?;
                input.parse::<syn::Token![=>]>()?;
                let transcriber: Group = input.parse()?;
                arms.push((
                    pattern(matcher.stream()).map_err(|e| input.error(e))?,
                    transcriber.stream(),
                ));
                if input.peek(syn::Token![;]) {
                    input.parse::<syn::Token![;]>()?;
                } else if input.peek(syn::Token![,]) {
                    input.parse::<syn::Token![,]>()?;
                }
            }
            Ok(Self { arms })
        };
        parser.parse2(stream).map_err(|e| e.to_string())
    }

    pub fn generates_items(&self) -> bool {
        self.arms.iter().any(|(matcher, body)| {
            let captures = samples(matcher);
            transcribe(body.clone(), &captures)
                .ok()
                .and_then(|tokens| syn::parse2::<syn::File>(tokens).ok())
                .is_some_and(|file| {
                    file.items
                        .iter()
                        .any(|item| !matches!(item, syn::Item::Macro(_)))
                })
        })
    }

    pub fn expand(&self, input: TokenStream) -> Result<TokenStream, String> {
        for (matcher, body) in &self.arms {
            let parser = |input: ParseStream<'_>| match_pattern(matcher, input);
            if let Ok(captures) = parser.parse2(input.clone()) {
                return transcribe(body.clone(), &captures);
            }
        }
        Err(format!("no supported macro arm matches `{input}`"))
    }
}

#[derive(Clone)]
enum Capture {
    One(TokenStream),
    Many(Vec<Capture>),
}
type Captures = BTreeMap<String, Capture>;

fn fragment(kind: &str, input: ParseStream<'_>) -> syn::Result<TokenStream> {
    Ok(match kind {
        "ident" => input.parse::<syn::Ident>()?.to_token_stream(),
        "ty" => input.parse::<syn::Type>()?.to_token_stream(),
        "expr" => input.parse::<syn::Expr>()?.to_token_stream(),
        "path" => input.parse::<syn::Path>()?.to_token_stream(),
        "literal" => input.parse::<syn::Lit>()?.to_token_stream(),
        "block" => input.parse::<syn::Block>()?.to_token_stream(),
        "tt" => input.parse::<TokenTree>()?.into(),
        _ => return Err(input.error(format!("unsupported macro fragment {kind}"))),
    })
}

fn match_token(expected: &TokenTree, input: ParseStream<'_>) -> syn::Result<()> {
    let actual: TokenTree = input.parse()?;
    if actual.to_string() != expected.to_string() {
        return Err(input.error("macro token mismatch"));
    }
    Ok(())
}

fn capture_names(pattern: &[Pattern]) -> BTreeSet<String> {
    pattern
        .iter()
        .flat_map(|p| match p {
            Pattern::Capture(name, _) => [name.clone()].into(),
            Pattern::Group(_, p) | Pattern::Repeat(p, _, _) => capture_names(p),
            _ => BTreeSet::new(),
        })
        .collect()
}

fn match_pattern(pattern: &[Pattern], input: ParseStream<'_>) -> syn::Result<Captures> {
    let mut captures = Captures::new();
    for part in pattern {
        match part {
            Pattern::Token(t) => match_token(t, input)?,
            Pattern::Capture(name, kind) => {
                captures.insert(name.clone(), Capture::One(fragment(kind, input)?));
            }
            Pattern::Group(delimiter, nested) => {
                let group: Group = input.parse()?;
                if group.delimiter() != *delimiter {
                    return Err(input.error("macro delimiter mismatch"));
                }
                let parser = |input: ParseStream<'_>| match_pattern(nested, input);
                captures.extend(parser.parse2(group.stream())?);
            }
            Pattern::Repeat(nested, separator, op) => {
                let mut rows = Vec::new();
                loop {
                    let fork = input.fork();
                    if !rows.is_empty() {
                        if *op == '?' {
                            break;
                        }
                        if let Some(separator) = separator {
                            if match_token(separator, &fork).is_err() {
                                break;
                            }
                        }
                    }
                    let before = fork.cursor();
                    let Ok(row) = match_pattern(nested, &fork) else {
                        break;
                    };
                    if before == fork.cursor() {
                        return Err(input.error("empty macro repetition"));
                    }
                    input.advance_to(&fork);
                    rows.push(row);
                }
                if *op == '+' && rows.is_empty() {
                    return Err(input.error("expected macro repetition"));
                }
                for name in capture_names(nested) {
                    captures.insert(
                        name.clone(),
                        Capture::Many(rows.iter().map(|row| row[&name].clone()).collect()),
                    );
                }
            }
        }
    }
    Ok(captures)
}

fn samples(pattern: &[Pattern]) -> Captures {
    let mut captures = Captures::new();
    for p in pattern {
        match p {
            Pattern::Capture(name, kind) => {
                let value = match kind.as_str() {
                    "item" => quote!(
                        fn quality_macro_sample() {}
                    ),
                    "vis" => quote!(pub),
                    "literal" => quote!(0),
                    "block" => quote!({}),
                    "expr" => quote!(0),
                    _ => quote!(QualityMacroSample),
                };
                captures.insert(name.clone(), Capture::One(value));
            }
            Pattern::Group(_, p) => captures.extend(samples(p)),
            Pattern::Repeat(p, _, _) => captures.extend(
                samples(p)
                    .into_iter()
                    .map(|(k, v)| (k, Capture::Many(vec![v]))),
            ),
            _ => {}
        }
    }
    captures
}

fn referenced(stream: TokenStream) -> BTreeSet<String> {
    let mut result = BTreeSet::new();
    let mut tokens = stream.into_iter();
    while let Some(t) = tokens.next() {
        match t {
            TokenTree::Punct(p) if p.as_char() == '$' => match tokens.next() {
                Some(TokenTree::Ident(i)) => {
                    result.insert(i.to_string());
                }
                Some(TokenTree::Group(g)) => result.extend(referenced(g.stream())),
                _ => {}
            },
            TokenTree::Group(g) => result.extend(referenced(g.stream())),
            _ => {}
        }
    }
    result
}

fn transcribe(stream: TokenStream, captures: &Captures) -> Result<TokenStream, String> {
    let mut output = TokenStream::new();
    let mut tokens = stream.into_iter();
    while let Some(t) = tokens.next() {
        match t {
            TokenTree::Punct(p) if p.as_char() == '$' => match tokens.next() {
                Some(TokenTree::Ident(name)) if name == "crate" => output.extend(quote!(crate)),
                Some(TokenTree::Ident(name)) => match captures.get(&name.to_string()) {
                    Some(Capture::One(value)) => output.extend(value.clone()),
                    _ => return Err(format!("unbound macro capture {name}")),
                },
                Some(TokenTree::Group(group)) => {
                    let next = tokens.next().ok_or("missing macro repetition operator")?;
                    let separator = if ['*', '+', '?'].iter().any(|c| punct(&next, *c)) {
                        None
                    } else {
                        let op = tokens.next().ok_or("missing macro repetition operator")?;
                        if !['*', '+', '?'].iter().any(|c| punct(&op, *c)) {
                            return Err("unsupported transcriber repetition".into());
                        }
                        Some(next)
                    };
                    let lengths: BTreeSet<_> = referenced(group.stream())
                        .iter()
                        .filter_map(|name| match captures.get(name) {
                            Some(Capture::Many(rows)) => Some(rows.len()),
                            _ => None,
                        })
                        .collect();
                    if lengths.len() != 1 {
                        return Err("macro repetition has inconsistent capture lengths".into());
                    }
                    let length = *lengths.first().unwrap();
                    for i in 0..length {
                        if i != 0 {
                            output.extend(separator.clone());
                        }
                        let row = captures
                            .iter()
                            .map(|(k, v)| {
                                (
                                    k.clone(),
                                    match v {
                                        Capture::Many(rows)
                                            if referenced(group.stream()).contains(k) =>
                                        {
                                            rows[i].clone()
                                        }
                                        _ => v.clone(),
                                    },
                                )
                            })
                            .collect();
                        output.extend(transcribe(group.stream(), &row)?);
                    }
                }
                _ => return Err("unsupported macro transcriber".into()),
            },
            TokenTree::Group(g) => {
                let mut group = Group::new(g.delimiter(), transcribe(g.stream(), captures)?);
                group.set_span(g.span());
                output.extend([TokenTree::Group(group)]);
            }
            other => output.extend([other]),
        }
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expansion_preserves_function_bodies_and_repeated_arguments() {
        let definition = Definition::parse(quote! {
            ($name:ident($($arg:ident: $ty:ty),*) $(-> $ret:ty)? $body:block) => {
                pub unsafe fn $name($($arg: $ty),*) $(-> $ret)? $body
            };
        })
        .unwrap();
        let file: syn::File = syn::parse2(
            definition
                .expand(quote!(run(a: *mut u8, b: i32) -> bool { unsafe { *a = b as u8; } true }))
                .unwrap(),
        )
        .unwrap();
        assert_eq!(file.items.len(), 1);
        let syn::Item::Fn(f) = &file.items[0] else {
            panic!("not a function")
        };
        assert_eq!(f.sig.inputs.len(), 2);
        assert!(f.sig.unsafety.is_some());
        assert!(f.block.to_token_stream().to_string().contains("unsafe"));
        assert!(definition.expand(quote!(invalid)).is_err());
    }

    #[test]
    fn unknown_item_transcribers_are_detected() {
        for definition in [
            quote! { ($name:ident) => { pub fn $name() {} }; },
            quote! { () => { static VALUE: u8 = 1; }; },
        ] {
            assert!(Definition::parse(definition).unwrap().generates_items());
        }
        assert!(
            !Definition::parse(quote! { ($value:expr) => { $value + 1 }; })
                .unwrap()
                .generates_items()
        );
    }
}
