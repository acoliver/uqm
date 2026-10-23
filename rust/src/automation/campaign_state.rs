//! Read-only campaign state for `assert_state` and `wait_for_state`.
//!
//! A script names the values it expects by key (`story:MOONBASE_DESTROYED`,
//! `sis:fuel`, `activity`, ...) and compares each with a closed set of
//! operators. Keys are validated when the script is parsed, so a misspelled
//! story bit fails admission instead of reading zero at run time.
//!
//! The comparison is pure: values come from a [`StateSource`], which is the
//! live game on the game thread and a fake in tests. A value the source cannot
//! read is reported as unavailable and never compares as zero, so
//! `story:X eq 0` cannot pass before the game state exists.

use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};

use serde::{Deserialize, Serialize};

use crate::state::game_state_keys::{bit_range, STATE_KEYS};

/// A value a script can observe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StateKey {
    /// `GLOBAL(CurrentActivity)`, the whole word.
    Activity,
    Ending(u8),
    BossRemaining,
    BossDestroyed,
    BossGateOpen,
    BossGateEntered,
    BossBombCarrier,
    BossSelectedSpecies,
    StarCurrent,
    HyperX,
    HyperY,
    StarmapCursorX,
    StarmapCursorY,
    /// `GLOBAL_SIS(FuelOnBoard)`, raw fuel points.
    SisFuel,
    /// `GLOBAL_SIS(ResUnits)`.
    SisRu,
    /// `GLOBAL_SIS(CrewEnlisted)`; `0xffff` is the game's dead-flagship value.
    SisCrew,
    /// `GLOBAL_SIS(NumLanders)`.
    SisLanders,
    SisDrive(u8),
    SisJet(u8),
    SisModule(u8),
    SisMineralMass,
    EscortCount,
    EscortSpecies(u8),
    CyborgEnabled,
    CombatSpeed,
    EncounterMenu,
    DateDay,
    DateMonth,
    DateYear,
    /// 1 once the main loop has ended a game because the flagship was
    /// destroyed (`CrewEnlisted == 0xffff`), 0 before. Never cleared during a run.
    FlagshipDestroyed,
    /// Ships left on side 0 of the battle in progress, or of the last one
    /// (`battle_counter[0]`).
    BattleShipsP1,
    /// Ships left on side 1 (`battle_counter[1]`).
    BattleShipsP2,
    /// A named story bit from `STATE_KEYS`.
    Story(&'static str),
}

