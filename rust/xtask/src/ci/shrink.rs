//! Pure "native code may only shrink" rules for the zero-native delta gate.
//!
//! A changed native path passes when it was deleted or when every hunk of its
//! base-to-head diff only rearranges or removes tokens. A path added at head
//! always fails. Manifest entries may be removed or have their content change,
//! but no entry may appear that the base did not have. The delta evaluator
//! separately verifies added measurements of an existing ABI retired to Rust;
//! it never applies that source-proof exception to objects or providers.

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    Pass,
    Fail,
    Waived,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PathVerdict {
    pub path: String,
    pub verdict: Verdict,
    pub reason: String,
}

impl PathVerdict {
    fn new(path: &str, verdict: Verdict, reason: impl Into<String>) -> Self {
        Self {
            path: path.to_string(),
            verdict,
            reason: reason.into(),
        }
    }
}

/// Turn every failure into a waived pass; the entry stays listed in the report.
pub fn waive_failures(verdicts: &mut [PathVerdict]) {
    for verdict in verdicts {
        if verdict.verdict == Verdict::Fail {
            verdict.verdict = Verdict::Waived;
        }
    }
}

/// Split into word tokens (alphanumerics and `_`) and single punctuation
/// tokens. Whitespace separates tokens and is otherwise ignored.
fn tokens(text: &str) -> impl Iterator<Item = String> + '_ {
    let mut chars = text.char_indices().peekable();
    std::iter::from_fn(move || {
        while let Some(&(_, c)) = chars.peek() {
            if c.is_whitespace() {
                chars.next();
            } else {
                break;
            }
        }
        let (start, first) = chars.next()?;
        if !is_word_char(first) {
            return Some(first.to_string());
        }
        let mut end = start + first.len_utf8();
        while let Some(&(index, c)) = chars.peek() {
            if !is_word_char(c) {
                break;
            }
            end = index + c.len_utf8();
            chars.next();
        }
        Some(text[start..end].to_string())
    })
}

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

fn token_counts<'a>(lines: impl Iterator<Item = &'a str>) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for line in lines {
        for token in tokens(line) {
            *counts.entry(token).or_insert(0) += 1;
        }
    }
    counts
}

struct Hunk<'a> {
    header: &'a str,
    added: Vec<&'a str>,
    removed: Vec<&'a str>,
}

fn hunks(diff: &str) -> Vec<Hunk<'_>> {
    let mut result: Vec<Hunk<'_>> = Vec::new();
    for line in diff.lines() {
        if line.starts_with("@@") {
            result.push(Hunk {
                header: line,
                added: Vec::new(),
                removed: Vec::new(),
            });
        } else if let Some(hunk) = result.last_mut() {
            if let Some(text) = line.strip_prefix('+') {
                hunk.added.push(text);
            } else if let Some(text) = line.strip_prefix('-') {
                hunk.removed.push(text);
            }
        }
    }
    result
}

/// Every hunk's added tokens must be a sub-multiset of that hunk's removed
/// tokens. Returns the first offending hunk's description otherwise.
pub fn removal_only(diff: &str) -> Result<(), String> {
    let hunks = hunks(diff);
    if hunks.is_empty() {
        return Err("changed without a textual hunk (binary or mode-only change)".into());
    }
    for hunk in hunks {
        let removed = token_counts(hunk.removed.iter().copied());
        let added = token_counts(hunk.added.iter().copied());
        let mut novel = added
            .iter()
            .filter(|(token, count)| **count > removed.get(*token).copied().unwrap_or(0))
            .map(|(token, _)| token.as_str())
            .collect::<Vec<_>>();
        // Identifiers explain a failure better than punctuation does.
        novel.sort_by_key(|token| !token.chars().all(is_word_char));
        novel.truncate(8);
        if !novel.is_empty() {
            return Err(format!(
                "{} adds tokens not removed in the same hunk: {}",
                hunk.header,
                novel.join(" ")
            ));
        }
    }
    Ok(())
}

/// Verdict for a path-based category entry. `diff` is evaluated only when the
/// path exists on both sides.
pub fn path_verdict(
    path: &str,
    in_base: bool,
    in_head: bool,
    diff: impl FnOnce() -> Result<String, String>,
) -> Result<PathVerdict, String> {
    Ok(match (in_base, in_head) {
        (_, false) => PathVerdict::new(path, Verdict::Pass, "deleted at head"),
        (false, true) => PathVerdict::new(path, Verdict::Fail, "added at head"),
        (true, true) => match removal_only(&diff()?) {
            Ok(()) => PathVerdict::new(path, Verdict::Pass, "removal-only edit"),
            Err(reason) => PathVerdict::new(path, Verdict::Fail, reason),
        },
    })
}

