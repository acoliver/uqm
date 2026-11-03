use super::*;

const RACE_PREFIX: &str = "ships::races::";

/// The callbacks one converted melee species must run. `weapon` is its
/// `init_weapon` callback. `special` is the callback of the element its SPECIAL
/// input spawns. `None` marks a special implemented inline in a race callback
/// that has no identity of its own, which validates only with an `Absence`.
#[derive(Debug)]
pub(super) struct SpeciesCallbacks {
    pub species: u32,
    pub weapon: Option<&'static str>,
    pub special: Option<&'static str>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Kind {
    Weapon,
    Special,
}

/// A species that legitimately lacks a callback of `kind`, with the plan line that approves it.
#[derive(Debug)]
pub(super) struct Absence {
    pub species: u32,
    pub kind: Kind,
    pub plan: &'static str,
}

pub(super) const APPROVED_ABSENCES: &[Absence] = &[];

pub(super) const MELEE_SPECIES: [SpeciesCallbacks; 25] = [
    SpeciesCallbacks {
        species: 1,
        weapon: Some("arilou_native::initialize_autoaim_laser"),
        special: Some("arilou_native::special"),
    },
    SpeciesCallbacks {
        species: 2,
        weapon: Some("chmmr_native::initialize_megawatt_laser"),
        special: Some("chmmr_native::special"),
    },
    SpeciesCallbacks {
        species: 3,
        weapon: Some("human_native::initialize_nuke"),
        special: Some("human_native::spawn_point_defense"),
    },
    SpeciesCallbacks {
        species: 4,
        weapon: Some("orz_native::initialize_turret_missile"),
        special: Some("orz_native::marine_preprocess"),
    },
    SpeciesCallbacks {
        species: 5,
        weapon: Some("pkunk_native::initialize_bug_missile"),
        special: Some("pkunk_native::special"),
    },
    SpeciesCallbacks {
        species: 6,
        weapon: Some("shofixti_native::initialize_standard_missile"),
        special: Some("shofixti_native::destruct_preprocess"),
    },
    SpeciesCallbacks {
        species: 7,
        weapon: Some("spathi_native::initialize_standard_missile"),
        special: Some("spathi_native::butt_missile_preprocess"),
    },
    SpeciesCallbacks {
        species: 8,
        weapon: Some("supox_native::initialize_horn"),
        special: Some("supox_native::special"),
    },
    SpeciesCallbacks {
        species: 9,
        weapon: Some("thraddash_native::initialize_horn"),
        special: Some("thraddash_native::special"),
    },
    SpeciesCallbacks {
        species: 10,
        weapon: Some("utwig_native::initialize_lance"),
        special: Some("utwig_native::special"),
    },
    SpeciesCallbacks {
        species: 11,
        weapon: Some("vux_native::initialize_horrific_laser"),
        special: Some("vux_native::limpet_preprocess"),
    },
    SpeciesCallbacks {
        species: 12,
        weapon: Some("yehat_native::initialize_standard_missiles"),
        special: Some("yehat_native::special"),
    },
    SpeciesCallbacks {
        species: 13,
        weapon: Some("melnorme_native::initialize_pump_up"),
        special: Some("melnorme_native::initialize_confusion"),
    },
    SpeciesCallbacks {
        species: 14,
        weapon: Some("druuge_native::initialize_cannon"),
        special: Some("druuge_native::special"),
    },
    SpeciesCallbacks {
        species: 15,
        weapon: Some("ilwrath_native::initialize_flame"),
        special: Some("ilwrath_native::special"),
    },
    SpeciesCallbacks {
        species: 16,
        weapon: Some("mycon_native::initialize_plasma"),
        special: Some("mycon_native::plasma_blast_preprocess"),
    },
    SpeciesCallbacks {
        species: 17,
        weapon: Some("slylandro_native::initialize_lightning"),
        special: Some("slylandro_native::special"),
    },
    SpeciesCallbacks {
        species: 18,
        weapon: Some("umgah_native::initialize_cone"),
        special: Some("umgah_native::special"),
    },
    SpeciesCallbacks {
        species: 19,
        weapon: Some("urquan_native::initialize_fusion"),
        special: Some("urquan_native::fighter_preprocess"),
    },
    SpeciesCallbacks {
        species: 20,
        weapon: Some("zoqfot_native::initialize_spit"),
        special: Some("zoqfot_native::tongue_postprocess"),
    },
    SpeciesCallbacks {
        species: 21,
        weapon: Some("syreen_native::initialize_dagger"),
        special: Some("syreen_native::spawn_crew"),
    },
    SpeciesCallbacks {
        species: 22,
        weapon: Some("blackurq_native::initialize_buzzsaw"),
        special: Some("blackurq_native::gas_cloud_preprocess"),
    },
    SpeciesCallbacks {
        species: 23,
        weapon: Some("androsynth_native::initialize_bubble"),
        special: Some("androsynth_native::blazer_collision"),
    },
    SpeciesCallbacks {
        species: 24,
        weapon: Some("chenjesu_native::initialize_crystal"),
        special: Some("chenjesu_native::doggy_preprocess"),
    },
    SpeciesCallbacks {
        species: 25,
        weapon: Some("mmrnmhrm_native::initialize_dual_weapons"),
        special: Some("mmrnmhrm_native::special"),
    },
];

