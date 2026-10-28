use super::*;
use toml_edit::{DocumentMut, Table};

pub(super) const CORPUS: &[&str] = &[
    "melee-cyborg-a",
    "melee-cyborg-b",
    "melee-cyborg-c",
    "melee-cyborg-d",
    "melee-input",
    "campaign-battle",
    "samatra",
    "hyperspace",
    "solar-flight",
    "lander",
    "comm",
    "new-game",
];
pub(super) const FIXTURES: &[&str] = &[
    "androsynth-blazer",
    "pkunk-rebirth",
    "chmmr-satellites",
    "death-spawn",
    "mixed-collision",
    "pool-full",
    "frame-sets",
    "star-contact",
];

#[derive(Debug, Clone)]
pub(super) struct Entry {
    pub name: String,
    pub scenario: String,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Classification {
    Success,
    ExpectedFailure,
    KnownBug,
    UnexpectedFailure,
}

#[derive(Debug, Clone)]
pub(super) struct Expectation {
    pub classification: Classification,
    pub terminal: counts::Terminal,
    pub issue: Option<u64>,
}

pub(super) struct Manifest {
    pub entries: Vec<Entry>,
    pub expectations: BTreeMap<String, Expectation>,
}

pub(super) fn read(root: &Path) -> Result<Manifest> {
    parse(
        &String::from_utf8(super::read(&root.join("rust/xtask/parity-corpus.toml"))?)
            .map_err(|e| e.to_string())?,
    )
}

pub(super) fn parse(text: &str) -> Result<Manifest> {
    let doc = text
        .parse::<DocumentMut>()
        .map_err(|error| format!("corpus TOML: {error}"))?;
    if doc
        .iter()
        .any(|(key, _)| !matches!(key, "entry" | "expectation"))
    {
        return Err("unknown corpus manifest table".into());
    }
    let tables = doc
        .get("entry")
        .and_then(|item| item.as_array_of_tables())
        .ok_or("manifest lacks [[entry]]")?;
    let mut entries = Vec::new();
    let mut names = BTreeSet::new();
    let mut scenarios = BTreeSet::new();
    for table in tables {
        keys(table, &["name", "scenario", "outcome"])?;
        let name = field(table, "name")?;
        let scenario = field(table, "scenario")?;
        if field(table, "outcome")? != "success" {
            return Err("corpus and fixtures must succeed".into());
        }
        if !identifier(&name) || !identifier(&scenario) {
            return Err("invalid corpus identifier".into());
        }
        if !names.insert(name.clone()) || !scenarios.insert(scenario.clone()) {
            return Err("duplicate corpus name or scenario".into());
        }
        entries.push(Entry { name, scenario });
    }
    let required: BTreeSet<_> = CORPUS.iter().chain(FIXTURES).copied().collect();
    if names.iter().map(String::as_str).collect::<BTreeSet<_>>() != required {
        return Err(
            "manifest must register all 12 corpus entries and 8 fixtures exactly once".into(),
        );
    }
    validate_mappings(&entries)?;
    let expectations = parse_expectations(&doc)?;
    if entries
        .iter()
        .any(|entry| expectations.contains_key(&entry.scenario))
    {
        return Err("corpus/fixture scenarios cannot be classified as failures".into());
    }
    Ok(Manifest {
        entries,
        expectations,
    })
}

fn validate_mappings(entries: &[Entry]) -> Result<()> {
    for entry in entries {
        let expected = match entry.name.as_str() {
            "melee-input" => "super-melee-v1".into(),
            "campaign-battle" => "lost-battle-v1".into(),
            "hyperspace" => "out-of-fuel-v1".into(),
            "solar-flight" => "real-sol-starbase-to-mars".into(),
            "lander" => "lander-destroyed-v1".into(),
            "comm" => "comm-encounter-v1".into(),
            name => format!("parity-{name}"),
        };
        if entry.scenario != expected {
            return Err(format!(
                "{} must use scenario {expected}, got {}",
                entry.name, entry.scenario
            ));
        }
    }
    Ok(())
}

fn parse_expectations(doc: &DocumentMut) -> Result<BTreeMap<String, Expectation>> {
    let mut result = BTreeMap::new();
    let Some(item) = doc.get("expectation") else {
        return Ok(result);
    };
    for table in item
        .as_array_of_tables()
        .ok_or("expectation must be array of tables")?
    {
        keys(table, &["scenario", "classification", "terminal", "issue"])?;
        let scenario = field(table, "scenario")?;
        let classification = match field(table, "classification")?.as_str() {
            "expected_failure" => Classification::ExpectedFailure,
            "known_bug" => Classification::KnownBug,
            _ => {
                return Err("expectations must distinguish expected_failure from known_bug".into())
            }
        };
        let terminal = serde_json::from_value(serde_json::Value::String(field(table, "terminal")?))
            .map_err(|error| format!("invalid expectation terminal: {error}"))?;
        if terminal == counts::Terminal::Success {
            return Err("failure expectation cannot expect success".into());
        }
        let issue = table
            .get("issue")
            .map(|item| {
                item.as_integer()
                    .and_then(|v| u64::try_from(v).ok())
                    .filter(|v| *v > 0)
                    .ok_or("invalid issue number")
            })
            .transpose()?;
        if classification == Classification::KnownBug && issue.is_none() {
            return Err("known_bug requires issue number".into());
        }
        if !identifier(&scenario)
            || result
                .insert(
                    scenario,
                    Expectation {
                        classification,
                        terminal,
                        issue,
                    },
                )
                .is_some()
        {
            return Err("invalid or duplicate expectation scenario".into());
        }
    }
    Ok(result)
}

fn keys(table: &Table, allowed: &[&str]) -> Result<()> {
    if let Some((key, _)) = table.iter().find(|(key, _)| !allowed.contains(key)) {
        return Err(format!("unknown manifest key {key}"));
    }
    Ok(())
}

fn field(table: &Table, key: &str) -> Result<String> {
    table
        .get(key)
        .and_then(|item| item.as_str())
        .map(str::to_owned)
        .ok_or_else(|| format!("manifest lacks string {key}"))
}

pub(super) fn identifier(text: &str) -> bool {
    !text.is_empty()
        && text
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
}

impl Manifest {
    pub fn select(&self, entry: Option<&str>) -> Result<Vec<Entry>> {
        match entry {
            None => Ok(self.entries.clone()),
            Some(name) => self
                .entries
                .iter()
                .find(|entry| entry.name == name)
                .cloned()
                .map(|entry| vec![entry])
                .ok_or_else(|| format!("unregistered parity entry {name}")),
        }
    }

