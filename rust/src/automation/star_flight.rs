//! Catalog-resolved, input-only interstellar flight.

use super::script::MenuKey;
use crate::savegame::runtime::star::NUM_SOLAR_SYSTEMS;
use serde::{Deserialize, Deserializer, Serialize};

/// Zero-based position in the game's star catalog, not its special-system ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct StarTarget(u16);

impl StarTarget {
    pub fn index(self) -> u16 {
        self.0
    }

    pub fn coordinates(self) -> (i16, i16) {
        // The catalog is initialized statically and never modified by gameplay.
        let star = unsafe {
            std::ptr::addr_of!(crate::planets::tables::STARMAP_ARRAY)
                .cast::<crate::savegame::runtime::star::CStarDesc>()
                .add(usize::from(self.0))
                .read()
        };
        (star.star_pt.x, star.star_pt.y)
    }

    pub fn name(self) -> String {
        let star = unsafe {
            std::ptr::addr_of!(crate::planets::tables::STARMAP_ARRAY)
                .cast::<crate::savegame::runtime::star::CStarDesc>()
                .add(usize::from(self.0))
                .read()
        };
        static STRINGS: std::sync::OnceLock<Vec<&'static str>> = std::sync::OnceLock::new();
        let strings = STRINGS.get_or_init(|| {
            include_str!("../../../sc2/content/base/gamestrings.txt")
                .split("#(")
                .skip(1)
                .map(|entry| entry.lines().nth(1).expect("game string text").trim())
                .collect()
        });
        let suffix = strings[usize::from(star.postfix)];
        if star.prefix == 0 {
            suffix.to_string()
        } else {
            // STAR_NUMBER_BASE, followed by Alpha..Nu. Prefix zero is unnamed.
            format!(
                "{} {suffix}",
                strings[133 + 29 + 10 + 133 + 56 + usize::from(star.prefix) - 1]
            )
        }
    }

    pub fn resolve(name: &str) -> Result<Self, String> {
        (0..NUM_SOLAR_SYSTEMS)
            .map(|index| Self(index as u16))
            .find(|target| target.name().eq_ignore_ascii_case(name))
            .ok_or_else(|| format!("unknown catalog star '{name}'"))
    }
}

