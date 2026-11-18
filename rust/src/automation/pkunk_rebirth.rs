//! Ordered observations of one live Pkunk, without invoking its lifecycle hooks.

use crate::engine::observe::battle::{BattlePhase, BattleShip, BattleTick};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Dead,
    Restored,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct WaitStep {
    pub side: u8,
    pub phase: Phase,
    pub max_ticks: u64,
}

#[derive(Debug, Clone, Copy)]
struct Living {
    identity: [u32; 2],
    tick: u64,
    counters: [u8; 2],
}

#[derive(Debug, Clone, Copy)]
pub(super) struct Proof {
    pub side: u8,
    pub identity: [u32; 2],
    pub alive_tick: u64,
    pub death_tick: u64,
    pub restored_tick: Option<u64>,
    pub crew: u16,
}

impl Proof {
    pub fn trace_label(self) -> String {
        let prefix = if self.restored_tick.is_some() {
            "pkunk_restored"
        } else {
            "pkunk_dead"
        };
        let mut label = format!(
            "{prefix}:side={}:slot={}:generation={}:alive_tick={}:death_tick={}:crew={}",
            self.side,
            self.identity[0],
            self.identity[1],
            self.alive_tick,
            self.death_tick,
            self.crew
        );
        if let Some(tick) = self.restored_tick {
            label.push_str(&format!(":restored_tick={tick}"));
        }
        label
    }
}

#[derive(Debug, Default)]
pub(super) struct Tracker {
    side: Option<u8>,
    living: Option<Living>,
    death: Option<Proof>,
    restored: Option<Proof>,
    last_tick: Option<u64>,
}

impl Tracker {
    pub fn begin(&mut self, side: u8) {
        if self.side.is_none() {
            self.side = Some(side);
        }
    }

    pub fn observe(&mut self, tick: u64, sample: &BattleTick) {
        let Some(side) = self.side else { return };
        if sample.phase != BattlePhase::Advanced || self.last_tick.is_some_and(|last| tick <= last)
        {
            return;
        }
        self.last_tick = Some(tick);
        let ship = sample.ships[usize::from(side)];
        self.observe_ship(side, tick, sample.counters, ship);
    }

    fn observe_ship(&mut self, side: u8, tick: u64, counters: [u8; 2], ship: Option<BattleShip>) {
        let Some(ship) =
            ship.filter(|ship| ship.species == crate::ships::native::abi::SPECIES_ID_PKUNK_ID)
        else {
            self.living = None;
            self.death = None;
            self.restored = None;
            return;
        };
        if self
            .living
            .is_some_and(|living| living.identity != ship.identity || living.counters != counters)
        {
            self.living = None;
            self.death = None;
            self.restored = None;
        }
        if ship.crew > 0 {
            if let Some(death) = self.death {
                if tick > death.death_tick && self.restored.is_none() {
                    self.restored = Some(Proof {
                        restored_tick: Some(tick),
                        crew: ship.crew,
                        ..death
                    });
                }
            } else {
                self.living = Some(Living {
                    identity: ship.identity,
                    tick,
                    counters,
                });
            }
        } else if let Some(living) = self.living {
            if self.death.is_none() && tick > living.tick {
                self.death = Some(Proof {
                    side,
                    identity: living.identity,
                    alive_tick: living.tick,
                    death_tick: tick,
                    restored_tick: None,
                    crew: 0,
                });
            }
        }
    }

    pub fn proof(&self, step: WaitStep) -> Option<Proof> {
        if self.side != Some(step.side) {
            return None;
        }
        match step.phase {
            Phase::Dead => self.death,
            Phase::Restored => self.restored,
        }
    }

    pub fn finish(&mut self) {
        *self = Self::default();
    }
}

use super::native_predicate::{advance, number, shape};
use super::trace::{RecordKind, TraceRecord};

pub(super) fn satisfies_label(value: &WaitStep, label: &str) -> bool {
    let restored = value.phase == Phase::Restored;
    let keys: &[&str] = if restored {
        &[
            "side=",
            "slot=",
            "generation=",
            "alive_tick=",
            "death_tick=",
            "crew=",
            "restored_tick=",
        ]
    } else {
        &[
            "side=",
            "slot=",
            "generation=",
            "alive_tick=",
            "death_tick=",
            "crew=",
        ]
    };
    shape(
        label,
        if restored {
            "pkunk_restored"
        } else {
            "pkunk_dead"
        },
        keys,
    ) && number(label, "side=") == Some(u64::from(value.side))
        && pkunk_identity(label).is_some()
        && number(label, "alive_tick=")
            .zip(number(label, "death_tick="))
            .is_some_and(|(alive, death)| death > alive)
        && if restored {
            number(label, "crew=").is_some_and(|crew| crew > 0 && crew <= 8)
                && number(label, "restored_tick=")
                    .zip(number(label, "death_tick="))
                    .is_some_and(|(restored, death)| restored > death)
        } else {
            number(label, "crew=") == Some(0)
        }
}

fn pkunk_identity(label: &str) -> Option<[u64; 5]> {
    Some([
        number(label, "side=")?,
        number(label, "slot=")?,
        number(label, "generation=")?,
        number(label, "alive_tick=")?,
        number(label, "death_tick=")?,
    ])
}

