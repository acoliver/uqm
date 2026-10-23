//! Catalog-resolved, input-only interstellar flight.

use super::script::MenuKey;
use crate::savegame::runtime::star::NUM_SOLAR_SYSTEMS;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Zero-based position in the game's star catalog, not its special-system ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StarTarget(u16);

impl StarTarget {
    const QUASISPACE_ENTRANCE: Self = Self(u16::MAX);
    const ARILOU_HOME: Self = Self(518);
    const UMGAH_EXIT: Self = Self(515);
    const SOL_EXIT: Self = Self(507);

    pub fn is_portal(self) -> bool {
        self == Self::QUASISPACE_ENTRANCE || self == Self::ARILOU_HOME || self.is_exit()
    }

    fn is_exit(self) -> bool {
        self == Self::UMGAH_EXIT || self == Self::SOL_EXIT
    }

    pub fn is_arilou_home(self) -> bool {
        self == Self::ARILOU_HOME
    }

    pub fn index(self) -> u16 {
        self.0
    }

    pub fn coordinates(self) -> (i16, i16) {
        if self == Self::QUASISPACE_ENTRANCE {
            return (438, 6372);
        }
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
        if self.is_portal() {
            return if self.is_arilou_home() {
                "Arilou home"
            } else if self == Self::UMGAH_EXIT {
                "QuasiSpace Umgah exit"
            } else if self == Self::SOL_EXIT {
                "QuasiSpace Sol exit"
            } else {
                "QuasiSpace entrance"
            }
            .into();
        }
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
        for target in [
            Self::QUASISPACE_ENTRANCE,
            Self::ARILOU_HOME,
            Self::UMGAH_EXIT,
            Self::SOL_EXIT,
        ] {
            if target.name().eq_ignore_ascii_case(name) {
                return Ok(target);
            }
        }
        (0..NUM_SOLAR_SYSTEMS)
            .map(|index| Self(index as u16))
            .find(|target| target.name().eq_ignore_ascii_case(name))
            .ok_or_else(|| format!("unknown catalog star '{name}'"))
    }
}

