use std::path::Path;
use std::process::Command;

pub const VERSION: &str = "0.20.2";

pub fn run(root: &Path, args: &[String]) -> Result<(), String> {
    run_with_cargo(root, args, std::ffi::OsStr::new(&crate::quality::cargo()))
}

fn run_with_cargo(root: &Path, args: &[String], cargo: &std::ffi::OsStr) -> Result<(), String> {
    if !args.is_empty() {
        return Err("usage: cargo xtask deny".into());
    }
    let version = Command::new(cargo)
        .args(["deny", "--version"])
        .output()
        .map_err(|e| format!("cargo-deny: {e}"))?;
    let actual = String::from_utf8_lossy(&version.stdout);
    if !version.status.success() || actual.trim() != format!("cargo-deny {VERSION}") {
        return Err(format!(
            "install cargo-deny {VERSION} with --locked; found {}",
            actual.trim()
        ));
    }
    crate::quality::tool::status(Command::new(cargo).current_dir(root.join("rust")).args([
        "deny",
        "--locked",
        "check",
        "advisories",
        "bans",
        "licenses",
        "sources",
    ]))?;
    println!("deny: 0 findings");
    Ok(())
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use crate::quality::tool::fixture_program;

    #[test]
    fn pinned_tool_runs_all_supply_checks_and_propagates_failures() {
        let root = crate::quality::tool::fixture_directory();
        std::fs::create_dir(root.path().join("rust")).unwrap();
        let cargo = fixture_program(
            root.path(),
            r#"
if [ "$*" = 'deny --version' ]; then echo 'cargo-deny 0.20.2'; exit 0; fi
[ "$*" = 'deny --locked check advisories bans licenses sources' ]
"#,
        );
        assert!(run_with_cargo(root.path(), &[], cargo.as_os_str()).is_ok());
        let cargo = fixture_program(root.path(), "echo cargo-deny wrong-version");
        let error = run_with_cargo(root.path(), &[], cargo.as_os_str()).unwrap_err();
        assert!(error.contains("install cargo-deny"), "{error}");
        let cargo = fixture_program(
            root.path(),
            r#"
if [ "$*" = 'deny --version' ]; then echo 'cargo-deny 0.20.2'; exit 0; fi
exit 1
"#,
        );
        assert!(run_with_cargo(root.path(), &[], cargo.as_os_str()).is_err());
        assert!(run(root.path(), &["unexpected".into()]).is_err());
        assert!(run_with_cargo(root.path(), &[], root.path().join("missing").as_os_str()).is_err());
    }
}
