//! Reader for commit-message waiver lines such as `Adds-native: <reason>`.
//!
//! A waiver line starts exactly with `<Key>: ` at column zero and carries a
//! non-empty reason. Waivers are ledger-visible escape hatches: agents may only
//! write one with Andrew's explicit approval.

/// Reasons from every message line that starts exactly with `<key>: `.
pub fn waiver_reasons(message: &str, key: &str) -> Vec<String> {
    let prefix = format!("{key}: ");
    message
        .lines()
        .filter_map(|line| line.strip_prefix(&prefix))
        .map(str::trim)
        .filter(|reason| !reason.is_empty())
        .map(str::to_string)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_exact_prefix_lines_with_non_empty_reasons() {
        let message = "Subject\n\nBody\nAdds-native: bootstrap shim approved by Andrew\n";
        assert_eq!(
            waiver_reasons(message, "Adds-native"),
            ["bootstrap shim approved by Andrew"]
        );
    }

    #[test]
    fn ignores_empty_indented_lowercase_and_inline_mentions() {
        let message = "Adds-native:\nAdds-native:   \n Adds-native: indented\nadds-native: lower\nsee Adds-native: inline\nAdds-native:nospace";
        assert!(waiver_reasons(message, "Adds-native").is_empty());
    }
}
