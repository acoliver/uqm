use std::path::Path;

use crate::quality::tool;

/// Runs Clippy over every workspace target with the feature set CI uses.
///
/// Lint levels live in `[workspace.lints]` in `rust/Cargo.toml`, so any hit,
/// including `unwrap_used`, `expect_used`, `undocumented_unsafe_blocks`,
/// `mem_forget` and `unsafe_op_in_unsafe_fn`, fails the run.
pub fn run(root: &Path, args: &[String]) -> Result<(), String> {
    if let Some(arg) = args.first() {
        return Err(format!("unexpected clippy argument {arg}"));
    }
    tool::status(
        tool::command(crate::quality::cargo())
            .current_dir(root.join("rust"))
            .args([
                "clippy",
                "--workspace",
                "--all-targets",
                "--locked",
                "--no-default-features",
                "--features",
                "audio_heart,debug-process,native",
                "--",
                "-D",
                "warnings",
            ]),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arguments_are_rejected_before_cargo_runs() {
        let error = run(Path::new("/nonexistent"), &["--tighten".into()]).unwrap_err();
        assert!(error.contains("--tighten"), "{error}");
    }
}
