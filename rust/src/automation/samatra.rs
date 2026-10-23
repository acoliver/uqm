//! Input-only Sa-Matra controller. Boss health and lifecycle come from its owner.

use super::battle_fight::{bearing_facing, FightControl};
use super::fleet_fight::Picker;
use super::script::MenuKey;
use crate::battle::battle_types::shortest_path_delta;
use crate::pickship::{Cursor, FLAGSHIP_COLUMN};
use crate::ships::races::samatra::observation::{Events, Snapshot};

#[derive(Debug, Default, Clone, Copy, serde::Deserialize, serde::Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Until {
    Ready,
    GateOpen,
    #[default]
    GateEntered,
}

#[derive(Debug, Clone, Copy, serde::Deserialize, serde::Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct FightSamatraStep {
    pub max_ticks: u64,
    #[serde(default)]
    pub until: Until,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Controls {
    pub menu: Option<MenuKey>,
    pub flight: FightControl,
    pub escape: bool,
    pub special: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Progress {
    Running(Controls),
    Complete(String),
    Failed(&'static str),
}

#[derive(Debug, Default)]
pub struct Controller {
    ticks: u64,
    phase: Option<Until>,
    generation: Option<u64>,
    selected_generation: u64,
    saw_eight: bool,
    saw_open_gate: bool,
    used_escort: bool,
    used_bomb: bool,
}

impl Controller {
    pub fn observe(
        &mut self,
        max_ticks: u64,
        until: Until,
        activity: u16,
        events: Events,
        snapshot: Option<&Snapshot>,
        picker: Option<&Picker>,
    ) -> Progress {
        if self.phase != Some(until) {
            self.phase = Some(until);
            self.ticks = 0;
        }
        self.ticks += 1;
        if activity & (0x1000 | 0x2000 | 0x4000) != 0 {
            return Progress::Failed("samatra_aborted_or_loaded");
        }
        if let Some(generation) = self.generation {
            if generation != events.generation {
                return Progress::Failed("samatra_generation_changed");
            }
        } else if events.generation != 0 {
            if events.destroyed != 0 || events.gate_entered {
                return Progress::Failed("samatra_requires_fresh_eight_generators");
            }
            self.generation = Some(events.generation);
        }
        if events.gate_entered {
            return if self.saw_eight
                && self.saw_open_gate
                && self.used_escort
                && self.used_bomb
                && events.destroyed == u8::MAX
            {
                Progress::Complete(format!("samatra_won:generators_destroyed=8:gate_entered=1:escort=1:bomb=1:generation={}:ticks={}", events.generation, self.ticks))
            } else {
                Progress::Failed("samatra_gate_without_full_controller_proof")
            };
        }
        if self.ticks >= max_ticks {
            return Progress::Failed("samatra_timeout");
        }
        let remaining = snapshot.map_or(8 - events.destroyed.count_ones() as u8, |s| s.remaining);
        if let Some(s) = snapshot {
            self.saw_eight |= s.remaining == 8 && s.generators.len() == 8;
            self.saw_open_gate |= s.remaining == 0 && s.gate_open;
        }
        if until == Until::Ready
            && self.saw_eight
            && snapshot.is_some_and(|s| s.player.is_some_and(|p| p.crew > 0))
        {
            return Progress::Complete(format!(
                "samatra_ready:generators=8:generation={}",
                events.generation
            ));
        }
        if until == Until::GateOpen
            && self.saw_eight
            && self.used_escort
            && self.saw_open_gate
            && events.destroyed == 255
        {
            return Progress::Complete(format!(
                "samatra_gate_open:generators_destroyed=8:gate_open=1:generation={}",
                events.generation
            ));
        }
        if let Some(picker) = picker {
            if self.ticks % 4 != 1 || self.selected_generation == picker.generation {
                return Progress::Running(Controls::default());
            }
            let target = if self.saw_open_gate && events.destroyed == u8::MAX {
                Cursor::START
            } else if let Some(index) = picker.escorts.first() {
                Cursor {
                    x: (index % 6) as i16 + i16::from(index % 6 >= FLAGSHIP_COLUMN as u16),
                    y: (index / 6) as i16,
                }
            } else {
                return Progress::Failed("samatra_escorts_exhausted");
            };
            let key = if picker.cursor.x < target.x {
                MenuKey::Right
            } else if picker.cursor.x > target.x {
                MenuKey::Left
            } else if picker.cursor.y != target.y {
                MenuKey::Down
            } else {
                self.selected_generation = picker.generation;
                MenuKey::Select
            };
            return Progress::Running(Controls {
                menu: Some(key),
                ..Controls::default()
            });
        }
        let Some(s) = snapshot else {
            return Progress::Running(Controls::default());
        };
        let Some(player) = s.player.filter(|p| p.crew > 0) else {
            return Progress::Running(Controls::default());
        };
        if s.bomb_carrier {
            if remaining != 0 || !s.gate_open {
                return Progress::Failed("samatra_bomb_selected_before_gate_open");
            }
            self.used_bomb = true;
        } else {
            self.used_escort = true;
            if remaining == 0 {
                // The ordinary escort retreat leaves the flagship available in
                // the next picker. BOMB_CARRIER disables retreat for the SIS.
                return Progress::Running(Controls {
                    escape: true,
                    ..Controls::default()
                });
            }
        }
        let distance = |x, y| {
            let dx = shortest_path_delta(player.x, x, s.arena.0);
            let dy = shortest_path_delta(player.y, y, s.arena.1);
            (
                dx,
                dy,
                i64::from(dx) * i64::from(dx) + i64::from(dy) * i64::from(dy),
            )
        };
        let (dx, dy, squared) = if s.bomb_carrier {
            distance(s.gate.0, s.gate.1)
        } else if let Some(target) = s
            .generators
            .iter()
            .filter(|g| g.hit_points > 0)
            .min_by_key(|g| distance(g.x, g.y).2)
        {
            distance(target.x, target.y)
        } else {
            return Progress::Running(Controls::default());
        };
        let bearing = bearing_facing(dx, dy).unwrap_or(player.facing);
        let error = (i32::from(bearing) - i32::from(player.facing)).rem_euclid(16);
        let aimed = error <= 1 || error >= 15;
        Progress::Running(Controls {
            flight: FightControl {
                left: error > 8,
                right: (1..=8).contains(&error),
                thrust: aimed && (s.bomb_carrier || squared > 400 * 400),
                weapon: !s.bomb_carrier && aimed && squared < 650 * 650,
            },
            // Fury's normal special recharges its weapon energy.
            special: s.selected_species == Some(5),
            ..Controls::default()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::automation::battle_fight::ShipState;
    use crate::ships::races::samatra::observation::Generator;

    fn snapshot() -> Snapshot {
        Snapshot {
            remaining: 8,
            generators: (0..8)
                .map(|id| Generator {
                    id,
                    x: 4600,
                    y: 4000,
                    hit_points: 15,
                })
                .collect(),
            gate_open: false,
            gate: (4096, 3840),
            bomb_carrier: false,
            selected_species: Some(2),
            player: Some(ShipState {
                x: 4000,
                y: 4000,
                facing: 4,
                crew: 42,
            }),
            arena: (8192, 7680),
        }
    }

    #[test]
    fn escort_uses_real_thrust_fire_and_gate_requires_all_eight_deaths() {
        let mut controller = Controller::default();
        let mut s = snapshot();
        let mut events = Events {
            generation: 1,
            ..Events::default()
        };
        let Progress::Running(keys) =
            controller.observe(100, Until::GateEntered, 0x201, events, Some(&s), None)
        else {
            panic!()
        };
        assert!(keys.flight.thrust && keys.flight.weapon);
        s.remaining = 1;
        s.generators.truncate(1);
        events.destroyed = 127;
        assert!(matches!(
            controller.observe(100, Until::GateEntered, 0x201, events, Some(&s), None),
            Progress::Running(_)
        ));
        s.remaining = 0;
        s.generators.clear();
        s.gate_open = true;
        events.destroyed = 255;
        let Progress::Running(keys) =
            controller.observe(100, Until::GateEntered, 0x201, events, Some(&s), None)
        else {
            panic!()
        };
        assert!(keys.escape && !keys.flight.weapon);
        s.bomb_carrier = true;
        s.selected_species = Some(26);
        let Progress::Running(keys) =
            controller.observe(100, Until::GateEntered, 0x201, events, Some(&s), None)
        else {
            panic!()
        };
        assert!(!keys.escape && !keys.flight.weapon);
        events.gate_entered = true;
        assert!(matches!(
            controller.observe(100, Until::GateEntered, 1, events, None, None),
            Progress::Complete(_)
        ));
    }

    #[test]
    fn milestone_bounds_are_independent_but_keep_the_same_boss_proof() {
        let mut controller = Controller::default();
        let mut s = snapshot();
        let mut e = Events {
            generation: 1,
            ..Events::default()
        };
        assert!(matches!(
            controller.observe(2, Until::Ready, 0x201, e, Some(&s), None),
            Progress::Complete(_)
        ));
        assert!(matches!(
            controller.observe(2, Until::GateOpen, 0x201, e, Some(&s), None),
            Progress::Running(_)
        ));
        s.remaining = 0;
        s.generators.clear();
        s.gate_open = true;
        e.destroyed = 255;
        // The exact inclusive bound is enforced by the scheduler before
        // another callback, so leave room for the terminal observation.
        assert!(matches!(
            controller.observe(3, Until::GateOpen, 0x201, e, Some(&s), None),
            Progress::Complete(_)
        ));
        s.bomb_carrier = true;
        assert!(matches!(
            controller.observe(2, Until::GateEntered, 0x201, e, Some(&s), None),
            Progress::Running(_)
        ));
        e.gate_entered = true;
        assert!(matches!(
            controller.observe(3, Until::GateEntered, 1, e, None, None),
            Progress::Complete(_)
        ));
    }

    #[test]
    fn picker_selects_an_escort_then_the_flagship_only_after_the_gate_opens() {
        let mut controller = Controller::default();
        let mut s = snapshot();
        let mut e = Events {
            generation: 1,
            ..Events::default()
        };
        let mut picker = Picker {
            generation: 1,
            cursor: Cursor::START,
            escorts: vec![0],
        };
        let Progress::Running(keys) =
            controller.observe(100, Until::GateEntered, 0x201, e, None, Some(&picker))
        else {
            panic!()
        };
        assert_eq!(keys.menu, Some(MenuKey::Left));
        picker.cursor.x = 0;
        for _ in 0..3 {
            controller.observe(100, Until::GateEntered, 0x201, e, None, Some(&picker));
        }
        let Progress::Running(keys) =
            controller.observe(100, Until::GateEntered, 0x201, e, None, Some(&picker))
        else {
            panic!()
        };
        assert_eq!(keys.menu, Some(MenuKey::Select));
        controller.observe(100, Until::GateEntered, 0x201, e, Some(&s), None);
        s.remaining = 0;
        s.generators.clear();
        s.gate_open = true;
        e.destroyed = 255;
        controller.observe(100, Until::GateEntered, 0x201, e, Some(&s), None);
        picker.generation = 2;
        picker.cursor = Cursor::START;
        for _ in 0..1 {
            controller.observe(100, Until::GateEntered, 0x201, e, None, Some(&picker));
        }
        let Progress::Running(keys) =
            controller.observe(100, Until::GateEntered, 0x201, e, None, Some(&picker))
        else {
            panic!()
        };
        assert_eq!(keys.menu, Some(MenuKey::Select));
    }

    #[test]
    fn picker_never_treats_an_uninitialized_zero_counter_as_an_open_gate() {
        let mut controller = Controller::default();
        let mut uninitialized = snapshot();
        uninitialized.remaining = 0;
        uninitialized.generators.clear();
        uninitialized.player = None;
        let picker = Picker {
            generation: 1,
            cursor: Cursor::START,
            escorts: vec![0],
        };
        let Progress::Running(keys) = controller.observe(
            100,
            Until::Ready,
            0x201,
            Events::default(),
            Some(&uninitialized),
            Some(&picker),
        ) else {
            panic!()
        };
        assert_eq!(keys.menu, Some(MenuKey::Left));
    }

    #[test]
    fn stale_progress_new_generation_abort_and_premature_bomb_fail() {
        let e = Events {
            generation: 1,
            destroyed: 255,
            gate_entered: true,
        };
        assert!(matches!(
            Controller::default().observe(100, Until::GateEntered, 1, e, None, None),
            Progress::Failed(_)
        ));
        let mut c = Controller::default();
        let mut s = snapshot();
        let mut e = Events {
            generation: 1,
            ..Events::default()
        };
        c.observe(100, Until::GateEntered, 0x201, e, Some(&s), None);
        e.generation = 2;
        assert!(matches!(
            c.observe(100, Until::GateEntered, 0x201, e, Some(&s), None),
            Progress::Failed("samatra_generation_changed")
        ));
        s.bomb_carrier = true;
        assert!(matches!(
            Controller::default().observe(100, Until::GateEntered, 0x201, e, Some(&s), None),
            Progress::Failed("samatra_bomb_selected_before_gate_open")
        ));
        assert!(matches!(
            Controller::default().observe(100, Until::GateEntered, 0x4201, e, None, None),
            Progress::Failed(_)
        ));
        assert!(matches!(
            Controller::default().observe(
                1,
                Until::GateEntered,
                0x201,
                Events::default(),
                None,
                None
            ),
            Progress::Failed("samatra_timeout")
        ));
    }
}
