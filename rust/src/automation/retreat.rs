//! Input-only flagship retreat for an interrupted hyperspace journey.

use super::fleet_fight::Observation;
use super::script::MenuKey;
use crate::pickship::Cursor;

#[derive(Debug, Default)]
pub struct Retreat {
    ticks: u64,
    preview_ticks: u64,
    started: bool,
    last_fleets: [u8; 2],
    selected_generation: u64,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Progress {
    Input { menu: Option<MenuKey>, escape: bool },
    Escaped(String),
    Won(String),
    Failed(String),
}

impl Retreat {
    pub fn observe(&mut self, max_ticks: u64, obs: Observation) -> Progress {
        self.ticks += 1;
        let Some(crew) = obs
            .flagship_crew
            .filter(|crew| *crew > 0 && *crew != u16::MAX)
        else {
            return Progress::Failed(format!(
                "retreat_flagship_lost:crew={:?}:activity={}",
                obs.flagship_crew, obs.activity
            ));
        };
        if obs.activity & (0x1000 | 0x2000 | 0x4000) != 0 {
            return Progress::Failed("retreat_aborted_or_loaded".into());
        }
        if self.ticks >= max_ticks {
            return Progress::Failed("retreat_timeout".into());
        }
        if !self.started && obs.encounter_menu {
            self.preview_ticks += 1;
            return Progress::Input {
                menu: match self.preview_ticks {
                    3 => Some(MenuKey::Down),
                    7 => Some(MenuKey::Select),
                    _ => None,
                },
                escape: false,
            };
        }
        let battle = obs.activity & 0x0200 != 0 && obs.activity & 0xff == 2;
        if !self.started {
            if !battle || obs.activity & 0xff != 2 || obs.counters[1] == 0 {
                return Progress::Failed("retreat_requires_ordinary_encounter".into());
            }
            self.started = true;
        }
        if battle {
            self.last_fleets = obs.counters;
        }
        if !battle {
            if obs.activity & 0xff == 3 && self.last_fleets[0] > 0 && self.last_fleets[1] == 0 {
                return Progress::Won(format!(
                    "transit_encounter_won:enemy_left=0:flagship_crew={crew}:ticks={}",
                    self.ticks
                ));
            }
            // EncounterBattle clears IN_BATTLE before restoring the hyperspace activity.
            if obs.activity & 0xff == 2 && self.last_fleets[1] > 0 {
                return Progress::Input {
                    menu: None,
                    escape: false,
                };
            }
            if obs.activity & 0xff != 3 || self.last_fleets[0] != 0 || self.last_fleets[1] == 0 {
                return Progress::Failed(format!(
                    "retreat_did_not_return_to_hyperspace:activity={}:last={:?}:now={:?}",
                    obs.activity, self.last_fleets, obs.counters
                ));
            }
            return Progress::Escaped(format!(
                "encounter_escaped:enemy_left={}:flagship_crew={crew}:ticks={}",
                self.last_fleets[1], self.ticks
            ));
        }
        let mut menu = None;
        if let Some(picker) = obs.picker {
            if self.ticks % 4 == 1 && picker.generation != self.selected_generation {
                menu = Some(if picker.cursor.x < Cursor::START.x {
                    MenuKey::Right
                } else if picker.cursor.x > Cursor::START.x {
                    MenuKey::Left
                } else if picker.cursor.y != Cursor::START.y {
                    MenuKey::Down
                } else {
                    self.selected_generation = picker.generation;
                    MenuKey::Select
                });
            }
        }
        Progress::Input { menu, escape: true }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::automation::fleet_fight::Picker;

    fn observation(activity: u16, crew: u16, enemies: u8) -> Observation {
        Observation {
            activity,
            cyborg: true,
            speed: 1,
            counters: [1, enemies],
            flagship_crew: Some(crew),
            picker: None,
            encounter_menu: false,
        }
    }

    #[test]
    fn retreat_picks_the_flagship_even_when_escorts_exist() {
        let mut retreat = Retreat::default();
        let mut obs = observation(0x0202, 32, 5);
        obs.picker = Some(Picker {
            generation: 1,
            cursor: Cursor::START,
            escorts: vec![0],
        });
        assert_eq!(
            retreat.observe(100, obs),
            Progress::Input {
                menu: Some(MenuKey::Select),
                escape: true,
            }
        );
    }

    #[test]
    fn escape_input_is_held_until_a_live_flagship_returns_with_enemies_remaining() {
        let mut retreat = Retreat::default();
        assert_eq!(
            retreat.observe(100, observation(0x0202, 32, 5)),
            Progress::Input {
                menu: None,
                escape: true
            }
        );
        let mut fleeing = observation(0x0202, 29, 5);
        fleeing.counters[0] = 0;
        retreat.observe(100, fleeing);
        assert_eq!(
            retreat.observe(100, observation(2, 29, 0)),
            Progress::Input {
                menu: None,
                escape: false
            }
        );
        let Progress::Escaped(label) = retreat.observe(100, observation(0x0203, 29, 0)) else {
            panic!("a surviving retreat must complete");
        };
        assert!(label.contains("enemy_left=5:flagship_crew=29"));
    }

    #[test]
    fn a_natural_cyborg_victory_also_allows_transit_to_resume() {
        let mut retreat = Retreat::default();
        retreat.observe(100, observation(0x0202, 32, 5));
        retreat.observe(100, observation(0x0202, 32, 0));
        assert!(matches!(
            retreat.observe(100, observation(3, 32, 0)),
            Progress::Won(_)
        ));
    }

    #[test]
    fn destruction_loading_and_a_non_hyperspace_return_do_not_prove_retreat() {
        for obs in [
            observation(3, u16::MAX, 5),
            observation(0x1003, 32, 5),
            observation(4, 32, 5),
        ] {
            let mut retreat = Retreat::default();
            retreat.observe(100, observation(0x0202, 32, 5));
            assert!(matches!(retreat.observe(100, obs), Progress::Failed(_)));
        }
    }
}
