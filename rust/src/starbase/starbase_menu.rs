//! The four-state Starbase menu.
//!
//! Ports the decision logic of `DoStarBase` (`starbase.c:293-414`) as a pure
//! frame step. One call to [`StarbaseMenu::step`] corresponds to one call of
//! the C `InputFunc`. The step returns what the frame decided as a
//! [`StarbaseStep`]. Drawing the strings, the animation, the music and the
//! 1/30 second sleep stay with the caller.
//!
//! # Selection versus availability
//!
//! The state machine selects a route. It does not know whether the route
//! exists. A caller that cannot run the Shipyard yet must fail visibly when
//! it receives [`MenuCommand::Shipyard`]. It must not report the route as
//! completed.

use super::input::MenuPulses;
use crate::mainloop::types::{activity_flags, ActivityValue};

/// `STARBASE_STATE` (`starbase.h:28-33`).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StarbaseState {
    TalkCommander = 0,
    OutfitStarship = 1,
    Shipyard = 2,
    DepartBase = 3,
}

impl StarbaseState {
    /// Step back one entry, wrapping from `TalkCommander` to `DepartBase`
    /// (`starbase.c:392-397`).
    #[must_use]
    pub const fn previous(self) -> Self {
        match self {
            StarbaseState::TalkCommander => StarbaseState::DepartBase,
            StarbaseState::OutfitStarship => StarbaseState::TalkCommander,
            StarbaseState::Shipyard => StarbaseState::OutfitStarship,
            StarbaseState::DepartBase => StarbaseState::Shipyard,
        }
    }

    /// Step forward one entry, wrapping from `DepartBase` to
    /// `TalkCommander` (`starbase.c:398-402`).
    #[must_use]
    pub const fn next(self) -> Self {
        match self {
            StarbaseState::TalkCommander => StarbaseState::OutfitStarship,
            StarbaseState::OutfitStarship => StarbaseState::Shipyard,
            StarbaseState::Shipyard => StarbaseState::DepartBase,
            StarbaseState::DepartBase => StarbaseState::TalkCommander,
        }
    }
}

/// A route out of the menu that the caller runs before the menu comes back.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MenuCommand {
    /// Flush input, run the Commander conversation, then restore
    /// `GLOBAL_FLAGS_AND_DATA` to [`super::visit::IN_STARBASE_MARK`]
    /// (`starbase.c:357-363`).
    Talk,
    /// Run the Outfit screen (`starbase.c:371-373`).
    Outfit,
    /// Run the Shipyard screen (`starbase.c:374-376`).
    Shipyard,
}

/// Inputs of one frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StarbaseFrameInput {
    /// Pulsed menu keys.
    pub pulses: MenuPulses,
    /// `GLOBAL (CurrentActivity)`.
    pub activity: ActivityValue,
    /// `LastActivity`, used only on the first frame.
    pub last_activity: ActivityValue,
}

/// What a frame decided.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StarbaseStep {
    /// First frame: load the animation and music, draw the strings and play
    /// the music. `last_activity` is the new `LastActivity` with
    /// `CHECK_LOAD` cleared (`starbase.c:307`).
    Initialize { last_activity: ActivityValue },
    /// Nothing changed. The caller keeps animating.
    Idle,
    /// The highlighted entry moved: redraw the strings.
    Highlight {
        from: StarbaseState,
        to: StarbaseState,
    },
    /// Release the animation and music, run the route, then step again. The
    /// next step is a first frame, because the menu re-initialises
    /// (`starbase.c:356`).
    Run(MenuCommand),
    /// Release the animation and music and leave the menu loop.
    /// `clear_starbase_visited` is true when the caller must set
    /// `STARBASE_VISITED` to 0 (`starbase.c:347-353`).
    Depart { clear_starbase_visited: bool },
}

/// `MENU_STATE.CurState` and `MENU_STATE.Initialized` for the Starbase
/// menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StarbaseMenu {
    state: StarbaseState,
    initialized: bool,
}

impl Default for StarbaseMenu {
    fn default() -> Self {
        Self::new()
    }
}

