//! Function locations from the production quality walk, not filename inference.

use super::lcov::LineHits;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fs, path::Path};

#[derive(Clone, Debug)]
pub(crate) struct Function {
    pub identity: String,
    pub file: String,
    pub start: (usize, usize),
    pub end: (usize, usize),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Proof {
    pub file: String,
    pub first: usize,
    pub text: String,
    pub lines: LineHits,
}

impl Function {
    pub fn text(&self, root: &Path) -> Result<String, String> {
        let text =
            fs::read_to_string(root.join("rust").join(&self.file)).map_err(|e| e.to_string())?;
        self.slice(&text)
    }

    pub fn slice(&self, text: &str) -> Result<String, String> {
        let offsets: Vec<_> = std::iter::once(0)
            .chain(text.match_indices('\n').map(|(i, _)| i + 1))
            .collect();
        let start = offsets
            .get(self.start.0 - 1)
            .ok_or("coverage function start outside source")?
            + self.start.1;
        let end = offsets
            .get(self.end.0 - 1)
            .ok_or("coverage function end outside source")?
            + self.end.1;
        text.get(start..end)
            .map(str::to_owned)
            .ok_or_else(|| "coverage function span is invalid".into())
    }

    pub fn proof(
        &self,
        root: &Path,
        measured: &BTreeMap<String, LineHits>,
    ) -> Result<Proof, String> {
        let lines = measured
            .get(&self.file)
            .ok_or_else(|| format!("no LLVM lines for {}", self.file))?;
        Ok(Proof {
            file: self.file.clone(),
            first: self.start.0,
            text: self.text(root)?,
            lines: lines
                .range(self.start.0..=self.end.0)
                .map(|(line, hit)| (line - self.start.0 + 1, *hit))
                .collect(),
        })
    }
}

impl Proof {
    pub fn validate(&self) -> Result<(), String> {
        syn::parse_str::<syn::ItemFn>(&self.text)
            .map_err(|e| format!("coverage proof function: {e}"))?;
        if self.first == 0
            || self.lines.is_empty()
            || self
                .lines
                .keys()
                .any(|line| *line == 0 || *line > self.text.lines().count())
        {
            return Err("coverage proof has invalid executable lines".into());
        }
        Ok(())
    }
}

pub(crate) fn unique<'a>(
    functions: &'a [Function],
    identity: &str,
) -> Result<&'a Function, String> {
    let found: Vec<_> = functions
        .iter()
        .filter(|f| f.identity == identity)
        .collect();
    match found.as_slice() {
        [function] => Ok(function),
        _ => Err(format!(
            "coverage needs one production function for {identity}, found {}",
            found.len()
        )),
    }
}
