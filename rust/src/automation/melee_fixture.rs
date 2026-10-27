//! Bounded parity setups on the loaded game world. No fixture owns a global.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

mod live;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum FixtureName {
    AndrosynthBlazer,
    PkunkRebirth,
    ChmmrSatellites,
    DeathSpawn,
    MixedCollision,
    PoolFull,
    FrameSets,
    StarContact,
}

impl FixtureName {
    pub const ALL: [Self; 8] = [
        Self::AndrosynthBlazer,
        Self::PkunkRebirth,
        Self::ChmmrSatellites,
        Self::DeathSpawn,
        Self::MixedCollision,
        Self::PoolFull,
        Self::FrameSets,
        Self::StarContact,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Self::AndrosynthBlazer => "androsynth-blazer",
            Self::PkunkRebirth => "pkunk-rebirth",
            Self::ChmmrSatellites => "chmmr-satellites",
            Self::DeathSpawn => "death-spawn",
            Self::MixedCollision => "mixed-collision",
            Self::PoolFull => "pool-full",
            Self::FrameSets => "frame-sets",
            Self::StarContact => "star-contact",
        }
    }

    pub const fn ships(self) -> [u8; 2] {
        match self {
            Self::AndrosynthBlazer => [0, 5],
            Self::PkunkRebirth => [12, 5],
            Self::ChmmrSatellites => [3, 5],
            Self::DeathSpawn => [5, 13],
            Self::MixedCollision => [22, 11],
            Self::PoolFull => [20, 20],
            Self::FrameSets => [1, 5],
            Self::StarContact => [255, 255],
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct FixtureRequest {
    pub fixture: FixtureName,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MeleeSetupRequest {
    pub ships: [[u8; 7]; 2],
    pub cyborg: bool,
}

impl MeleeSetupRequest {
    pub fn validate(&self) -> Result<(), String> {
        for (side, fleet) in self.ships.iter().enumerate() {
            if !fleet.iter().any(|ship| *ship < 25) {
                return Err(format!("parity fleet {side} is empty"));
            }
            for &ship in fleet {
                if ship >= 25 && ship != 255 {
                    return Err(format!("invalid melee index {ship}; expected 0..24 or 255"));
                }
            }
        }
        Ok(())
    }
}

/// Four groups of seven. The last group repeats three ships, never a sentinel.
pub(crate) const FLEETS: [[u8; 7]; 4] = [
    [0, 1, 2, 3, 4, 5, 6],
    [7, 8, 9, 10, 11, 12, 13],
    [14, 15, 16, 17, 18, 19, 20],
    [21, 22, 23, 24, 0, 7, 14],
];

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CorpusRequest {
    pub fleet: u8,
}

impl CorpusRequest {
    pub fn validate(self) -> Result<(), String> {
        if usize::from(self.fleet) < FLEETS.len() {
            Ok(())
        } else {
            Err(format!(
                "invalid parity fleet {}; expected 0..3",
                self.fleet
            ))
        }
    }

    pub fn bouts(self) -> Result<Vec<[u8; 2]>, String> {
        self.validate()?;
        let fleet = FLEETS[usize::from(self.fleet)];
        Ok((1..=2)
            .flat_map(|offset| (0..7).map(move |slot| [fleet[slot], fleet[(slot + offset) % 7]]))
            .collect())
    }
}

/// Returned only after live predicates hold. Coordinator retains this receipt.
#[derive(Debug)]
pub(crate) struct FixtureRun {
    pub fixture: FixtureName,
    pub steps: u16,
    checks: BTreeSet<&'static str>,
    star: StarProgress,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SavedPosition {
    slot: u16,
    log_x: i32,
    log_y: i32,
    star: Option<(i16, i16)>,
}

#[derive(Debug, Default)]
struct StarProgress {
    phase: u8,
    saved: Option<SavedPosition>,
    loaded: bool,
}

impl FixtureRun {
    fn new(fixture: FixtureName) -> Self {
        Self {
            fixture,
            steps: 0,
            checks: BTreeSet::new(),
            star: StarProgress::default(),
        }
    }

    pub fn advance(&mut self) -> Result<(), String> {
        if self.fixture == FixtureName::StarContact {
            execute(Operation::Advance(self))?;
        }
        Ok(())
    }

    pub fn saved(&mut self, slot: u16) -> Result<(), String> {
        execute(Operation::SaveWitness(self, slot, false)).map(|_| ())
    }

    pub fn loaded(&mut self, slot: u16) -> Result<(), String> {
        execute(Operation::SaveWitness(self, slot, true)).map(|_| ())
    }

    fn require(&mut self, condition: bool, check: &'static str) -> Result<(), String> {
        if !condition {
            return Err(format!(
                "{}: live assertion failed: {check}",
                self.fixture.name()
            ));
        }
        self.checks.insert(check);
        Ok(())
    }

    pub fn verify(&self) -> Result<String, String> {
        let expected: &[&str] = match self.fixture {
            FixtureName::AndrosynthBlazer => &[
                "blazer-installed",
                "collision-restored",
                "ship-frames-restored",
            ],
            FixtureName::PkunkRebirth => &[
                "phoenix-installed",
                "reborn",
                "target-hack",
                "hooks-restored",
            ],
            FixtureName::ChmmrSatellites => {
                &["three-satellites", "delayed-defense", "defense-fired"]
            }
            FixtureName::DeathSpawn => {
                &["collision-killed", "death-hook", "spawn-during-traversal"]
            }
            FixtureName::MixedCollision => &["limpet-hit", "howitzer-hit"],
            FixtureName::PoolFull => &[
                "fighters-filled-pool",
                "allocation-failed",
                "pool-recovered",
            ],
            FixtureName::FrameSets => &[
                "orphan-weapon",
                "same-species-entry",
                "arilou-threat",
                "blast-teardown",
                "trilinear-index",
            ],
            FixtureName::StarContact => &[
                "star-contact",
                "repeat-suppressed",
                "system-exit",
                "save-reload",
            ],
        };
        let missing: Vec<_> = expected
            .iter()
            .filter(|check| !self.checks.contains(**check))
            .copied()
            .collect();
        if self.steps == 0
            || self.steps > 512
            || !missing.is_empty()
            || (self.fixture == FixtureName::StarContact && self.star.phase != 4)
        {
            return Err(format!(
                "{}: incomplete fixture after {} steps; missing {missing:?}",
                self.fixture.name(),
                self.steps
            ));
        }
        Ok(format!(
            "parity_fixture_verified:{}:steps={}:checks={}",
            self.fixture.name(),
            self.steps,
            expected.len()
        ))
    }
}

/// Coverage accepts only actual production spawn/hook observations. A manifest
/// or a requested pair alone never counts as a fought opponent.
#[derive(Debug, Default)]
pub(crate) struct CorpusCoverage {
    live: [Option<u32>; 2],
    opponents: BTreeMap<u32, BTreeSet<u32>>,
    fought: [bool; 2],
}

impl CorpusCoverage {
    pub fn spawn(&mut self, side: i16, species: u32) -> Result<(), String> {
        let side = usize::try_from(side)
            .ok()
            .filter(|side| *side < 2)
            .ok_or_else(|| format!("invalid battle side {side}"))?;
        self.live[side] = Some(species);
        self.fought = [false; 2];
        Ok(())
    }

    pub fn hook(&mut self, side: usize, name: &str) {
        if side > 1
            || !(name.ends_with("intelligence")
                || name.contains("initialize_")
                || name.ends_with("postprocess"))
        {
            return;
        }
        self.fought[side] = true;
        if self.fought == [true, true] {
            if let [Some(a), Some(b)] = self.live {
                if a != b {
                    self.opponents.entry(a).or_default().insert(b);
                    self.opponents.entry(b).or_default().insert(a);
                }
            }
        }
    }

    pub fn verify(&self, species: &[u32]) -> Result<(), String> {
        for id in species {
            let actual = self.opponents.get(id).map_or(0, BTreeSet::len);
            if actual < 2 {
                return Err(format!(
                    "species {id} fought {actual} distinct opponents; expected at least two"
                ));
            }
        }
        Ok(())
    }
}

pub(crate) fn configure(request: &MeleeSetupRequest) -> Result<(), String> {
    request.validate()?;
    crate::collections::observation::enable();
    execute(Operation::Configure(request)).map(|_| ())
}

pub(crate) fn run(request: &FixtureRequest) -> Result<FixtureRun, String> {
    crate::collections::observation::enable();
    let proof = execute(Operation::Fixture(request.fixture))?
        .ok_or("fixture operation returned no proof")?;
    if request.fixture != FixtureName::StarContact {
        proof.verify()?;
    }
    Ok(proof)
}

pub(crate) fn corpus(request: CorpusRequest) -> Result<(), String> {
    request.validate()?;
    crate::collections::observation::enable();
    execute(Operation::Corpus(request)).map(|_| ())
}

pub(crate) fn trigger() -> Result<(), String> {
    execute(Operation::MutualLoss).map(|_| ())
}

enum Operation<'a> {
    Configure(&'a MeleeSetupRequest),
    Fixture(FixtureName),
    Corpus(CorpusRequest),
    MutualLoss,
    Advance(&'a mut FixtureRun),
    SaveWitness(&'a mut FixtureRun, u16, bool),
}

#[cfg(not(test))]
use live::execute;
#[cfg(test)]
fn execute(_operation: Operation<'_>) -> Result<Option<FixtureRun>, String> {
    Err("parity fixtures require the loaded game on its input thread".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixture_names_are_closed_and_roundtrip() {
        for name in FixtureName::ALL {
            assert_eq!(
                serde_json::from_str::<FixtureName>(&serde_json::to_string(&name).unwrap())
                    .unwrap(),
                name
            );
            assert_eq!(serde_json::to_value(name).unwrap(), name.name());
        }
        assert!(serde_json::from_str::<FixtureName>("\"mutual-loss\"").is_err());
        assert!(
            serde_json::from_str::<FixtureRequest>(r#"{"fixture":"pool-full","steps":0}"#).is_err()
        );
    }

    #[test]
    fn setup_rejects_missing_opponents_and_invalid_ship_ids() {
        let mut request = MeleeSetupRequest {
            ships: [[255; 7]; 2],
            cyborg: true,
        };
        assert!(request.validate().is_err());
        request.ships[0][0] = 0;
        assert!(request.validate().is_err());
        request.ships[1][0] = 24;
        assert!(request.validate().is_ok());
        for invalid in [25, 26, 254] {
            request.ships[1][1] = invalid;
            assert!(request.validate().is_err());
        }
    }

    #[test]
    fn four_seven_ship_fleets_give_every_ship_two_distinct_opponents() {
        let mut opponents = BTreeMap::<u8, BTreeSet<u8>>::new();
        for fleet in 0..4 {
            let bouts = CorpusRequest { fleet }.bouts().unwrap();
            assert_eq!(bouts.len(), 14);
            for [a, b] in bouts {
                assert_ne!(a, b);
                opponents.entry(a).or_default().insert(b);
                opponents.entry(b).or_default().insert(a);
            }
        }
        assert_eq!(opponents.len(), 25);
        assert!(opponents.values().all(|opponents| opponents.len() >= 2));
        assert!(CorpusRequest { fleet: 4 }.bouts().is_err());
    }

    #[test]
    fn a_receipt_cannot_pass_without_each_live_assertion_and_bounded_steps() {
        let mut proof = FixtureRun::new(FixtureName::MixedCollision);
        assert!(proof.verify().is_err());
        proof.require(true, "limpet-hit").unwrap();
        proof.require(true, "howitzer-hit").unwrap();
        assert!(proof.verify().is_err());
        proof.steps = 2;
        assert!(proof.verify().is_ok());
        proof.steps = 513;
        assert!(proof.verify().is_err());
        assert!(proof.require(false, "not-observed").is_err());
        assert!(!proof.checks.contains("not-observed"));
    }

    #[test]
    fn requested_pairs_and_spawns_without_both_fighting_never_count() {
        let mut coverage = CorpusCoverage::default();
        coverage.spawn(0, 1).unwrap();
        coverage.spawn(1, 2).unwrap();
        assert!(coverage.verify(&[1]).is_err());
        coverage.hook(0, "ships::arilou::arilou_intelligence");
        assert!(coverage.verify(&[1]).is_err());
        coverage.hook(1, "ships::chmmr::chmmr_postprocess");
        assert!(coverage.verify(&[1]).is_err());
        coverage.spawn(1, 3).unwrap();
        coverage.hook(1, "ships::human::initialize_missile");
        assert!(coverage.verify(&[1]).is_err());
        coverage.hook(0, "ships::arilou::arilou_intelligence");
        coverage.verify(&[1]).unwrap();
        assert!(coverage.spawn(2, 1).is_err());
        assert!(coverage.verify(&[4]).is_err());
    }
    #[test]
    fn star_receipt_requires_the_loaded_mode_not_only_a_successful_load() {
        let mut proof = FixtureRun::new(FixtureName::StarContact);
        proof.steps = 2;
        for check in [
            "star-contact",
            "repeat-suppressed",
            "system-exit",
            "save-reload",
        ] {
            proof.require(true, check).unwrap();
        }
        proof.star.phase = 3;
        assert!(proof.verify().is_err());
        proof.star.phase = 4;
        assert!(proof.verify().is_ok());
    }

    #[test]
    fn new_scripts_use_closed_requests_and_keep_seed_and_budget_contracts() {
        let scripts = [
            include_str!("../../scripts/parity-androsynth-blazer.json"),
            include_str!("../../scripts/parity-chmmr-satellites.json"),
            include_str!("../../scripts/parity-death-spawn.json"),
            include_str!("../../scripts/parity-frame-sets.json"),
            include_str!("../../scripts/parity-melee-cyborg-a.json"),
            include_str!("../../scripts/parity-melee-cyborg-b.json"),
            include_str!("../../scripts/parity-melee-cyborg-c.json"),
            include_str!("../../scripts/parity-melee-cyborg-d.json"),
            include_str!("../../scripts/parity-mixed-collision.json"),
            include_str!("../../scripts/parity-new-game.json"),
            include_str!("../../scripts/parity-pkunk-rebirth.json"),
            include_str!("../../scripts/parity-pool-full.json"),
            include_str!("../../scripts/parity-samatra.json"),
            include_str!("../../scripts/parity-star-contact.json"),
        ];
        assert_eq!(scripts.len(), 14);
        let mut names = BTreeSet::new();
        let mut fixture_names = BTreeSet::new();
        let mut fleets = BTreeSet::new();
        for source in scripts {
            let document: serde_json::Value = serde_json::from_str(source).unwrap();
            assert!(names.insert(document["name"].as_str().unwrap().to_string()));
            assert_eq!(document["seed"], 42);
            let parsed = crate::automation::script::parse_script(
                source.as_bytes(),
                std::path::Path::new("fixture.json"),
            )
            .unwrap();
            let parsed = crate::automation::script::validate_script(
                parsed,
                std::path::Path::new("fixture.json"),
            )
            .unwrap();
            assert_eq!(document["budgets"]["max_input_ticks"], 12000);
            assert_eq!(
                document["steps"].as_array().unwrap().last().unwrap()["action"],
                "finish"
            );
            for step in parsed.steps {
                use crate::automation::script::Action;
                match step {
                    Action::ConfigureParityMelee(request) => request.validate().unwrap(),
                    Action::RunParityFixture(request) => {
                        fixture_names.insert(request.fixture.name());
                    }
                    Action::RunParityCorpus(request) => {
                        request.validate().unwrap();
                        fleets.insert(request.fleet);
                    }
                    _ => {}
                }
            }
        }
        assert_eq!(
            fixture_names,
            FixtureName::ALL
                .map(FixtureName::name)
                .into_iter()
                .collect()
        );
        assert_eq!(fleets, [0, 1, 2, 3].into_iter().collect());
    }
}