impl StateKey {
    /// Parse a script key.
    ///
    /// # Errors
    ///
    /// Returns a message naming the accepted forms when the key is unknown.
    pub fn parse(text: &str) -> Result<Self, String> {
        if let Some(name) = text.strip_prefix("story:") {
            return STATE_KEYS
                .iter()
                .find(|(known, _, _)| *known == name)
                .map(|(known, _, _)| Self::Story(known))
                .ok_or_else(|| format!("unknown story bit '{name}'"));
        }
        for (prefix, slots, make) in [
            ("sis:drive:", 11, Self::SisDrive as fn(u8) -> Self),
            ("sis:jet:", 8, Self::SisJet as fn(u8) -> Self),
            ("sis:module:", 16, Self::SisModule as fn(u8) -> Self),
        ] {
            if let Some(slot) = text.strip_prefix(prefix) {
                return slot
                    .parse::<u8>()
                    .ok()
                    .filter(|slot| *slot < slots)
                    .map(make)
                    .ok_or_else(|| format!("invalid fitting slot '{text}'"));
            }
        }
        if let Some(species) = text.strip_prefix("escorts:species:") {
            return species
                .parse::<u8>()
                .ok()
                .filter(|race| *race < 25)
                .map(Self::EscortSpecies)
                .ok_or_else(|| format!("invalid escort race '{text}'"));
        }
        match text {
            "ending:won" => Ok(Self::Ending(0)),
            "ending:presentation_started" => Ok(Self::Ending(1)),
            "ending:presentation_completed" => Ok(Self::Ending(2)),
            "ending:credits_started" => Ok(Self::Ending(3)),
            "ending:credits_completed" => Ok(Self::Ending(4)),
            "ending:credits_controls_ready" => Ok(Self::Ending(5)),
            "samatra:remaining" => Ok(Self::BossRemaining),
            "samatra:destroyed" => Ok(Self::BossDestroyed),
            "samatra:gate_open" => Ok(Self::BossGateOpen),
            "samatra:gate_entered" => Ok(Self::BossGateEntered),
            "samatra:bomb_carrier" => Ok(Self::BossBombCarrier),
            "samatra:selected_species" => Ok(Self::BossSelectedSpecies),
            "activity" => Ok(Self::Activity),
            "star:current" => Ok(Self::StarCurrent),
            "hyper:x" => Ok(Self::HyperX),
            "hyper:y" => Ok(Self::HyperY),
            "starmap:cursor_x" => Ok(Self::StarmapCursorX),
            "starmap:cursor_y" => Ok(Self::StarmapCursorY),
            "escorts:count" => Ok(Self::EscortCount),
            "combat:cyborg" => Ok(Self::CyborgEnabled),
            "combat:speed" => Ok(Self::CombatSpeed),
            "encounter:menu" => Ok(Self::EncounterMenu),
            "sis:fuel" => Ok(Self::SisFuel),
            "sis:ru" => Ok(Self::SisRu),
            "sis:crew" => Ok(Self::SisCrew),
            "sis:landers" => Ok(Self::SisLanders),
            "sis:mineral_mass" => Ok(Self::SisMineralMass),
            "date:day" => Ok(Self::DateDay),
            "date:month" => Ok(Self::DateMonth),
            "date:year" => Ok(Self::DateYear),
            "game:flagship_destroyed" => Ok(Self::FlagshipDestroyed),
            "battle:ships_p1" => Ok(Self::BattleShipsP1),
            "battle:ships_p2" => Ok(Self::BattleShipsP2),
            _ => Err(format!(
                "unknown state key '{text}'; use activity, sis:fuel|ru|crew|landers, \
                 date:day|month|year, star:current, hyper:x|y, starmap:cursor_x|cursor_y, game:flagship_destroyed, battle:ships_p1|p2 or story:NAME"
            )),
        }
    }

    /// The largest value the underlying field can hold.
    #[must_use]
    pub fn max_value(self) -> u32 {
        match self {
            Self::Ending(_)
            | Self::BossGateOpen
            | Self::BossGateEntered
            | Self::BossBombCarrier => 1,
            Self::BossRemaining | Self::BossDestroyed => 8,
            Self::BossSelectedSpecies => 26,
            Self::StarCurrent => (crate::savegame::runtime::star::NUM_SOLAR_SYSTEMS - 1) as u32,
            Self::HyperX | Self::HyperY => i32::MAX as u32,
            Self::StarmapCursorX | Self::StarmapCursorY => 9999,
            Self::Activity | Self::SisCrew | Self::DateYear | Self::SisMineralMass => {
                u32::from(u16::MAX)
            }
            Self::SisFuel | Self::SisRu => u32::MAX,
            Self::SisLanders
            | Self::DateDay
            | Self::DateMonth
            | Self::SisDrive(_)
            | Self::SisJet(_)
            | Self::SisModule(_) => u32::from(u8::MAX),
            Self::FlagshipDestroyed | Self::CyborgEnabled | Self::EncounterMenu => 1,
            Self::CombatSpeed => 3,
            Self::EscortCount | Self::EscortSpecies(_) => 12,
            Self::BattleShipsP1 | Self::BattleShipsP2 => u32::from(u8::MAX),
            Self::Story(name) => {
                let (start, end) = bit_range(name).unwrap_or((0, 0));
                (1u32 << (end - start + 1)) - 1
            }
        }
    }
}

/// How an observed value is compared with the expected one.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum StateOp {
    Eq,
    Ne,
    Ge,
    Le,
}

impl StateOp {
    fn holds(self, observed: u32, wanted: u32) -> bool {
        match self {
            Self::Eq => observed == wanted,
            Self::Ne => observed != wanted,
            Self::Ge => observed >= wanted,
            Self::Le => observed <= wanted,
        }
    }
}

impl fmt::Display for StateOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Eq => "eq",
            Self::Ne => "ne",
            Self::Ge => "ge",
            Self::Le => "le",
        })
    }
}

/// One comparison. `key` is validated by [`StatePredicate::validate`].
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct StateCondition {
    pub key: String,
    pub op: StateOp,
    pub value: u32,
}

