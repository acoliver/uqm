use std::collections::BTreeSet;
use std::process::Command;

use quote::ToTokens;
use syn::{punctuated::Punctuated, Attribute, Meta, Token};

#[derive(Clone, Debug)]
pub struct Configuration {
    flags: BTreeSet<String>,
    values: BTreeSet<(String, String)>,
    features: BTreeSet<String>,
}

impl Configuration {
    pub fn ci(features: &BTreeSet<String>) -> Result<Vec<Self>, String> {
        let mut configurations = Vec::new();
        for target in [
            "x86_64-unknown-linux-gnu",
            "aarch64-unknown-linux-gnu",
            "aarch64-apple-darwin",
            "x86_64-pc-windows-msvc",
            "aarch64-pc-windows-msvc",
        ] {
            let output = Command::new(std::env::var("RUSTC").unwrap_or_else(|_| "rustc".into()))
                .args(["--print", "cfg", "--target", target])
                .output()
                .map_err(|e| e.to_string())?;
            if !output.status.success() {
                return Err(format!(
                    "rustc cfg for {target}: {}",
                    String::from_utf8_lossy(&output.stderr)
                ));
            }
            let text = String::from_utf8(output.stdout).map_err(|e| e.to_string())?;
            for debug in [false, true] {
                let mut cfg = Self {
                    flags: BTreeSet::new(),
                    values: BTreeSet::new(),
                    features: features.clone(),
                };
                for line in text.lines() {
                    if let Some((key, value)) = line.split_once('=') {
                        cfg.values
                            .insert((key.into(), value.trim_matches('"').into()));
                    } else {
                        cfg.flags.insert(line.into());
                    }
                }
                cfg.flags.remove("test");
                cfg.flags.remove("debug_assertions");
                if debug {
                    cfg.flags.insert("debug_assertions".into());
                }
                for feature in ["audio_heart", "debug-process", "bundled-sdl", "native"] {
                    if features.contains(feature) {
                        cfg.values.insert(("feature".into(), feature.into()));
                    }
                }
                configurations.push(cfg);
            }
        }
        Ok(configurations)
    }

    fn evaluate(&self, meta: &Meta) -> Result<bool, String> {
        let name = meta
            .path()
            .get_ident()
            .map(ToString::to_string)
            .unwrap_or_default();
        match meta {
            Meta::Path(_)
                if [
                    "unix",
                    "windows",
                    "test",
                    "debug_assertions",
                    "proc_macro",
                    "target_thread_local",
                ]
                .contains(&name.as_str()) =>
            {
                Ok(self.flags.contains(&name))
            }
            Meta::NameValue(value) => {
                let syn::Expr::Lit(value) = &value.value else {
                    return Err(format!("unsupported cfg {meta:?}"));
                };
                let syn::Lit::Str(value) = &value.lit else {
                    return Err(format!("unsupported cfg {meta:?}"));
                };
                let value = value.value();
                if name == "feature" {
                    if !self.features.contains(&value) {
                        return Err(format!("unknown cfg feature {value}"));
                    }
                } else if ![
                    "target_arch",
                    "target_os",
                    "target_env",
                    "target_vendor",
                    "target_family",
                    "target_endian",
                    "target_pointer_width",
                    "target_abi",
                    "target_has_atomic",
                    "target_has_atomic_load_store",
                    "target_has_atomic_equal_alignment",
                    "target_feature",
                    "panic",
                ]
                .contains(&name.as_str())
                {
                    return Err(format!("unknown cfg key {name}"));
                }
                Ok(self.values.contains(&(name, value)))
            }
            Meta::List(list) => {
                let nested = list
                    .parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)
                    .map_err(|e| e.to_string())?;
                let values = nested
                    .iter()
                    .map(|m| self.evaluate(m))
                    .collect::<Result<Vec<_>, _>>()?;
                match name.as_str() {
                    "all" => Ok(values.iter().all(|v| *v)),
                    "any" => Ok(values.iter().any(|v| *v)),
                    "not" if values.len() == 1 => Ok(!values[0]),
                    _ => Err(format!("unsupported cfg {}", meta.to_token_stream())),
                }
            }
            _ => Err(format!("unsupported cfg {}", meta.to_token_stream())),
        }
    }
}