#[derive(Default)]
pub(super) struct Coverage {
    opponents: BTreeMap<u32, BTreeSet<u32>>,
    callbacks: BTreeMap<u32, BTreeSet<String>>,
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
        if let Some(callback) = name.strip_prefix(RACE_PREFIX) {
            self.record_callback(species, callback);
        }
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

    fn record_callback(&mut self, species: u32, callback: &str) {
        let seen = self.callbacks.entry(species).or_default();
        if !seen.contains(callback) {
            seen.insert(callback.to_owned());
        }
    }

    pub fn validate(&self) -> Result<()> {
        self.validate_with(&MELEE_SPECIES, APPROVED_ABSENCES)
    }

    pub(super) fn validate_with(
        &self,
        species: &[SpeciesCallbacks],
        approved: &[Absence],
    ) -> Result<()> {
        self.validate_bilateral()?;
        self.validate_callbacks(species, approved)
    }

    pub(super) fn validate_bilateral(&self) -> Result<()> {
        let missing: Vec<_> = (1..=25)
            .filter(|species| self.opponents.get(species).map_or(0, BTreeSet::len) < 2)
            .collect();
        if !missing.is_empty() {
            return Err(format!("25-ship coverage failed: species {missing:?} lack two actual opponents with live spawn and bilateral race-hook evidence; observed {:?}", self.opponents));
        }
        Ok(())
    }

    fn validate_callbacks(&self, species: &[SpeciesCallbacks], approved: &[Absence]) -> Result<()> {
        let mut problems = Vec::new();
        for absence in approved {
            if !species.iter().any(|entry| entry.species == absence.species) {
                problems.push(format!(
                    "approval {} names unknown species {}",
                    absence.plan, absence.species
                ));
            }
        }
        for entry in species {
            let seen = self.callbacks.get(&entry.species);
            for (kind, identity) in [(Kind::Weapon, entry.weapon), (Kind::Special, entry.special)] {
                let approval = approved
                    .iter()
                    .find(|absence| absence.species == entry.species && absence.kind == kind);
                match (identity, approval) {
                    (Some(name), None) => {
                        if !seen.is_some_and(|seen| seen.contains(name)) {
                            problems.push(format!(
                                "species {} {kind:?} callback {RACE_PREFIX}{name} never ran",
                                entry.species
                            ));
                        }
                    }
                    (None, None) => problems.push(format!(
                        "species {} declares no {kind:?} callback and has no approved absence",
                        entry.species
                    )),
                    (None, Some(_)) => (),
                    (Some(name), Some(absence)) => problems.push(format!(
                        "species {} {kind:?} callback {name} contradicts approved absence {}",
                        entry.species, absence.plan
                    )),
                }
            }
        }
        if !problems.is_empty() {
            return Err(format!("callback evidence failed: {}", problems.join("; ")));
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
