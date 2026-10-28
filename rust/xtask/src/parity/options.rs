use super::*;

#[derive(Debug, Default)]
pub(super) struct Options {
    pub entry: Option<String>,
    pub out: Option<PathBuf>,
    pub base: Option<String>,
    pub recording: Option<PathBuf>,
    pub game_bin: Option<PathBuf>,
    pub allow_golden_update: bool,
}

impl Options {
    pub fn parse(command: &str, args: &[String]) -> Result<Self> {
        let mut result = Self::default();
        let mut seen = BTreeSet::new();
        let mut args = args.iter();
        while let Some(arg) = args.next() {
            if !seen.insert(arg.as_str()) {
                return Err(format!("duplicate parity option {arg}"));
            }
            match arg.as_str() {
                "--all" => (),
                "--allow-golden-update" => result.allow_golden_update = true,
                "--entry" => result.entry = Some(value(&mut args, arg)?),
                "--out" => result.out = Some(value(&mut args, arg)?.into()),
                "--base" => result.base = Some(value(&mut args, arg)?),
                "--recording" => result.recording = Some(value(&mut args, arg)?.into()),
                "--game-bin" => result.game_bin = Some(value(&mut args, arg)?.into()),
                _ => return Err(format!("unexpected parity option {arg}")),
            }
        }
        result.validate(command, &seen)?;
        Ok(result)
    }

    fn validate(&self, command: &str, seen: &BTreeSet<&str>) -> Result<()> {
        if seen.contains("--all") && self.entry.is_some() {
            return Err("--all and --entry are mutually exclusive".into());
        }
        if command == "record"
            && (self.out.is_none() || !seen.contains("--all") && self.entry.is_none())
        {
            return Err("record requires --out and --all or --entry".into());
        }
        if command != "check" && self.recording.is_some() {
            return Err("--recording is only valid for check".into());
        }
        if self.recording.is_some() && (self.out.is_some() || self.game_bin.is_some()) {
            return Err("--recording cannot be combined with --out or --game-bin".into());
        }
        if !matches!(command, "check" | "protect")
            && (self.base.is_some() || self.allow_golden_update)
        {
            return Err("golden protection options require check or protect".into());
        }
        if self.allow_golden_update && self.base.is_none() {
            return Err("--allow-golden-update requires --base".into());
        }
        self.validate_mode(command, seen)
    }

    fn validate_mode(&self, command: &str, seen: &BTreeSet<&str>) -> Result<()> {
        if command == "counts" && (!seen.contains("--all") || self.entry.is_some()) {
            return Err("counts requires --all".into());
        }
        if command == "protect" {
            if self.base.is_none() {
                return Err("protect requires --base".into());
            }
            if seen
                .iter()
                .any(|key| !matches!(*key, "--base" | "--allow-golden-update"))
            {
                return Err("protect accepts only --base and --allow-golden-update".into());
            }
        }
        Ok(())
    }

    pub fn output(&self, root: &Path, command: &str) -> PathBuf {
        self.out.clone().unwrap_or_else(|| {
            root.join(format!(
                "tmp/verify/parity-{command}-{}",
                std::process::id()
            ))
        })
    }
}

fn value(args: &mut std::slice::Iter<'_, String>, key: &str) -> Result<String> {
    let value = args
        .next()
        .ok_or_else(|| format!("{key} requires a value"))?;
    if value.is_empty() || value.starts_with('-') {
        return Err(format!("{key} requires a non-option value"));
    }
    Ok(value.clone())
}