fn pkunk_sample<'a>(
    records: &'a [TraceRecord],
    label: &str,
    key: &str,
    crew: Option<u16>,
) -> Option<(usize, &'a crate::engine::observe::battle::BattleTick)> {
    use crate::engine::observe::battle::BattlePhase;
    let [side, slot, generation, _, _] = pkunk_identity(label)?;
    let frame = number(label, key)?;
    records.iter().enumerate().find_map(|(index, record)| {
        if record.kind != RecordKind::NativeBattle {
            return None;
        }
        let evidence = record.native_battle.as_ref()?;
        let tick = evidence.tick.as_ref()?;
        let ship = tick.ships.get(usize::try_from(side).ok()?)?.as_ref()?;
        (evidence.frame == frame
            && tick.phase == BattlePhase::Advanced
            && tick.activity & (1 << 9) != 0
            && ship.species == crate::ships::native::abi::SPECIES_ID_PKUNK_ID
            && ship.identity.map(u64::from) == [slot, generation]
            && crew.map_or(ship.crew > 0, |crew| ship.crew == crew))
        .then_some((index, tick))
    })
}

#[derive(Clone, Debug, Default)]
pub(super) struct History {
    pkunk_death: Option<[u64; 5]>,
    pkunk_last_death: u64,
    pkunk_counters: Option<[u8; 2]>,
}
impl History {
    pub fn verify(&mut self, value: &WaitStep, label: &str, records: &[TraceRecord]) -> bool {
        let identity = pkunk_identity(label);
        let key = match value.phase {
            Phase::Dead => "death_tick=",
            Phase::Restored => "restored_tick=",
        };
        let crew = number(label, "crew=").and_then(|crew| u16::try_from(crew).ok());
        let Some((actual_index, actual)) = pkunk_sample(records, label, key, crew) else {
            return false;
        };
        match value.phase {
            Phase::Dead => {
                let Some((alive_index, alive)) = pkunk_sample(records, label, "alive_tick=", None)
                else {
                    return false;
                };
                if alive_index >= actual_index
                    || self.pkunk_death.is_some()
                    || alive.counters != actual.counters
                    || !advance(&mut self.pkunk_last_death, number(label, "death_tick="))
                {
                    return false;
                }
                self.pkunk_death = identity;
                self.pkunk_counters = Some(actual.counters);
                true
            }
            Phase::Restored => {
                if identity.is_none()
                    || self.pkunk_death != identity
                    || self.pkunk_counters != Some(actual.counters)
                {
                    return false;
                }
                self.pkunk_death = None;
                self.pkunk_counters = None;
                true
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(side: usize, crew: u16) -> BattleTick {
        let mut ships = [None; 2];
        ships[side] = Some(BattleShip {
            identity: [3, 7],
            species: crate::ships::native::abi::SPECIES_ID_PKUNK_ID,
            position: [0; 2],
            velocity: [0; 2],
            facing: 0,
            crew,
            energy: 12,
            input: 0,
            status: 0,
        });
        BattleTick {
            phase: BattlePhase::Advanced,
            activity: 0x202,
            keys: [[0; 7]; 2],
            pulsed: [[0; 7]; 2],
            menu: [0; 24],
            menu_pulsed: [0; 24],
            rng: 42,
            ditty: 0,
            ships,
            counters: [1, 1],
        }
    }
    fn step(phase: Phase) -> WaitStep {
        WaitStep {
            side: 0,
            phase,
            max_ticks: 100,
        }
    }
    fn alive_then_dead() -> Tracker {
        let mut tracker = Tracker::default();
        tracker.begin(0);
        tracker.observe(10, &sample(0, 8));
        tracker.observe(11, &sample(0, 0));
        assert!(tracker.proof(step(Phase::Dead)).is_some());
        tracker
    }

    #[test]
    fn live_death_and_restoration_require_distinct_advanced_ticks() {
        let mut tracker = alive_then_dead();
        tracker.observe(11, &sample(0, 8));
        assert!(tracker.proof(step(Phase::Restored)).is_none());
        let mut input = sample(0, 8);
        input.phase = BattlePhase::Input;
        tracker.observe(12, &input);
        assert!(tracker.proof(step(Phase::Restored)).is_none());
        tracker.observe(12, &sample(0, 8));
        let proof = tracker.proof(step(Phase::Restored)).unwrap();
        assert_eq!(proof.trace_label(), "pkunk_restored:side=0:slot=3:generation=7:alive_tick=10:death_tick=11:crew=8:restored_tick=12");
        tracker.finish();
        assert!(tracker.proof(step(Phase::Restored)).is_none());
    }

    #[test]
    fn startup_zero_crew_and_fixture_startup_are_not_death() {
        let mut tracker = Tracker::default();
        tracker.begin(0);
        tracker.observe(1, &sample(0, 0));
        tracker.observe(2, &sample(0, 8));
        assert!(tracker.proof(step(Phase::Dead)).is_none());
        assert!(tracker.proof(step(Phase::Restored)).is_none());
    }

    #[test]
    fn ordinary_death_and_replacement_are_not_restoration() {
        for replacement in [None, Some([3, 8]), Some([4, 7])] {
            let mut tracker = alive_then_dead();
            let mut next = sample(0, 8);
            next.ships[0] = replacement.map(|identity| BattleShip {
                identity,
                ..next.ships[0].unwrap()
            });
            tracker.observe(12, &next);
            assert!(tracker.proof(step(Phase::Restored)).is_none());
        }
        let mut tracker = alive_then_dead();
        let mut retired = sample(0, 8);
        retired.counters[0] = 0;
        tracker.observe(12, &retired);
        assert!(tracker.proof(step(Phase::Restored)).is_none());
    }

    #[test]
    fn another_species_or_side_cannot_satisfy_the_observation() {
        let mut tracker = alive_then_dead();
        tracker.observe(12, &sample(1, 8));
        assert!(tracker.proof(step(Phase::Restored)).is_none());
        let mut tracker = alive_then_dead();
        let mut wrong = sample(0, 8);
        wrong.ships[0].as_mut().unwrap().species = 1;
        tracker.observe(12, &wrong);
        assert!(tracker.proof(step(Phase::Restored)).is_none());
    }
}
