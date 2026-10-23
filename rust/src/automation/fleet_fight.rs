//! Whole-encounter completion using the player's normal-speed Cyborg setting.
//!
//! Combat remains the game's own AI. The controller only operates the ship
//! picker, preferring live escorts before committing the flagship.

use super::script::MenuKey;
use crate::pickship::{Cursor, FLAGSHIP_COLUMN};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Picker {
    pub generation: u64,
    pub cursor: Cursor,
    pub escorts: Vec<u16>,
}

#[derive(Debug, Clone)]
pub struct Observation {
    pub activity: u16,
    pub cyborg: bool,
    /// Setting 1 is normal rendered speed, not the accelerated settings.
    pub speed: u8,
    pub counters: [u8; 2],
    pub flagship_crew: Option<u16>,
    pub picker: Option<Picker>,
    pub encounter_menu: bool,
    pub escape_generation: u64,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Progress {
    Input(Option<MenuKey>),
    Won(String),
    Failed(String),
}

#[derive(Debug, Default)]
pub struct FleetFight {
    ticks: u64,
    started: bool,
    preview_ticks: u64,
    initial: [u8; 2],
    last: [u8; 2],
    picks: u32,
    selected_generation: u64,
}

impl FleetFight {
    pub fn observe(&mut self, max_ticks: u64, obs: Observation) -> Progress {
        self.ticks += 1;
        if !obs.cyborg || obs.speed != 1 {
            return Progress::Failed(format!(
                "fleet_fight_requires_normal_cyborg:enabled={}:speed={}",
                obs.cyborg, obs.speed
            ));
        }
        let Some(crew) = obs
            .flagship_crew
            .filter(|crew| *crew > 0 && *crew != u16::MAX)
        else {
            return Progress::Failed("fleet_fight_flagship_lost".into());
        };
        if obs.activity & (0x1000 | 0x2000 | 0x4000) != 0 {
            return Progress::Failed("fleet_fight_aborted_or_loaded".into());
        }
        if !self.started && obs.encounter_menu {
            if self.ticks >= max_ticks {
                return Progress::Failed("fleet_fight_timeout:encounter_menu".into());
            }
            self.preview_ticks += 1;
            // Its first callback initializes the menu and ignores keys. Release
            // through initialization, then move from Hail to Attack and confirm.
            return Progress::Input(match self.preview_ticks {
                3 => Some(MenuKey::Down),
                7 => Some(MenuKey::Select),
                _ => None,
            });
        }
        let in_battle = obs.activity & 0x0200 != 0;
        if !self.started {
            if !in_battle || obs.activity & 0xff != 2 || obs.counters[1] == 0 {
                return Progress::Failed("fleet_fight_requires_ordinary_encounter".into());
            }
            self.started = true;
            self.initial = obs.counters;
        }
        if in_battle {
            self.last = obs.counters;
        } else {
            if self.last[1] != 0 || self.last[0] == 0 {
                return Progress::Failed(format!(
                    "fleet_fight_incomplete:player_left={}:enemy_left={}",
                    self.last[0], self.last[1]
                ));
            }
            return Progress::Won(format!(
                "fleet_won:player_left={}:enemy_left=0:ticks={}:flagship_crew={crew}:enemy_initial={}:picks={}:speed=1",
                self.last[0], self.ticks, self.initial[1], self.picks
            ));
        }
        if self.ticks >= max_ticks {
            return Progress::Failed(format!(
                "fleet_fight_timeout:player_left={}:enemy_left={}",
                self.last[0], self.last[1]
            ));
        }
        let Some(picker) = obs.picker else {
            return Progress::Input(None);
        };
        // Release between each pulse, including after confirming a selection.
        if self.ticks % 4 != 1 || picker.generation == self.selected_generation {
            return Progress::Input(None);
        }
        let target = picker
            .escorts
            .first()
            .map_or(Cursor::START, |index| Cursor {
                x: (index % 6) as i16 + i16::from(index % 6 >= FLAGSHIP_COLUMN as u16),
                y: (index / 6) as i16,
            });
        let key = if picker.cursor.x < target.x {
            MenuKey::Right
        } else if picker.cursor.x > target.x {
            MenuKey::Left
        } else if picker.cursor.y != target.y {
            MenuKey::Down
        } else {
            self.selected_generation = picker.generation;
            self.picks += 1;
            MenuKey::Select
        };
        Progress::Input(Some(key))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn obs(counters: [u8; 2]) -> Observation {
        Observation {
            activity: 0x202,
            cyborg: true,
            speed: 1,
            counters,
            flagship_crew: Some(46),
            picker: None,
            encounter_menu: false,
            escape_generation: 0,
        }
    }

    #[test]
    fn encounter_preview_attacks_by_menu_input_before_observing_battle_counters() {
        let mut fight = FleetFight::default();
        let mut preview = obs([0, 0]);
        preview.activity = 3;
        preview.encounter_menu = true;
        for _ in 0..2 {
            assert_eq!(fight.observe(100, preview.clone()), Progress::Input(None));
        }
        assert_eq!(
            fight.observe(100, preview.clone()),
            Progress::Input(Some(MenuKey::Down))
        );
        for _ in 0..3 {
            assert_eq!(fight.observe(100, preview.clone()), Progress::Input(None));
        }
        assert_eq!(
            fight.observe(100, preview),
            Progress::Input(Some(MenuKey::Select))
        );
        assert_eq!(fight.observe(100, obs([5, 3])), Progress::Input(None));
    }

    #[test]
    fn one_kill_cannot_satisfy_a_fleet_result() {
        let mut fight = FleetFight::default();
        assert_eq!(fight.observe(100, obs([5, 3])), Progress::Input(None));
        assert_eq!(fight.observe(100, obs([5, 2])), Progress::Input(None));
        let mut ended = obs([0, 0]);
        ended.activity = 4;
        assert!(matches!(fight.observe(100, ended), Progress::Failed(_)));
    }

    #[test]
    fn losses_and_new_picker_generations_continue_until_the_enemy_fleet_is_empty() {
        let mut fight = FleetFight::default();
        let mut first = obs([5, 3]);
        first.picker = Some(Picker {
            generation: 1,
            cursor: Cursor::START,
            escorts: vec![2, 3],
        });
        assert_eq!(
            fight.observe(100, first.clone()),
            Progress::Input(Some(MenuKey::Left))
        );
        first.picker.as_mut().unwrap().cursor.x = 2;
        for _ in 0..3 {
            fight.observe(100, first.clone());
        }
        assert_eq!(
            fight.observe(100, first.clone()),
            Progress::Input(Some(MenuKey::Select))
        );
        for _ in 0..4 {
            assert_eq!(fight.observe(100, first.clone()), Progress::Input(None));
        }
        let mut next = obs([4, 2]);
        next.picker = Some(Picker {
            generation: 2,
            cursor: Cursor::START,
            escorts: vec![3],
        });
        for _ in 0..3 {
            fight.observe(100, next.clone());
        }
        assert_eq!(
            fight.observe(100, next.clone()),
            Progress::Input(Some(MenuKey::Right))
        );
        next.picker.as_mut().unwrap().cursor.x = 4;
        for _ in 0..3 {
            assert_eq!(fight.observe(100, next.clone()), Progress::Input(None));
        }
        assert_eq!(
            fight.observe(100, next),
            Progress::Input(Some(MenuKey::Select))
        );
        fight.observe(100, obs([4, 0]));
        let mut ended = obs([0, 0]);
        ended.activity = 4;
        let Progress::Won(label) = fight.observe(100, ended) else {
            panic!("expected whole fleet win")
        };
        assert!(label.contains("enemy_left=0"));
        assert!(label.contains("enemy_initial=3"));
        assert!(label.contains("picks=2"));
        assert!(label.contains("flagship_crew=46"));
    }

    #[test]
    fn requires_normal_rendered_cyborg_and_a_living_flagship() {
        for speed in [0, 2, 3] {
            let mut observation = obs([5, 2]);
            observation.speed = speed;
            assert!(matches!(
                FleetFight::default().observe(100, observation),
                Progress::Failed(_)
            ));
        }
        for crew in [None, Some(0), Some(u16::MAX)] {
            let mut observation = obs([5, 2]);
            observation.flagship_crew = crew;
            assert!(matches!(
                FleetFight::default().observe(100, observation),
                Progress::Failed(_)
            ));
        }
    }

    #[test]
    fn stale_counters_outside_a_battle_and_budget_exhaustion_fail() {
        let mut observation = obs([5, 0]);
        observation.activity = 4;
        assert!(matches!(
            FleetFight::default().observe(100, observation),
            Progress::Failed(_)
        ));
        assert!(matches!(
            FleetFight::default().observe(1, obs([5, 2])),
            Progress::Failed(_)
        ));
    }
}

#[derive(Debug, Default)]
pub struct FleetRetreat {
    ticks: u64,
    started: bool,
    escape_pressed: bool,
    preview_ticks: u64,
    enemy_left: u8,
    escape_baseline: Option<u64>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum RetreatProgress {
    Input { menu: Option<MenuKey>, escape: bool },
    Escaped(String),
    Failed(String),
}

impl FleetRetreat {
    pub fn observe(&mut self, max_ticks: u64, obs: Observation) -> RetreatProgress {
        use RetreatProgress::*;
        let baseline = *self.escape_baseline.get_or_insert(obs.escape_generation);
        self.ticks += 1;
        if obs
            .flagship_crew
            .is_none_or(|crew| crew == 0 || crew == u16::MAX)
            || obs.activity & (0x1000 | 0x2000 | 0x4000) != 0
        {
            return Failed(format!(
                "fleet_retreat_flagship_lost_or_aborted:crew={:?}:activity={}:ticks={}:escape_baseline={baseline}:escape_generation={}",
                obs.flagship_crew, obs.activity, self.ticks, obs.escape_generation
            ));
        }
        if self.ticks >= max_ticks {
            return Failed("fleet_retreat_timeout".into());
        }
        if !self.started && obs.encounter_menu {
            self.preview_ticks += 1;
            return Input {
                menu: match self.preview_ticks {
                    3 => Some(MenuKey::Down),
                    7 => Some(MenuKey::Select),
                    _ => None,
                },
                escape: false,
            };
        }
        if obs.activity & 0x0200 == 0 || obs.activity & 0xff != 2 {
            return if self.started
                && self.escape_pressed
                && obs.escape_generation > baseline
                && self.enemy_left > 0
                && matches!(obs.activity & 0xff, 3 | 4)
            {
                Escaped(format!(
                    "fleet_escaped:enemy_initial={}:flagship_crew={}:ticks={}",
                    self.enemy_left,
                    obs.flagship_crew.unwrap(),
                    self.ticks
                ))
            } else {
                Input {
                    menu: None,
                    escape: false,
                }
            };
        }
        self.started = true;
        self.enemy_left = self.enemy_left.max(obs.counters[1]);
        if let Some(picker) = obs.picker {
            let target = Cursor::START;
            let menu = if self.ticks % 4 != 1 {
                None
            } else if picker.cursor.x < target.x {
                Some(MenuKey::Right)
            } else if picker.cursor.x > target.x {
                Some(MenuKey::Left)
            } else if picker.cursor.y != target.y {
                Some(MenuKey::Down)
            } else {
                Some(MenuKey::Select)
            };
            return Input {
                menu,
                escape: false,
            };
        }
        self.escape_pressed = true;
        Input {
            menu: None,
            escape: true,
        }
    }
}

#[cfg(test)]
mod retreat_tests {
    use super::*;
    fn obs(activity: u16) -> Observation {
        Observation {
            activity,
            cyborg: true,
            speed: 1,
            counters: [4, 2],
            flagship_crew: Some(32),
            picker: None,
            encounter_menu: false,
            escape_generation: 0,
        }
    }
    #[test]
    fn encounter_handoff_waits_but_cannot_outlive_its_budget() {
        let mut retreat = FleetRetreat::default();
        assert!(matches!(
            retreat.observe(3, obs(2)),
            RetreatProgress::Input { escape: false, .. }
        ));
        assert!(matches!(
            retreat.observe(3, obs(2)),
            RetreatProgress::Input { escape: false, .. }
        ));
        assert!(matches!(
            retreat.observe(3, obs(2)),
            RetreatProgress::Failed(_)
        ));
    }