/// Verdicts for a manifest section. Identities that exist only at head fail;
/// otherwise any difference (removals or content changes of kept entries) is
/// one passing verdict. Equal sections yield no verdicts.
pub fn manifest_verdicts(
    path: &str,
    base: &BTreeSet<String>,
    head: &BTreeSet<String>,
    kept_content_changed: bool,
) -> Vec<PathVerdict> {
    let added = head.difference(base).collect::<Vec<_>>();
    if !added.is_empty() {
        return added
            .into_iter()
            .map(|identity| PathVerdict::new(path, Verdict::Fail, format!("new entry {identity}")))
            .collect();
    }
    if base == head && !kept_content_changed {
        return Vec::new();
    }
    let removed = base.difference(head).count();
    vec![PathVerdict::new(
        path,
        Verdict::Pass,
        format!("{removed} entries removed, no entries added"),
    )]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn verdict(in_base: bool, in_head: bool, diff: &str) -> PathVerdict {
        path_verdict("sc2/src/x.c", in_base, in_head, || Ok(diff.to_string())).unwrap()
    }

    fn set(items: &[&str]) -> BTreeSet<String> {
        items.iter().map(|item| item.to_string()).collect()
    }

    #[test]
    fn deleted_file_passes_without_reading_its_diff() {
        let result = path_verdict("sc2/src/x.c", true, false, || {
            Err("diff must not be read".into())
        })
        .unwrap();
        assert_eq!(result.verdict, Verdict::Pass);
        assert_eq!(result.reason, "deleted at head");
    }

    #[test]
    fn new_file_fails_even_with_no_novel_tokens() {
        let result = path_verdict("sc2/src/new.c", false, true, || Ok(String::new())).unwrap();
        assert_eq!(result.verdict, Verdict::Fail);
        assert_eq!(result.reason, "added at head");
    }

    #[test]
    fn makeinfo_file_list_shrink_passes() {
        let diff = "@@ -3 +3 @@\n-uqm_CFILES=\"decoder.c aiffaud.c wav.c dukaud.c modaud.c\"\n+uqm_CFILES=\"decoder.c aiffaud.c wav.c dukaud.c\"\n";
        assert_eq!(verdict(true, true, diff).verdict, Verdict::Pass);
    }

    #[test]
    fn include_whitespace_normalisation_passes() {
        let diff = "@@ -10,2 +10 @@\n-#\tinclude \"rust_mod.h\"\n-extern int gone;\n+#include \"rust_mod.h\"\n";
        assert_eq!(verdict(true, true, diff).verdict, Verdict::Pass);
    }

    #[test]
    fn shortened_comment_passes() {
        let diff = "@@ -20 +20 @@\n-/* Decode MOD files through the Rust mod decoder. */\n+/* Decode MOD files. */\n";
        assert_eq!(verdict(true, true, diff).verdict, Verdict::Pass);
    }

    #[test]
    fn added_extern_and_call_fail() {
        let diff = "@@ -5,0 +6 @@\n+extern void rust_automation_observe_ship_picker(void);\n@@ -90,0 +92 @@\n+\trust_automation_observe_ship_picker();\n";
        let result = verdict(true, true, diff);
        assert_eq!(result.verdict, Verdict::Fail);
        assert!(result
            .reason
            .contains("rust_automation_observe_ship_picker"));
    }

    #[test]
    fn a_token_only_removed_in_another_hunk_cannot_be_reused() {
        let diff = "@@ -1 +0,0 @@\n-int moved_symbol;\n@@ -50,0 +49 @@\n+int moved_symbol;\n";
        assert_eq!(verdict(true, true, diff).verdict, Verdict::Fail);
    }

    #[test]
    fn repeated_token_beyond_removed_count_fails() {
        let diff = "@@ -1 +1 @@\n-f(a);\n+f(a); f(a);\n";
        assert_eq!(verdict(true, true, diff).verdict, Verdict::Fail);
    }

    #[test]
    fn reordered_hunk_with_the_same_tokens_passes() {
        let diff = "@@ -4,2 +4,2 @@\n-int a = 1;\n-int b = 2;\n+int b = 2;\n+int a = 1;\n";
        assert_eq!(verdict(true, true, diff).verdict, Verdict::Pass);
    }

    #[test]
    fn change_without_a_hunk_fails() {
        let result = verdict(true, true, "Binary files a/x and b/x differ\n");
        assert_eq!(result.verdict, Verdict::Fail);
    }

    #[test]
    fn file_header_lines_are_not_added_content() {
        let diff = "diff --git a/x b/x\n--- a/x\n+++ b/x\n@@ -1 +0,0 @@\n-gone\n";
        assert_eq!(verdict(true, true, diff).verdict, Verdict::Pass);
    }

    #[test]
    fn manifest_entry_removal_passes() {
        let result = manifest_verdicts("m.json", &set(&["a", "b"]), &set(&["a"]), false);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].verdict, Verdict::Pass);
    }

    #[test]
    fn manifest_entry_addition_fails_per_entry() {
        let result = manifest_verdicts("m.json", &set(&["a"]), &set(&["a", "b", "c"]), false);
        assert_eq!(result.len(), 2);
        assert!(result.iter().all(|entry| entry.verdict == Verdict::Fail));
        assert!(result[0].reason.contains("new entry b"));
    }

    #[test]
    fn manifest_hash_change_of_kept_entry_passes() {
        let result = manifest_verdicts("m.json", &set(&["a"]), &set(&["a"]), true);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].verdict, Verdict::Pass);
    }

    #[test]
    fn identical_manifest_section_has_no_verdicts() {
        assert!(manifest_verdicts("m.json", &set(&["a"]), &set(&["a"]), false).is_empty());
    }

    #[test]
    fn waiver_converts_failures_and_keeps_them_listed() {
        let mut verdicts = vec![
            PathVerdict::new("a.c", Verdict::Fail, "added at head"),
            PathVerdict::new("b.c", Verdict::Pass, "deleted at head"),
        ];
        waive_failures(&mut verdicts);
        assert_eq!(verdicts[0].verdict, Verdict::Waived);
        assert_eq!(verdicts[0].reason, "added at head");
        assert_eq!(verdicts[1].verdict, Verdict::Pass);
    }
}