impl<'de> Deserialize<'de> for StarTarget {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Target {
            Index(u16),
            Name(String),
        }
        match Target::deserialize(deserializer)? {
            Target::Index(index) if usize::from(index) < NUM_SOLAR_SYSTEMS => Ok(Self(index)),
            Target::Index(index) => Err(serde::de::Error::custom(format!(
                "catalog star index {index} outside 0..{NUM_SOLAR_SYSTEMS}"
            ))),
            Target::Name(name) => Self::resolve(&name).map_err(serde::de::Error::custom),
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct FlyToStarStep {
    pub target: StarTarget,
    pub max_ticks: u64,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct Observation {
    pub activity: u16,
    pub current_star: Option<u16>,
    pub solar_active: bool,
    pub cursor: Option<(i16, i16)>,
    /// Target screen pixel minus cursor screen pixel, in the live map projection.
    pub cursor_delta: (i16, i16),
    pub autopilot: Option<(i16, i16)>,
    pub position: Option<(i32, i32)>,
    pub fuel: Option<u32>,
    pub instant_move: bool,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum Phase {
    #[default]
    Open,
    Steer,
    Select,
    Close,
    Fly,
}

#[derive(Debug, Default)]
pub struct StarFlight {
    phase: Phase,
    ticks: u64,
    initial_fuel: Option<u32>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Progress {
    Input(Option<MenuKey>),
    Arrived(String),
    Selected(String),
    Failed(String),
}

impl StarFlight {
    pub fn observe(&mut self, step: FlyToStarStep, obs: Observation) -> Progress {
        let ticks = self.ticks;
        self.ticks += 1;
        if ticks == 0 {
            self.initial_fuel = obs.fuel;
            if obs.activity & 0xff != 3 || obs.instant_move {
                return Progress::Failed(format!(
                    "fly_to_star_requires_hyperspace_without_instant_move:{obs:?}"
                ));
            }
        }
        if self.phase == Phase::Fly && obs.activity & 0xff == 4 && obs.solar_active {
            if obs.current_star != Some(step.target.index()) {
                return Progress::Failed(format!(
                    "fly_to_star_wrong_star:target={}:observed={obs:?}",
                    step.target.index()
                ));
            }
            return Progress::Arrived(format!(
                "star_arrived:target={}:star={}:x={}:y={}:activity={}:fuel_before={}:fuel_after={}",
                step.target.index(),
                step.target.index(),
                step.target.coordinates().0,
                step.target.coordinates().1,
                obs.activity,
                self.initial_fuel.expect("live fuel"),
                obs.fuel.expect("live fuel")
            ));
        }
        if ticks >= step.max_ticks.saturating_sub(1) {
            return Progress::Failed(format!(
                "fly_to_star_timeout:target={}:phase={:?}:fuel_before={:?}:observed={obs:?}",
                step.target.index(),
                self.phase,
                self.initial_fuel
            ));
        }
        // Encounters own their own input. Never dismiss a dialogue or fight by
        // accidentally reusing a map pulse; the bounded action reports a block.
        if obs.activity & 0xff != 3 {
            return Progress::Input(None);
        }
        let pulse = ticks % 4 < 2;
        match self.phase {
            Phase::Open => {
                if obs.cursor.is_some() {
                    self.phase = Phase::Steer;
                    return Progress::Input(None);
                }
                Progress::Input(if ticks.is_multiple_of(40) {
                    Some(MenuKey::Cancel)
                } else if ticks % 40 == 12 {
                    Some(MenuKey::Select)
                } else {
                    None
                })
            }
            Phase::Steer => {
                if obs.cursor == Some(step.target.coordinates()) {
                    self.phase = Phase::Select;
                    return Progress::Input(None);
                }
                let (dx, dy) = obs.cursor_delta;
                let key = if dx < 0 {
                    Some(MenuKey::Left)
                } else if dx > 0 {
                    Some(MenuKey::Right)
                } else if dy < 0 {
                    Some(MenuKey::Up)
                } else if dy > 0 {
                    Some(MenuKey::Down)
                } else {
                    return Progress::Failed(format!("fly_to_star_cursor_not_target:{obs:?}"));
                };
                Progress::Input(if pulse { key } else { None })
            }
            Phase::Select => {
                if obs.autopilot == Some(step.target.coordinates()) {
                    self.phase = Phase::Close;
                    Progress::Selected(format!(
                        "star_autopilot_selected:target={}:cursor={},{}:autopilot={},{}",
                        step.target.index(),
                        step.target.coordinates().0,
                        step.target.coordinates().1,
                        step.target.coordinates().0,
                        step.target.coordinates().1
                    ))
                } else {
                    Progress::Input(pulse.then_some(MenuKey::Select))
                }
            }
            Phase::Close => {
                if obs.cursor.is_none() {
                    self.phase = Phase::Fly;
                    Progress::Input(None)
                } else {
                    Progress::Input(pulse.then_some(MenuKey::Cancel))
                }
            }
            Phase::Fly => Progress::Input(None),
        }
    }
}

/// Check retained arrival fields against the action's resolved catalog target.
pub fn arrival_satisfies(label: &str, target: StarTarget) -> bool {
    let prefix = format!(
        "star_arrived:target={}:star={}:x={}:y={}:activity=",
        target.index(),
        target.index(),
        target.coordinates().0,
        target.coordinates().1
    );
    let Some(rest) = label.strip_prefix(&prefix) else {
        return false;
    };
    let Some((activity, fuel)) = rest.split_once(":fuel_before=") else {
        return false;
    };
    let Some((before, after)) = fuel.split_once(":fuel_after=") else {
        return false;
    };
    activity.parse::<u16>().is_ok_and(|word| word & 0xff == 4)
        && before.parse::<u32>().is_ok()
        && after.parse::<u32>().is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn names_and_indexes_resolve_the_same_shipped_catalog_entry() {
        let alpha = StarTarget::resolve("Alpha Centauri").unwrap();
        assert_eq!(alpha.coordinates(), (1559, 993));
        assert_eq!(alpha.name(), "Alpha Centauri");
        assert_eq!(
            serde_json::from_str::<StarTarget>(&alpha.index().to_string()).unwrap(),
            alpha
        );
        assert_eq!(
            serde_json::from_str::<StarTarget>("\"alpha centauri\"").unwrap(),
            alpha
        );
        assert_eq!(
            StarTarget::resolve("Sol").unwrap().coordinates(),
            (1752, 1450)
        );
        for bad in ["\"missing star\"", "502", "65536", "-1", "{}"] {
            assert!(serde_json::from_str::<StarTarget>(bad).is_err(), "{bad}");
        }
    }
    fn step() -> FlyToStarStep {
        FlyToStarStep {
            target: StarTarget::resolve("Alpha Centauri").unwrap(),
            max_ticks: 200,
        }
    }
    fn hyper() -> Observation {
        Observation {
            activity: 3,
            fuel: Some(896),
            ..Observation::default()
        }
    }
    #[test]
    fn controller_requires_ui_selection_then_matching_real_solar_entry() {
        let step = step();
        let mut flight = StarFlight::default();
        assert_eq!(
            flight.observe(step, hyper()),
            Progress::Input(Some(MenuKey::Cancel))
        );
        let map = Observation {
            cursor: Some(step.target.coordinates()),
            ..hyper()
        };
        flight.observe(step, map);
        flight.observe(step, map);
        let selected = Observation {
            autopilot: map.cursor,
            ..map
        };
        flight.observe(step, selected);
        flight.observe(step, hyper());
        assert_eq!(flight.observe(step, hyper()), Progress::Input(None));
        let arrived = Observation {
            activity: 4,
            solar_active: true,
            current_star: Some(step.target.index()),
            fuel: Some(400),
            ..hyper()
        };
        let Progress::Arrived(label) = flight.observe(step, arrived) else {
            panic!("arrival")
        };
        assert!(arrival_satisfies(&label, step.target));
        assert!(!arrival_satisfies(
            &label,
            StarTarget::resolve("Sol").unwrap()
        ));
    }
    #[test]
    fn timeout_reports_actual_position_and_fuel_and_no_manual_flight() {
        let mut flight = StarFlight {
            phase: Phase::Fly,
            ticks: 1,
            initial_fuel: Some(896),
        };
        assert_eq!(flight.observe(step(), hyper()), Progress::Input(None));
        let mut short = step();
        short.max_ticks = 3;
        let Progress::Failed(label) = flight.observe(
            short,
            Observation {
                fuel: Some(0),
                position: Some((200, 300)),
                ..hyper()
            },
        ) else {
            panic!("timeout")
        };
        assert!(label.contains("(200, 300)"));
        assert!(label.contains("fuel: Some(0)"));
    }
    #[test]
    fn wrong_star_and_debug_movement_cannot_pass() {
        let mut flight = StarFlight {
            phase: Phase::Fly,
            ticks: 1,
            initial_fuel: Some(896),
        };
        assert!(matches!(
            flight.observe(
                step(),
                Observation {
                    activity: 4,
                    solar_active: true,
                    current_star: Some(0),
                    ..hyper()
                }
            ),
            Progress::Failed(_)
        ));
        assert!(matches!(
            StarFlight::default().observe(
                step(),
                Observation {
                    instant_move: true,
                    ..hyper()
                }
            ),
            Progress::Failed(_)
        ));
    }
}