    pub fn expected_files(&self, root: &Path, entry: Option<&str>) -> Result<BTreeSet<String>> {
        let mut paths = BTreeSet::new();
        for entry in self.select(entry)? {
            for file in [TRACE, FIELDS, "summary.json"] {
                paths.insert(format!("parity/{}/{file}", entry.name));
            }
        }
        if entry.is_none() {
            for name in script_names(root)? {
                paths.insert(format!("parity/counts/{name}.toml"));
            }
            for name in save_names(root)? {
                paths.insert(format!("saves/{name}.capture"));
                paths.insert(format!("saves/{name}.capture.json"));
            }
        }
        Ok(paths)
    }
}

pub(super) fn save_names(root: &Path) -> Result<BTreeSet<String>> {
    let mut names = BTreeSet::new();
    for entry in fs::read_dir(root.join("rust/scripts/saves")).map_err(|error| error.to_string())? {
        let path = entry.map_err(|error| error.to_string())?.path();
        if path.extension().is_some_and(|ext| ext == "sav") {
            names.insert(
                path.file_name()
                    .and_then(|v| v.to_str())
                    .ok_or("invalid save filename")?
                    .into(),
            );
        }
    }
    if names.len() != 73 {
        return Err(format!(
            "V3b requires 73 .sav inputs, found {}",
            names.len()
        ));
    }
    super::read(&root.join("rust/tests/fixtures/savegame/legacy-save.00"))?;
    names.insert("legacy-save.00".into());
    Ok(names)
}
