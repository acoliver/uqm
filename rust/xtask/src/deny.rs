use std::path::Path;
use std::process::Command;

pub const VERSION: &str = "0.20.2";

pub fn run(root: &Path, args: &[String]) -> Result<(), String> {
    if !args.is_empty() {
        return Err("usage: cargo xtask deny".into());
    }
    let version = Command::new(crate::quality::cargo())
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
    crate::quality::tool::status(
        Command::new(crate::quality::cargo())
            .current_dir(root.join("rust"))
            .args([
                "deny",
                "--locked",
                "check",
                "advisories",
                "bans",
                "licenses",
                "sources",
            ]),
    )?;
    println!("deny: 0 findings");
    Ok(())
}