pub fn attributes(attrs: &[Attribute], config: &Configuration) -> Result<Vec<Meta>, String> {
    fn expand(meta: &Meta, config: &Configuration, output: &mut Vec<Meta>) -> Result<(), String> {
        if meta.path().is_ident("cfg_attr") {
            let Meta::List(list) = meta else {
                return Err("invalid cfg_attr".into());
            };
            let nested = list
                .parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)
                .map_err(|e| e.to_string())?;
            let condition = nested.first().ok_or("empty cfg_attr")?;
            let active = config.evaluate(condition)?;
            // Validate nested cfg predicates even when this configuration does not activate them.
            let mut expanded = Vec::new();
            for item in nested.iter().skip(1) {
                expand(item, config, &mut expanded)?;
            }
            if active {
                output.extend(expanded);
            }
        } else {
            if meta.path().is_ident("cfg") {
                let Meta::List(list) = meta else {
                    return Err("invalid cfg".into());
                };
                config.evaluate(&list.parse_args::<Meta>().map_err(|e| e.to_string())?)?;
            }
            output.push(meta.clone());
        }
        Ok(())
    }
    let mut output = Vec::new();
    for attr in attrs {
        expand(&attr.meta, config, &mut output)?;
    }
    Ok(output)
}

pub fn active(attrs: &[Attribute], config: &Configuration) -> Result<bool, String> {
    let mut enabled = true;
    for meta in attributes(attrs, config)? {
        if let Meta::List(list) = meta {
            if list.path.is_ident("cfg") {
                enabled &=
                    config.evaluate(&list.parse_args::<Meta>().map_err(|e| e.to_string())?)?;
            }
        }
    }
    Ok(enabled)
}

pub fn expr_attributes(expr: &syn::Expr) -> &[Attribute] {
    match expr {
        syn::Expr::Array(e) => &e.attrs,
        syn::Expr::Assign(e) => &e.attrs,
        syn::Expr::Async(e) => &e.attrs,
        syn::Expr::Await(e) => &e.attrs,
        syn::Expr::Binary(e) => &e.attrs,
        syn::Expr::Block(e) => &e.attrs,
        syn::Expr::Break(e) => &e.attrs,
        syn::Expr::Call(e) => &e.attrs,
        syn::Expr::Cast(e) => &e.attrs,
        syn::Expr::Closure(e) => &e.attrs,
        syn::Expr::Const(e) => &e.attrs,
        syn::Expr::Continue(e) => &e.attrs,
        syn::Expr::Field(e) => &e.attrs,
        syn::Expr::ForLoop(e) => &e.attrs,
        syn::Expr::Group(e) => &e.attrs,
        syn::Expr::If(e) => &e.attrs,
        syn::Expr::Index(e) => &e.attrs,
        syn::Expr::Infer(e) => &e.attrs,
        syn::Expr::Let(e) => &e.attrs,
        syn::Expr::Lit(e) => &e.attrs,
        syn::Expr::Loop(e) => &e.attrs,
        syn::Expr::Macro(e) => &e.attrs,
        syn::Expr::Match(e) => &e.attrs,
        syn::Expr::MethodCall(e) => &e.attrs,
        syn::Expr::Paren(e) => &e.attrs,
        syn::Expr::Path(e) => &e.attrs,
        syn::Expr::Range(e) => &e.attrs,
        syn::Expr::RawAddr(e) => &e.attrs,
        syn::Expr::Reference(e) => &e.attrs,
        syn::Expr::Repeat(e) => &e.attrs,
        syn::Expr::Return(e) => &e.attrs,
        syn::Expr::Struct(e) => &e.attrs,
        syn::Expr::Try(e) => &e.attrs,
        syn::Expr::TryBlock(e) => &e.attrs,
        syn::Expr::Tuple(e) => &e.attrs,
        syn::Expr::Unary(e) => &e.attrs,
        syn::Expr::Unsafe(e) => &e.attrs,
        syn::Expr::While(e) => &e.attrs,
        syn::Expr::Yield(e) => &e.attrs,
        _ => &[],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cfg_union_and_unknown_predicates() {
        let configs = Configuration::ci(&["native".into()].into()).unwrap();
        let attrs: Vec<Attribute> = vec![syn::parse_quote!(#[cfg(all(windows, not(test)))])];
        assert!(configs.iter().any(|c| active(&attrs, c).unwrap()));
        assert!(!configs.iter().all(|c| active(&attrs, c).unwrap()));
        for attrs in [
            vec![syn::parse_quote!(#[cfg(any(unix, mystery))])],
            vec![syn::parse_quote!(#[cfg_attr(windows, cfg(mystery))])],
        ] {
            assert!(active(&attrs, &configs[0]).is_err());
        }
    }
}