impl Serialize for StarTarget {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        if self.is_portal() {
            serializer.serialize_str(&self.name())
        } else {
            serializer.serialize_u16(self.0)
        }
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

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct FlyToStarStep {
    #[serde(default)]
    pub retreat_encounters: bool,
    #[serde(default)]
    pub encounter_response: Option<String>,
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
    pub space_side: u8,
    pub portal_open: bool,
    pub conversation_active: bool,
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
    pub fn resume_after_encounter(&mut self) {
        self.phase = Phase::Open;
    }

    pub fn observe(&mut self, step: &FlyToStarStep, obs: Observation) -> Progress {
        let ticks = self.ticks;
        self.ticks += 1;
        if ticks == 0 {
            self.initial_fuel = obs.fuel;
            let wrong_space = if step.target == StarTarget::QUASISPACE_ENTRANCE {
                obs.space_side > 1
            } else if step.target.is_portal() {
                obs.space_side <= 1
            } else {
                false
            };
            if obs.activity & 0xff != 3 || obs.instant_move || wrong_space {
                return Progress::Failed(format!(
                    "fly_to_star_requires_hyperspace_without_instant_move:{obs:?}"
                ));
            }
        }
        if self.phase == Phase::Fly && step.target.is_portal() {
            let reached = if step.target.is_arilou_home() {
                obs.space_side == 3
                    && obs.current_star == Some(step.target.index())
                    && obs.conversation_active
            } else if step.target.is_exit() {
                obs.space_side == 0
            } else {
                obs.space_side == 3
            };
            if reached {
                return Progress::Arrived(format!(
                    "portal_arrived:target={}:x={}:y={}:space_side={}:star={}:conversation={}:fuel_before={}:fuel_after={}",
                    step.target.index(), step.target.coordinates().0, step.target.coordinates().1,
                    obs.space_side, obs.current_star.map_or(-1, i32::from), u8::from(obs.conversation_active),
                    self.initial_fuel.expect("live fuel"), obs.fuel.expect("live fuel")
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
        // The natural portal opens on the 17th. Wait in live hyperspace,
        // before the map pauses the game clock, rather than changing the date.
        if step.target == StarTarget::QUASISPACE_ENTRANCE
            && self.phase == Phase::Open
            && !obs.portal_open
        {
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
    if target.is_portal() {
        let fields: std::collections::BTreeMap<_, _> = label
            .split(':')
            .skip(1)
            .filter_map(|field| field.split_once('='))
            .collect();
        let number = |key| fields.get(key).and_then(|value| value.parse::<i64>().ok());
        return label.starts_with("portal_arrived:")
            && fields.len() == 8
            && number("target") == Some(i64::from(target.index()))
            && number("x") == Some(i64::from(target.coordinates().0))
            && number("y") == Some(i64::from(target.coordinates().1))
            && number("space_side") == Some(if target.is_exit() { 0 } else { 3 })
            && number("fuel_before").is_some_and(|fuel| (0..=u32::MAX as i64).contains(&fuel))
            && number("fuel_after").is_some_and(|fuel| (0..=u32::MAX as i64).contains(&fuel))
            && (!target.is_arilou_home()
                || (number("star") == Some(i64::from(target.index()))
                    && number("conversation") == Some(1)));
    }

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
    fn encounter_resume_reopens_map_without_resetting_fuel_or_budget() {
        let mut flight = StarFlight {
            phase: Phase::Fly,
            ticks: 120,
            initial_fuel: Some(9000),
        };
        flight.resume_after_encounter();
        assert_eq!(flight.phase, Phase::Open);
        assert_eq!(flight.ticks, 120);
        assert_eq!(flight.initial_fuel, Some(9000));
    }

    #[test]
    fn quasispace_route_uses_named_destinations_and_proves_real_transitions() {
        let entrance = StarTarget::resolve("QuasiSpace entrance").unwrap();
        let home = StarTarget::resolve("Arilou home").unwrap();
        assert_eq!(entrance.coordinates(), (438, 6372));
        assert_eq!(home.coordinates(), (6134, 5900));
        for target in [entrance, home] {
            assert_eq!(
                serde_json::from_str::<StarTarget>(&serde_json::to_string(&target).unwrap())
                    .unwrap(),
                target
            );
            let step = FlyToStarStep {
                retreat_encounters: false,
                encounter_response: None,
                target,
                max_ticks: 200,
            };
            let mut flight = StarFlight {
                phase: Phase::Fly,
                ticks: 10,
                initial_fuel: Some(9000),
            };
            let obs = Observation {
                current_star: Some(home.index()),
                space_side: 3,
                conversation_active: true,
                ..hyper()
            };
            let Progress::Arrived(label) = flight.observe(&step, obs) else {
                panic!("real portal/home transition")
            };
            assert!(arrival_satisfies(&label, target));
            assert!(!arrival_satisfies(
                &label.replace("space_side=3", "space_side=0"),
                target
            ));
            let mut not_arrived = StarFlight {
                phase: Phase::Fly,
                ticks: 10,
                initial_fuel: Some(9000),
            };
            assert!(matches!(
                not_arrived.observe(&step, hyper()),
                Progress::Input(_)
            ));
        }
    }

    #[test]
    fn sol_exit_requires_a_real_return_from_quasispace() {
        let target = StarTarget::resolve("QuasiSpace Sol exit").unwrap();
        assert_eq!(target.index(), 507);
        assert_eq!(target.coordinates(), (5060, 4740));
        assert_eq!(
            serde_json::from_str::<StarTarget>(&serde_json::to_string(&target).unwrap()).unwrap(),
            target
        );
        for side in [0, 3] {
            let mut flight = StarFlight {
                phase: Phase::Fly,
                ticks: 10,
                initial_fuel: Some(9000),
            };
            let step = FlyToStarStep {
                retreat_encounters: false,
                encounter_response: None,
                target,
                max_ticks: 200,
            };
            let observation = Observation {
                space_side: side,
                ..hyper()
            };
            match flight.observe(&step, observation) {
                Progress::Arrived(label) if side == 0 => {
                    assert!(arrival_satisfies(&label, target));
                    assert!(!arrival_satisfies(
                        &label.replace("space_side=0", "space_side=3"),
                        target
                    ));
                }
                Progress::Input(_) if side == 3 => {}
                other => panic!("unexpected transition {other:?}"),
            }
        }
    }

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
            retreat_encounters: false,
            encounter_response: None,
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
            flight.observe(&step, hyper()),
            Progress::Input(Some(MenuKey::Cancel))
        );
        let map = Observation {
            cursor: Some(step.target.coordinates()),
            ..hyper()
        };
        flight.observe(&step, map);
        flight.observe(&step, map);
        let selected = Observation {
            autopilot: map.cursor,
            ..map
        };
        flight.observe(&step, selected);
        flight.observe(&step, hyper());
        assert_eq!(flight.observe(&step, hyper()), Progress::Input(None));
        let arrived = Observation {
            activity: 4,
            solar_active: true,
            current_star: Some(step.target.index()),
            fuel: Some(400),
            ..hyper()
        };
        let Progress::Arrived(label) = flight.observe(&step, arrived) else {
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
        assert_eq!(flight.observe(&step(), hyper()), Progress::Input(None));
        let mut short = step();
        short.max_ticks = 3;
        let Progress::Failed(label) = flight.observe(
            &short,
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
                &step(),
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
                &step(),
                Observation {
                    instant_move: true,
                    ..hyper()
                }
            ),
            Progress::Failed(_)
        ));
    }
}