/// Every condition must hold.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct StatePredicate {
    pub all: Vec<StateCondition>,
}

impl StatePredicate {
    /// Reject an empty predicate, unknown keys and values the field cannot hold.
    ///
    /// # Errors
    ///
    /// Returns a message describing the first invalid condition.
    pub fn validate(&self) -> Result<(), String> {
        if self.all.is_empty() {
            return Err("predicate needs at least one condition in 'all'".into());
        }
        for condition in &self.all {
            let key = StateKey::parse(&condition.key)?;
            if condition.value > key.max_value() {
                return Err(format!(
                    "value {} exceeds the maximum {} for '{}'",
                    condition.value,
                    key.max_value(),
                    condition.key
                ));
            }
        }
        Ok(())
    }
}

/// Where observed values come from.
pub trait StateSource {
    /// The current value, or `None` when it cannot be read.
    fn read(&self, key: StateKey) -> Option<u32>;
}

/// The result of comparing a predicate with a source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Evaluation {
    /// Each condition's key with the value read, in predicate order.
    pub observed: Vec<(String, Option<u32>)>,
    /// True when every value was readable and every comparison held.
    pub held: bool,
}

/// Read every key of `predicate` once and compare.
///
/// The predicate must have passed [`StatePredicate::validate`].
#[must_use]
pub fn evaluate(predicate: &StatePredicate, source: &dyn StateSource) -> Evaluation {
    let mut held = true;
    let observed = predicate
        .all
        .iter()
        .map(|condition| {
            let value = StateKey::parse(&condition.key)
                .ok()
                .and_then(|key| source.read(key));
            held &= value.is_some_and(|value| condition.op.holds(value, condition.value));
            (condition.key.clone(), value)
        })
        .collect();
    Evaluation { observed, held }
}

impl Evaluation {
    /// A trace label of the form `prefix|key=value|...`.
    ///
    /// A key that could not be read prints `unavailable`, which no offline
    /// check accepts as a number.
    #[must_use]
    pub fn label(&self, prefix: &str) -> String {
        let mut label = prefix.to_string();
        for (key, value) in &self.observed {
            match value {
                Some(value) => label.push_str(&format!("|{key}={value}")),
                None => label.push_str(&format!("|{key}=unavailable")),
            }
        }
        label
    }

    /// A failure label that also carries what each condition required.
    #[must_use]
    pub fn failure_label(&self, prefix: &str, predicate: &StatePredicate) -> String {
        let mut label = self.label(prefix);
        for condition in &predicate.all {
            label.push_str(&format!(
                "|want:{}:{}:{}",
                condition.key, condition.op, condition.value
            ));
        }
        label
    }
}

/// Whether a retained `prefix|key=value|...` label proves `predicate`.
///
/// Re-checks the recorded numbers against the script's own conditions, so a
/// label from a different predicate or an unavailable value never satisfies it.
#[must_use]
pub fn label_satisfies(label: &str, prefix: &str, predicate: &StatePredicate) -> bool {
    let mut fields = label.split('|');
    if fields.next() != Some(prefix) {
        return false;
    }
    let mut conditions = predicate.all.iter();
    for field in fields {
        let (Some(condition), Some((key, value))) = (conditions.next(), field.split_once('='))
        else {
            return false;
        };
        let Ok(value) = value.parse::<u32>() else {
            return false;
        };
        if key != condition.key || !condition.op.holds(value, condition.value) {
            return false;
        }
    }
    conditions.next().is_none()
}

static FLAGSHIP_DESTROYED: AtomicBool = AtomicBool::new(false);

/// Record that the main loop ended a game because the flagship died.
///
/// The restart menu that follows tears the game structures down, so the
/// crew word is gone by the time a script could read it; this flag outlives it.
pub fn record_flagship_destroyed() {
    FLAGSHIP_DESTROYED.store(true, Ordering::SeqCst);
}

/// The running game, read on the game thread.
pub struct LiveCampaign;