    #[test]
    fn flagship_picker_does_not_send_escape_before_selection() {
        let mut retreat = FleetRetreat::default();
        let mut selecting = obs(0x202);
        selecting.picker = Some(Picker {
            generation: 1,
            cursor: Cursor::START,
            escorts: vec![0, 1],
        });
        assert_eq!(
            retreat.observe(100, selecting),
            RetreatProgress::Input {
                menu: Some(MenuKey::Select),
                escape: false
            }
        );
        let mut battle = obs(0x202);
        battle.counters[1] = 0;
        assert!(matches!(
            retreat.observe(100, battle),
            RetreatProgress::Input { escape: true, .. }
        ));
        let mut returned = obs(0x203);
        returned.escape_generation = 1;
        assert!(matches!(
            retreat.observe(100, returned),
            RetreatProgress::Escaped(_)
        ));
    }

    #[test]
    fn retreat_requires_escape_input_living_flagship_and_surviving_enemy() {
        let mut retreat = FleetRetreat::default();
        assert!(matches!(
            retreat.observe(100, obs(0x202)),
            RetreatProgress::Input { escape: true, .. }
        ));
        let mut returned = obs(3);
        assert!(matches!(
            retreat.observe(100, returned.clone()),
            RetreatProgress::Input { .. }
        ));
        returned.escape_generation = 1;
        assert!(matches!(
            retreat.observe(100, returned),
            RetreatProgress::Escaped(_)
        ));
        assert!(matches!(
            FleetRetreat::default().observe(1, obs(3)),
            RetreatProgress::Failed(_)
        ));
        let mut dead = obs(0x202);
        dead.flagship_crew = Some(u16::MAX);
        assert!(matches!(
            FleetRetreat::default().observe(100, dead),
            RetreatProgress::Failed(_)
        ));
        let mut win = obs(0x202);
        win.counters[1] = 0;
        let mut retreat = FleetRetreat::default();
        retreat.observe(100, win);
        assert!(matches!(
            retreat.observe(1, obs(3)),
            RetreatProgress::Failed(_)
        ));
    }
}