impl StarbaseMenu {
    /// The zeroed menu state `VisitStarBase` starts with (`starbase.c:506`):
    /// `TALK_COMMANDER`, not initialised.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            state: StarbaseState::TalkCommander,
            initialized: false,
        }
    }

    #[must_use]
    pub const fn state(&self) -> StarbaseState {
        self.state
    }

    #[must_use]
    pub const fn initialized(&self) -> bool {
        self.initialized
    }

    /// One `DoStarBase` call.
    #[must_use]
    pub fn step(self, input: StarbaseFrameInput) -> (Self, StarbaseStep) {
        let leaving = input.activity.0 & (activity_flags::CHECK_ABORT | activity_flags::CHECK_LOAD);
        if leaving != 0 {
            let menu = Self {
                state: StarbaseState::DepartBase,
                ..self
            };
            return (
                menu,
                StarbaseStep::Depart {
                    clear_starbase_visited: false,
                },
            );
        }

        if !self.initialized {
            let menu = Self {
                initialized: true,
                ..self
            };
            let last_activity = input.last_activity.clear_flag(activity_flags::CHECK_LOAD);
            return (menu, StarbaseStep::Initialize { last_activity });
        }

        if input.pulses.select {
            return match self.state {
                StarbaseState::DepartBase => (
                    self,
                    StarbaseStep::Depart {
                        clear_starbase_visited: true,
                    },
                ),
                StarbaseState::TalkCommander => self.run(MenuCommand::Talk),
                StarbaseState::OutfitStarship => self.run(MenuCommand::Outfit),
                StarbaseState::Shipyard => self.run(MenuCommand::Shipyard),
            };
        }

        let to = if input.pulses.previous() {
            self.state.previous()
        } else if input.pulses.next() {
            self.state.next()
        } else {
            return (self, StarbaseStep::Idle);
        };
        let menu = Self { state: to, ..self };
        (
            menu,
            StarbaseStep::Highlight {
                from: self.state,
                to,
            },
        )
    }

    fn run(self, command: MenuCommand) -> (Self, StarbaseStep) {
        (
            Self {
                initialized: false,
                ..self
            },
            StarbaseStep::Run(command),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use StarbaseState::{DepartBase, OutfitStarship, Shipyard, TalkCommander};

    const ALL: [StarbaseState; 4] = [TalkCommander, OutfitStarship, Shipyard, DepartBase];

    fn frame(pulses: MenuPulses) -> StarbaseFrameInput {
        StarbaseFrameInput {
            pulses,
            activity: ActivityValue::new(0),
            last_activity: ActivityValue::new(0),
        }
    }

    fn pressed(set: impl FnOnce(&mut MenuPulses)) -> MenuPulses {
        let mut pulses = MenuPulses::default();
        set(&mut pulses);
        pulses
    }

    /// A menu past its first frame, parked on `state`.
    fn ready_at(state: StarbaseState) -> StarbaseMenu {
        let (menu, _) = StarbaseMenu::new().step(frame(MenuPulses::default()));
        StarbaseMenu { state, ..menu }
    }

    #[test]
    fn a_fresh_menu_is_talk_commander_and_uninitialised() {
        let menu = StarbaseMenu::new();
        assert_eq!(menu.state(), TalkCommander);
        assert!(!menu.initialized());
        assert_eq!(StarbaseState::TalkCommander as u8, 0);
        assert_eq!(StarbaseState::DepartBase as u8, 3);
    }

    #[test]
    fn first_frame_initialises_and_clears_check_load_in_last_activity() {
        let input = StarbaseFrameInput {
            last_activity: ActivityValue::new(activity_flags::CHECK_LOAD | 0x0204),
            ..frame(MenuPulses::default())
        };
        let (menu, step) = StarbaseMenu::new().step(input);
        assert!(menu.initialized());
        assert_eq!(
            step,
            StarbaseStep::Initialize {
                last_activity: ActivityValue::new(0x0204)
            }
        );
    }

    #[test]
    fn first_frame_ignores_a_select_pulse() {
        let (menu, step) = StarbaseMenu::new().step(frame(pressed(|p| p.select = true)));
        assert!(matches!(step, StarbaseStep::Initialize { .. }));
        assert!(menu.initialized());
        assert_eq!(menu.state(), TalkCommander);
    }

    #[test]
    fn previous_and_next_wrap_in_both_directions() {
        assert_eq!(TalkCommander.previous(), DepartBase);
        assert_eq!(DepartBase.next(), TalkCommander);
        for state in ALL {
            assert_eq!(state.next().previous(), state);
            assert_eq!(state.previous().next(), state);
        }
    }

    #[test]
    fn left_and_up_move_back_and_right_and_down_move_forward() {
        let backward: [fn(&mut MenuPulses); 2] = [|p| p.left = true, |p| p.up = true];
        let forward: [fn(&mut MenuPulses); 2] = [|p| p.right = true, |p| p.down = true];
        for start in ALL {
            for set in backward {
                let (menu, step) = ready_at(start).step(frame(pressed(set)));
                assert_eq!(menu.state(), start.previous());
                assert_eq!(
                    step,
                    StarbaseStep::Highlight {
                        from: start,
                        to: start.previous()
                    }
                );
            }
            for set in forward {
                let (menu, step) = ready_at(start).step(frame(pressed(set)));
                assert_eq!(menu.state(), start.next());
                assert_eq!(
                    step,
                    StarbaseStep::Highlight {
                        from: start,
                        to: start.next()
                    }
                );
            }
        }
    }

    #[test]
    fn four_forward_steps_return_to_the_start() {
        let mut menu = ready_at(OutfitStarship);
        for _ in 0..4 {
            menu = menu.step(frame(pressed(|p| p.down = true))).0;
        }
        assert_eq!(menu.state(), OutfitStarship);
    }

    #[test]
    fn previous_wins_when_both_directions_are_pulsed() {
        let both = pressed(|p| {
            p.left = true;
            p.right = true;
        });
        let (menu, _) = ready_at(OutfitStarship).step(frame(both));
        assert_eq!(menu.state(), TalkCommander);
    }

    #[test]
    fn cancel_and_fuel_keys_do_nothing() {
        let setters: [fn(&mut MenuPulses); 3] = [
            |p| p.cancel = true,
            |p| p.page_up = true,
            |p| p.page_down = true,
        ];
        for set in setters {
            let before = ready_at(Shipyard);
            let (menu, step) = before.step(frame(pressed(set)));
            assert_eq!(step, StarbaseStep::Idle);
            assert_eq!(menu, before);
        }
        let (_, step) = ready_at(Shipyard).step(frame(MenuPulses::default()));
        assert_eq!(step, StarbaseStep::Idle);
    }

    #[test]
    fn select_returns_a_typed_command_per_state_and_reinitialises() {
        let select = pressed(|p| p.select = true);
        for (state, command) in [
            (TalkCommander, MenuCommand::Talk),
            (OutfitStarship, MenuCommand::Outfit),
            (Shipyard, MenuCommand::Shipyard),
        ] {
            let (menu, step) = ready_at(state).step(frame(select));
            assert_eq!(step, StarbaseStep::Run(command));
            assert_eq!(menu.state(), state, "selection is kept for the return");
            assert!(!menu.initialized());
            let (menu, step) = menu.step(frame(MenuPulses::default()));
            assert!(matches!(step, StarbaseStep::Initialize { .. }));
            assert!(menu.initialized());
        }
    }

    #[test]
    fn select_beats_movement() {
        let both = pressed(|p| {
            p.select = true;
            p.down = true;
        });
        let (menu, step) = ready_at(TalkCommander).step(frame(both));
        assert_eq!(step, StarbaseStep::Run(MenuCommand::Talk));
        assert_eq!(menu.state(), TalkCommander);
    }

    #[test]
    fn depart_clears_starbase_visited_in_a_normal_exit() {
        let (menu, step) = ready_at(DepartBase).step(frame(pressed(|p| p.select = true)));
        assert_eq!(
            step,
            StarbaseStep::Depart {
                clear_starbase_visited: true
            }
        );
        assert_eq!(menu.state(), DepartBase);
    }

    #[test]
    fn abort_or_load_forces_depart_and_keeps_starbase_visited() {
        for flag in [activity_flags::CHECK_ABORT, activity_flags::CHECK_LOAD] {
            for menu in [StarbaseMenu::new(), ready_at(OutfitStarship)] {
                let input = StarbaseFrameInput {
                    activity: ActivityValue::new(flag),
                    ..frame(pressed(|p| p.select = true))
                };
                let (menu, step) = menu.step(input);
                assert_eq!(
                    step,
                    StarbaseStep::Depart {
                        clear_starbase_visited: false
                    }
                );
                assert_eq!(menu.state(), DepartBase);
            }
        }
    }

    #[test]
    fn unrelated_activity_flags_do_not_force_depart() {
        let input = StarbaseFrameInput {
            activity: ActivityValue::new(activity_flags::START_INTERPLANETARY | 0x0004),
            ..frame(pressed(|p| p.down = true))
        };
        let (_, step) = ready_at(TalkCommander).step(input);
        assert!(matches!(step, StarbaseStep::Highlight { .. }));
    }

    #[test]
    fn menu_pulses_are_separate_from_flight_state() {
        // The step takes only menu pulses and activity words. A held flight
        // key has no field to land in, so it cannot move the highlight.
        let (_, step) = ready_at(TalkCommander).step(frame(MenuPulses::default()));
        assert_eq!(step, StarbaseStep::Idle);
    }
}
