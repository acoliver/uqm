//! Input-only device invocation and surface artifact collection.

use serde::{Deserialize, Serialize};

use super::navigation::NavigationControl;
use super::script::MenuKey;

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Device {
    PortalSpawner,
    TalkingPet,
    SunDevice,
    RosySphere,
    AquaHelix,
    ClearSpindle,
    UmgahCaster,
    BurvixeseCaster,
}

impl Device {
    pub const fn id(self) -> u8 {
        match self {
            Self::PortalSpawner => 0,
            Self::TalkingPet => 1,
            Self::SunDevice => 3,
            Self::RosySphere => 4,
            Self::AquaHelix => 5,
            Self::ClearSpindle => 6,
            Self::UmgahCaster => 12,
            Self::BurvixeseCaster => 13,
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::PortalSpawner => "portal_spawner",
            Self::TalkingPet => "talking_pet",
            Self::SunDevice => "sun_device",
            Self::RosySphere => "rosy_sphere",
            Self::AquaHelix => "aqua_helix",
            Self::ClearSpindle => "clear_spindle",
            Self::UmgahCaster => "umgah_caster",
            Self::BurvixeseCaster => "burvixese_caster",
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct UseDeviceStep {
    pub device: Device,
    pub max_ticks: u64,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CollectEnergyNodeStep {
    pub max_ticks: u64,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct DeviceObservation {
    pub in_orbit: bool,
    pub list: Option<Vec<u8>>,
    pub selected: usize,
    /// Successful invocation published by the device owner, with actual state effects.
    pub activation: Option<(u64, u8, String, bool)>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum DeviceProgress {
    Input(Option<MenuKey>),
    Complete(String),
    Failed(String),
}

#[derive(Debug, Default)]
pub struct DeviceTracker {
    ticks: u64,
    baseline: u64,
    selected: bool,
}

impl DeviceTracker {
    pub fn observe(&mut self, step: UseDeviceStep, obs: DeviceObservation) -> DeviceProgress {
        if self.ticks == 0 {
            self.baseline = obs
                .activation
                .as_ref()
                .map_or(0, |(generation, _, _, _)| *generation);
        }
        self.ticks += 1;
        if self.selected {
            if let Some((generation, id, state, verified)) = &obs.activation {
                if *generation > self.baseline && *id == step.device.id() && *verified {
                    return DeviceProgress::Complete(format!(
                        "device_used:device={}:generation={generation}:verified=1:state={state}",
                        step.device.name()
                    ));
                }
            }
        }
        if self.ticks >= step.max_ticks {
            return DeviceProgress::Failed(format!(
                "use_device_timeout:device={}:observed={obs:?}",
                step.device.name()
            ));
        }
        // Press/release pairs, never a held Select that could spill into dialogue.
        if self.ticks % 4 != 1 {
            return DeviceProgress::Input(None);
        }
        if let Some(list) = &obs.list {
            let Some(target) = list.iter().position(|id| *id == step.device.id()) else {
                return DeviceProgress::Failed(format!(
                    "device_not_aboard:device={}:list={list:?}",
                    step.device.name()
                ));
            };
            if self.selected {
                return DeviceProgress::Failed(format!(
                    "device_activation_refused:device={}",
                    step.device.name()
                ));
            }
            let key = match obs.selected.cmp(&target) {
                std::cmp::Ordering::Less => MenuKey::Down,
                std::cmp::Ordering::Greater => MenuKey::Up,
                std::cmp::Ordering::Equal => {
                    self.selected = true;
                    MenuKey::Select
                }
            };
            return DeviceProgress::Input(Some(key));
        }
        // From flight: Cancel opens the ship menu (Starmap), Down selects Devices.
        let key = match self.ticks {
            1 if obs.in_orbit => Some(MenuKey::Down),
            1 => Some(MenuKey::Cancel),
            13 => Some(MenuKey::Down),
            25 => Some(MenuKey::Select),
            _ => None,
        };
        DeviceProgress::Input(key)
    }
}

/// Only acquisition flags, excluding incidental visits and scan acknowledgements.
pub const ARTIFACT_FLAGS: &[&str] = &[
    "MOONBASE_ON_SHIP",
    "AQUA_HELIX_ON_SHIP",
    "TAALO_PROTECTOR_ON_SHIP",
    "SUN_DEVICE_ON_SHIP",
    "UTWIG_BOMB_ON_SHIP",
    "BURV_BROADCASTERS_ON_SHIP",
    "UMGAH_BROADCASTERS_ON_SHIP",
    "EGG_CASE0_ON_SHIP",
    "EGG_CASE1_ON_SHIP",
    "EGG_CASE2_ON_SHIP",
    "PORTAL_KEY_ON_SHIP",
    "SYREEN_SHUTTLE_ON_SHIP",
];

#[derive(Debug, Clone)]
pub struct EnergyObservation {
    pub report_active: bool,
    pub lander: super::planet_minerals::PlanetSnapshot,
    pub nodes: Vec<(u8, i32, i32)>,
    pub retrieved: u32,
    pub flags: Vec<u32>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum EnergyProgress {
    DismissReport(bool),
    Steering(NavigationControl),
    Complete(String),
    Failed(String),
}

#[derive(Debug, Default)]
pub struct EnergyTracker {
    ticks: u64,
    generation: Option<u64>,
    target: Option<u8>,
    flags: Vec<u32>,
    retrieved: u32,
}

impl EnergyTracker {
    pub fn observe(
        &mut self,
        step: CollectEnergyNodeStep,
        obs: &EnergyObservation,
    ) -> EnergyProgress {
        if self.generation.is_none() {
            self.generation = Some(obs.lander.generation);
            self.flags.clone_from(&obs.flags);
            self.retrieved = obs.retrieved;
        }
        self.ticks += 1;
        if self.generation != Some(obs.lander.generation) {
            return EnergyProgress::Failed("energy_collection_changed_trip".into());
        }
        if let Some(node) = self.target {
            let bit = 1u32 << node;
            if obs.retrieved & bit != 0 && self.retrieved & bit == 0 {
                if let Some(index) = self
                    .flags
                    .iter()
                    .zip(&obs.flags)
                    .position(|(before, after)| *before == 0 && *after != 0)
                {
                    return EnergyProgress::Complete(format!(
                        "energy_node_collected:generation={}:node={node}:retrieved={}:flag={}:value={}",
                        obs.lander.generation, obs.retrieved, ARTIFACT_FLAGS[index], obs.flags[index]
                    ));
                }
            }
        }
        if self.ticks >= step.max_ticks || !obs.lander.active || obs.lander.phase > 4 {
            return EnergyProgress::Failed(format!(
                "energy_collection_failed:trip={}:phase={}:retrieved={}:flags={:?}",
                obs.lander.generation, obs.lander.phase, obs.retrieved, obs.flags
            ));
        }
        if obs.report_active {
            return EnergyProgress::DismissReport(self.ticks % 8 < 2);
        }

        if obs.lander.phase < 4 {
            return EnergyProgress::Steering(NavigationControl::default());
        }
        if self.target.is_none() {
            self.target = obs.nodes.first().map(|(id, _, _)| *id);
        }
        let Some((_, x, y)) = obs.nodes.iter().find(|(id, _, _)| Some(*id) == self.target) else {
            return EnergyProgress::Failed("energy_node_missing_or_unretrieved".into());
        };
        EnergyProgress::Steering(super::planet_minerals::steer(
            obs.lander.position,
            obs.lander.facing,
            (*x, *y),
        ))
    }
}

pub const DEVICE_STORY_KEYS: &[&str] = &[
    "ULTRON_CONDITION",
    "ROSY_SPHERE_ON_SHIP",
    "AQUA_HELIX_ON_SHIP",
    "CLEAR_SPINDLE_ON_SHIP",
    "USED_BROADCASTER",
    "GLOBAL_FLAGS_AND_DATA",
    "CHMMR_EMERGING",
    "READY_TO_CONFUSE_URQUAN",
    "SHIP_TO_COMPEL",
    "PORTAL_COUNTER",
];

pub fn device_evidence_holds(device: Device, state: &str) -> bool {
    let mut before = std::collections::BTreeMap::new();
    let mut after = std::collections::BTreeMap::new();
    for field in state.split(',') {
        let Some((key, value)) = field.split_once('=') else {
            return false;
        };
        let Ok(value) = value.parse::<u32>() else {
            return false;
        };
        let (map, key) = if let Some(key) = key.strip_prefix("before_") {
            (&mut before, key)
        } else if let Some(key) = key.strip_prefix("after_") {
            (&mut after, key)
        } else {
            return false;
        };
        let maximum = if DEVICE_STORY_KEYS.contains(&key) {
            super::campaign_state::StateKey::parse(&format!("story:{key}"))
                .expect("device story key")
                .max_value()
        } else {
            match key {
                "fuel" => u32::MAX,
                "activity" => u32::from(u16::MAX),
                _ => return false,
            }
        };
        if value > maximum || map.insert(key.to_owned(), value).is_some() {
            return false;
        }
    }
    if before.len() != DEVICE_STORY_KEYS.len() + 2 || after.len() != before.len() {
        return false;
    }
    device_effect_holds(device.id(), &before, &after)
}

pub fn device_effect_holds(
    id: u8,
    before: &std::collections::BTreeMap<String, u32>,
    after: &std::collections::BTreeMap<String, u32>,
) -> bool {
    let changed_to_one = |key: &str| before[key] == 0 && after[key] == 1;
    let encounter = before["activity"] & 0x0400 == 0 && after["activity"] & 0x0400 != 0;
    match id {
        0 => {
            before["fuel"].checked_sub(1000) == Some(after["fuel"])
                && changed_to_one("PORTAL_COUNTER")
        }
        1 => encounter,
        3 => changed_to_one("CHMMR_EMERGING"),
        4..=6 => {
            let key = [
                "ROSY_SPHERE_ON_SHIP",
                "AQUA_HELIX_ON_SHIP",
                "CLEAR_SPINDLE_ON_SHIP",
            ][usize::from(id - 4)];
            before[key] == 1
                && after[key] == 0
                && before["ULTRON_CONDITION"] > 0
                && Some(after["ULTRON_CONDITION"]) == before["ULTRON_CONDITION"].checked_add(1)
        }
        12 | 13 => {
            changed_to_one("USED_BROADCASTER")
                || (encounter && [16, 32, 128].contains(&after["GLOBAL_FLAGS_AND_DATA"]))
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::planet_side::model::SurfacePoint;

    #[test]
    fn device_menu_opens_with_normal_flight_and_orbit_inputs() {
        for (in_orbit, first) in [(false, MenuKey::Cancel), (true, MenuKey::Down)] {
            let mut tracker = DeviceTracker::default();
            let step = UseDeviceStep {
                device: Device::SunDevice,
                max_ticks: 100,
            };
            let observation = DeviceObservation {
                in_orbit,
                ..Default::default()
            };
            for tick in 1..=25 {
                let wanted = match tick {
                    1 => Some(first),
                    13 => Some(MenuKey::Down),
                    25 => Some(MenuKey::Select),
                    _ => None,
                };
                assert_eq!(
                    tracker.observe(step, observation.clone()),
                    DeviceProgress::Input(wanted)
                );
            }
        }
    }

    #[test]
    fn discovery_reports_are_dismissed_with_bounded_button_pulses() {
        let mut tracker = EnergyTracker::default();
        let step = CollectEnergyNodeStep { max_ticks: 10 };
        let mut observation = energy();
        observation.report_active = true;
        assert_eq!(
            tracker.observe(step, &observation),
            EnergyProgress::DismissReport(true)
        );
        assert_eq!(
            tracker.observe(step, &observation),
            EnergyProgress::DismissReport(false)
        );
    }

    fn energy() -> EnergyObservation {
        EnergyObservation {
            report_active: false,
            lander: super::super::planet_minerals::PlanetSnapshot {
                active: true,
                generation: 7,
                phase: 4,
                position: SurfacePoint { x: 100, y: 100 },
                facing: 4,
                units: [0; 8],
                deposits: None,
            },
            nodes: vec![(0, 120, 100)],
            retrieved: 0,
            flags: vec![0; ARTIFACT_FLAGS.len()],
        }
    }

    #[test]
    fn proximity_and_flags_without_collision_do_not_complete() {
        let mut tracker = EnergyTracker::default();
        let mut obs = energy();
        let step = CollectEnergyNodeStep { max_ticks: 100 };
        assert!(matches!(
            tracker.observe(step, &obs),
            EnergyProgress::Steering(_)
        ));
        obs.lander.position.x = 120;
        obs.flags[0] = 1;
        assert!(matches!(
            tracker.observe(step, &obs),
            EnergyProgress::Steering(_)
        ));
        obs.retrieved = 1;
        assert!(matches!(
            tracker.observe(step, &obs),
            EnergyProgress::Complete(_)
        ));
    }

    #[test]
    fn collision_without_acquisition_and_stale_trips_fail() {
        let mut tracker = EnergyTracker::default();
        let mut obs = energy();
        let step = CollectEnergyNodeStep { max_ticks: 100 };
        tracker.observe(step, &obs);
        obs.retrieved = 1;
        obs.nodes.clear();
        assert!(matches!(
            tracker.observe(step, &obs),
            EnergyProgress::Failed(_)
        ));
        obs.lander.generation += 1;
        obs.flags[0] = 1;
        assert!(matches!(
            tracker.observe(step, &obs),
            EnergyProgress::Failed(_)
        ));
    }

    #[test]
    fn device_selects_live_identity_and_requires_fresh_activation() {
        let step = UseDeviceStep {
            device: Device::ClearSpindle,
            max_ticks: 100,
        };
        let mut tracker = DeviceTracker::default();
        let mut obs = DeviceObservation {
            in_orbit: false,
            list: Some(vec![4, 6, 7]),
            selected: 0,
            activation: Some((9, 6, "old".into(), true)),
        };
        assert_eq!(
            tracker.observe(step, obs.clone()),
            DeviceProgress::Input(Some(MenuKey::Down))
        );
        for _ in 0..3 {
            tracker.observe(step, obs.clone());
        }
        obs.selected = 1;
        assert_eq!(
            tracker.observe(step, obs.clone()),
            DeviceProgress::Input(Some(MenuKey::Select))
        );
        assert!(matches!(
            tracker.observe(step, obs.clone()),
            DeviceProgress::Input(None)
        ));
        obs.activation = Some((
            10,
            6,
            "ULTRON_CONDITION=2,CLEAR_SPINDLE_ON_SHIP=0".into(),
            true,
        ));
        assert!(matches!(
            tracker.observe(step, obs),
            DeviceProgress::Complete(_)
        ));
    }

    #[test]
    fn device_effects_require_actual_consumption_or_activation() {
        let mut before = std::collections::BTreeMap::from([
            ("activity".into(), 4),
            ("fuel".into(), 1500),
            ("ULTRON_CONDITION".into(), 1),
            ("CLEAR_SPINDLE_ON_SHIP".into(), 1),
            ("CHMMR_EMERGING".into(), 0),
            ("PORTAL_COUNTER".into(), 0),
        ]);
        let mut after = before.clone();
        assert!(!device_effect_holds(6, &before, &after));
        after.insert("ULTRON_CONDITION".into(), 2);
        assert!(!device_effect_holds(6, &before, &after));
        after.insert("CLEAR_SPINDLE_ON_SHIP".into(), 0);
        assert!(device_effect_holds(6, &before, &after));
        assert!(!device_effect_holds(3, &before, &after));
        after.insert("CHMMR_EMERGING".into(), 1);
        assert!(device_effect_holds(3, &before, &after));
        before.insert("activity".into(), 3);
        after.insert("activity".into(), 3);
        after.insert("PORTAL_COUNTER".into(), 1);
        assert!(!device_effect_holds(0, &before, &after));
        after.insert("fuel".into(), 500);
        assert!(device_effect_holds(0, &before, &after));
    }

    #[test]
    fn devices_missing_or_refused_and_bounded_energy_fail() {
        let step = UseDeviceStep {
            device: Device::AquaHelix,
            max_ticks: 20,
        };
        assert!(matches!(
            DeviceTracker::default().observe(
                step,
                DeviceObservation {
                    list: Some(vec![6]),
                    ..Default::default()
                }
            ),
            DeviceProgress::Failed(_)
        ));
        let mut tracker = EnergyTracker::default();
        assert!(matches!(
            tracker.observe(CollectEnergyNodeStep { max_ticks: 1 }, &energy()),
            EnergyProgress::Failed(_)
        ));
    }
}
