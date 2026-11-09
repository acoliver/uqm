//! Function locations from the production quality walk, not filename inference.

use super::{capture, lcov::LineHits};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fs, path::Path};

#[derive(Clone, Debug)]
pub(crate) struct Function {
    pub identity: String,
    pub file: String,
    pub start: (usize, usize),
    pub end: (usize, usize),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Proof {
    pub file: String,
    pub first: usize,
    pub text: String,
    pub lines: LineHits,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Snapshot {
    pub commit: String,
    pub file: String,
    pub start: (usize, usize),
    pub end: (usize, usize),
    pub lines: LineHits,
}

fn slice(text: &str, first: (usize, usize), last: (usize, usize)) -> Result<String, String> {
    let offsets: Vec<_> = std::iter::once(0)
        .chain(text.match_indices('\n').map(|(i, _)| i + 1))
        .collect();
    let start = offsets
        .get(first.0 - 1)
        .and_then(|offset| offset.checked_add(first.1))
        .ok_or("coverage function start outside source")?;
    let end = offsets
        .get(last.0 - 1)
        .and_then(|offset| offset.checked_add(last.1))
        .ok_or("coverage function end outside source")?;
    text.get(start..end)
        .map(str::to_owned)
        .ok_or_else(|| "coverage function span is invalid".into())
}

impl Function {
    pub fn text(&self, root: &Path) -> Result<String, String> {
        let text =
            fs::read_to_string(root.join("rust").join(&self.file)).map_err(|e| e.to_string())?;
        self.slice(&text)
    }

    pub fn slice(&self, text: &str) -> Result<String, String> {
        slice(text, self.start, self.end)
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

    pub fn snapshot(&self, commit: &str, lines: LineHits) -> Snapshot {
        Snapshot {
            commit: commit.into(),
            file: self.file.clone(),
            start: self.start,
            end: self.end,
            lines,
        }
    }
}

impl Snapshot {
    pub fn validate(&self) -> Result<(), String> {
        if self.commit.len() != 40
            || !self.commit.bytes().all(|byte| byte.is_ascii_hexdigit())
            || self.start.0 == 0
            || self.start >= self.end
            || self.lines.is_empty()
            || self
                .lines
                .keys()
                .any(|line| *line == 0 || *line > self.end.0 - self.start.0 + 1)
        {
            return Err("invalid Git-backed coverage source snapshot".into());
        }
        Ok(())
    }

    pub fn resolve(&self, root: &Path) -> Result<Proof, String> {
        self.validate()?;
        let text = capture::git(
            root,
            &["show", &format!("{}:rust/{}", self.commit, self.file)],
        )?;
        let proof = Proof {
            file: self.file.clone(),
            first: self.start.0,
            text: slice(&text, self.start, self.end)?,
            lines: self.lines.clone(),
        };
        proof.validate()?;
        Ok(proof)
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