impl StateSource for LiveCampaign {
    fn read(&self, key: StateKey) -> Option<u32> {
        use crate::automation::input_ffi;
        match key {
            StateKey::Ending(index) => {
                let e = super::ending::snapshot();
                Some(u32::from(
                    [
                        e.won,
                        e.presentation_started,
                        e.presentation_completed,
                        e.credits_started,
                        e.credits_completed,
                        e.credits_controls_ready,
                    ][usize::from(index)],
                ))
            }
            StateKey::BossDestroyed => Some(
                crate::ships::races::samatra::observation::events()
                    .destroyed
                    .count_ones(),
            ),
            StateKey::BossGateEntered => Some(u32::from(
                crate::ships::races::samatra::observation::events().gate_entered,
            )),
            StateKey::BossRemaining
            | StateKey::BossGateOpen
            | StateKey::BossBombCarrier
            | StateKey::BossSelectedSpecies => {
                crate::ships::races::samatra::snapshot().and_then(|s| match key {
                    StateKey::BossRemaining => Some(u32::from(s.remaining)),
                    StateKey::BossGateOpen => Some(u32::from(s.gate_open)),
                    StateKey::BossBombCarrier => Some(u32::from(s.bomb_carrier)),
                    StateKey::BossSelectedSpecies => s.selected_species,
                    _ => unreachable!(),
                })
            }
            StateKey::StarCurrent | StateKey::StarmapCursorX | StateKey::StarmapCursorY => {
                let target =
                    super::star_flight::StarTarget::resolve("Sol").expect("Sol in catalog");
                let observation = input_ffi::star_flight_observation(target);
                match key {
                    StateKey::StarCurrent => observation.current_star.map(u32::from),
                    StateKey::StarmapCursorX => {
                        observation.cursor.and_then(|(x, _)| u32::try_from(x).ok())
                    }
                    StateKey::StarmapCursorY => {
                        observation.cursor.and_then(|(_, y)| u32::try_from(y).ok())
                    }
                    _ => unreachable!(),
                }
            }
            StateKey::HyperX => {
                input_ffi::hyperspace_position().and_then(|(x, _)| u32::try_from(x).ok())
            }
            StateKey::HyperY => {
                input_ffi::hyperspace_position().and_then(|(_, y)| u32::try_from(y).ok())
            }
            StateKey::Story(name) => crate::state::ffi::named_state_value(name).map(u32::from),
            StateKey::Activity => Some(u32::from(crate::mainloop::ffi::get_current_activity().0)),
            StateKey::SisDrive(slot) => input_ffi::sis_fittings()
                .map(|(drives, _, _, _)| u32::from(drives[usize::from(slot)])),
            StateKey::SisJet(slot) => {
                input_ffi::sis_fittings().map(|(_, jets, _, _)| u32::from(jets[usize::from(slot)]))
            }
            StateKey::SisModule(slot) => input_ffi::sis_fittings()
                .map(|(_, _, modules, _)| u32::from(modules[usize::from(slot)])),
            StateKey::SisMineralMass => {
                input_ffi::sis_fittings().map(|(_, _, _, mass)| u32::from(mass))
            }
            StateKey::EncounterMenu => {
                Some(u32::from(super::ui_observation::encounter_menu_active()))
            }
            StateKey::EscortCount => input_ffi::escort_roster().map(|ships| ships.len() as u32),
            StateKey::EscortSpecies(race) => input_ffi::escort_roster().map(|ships| {
                ships
                    .iter()
                    .filter(|(id, crew)| *id == race && *crew > 0)
                    .count() as u32
            }),
            StateKey::CyborgEnabled => {
                input_ffi::cyborg_settings().map(|(enabled, _)| u32::from(enabled))
            }
            StateKey::CombatSpeed => {
                input_ffi::cyborg_settings().map(|(_, speed)| u32::from(speed))
            }
            StateKey::SisFuel => input_ffi::ship_resources().map(|(fuel, _)| fuel),
            StateKey::SisRu => input_ffi::ship_resources().map(|(_, ru)| ru),
            StateKey::SisCrew => input_ffi::sis_crew_and_landers().map(|(crew, _)| u32::from(crew)),
            StateKey::SisLanders => {
                input_ffi::sis_crew_and_landers().map(|(_, landers)| u32::from(landers))
            }
            StateKey::DateDay => input_ffi::game_date().map(|(day, _, _)| u32::from(day)),
            StateKey::DateMonth => input_ffi::game_date().map(|(_, month, _)| u32::from(month)),
            StateKey::DateYear => input_ffi::game_date().map(|(_, _, year)| u32::from(year)),
            StateKey::FlagshipDestroyed => {
                Some(u32::from(FLAGSHIP_DESTROYED.load(Ordering::SeqCst)))
            }
            StateKey::BattleShipsP1 => input_ffi::battle_fleets().map(|(p1, _)| u32::from(p1)),
            StateKey::BattleShipsP2 => input_ffi::battle_fleets().map(|(_, p2)| u32::from(p2)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn boss_and_ending_keys_validate_widths_and_keep_unavailable_distinct_from_zero() {
        for (key, maximum) in [
            ("ending:won", 1),
            ("ending:presentation_started", 1),
            ("ending:presentation_completed", 1),
            ("ending:credits_started", 1),
            ("ending:credits_completed", 1),
            ("ending:credits_controls_ready", 1),
            ("samatra:remaining", 8),
            ("samatra:destroyed", 8),
            ("samatra:gate_open", 1),
            ("samatra:gate_entered", 1),
            ("samatra:bomb_carrier", 1),
            ("samatra:selected_species", 26),
        ] {
            assert_eq!(StateKey::parse(key).unwrap().max_value(), maximum);
            let mut wanted = StatePredicate {
                all: vec![StateCondition {
                    key: key.into(),
                    op: StateOp::Eq,
                    value: 0,
                }],
            };
            assert!(wanted.validate().is_ok());
            assert!(!evaluate(&wanted, &fake(&[])).held);
            wanted.all[0].value = maximum + 1;
            assert!(wanted.validate().is_err());
        }
        assert!(StateKey::parse("ending:win").is_err());
        assert!(StateKey::parse("samatra:generators").is_err());
    }

    #[test]
    fn escort_and_combat_keys_validate_and_never_invent_an_unavailable_roster() {
        assert_eq!(
            StateKey::parse("escorts:species:4"),
            Ok(StateKey::EscortSpecies(4))
        );
        for bad in [
            "escorts:species:25",
            "escorts:species:-1",
            "escorts:species:pkunk",
        ] {
            assert!(StateKey::parse(bad).is_err());
        }
        for key in [
            "escorts:count",
            "escorts:species:4",
            "combat:cyborg",
            "combat:speed",
        ] {
            let predicate = StatePredicate {
                all: vec![StateCondition {
                    key: key.into(),
                    op: StateOp::Eq,
                    value: 0,
                }],
            };
            assert!(predicate.validate().is_ok());
            assert!(!evaluate(&predicate, &fake(&[])).held);
        }
    }

    #[test]
    fn equipment_keys_validate_slot_bounds_and_byte_values() {
        for (key, expected) in [
            ("sis:drive:10", StateKey::SisDrive(10)),
            ("sis:jet:7", StateKey::SisJet(7)),
            ("sis:module:15", StateKey::SisModule(15)),
        ] {
            assert_eq!(StateKey::parse(key), Ok(expected));
            assert_eq!(expected.max_value(), 255);
        }
        for key in [
            "sis:drive:11",
            "sis:jet:8",
            "sis:module:16",
            "sis:module:-1",
            "sis:drive:x",
        ] {
            assert!(StateKey::parse(key).is_err(), "{key}");
        }
        assert!(StatePredicate {
            all: vec![StateCondition {
                key: "sis:drive:0".into(),
                op: StateOp::Eq,
                value: 256
            }]
        }
        .validate()
        .is_err());
    }

    struct Fake(HashMap<&'static str, u32>);

    impl StateSource for Fake {
        fn read(&self, key: StateKey) -> Option<u32> {
            let name = match key {
                StateKey::Ending(_) => "ending:test",
                StateKey::BossRemaining
                | StateKey::BossDestroyed
                | StateKey::BossGateOpen
                | StateKey::BossGateEntered
                | StateKey::BossBombCarrier
                | StateKey::BossSelectedSpecies => "samatra:test",
                StateKey::Activity => "activity",
                StateKey::StarCurrent => "star:current",
                StateKey::HyperX => "hyper:x",
                StateKey::HyperY => "hyper:y",
                StateKey::StarmapCursorX => "starmap:cursor_x",
                StateKey::StarmapCursorY => "starmap:cursor_y",
                StateKey::EscortCount => "escorts:count",
                StateKey::EscortSpecies(_) => "escorts:species",
                StateKey::CyborgEnabled => "combat:cyborg",
                StateKey::CombatSpeed => "combat:speed",
                StateKey::EncounterMenu => "encounter:menu",
                StateKey::SisFuel => "sis:fuel",
                StateKey::SisRu => "sis:ru",
                StateKey::SisCrew => "sis:crew",
                StateKey::SisDrive(_)
                | StateKey::SisJet(_)
                | StateKey::SisModule(_)
                | StateKey::SisMineralMass => "test:fittings",
                StateKey::SisLanders => "sis:landers",
                StateKey::DateDay => "date:day",
                StateKey::DateMonth => "date:month",
                StateKey::DateYear => "date:year",
                StateKey::FlagshipDestroyed => "game:flagship_destroyed",
                StateKey::BattleShipsP1 => "battle:ships_p1",
                StateKey::BattleShipsP2 => "battle:ships_p2",
                StateKey::Story(name) => name,
            };
            self.0.get(name).copied()
        }
    }

    fn predicate(json: &str) -> StatePredicate {
        serde_json::from_str(json).expect("predicate parses")
    }

    fn fake(values: &[(&'static str, u32)]) -> Fake {
        Fake(values.iter().copied().collect())
    }

    #[test]
    fn known_keys_parse_and_unknown_keys_are_rejected() {
        assert_eq!(
            StateKey::parse("story:MOONBASE_DESTROYED"),
            Ok(StateKey::Story("MOONBASE_DESTROYED"))
        );
        assert_eq!(StateKey::parse("sis:fuel"), Ok(StateKey::SisFuel));
        assert!(StateKey::parse("story:NO_SUCH_BIT")
            .unwrap_err()
            .contains("NO_SUCH_BIT"));
        assert!(StateKey::parse("sis:gold").is_err());
        assert!(StateKey::parse("").is_err());
    }

    #[test]
    fn validation_rejects_empty_unknown_and_oversized_conditions() {
        assert!(predicate(r#"{"all":[]}"#).validate().is_err());
        let unknown = predicate(r#"{"all":[{"key":"story:NOPE","op":"eq","value":1}]}"#);
        assert!(unknown.validate().is_err());
        let wide_bit =
            predicate(r#"{"all":[{"key":"story:MOONBASE_DESTROYED","op":"eq","value":2}]}"#);
        assert!(wide_bit.validate().unwrap_err().contains("maximum 1"));
        let crew = predicate(r#"{"all":[{"key":"sis:crew","op":"ge","value":65536}]}"#);
        assert!(crew.validate().is_err());
        let fine = predicate(r#"{"all":[{"key":"sis:fuel","op":"ge","value":4000000000}]}"#);
        assert!(fine.validate().is_ok());
    }

    #[test]
    fn parsing_rejects_unknown_operators_and_fields() {
        assert!(serde_json::from_str::<StatePredicate>(
            r#"{"all":[{"key":"sis:fuel","op":"gt","value":1}]}"#
        )
        .is_err());
        assert!(serde_json::from_str::<StatePredicate>(
            r#"{"all":[{"key":"sis:fuel","op":"eq","value":1,"extra":2}]}"#
        )
        .is_err());
    }

    #[test]
    fn flight_keys_preserve_unavailable_values_and_validate_widths() {
        for (key, maximum) in [
            ("star:current", 501),
            ("hyper:x", i32::MAX as u32),
            ("hyper:y", i32::MAX as u32),
            ("starmap:cursor_x", 9999),
            ("starmap:cursor_y", 9999),
        ] {
            assert_eq!(StateKey::parse(key).unwrap().max_value(), maximum);
            let wanted = StatePredicate {
                all: vec![StateCondition {
                    key: key.into(),
                    op: StateOp::Eq,
                    value: 0,
                }],
            };
            assert!(wanted.validate().is_ok());
            assert!(!evaluate(&wanted, &fake(&[])).held);
            let too_wide = StatePredicate {
                all: vec![StateCondition {
                    key: key.into(),
                    op: StateOp::Eq,
                    value: maximum + 1,
                }],
            };
            assert!(too_wide.validate().is_err());
        }
    }

    #[test]
    fn every_operator_compares_as_named() {
        let source = fake(&[("sis:fuel", 896)]);
        for (op, value, expected) in [
            ("eq", 896, true),
            ("eq", 895, false),
            ("ne", 895, true),
            ("ne", 896, false),
            ("ge", 896, true),
            ("ge", 897, false),
            ("le", 896, true),
            ("le", 895, false),
        ] {
            let text = format!(r#"{{"all":[{{"key":"sis:fuel","op":"{op}","value":{value}}}]}}"#);
            assert_eq!(
                evaluate(&predicate(&text), &source).held,
                expected,
                "{op} {value}"
            );
        }
    }

    #[test]
    fn an_unavailable_value_never_compares_as_zero() {
        let wants_zero =
            predicate(r#"{"all":[{"key":"story:STARBASE_AVAILABLE","op":"eq","value":0}]}"#);
        let evaluation = evaluate(&wants_zero, &fake(&[]));
        assert!(!evaluation.held);
        assert_eq!(
            evaluation.observed,
            vec![("story:STARBASE_AVAILABLE".into(), None)]
        );
        assert!(evaluation
            .label("state_verified")
            .ends_with("story:STARBASE_AVAILABLE=unavailable"));
        assert!(evaluate(&wants_zero, &fake(&[("STARBASE_AVAILABLE", 0)])).held);
    }

    #[test]
    fn a_mismatch_reports_every_actual_value_and_what_was_wanted() {
        let both = predicate(
            r#"{"all":[{"key":"sis:fuel","op":"eq","value":896},{"key":"sis:ru","op":"ge","value":2500}]}"#,
        );
        let evaluation = evaluate(&both, &fake(&[("sis:fuel", 900), ("sis:ru", 2500)]));
        assert!(!evaluation.held);
        assert_eq!(
            evaluation.failure_label("state_mismatch", &both),
            "state_mismatch|sis:fuel=900|sis:ru=2500|want:sis:fuel:eq:896|want:sis:ru:ge:2500"
        );
    }

    #[test]
    fn the_flagship_destroyed_flag_is_a_zero_or_one_key() {
        assert_eq!(
            StateKey::parse("game:flagship_destroyed"),
            Ok(StateKey::FlagshipDestroyed)
        );
        let two = predicate(r#"{"all":[{"key":"game:flagship_destroyed","op":"eq","value":2}]}"#);
        assert!(two.validate().is_err());
        let wanted =
            predicate(r#"{"all":[{"key":"game:flagship_destroyed","op":"eq","value":1}]}"#);
        assert!(wanted.validate().is_ok());
        assert!(!evaluate(&wanted, &fake(&[("game:flagship_destroyed", 0)])).held);
        assert!(evaluate(&wanted, &fake(&[("game:flagship_destroyed", 1)])).held);
    }

    #[test]
    fn a_dead_flagship_crew_word_is_preserved() {
        let dead = predicate(r#"{"all":[{"key":"sis:crew","op":"eq","value":65535}]}"#);
        assert!(evaluate(&dead, &fake(&[("sis:crew", 0xffff)])).held);
    }

    #[test]
    fn retained_labels_prove_only_the_predicate_they_were_recorded_for() {
        let wanted = predicate(
            r#"{"all":[{"key":"story:STARBASE_AVAILABLE","op":"eq","value":1},{"key":"sis:fuel","op":"ge","value":896}]}"#,
        );
        let good = "state_verified|story:STARBASE_AVAILABLE=1|sis:fuel=896";
        assert!(label_satisfies(good, "state_verified", &wanted));
        assert!(!label_satisfies(good, "state_reached", &wanted));
        assert!(!label_satisfies(
            "state_verified|story:STARBASE_AVAILABLE=0|sis:fuel=896",
            "state_verified",
            &wanted
        ));
        assert!(!label_satisfies(
            "state_verified|story:STARBASE_AVAILABLE=1|sis:fuel=unavailable",
            "state_verified",
            &wanted
        ));
        assert!(!label_satisfies(
            "state_verified|story:STARBASE_AVAILABLE=1",
            "state_verified",
            &wanted
        ));
        assert!(!label_satisfies(
            "state_verified|sis:fuel=896|story:STARBASE_AVAILABLE=1",
            "state_verified",
            &wanted
        ));
    }
}
