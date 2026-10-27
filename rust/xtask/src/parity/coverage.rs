use super::*;

#[derive(Default)]
pub(super) struct Coverage {
    opponents: BTreeMap<u32, BTreeSet<u32>>,
}

#[derive(Default)]
struct Battle {
    live: [Option<u32>; 2],
    fought: [bool; 2],
}

pub(super) fn check(recording: &Path) -> Result<()> {
    let mut coverage = Coverage::default();
    for fleet in ["a", "b", "c", "d"] {
        let path = recording.join(format!("parity/melee-cyborg-{fleet}/{TRACE}"));
        coverage.observe(std::str::from_utf8(&read(&path)?).map_err(|error| error.to_string())?)?;
    }
    coverage.validate()
}

impl Coverage {
    pub fn observe(&mut self, text: &str) -> Result<()> {
        let mut battle = Battle::default();
        let mut steps = 0;
        for line in text.lines().filter(|line| line.starts_with("S ")) {
            steps += 1;
            for fragment in line.split('[').skip(1) {
                let (event, _) = fragment.split_once(']').ok_or("unclosed parity event")?;
                self.event(&mut battle, &event.split_whitespace().collect::<Vec<_>>())?;
            }
        }
        if steps == 0 {
            return Err("cyborg recording contains no S checkpoints".into());
        }
        Ok(())
    }

    fn event(&mut self, battle: &mut Battle, words: &[&str]) -> Result<()> {
        match words {
            ["spawn", side, species] => {
                let (side, species) = identity(side, species)?;
                // Bounded bouts tear down living ships without a death event.
                battle.live[side] = Some(species);
                battle.fought = [false; 2];
            }
            ["death", side, species] => {
                let (side, species) = identity(side, species)?;
                if battle.live[side] != Some(species) {
                    return Err(format!(
                        "death without matching live spawn: {side} {species}"
                    ));
                }
                battle.live[side] = None;
                battle.fought = [false; 2];
            }
            ["hook", species, name] => {
                let species = species.parse::<u32>().map_err(|error| error.to_string())?;
                if !name.contains("::") {
                    return Err(format!("hook lacks registry-qualified name: {name}"));
                }
                self.hook(battle, species, name);
            }
            ["free", identity] => cleanup_identity(identity, false)?,
            ["drawable-free", identity] => cleanup_identity(identity, true)?,
            ["pool-full"] => (),
            ["space-active", "true" | "false"] => (),
            _ => return Err(format!("malformed parity event: {words:?}")),
        }
        Ok(())
    }

    fn hook(&mut self, battle: &mut Battle, species: u32, name: &str) {
        if !name.starts_with("ships::")
            || !(name.ends_with("intelligence")
                || name.contains("::initialize_")
                || name.ends_with("postprocess"))
        {
            return;
        }
        let [Some(left), Some(right)] = battle.live else {
            return;
        };
        if left == right {
            return;
        }
        battle.fought[0] |= species == left;
        battle.fought[1] |= species == right;
        if battle.fought == [true, true] {
            self.opponents.entry(left).or_default().insert(right);
            self.opponents.entry(right).or_default().insert(left);
        }
    }

    pub fn validate(&self) -> Result<()> {
        let missing: Vec<_> = (1..=25)
            .filter(|species| self.opponents.get(species).map_or(0, BTreeSet::len) < 2)
            .collect();
        if !missing.is_empty() {
            return Err(format!("25-ship coverage failed: species {missing:?} lack two actual opponents with live spawn and bilateral race-hook evidence; observed {:?}", self.opponents));
        }
        Ok(())
    }
}

fn identity(side: &str, species: &str) -> Result<(usize, u32)> {
    let side: usize = side
        .parse()
        .map_err(|error| format!("invalid event side: {error}"))?;
    let species: u32 = species
        .parse()
        .map_err(|error| format!("invalid event species: {error}"))?;
    if side > 1 || !(1..=25).contains(&species) {
        return Err(format!("invalid cyborg ship identity {side} {species}"));
    }
    Ok((side, species))
}

fn cleanup_identity(value: &str, drawable: bool) -> Result<()> {
    if value == "none" {
        return Ok(());
    }
    let (prefix, number) = value.split_once(':').ok_or("invalid cleanup identity")?;
    if drawable {
        if prefix != "drawable" {
            return Err(format!("invalid drawable cleanup identity: {value}"));
        }
    } else {
        event_number(prefix)?;
    }
    let number = event_number(number)?;
    if !drawable && number == 0 {
        return Err("element cleanup generation must be positive".into());
    }
    Ok(())
}

fn event_number(value: &str) -> Result<u64> {
    if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(format!("invalid cleanup identity number: {value}"));
    }
    value
        .parse()
        .map_err(|error| format!("invalid cleanup identity number: {error}"))
}
