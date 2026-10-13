//! Outfit menu states.
//!
//! Ports the decision logic of `DoOutfit` (`outfit.c:605-795`) and the
//! `PM_FUEL` stepping of `DoMenuChooser` (`menu.c:410-467`,
//! `NextMenuState` `menu.c:211`, `PreviousMenuState` `menu.c:245`) as a pure
//! frame step.
//!
//! This is the Outfit state machine only. The module chooser
//! (`DoInstallModule`, see [`super::module_chooser`]) and Save/Load
//! (`GameOptions`) are returned as typed steps for the screen to run. The
//! screen runs the chooser; Save/Load is not ported and the screen reports it
//! as unported.

use super::fuel::FuelKey;
use super::input::MenuPulses;
use crate::mainloop::types::{activity_flags, ActivityValue};

/// `OUTFIT_FUEL` .. `OUTFIT_DOFUEL` (`outfit.c:38-45`).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OutfitState {
    Fuel = 0,
    Modules = 1,
    SaveLoad = 2,
    Exit = 3,
    /// Fuel gauge is flashing and the keys buy or drain fuel.
    DoFuel = 4,
}

impl OutfitState {
    /// `PreviousMenuState (PM_FUEL, ...)`: wraps from `Fuel` to `Exit`.
    /// `DoFuel` is not part of the chooser and stays put.
    #[must_use]
    const fn previous(self) -> Self {
        match self {
            OutfitState::Fuel => OutfitState::Exit,
            OutfitState::Modules => OutfitState::Fuel,
            OutfitState::SaveLoad => OutfitState::Modules,
            OutfitState::Exit => OutfitState::SaveLoad,
            OutfitState::DoFuel => OutfitState::DoFuel,
        }
    }

    /// `NextMenuState (PM_FUEL, ...)`: wraps from `Exit` to `Fuel`.
    #[must_use]
    const fn next(self) -> Self {
        match self {
            OutfitState::Fuel => OutfitState::Modules,
            OutfitState::Modules => OutfitState::SaveLoad,
            OutfitState::SaveLoad => OutfitState::Exit,
            OutfitState::Exit => OutfitState::Fuel,
            OutfitState::DoFuel => OutfitState::DoFuel,
        }
    }
}

/// Inputs of one frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OutfitFrameInput {
    /// Pulsed menu keys.
    pub pulses: MenuPulses,
    /// `GLOBAL (CurrentActivity)`.
    pub activity: ActivityValue,
}

/// What a frame decided.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutfitStep {
    /// First frame: load art and music, draw the screen. The fuel on board
    /// is rounded with [`super::fuel::round_fuel_on_outfit_entry`] after the
    /// first draw (`outfit.c:702-706`).
    Initialize,
    /// Nothing changed.
    Idle,
    /// The highlighted menu entry moved: redraw the menu strings.
    Highlight { from: OutfitState, to: OutfitState },
    /// `Fuel` selected: flash the fuel gauge, keys now buy fuel.
    EnterFuelAdjust,
    /// Select or cancel in fuel adjust: stop flashing the gauge.
    LeaveFuelAdjust,
    /// A fuel key was pulsed in fuel adjust. Run
    /// [`super::fuel::change_fuel_quantity`] with it.
    AdjustFuel(FuelKey),
    /// `Modules` selected: hand control to the module chooser. The menu comes
    /// back in the `Modules` state.
    EnterModules,
    /// `SaveLoad` selected: hand control to Save/Load, then report with
    /// [`SaveLoadReturn::from_game_options`].
    OpenSaveLoad,
    /// Release the art and leave the Outfit screen.
    Exit,
}

/// What `GameOptions` returning means for the Outfit screen
/// (`outfit.c:763-768`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveLoadReturn {
    /// `GameOptions` returned FALSE (a game was loaded or the player quit):
    /// leave the Outfit screen.
    ExitOutfit,
    /// Redraw the menu strings, flash the menu and carry on.
    RedrawAndContinue,
}

impl SaveLoadReturn {
    #[must_use]
    pub const fn from_game_options(returned_true: bool) -> Self {
        if returned_true {
            SaveLoadReturn::RedrawAndContinue
        } else {
            SaveLoadReturn::ExitOutfit
        }
    }
}

/// `MENU_STATE.CurState` and `MENU_STATE.Initialized` for the Outfit
/// screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OutfitMenu {
    state: OutfitState,
    initialized: bool,
}

impl Default for OutfitMenu {
    fn default() -> Self {
        Self::new()
    }
}

impl OutfitMenu {
    /// An Outfit screen about to open. The state is set to `Fuel` by the
    /// first frame (`outfit.c:624`).
    #[must_use]
    pub const fn new() -> Self {
        Self {
            state: OutfitState::Fuel,
            initialized: false,
        }
    }

    #[must_use]
    pub const fn state(&self) -> OutfitState {
        self.state
    }

    #[must_use]
    pub const fn initialized(&self) -> bool {
        self.initialized
    }

    /// One `DoOutfit` call.
    #[must_use]
    pub fn step(self, input: OutfitFrameInput) -> (Self, OutfitStep) {
        let pulses = input.pulses;

        if input.activity.0 & activity_flags::CHECK_ABORT != 0 {
            return (self, OutfitStep::Exit);
        }

        if !self.initialized {
            let menu = Self {
                state: OutfitState::Fuel,
                initialized: true,
            };
            return (menu, OutfitStep::Initialize);
        }

        if pulses.cancel || (pulses.select && self.state == OutfitState::Exit) {
            return if self.state == OutfitState::DoFuel {
                (
                    self.with_state(OutfitState::Fuel),
                    OutfitStep::LeaveFuelAdjust,
                )
            } else {
                (self, OutfitStep::Exit)
            };
        }

        if pulses.select {
            return match self.state {
                OutfitState::Fuel => (
                    self.with_state(OutfitState::DoFuel),
                    OutfitStep::EnterFuelAdjust,
                ),
                OutfitState::DoFuel => (
                    self.with_state(OutfitState::Fuel),
                    OutfitStep::LeaveFuelAdjust,
                ),
                OutfitState::Modules => (self, OutfitStep::EnterModules),
                OutfitState::SaveLoad => (self, OutfitStep::OpenSaveLoad),
                OutfitState::Exit => (self, OutfitStep::Exit),
            };
        }

        if self.state == OutfitState::DoFuel {
            return match FuelKey::from_pulses(pulses) {
                Some(key) => (self, OutfitStep::AdjustFuel(key)),
                None => (self, OutfitStep::Idle),
            };
        }

        let to = if pulses.previous() {
            self.state.previous()
        } else if pulses.next() {
            self.state.next()
        } else {
            return (self, OutfitStep::Idle);
        };
        (
            self.with_state(to),
            OutfitStep::Highlight {
                from: self.state,
                to,
            },
        )
    }

    fn with_state(self, state: OutfitState) -> Self {
        Self { state, ..self }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use OutfitState::{DoFuel, Exit, Fuel, Modules, SaveLoad};

    fn frame(pulses: MenuPulses) -> OutfitFrameInput {
        OutfitFrameInput {
            pulses,
            activity: ActivityValue::new(0),
        }
    }

    fn pressed(set: impl FnOnce(&mut MenuPulses)) -> MenuPulses {
        let mut pulses = MenuPulses::default();
        set(&mut pulses);
        pulses
    }

    fn ready_at(state: OutfitState) -> OutfitMenu {
        let (menu, _) = OutfitMenu::new().step(frame(MenuPulses::default()));
        menu.with_state(state)
    }

    fn select() -> MenuPulses {
        pressed(|p| p.select = true)
    }

    fn cancel() -> MenuPulses {
        pressed(|p| p.cancel = true)
    }

    #[test]
    fn first_frame_initialises_on_the_fuel_entry() {
        let (menu, step) = OutfitMenu::new().step(frame(select()));
        assert_eq!(step, OutfitStep::Initialize);
        assert!(menu.initialized());
        assert_eq!(menu.state(), Fuel);
    }

    #[test]
    fn chooser_wraps_in_both_directions() {
        let (menu, step) = ready_at(Fuel).step(frame(pressed(|p| p.up = true)));
        assert_eq!(menu.state(), Exit);
        assert_eq!(
            step,
            OutfitStep::Highlight {
                from: Fuel,
                to: Exit
            }
        );
        let (menu, _) = ready_at(Exit).step(frame(pressed(|p| p.down = true)));
        assert_eq!(menu.state(), Fuel);
    }

    #[test]
    fn chooser_visits_every_entry_in_order() {
        let mut menu = ready_at(Fuel);
        let mut seen = Vec::new();
        for _ in 0..4 {
            menu = menu.step(frame(pressed(|p| p.right = true))).0;
            seen.push(menu.state());
        }
        assert_eq!(seen, [Modules, SaveLoad, Exit, Fuel]);
        for _ in 0..4 {
            menu = menu.step(frame(pressed(|p| p.left = true))).0;
            seen.push(menu.state());
        }
        assert_eq!(&seen[4..], [Exit, SaveLoad, Modules, Fuel]);
    }

    #[test]
    fn select_on_fuel_enters_fuel_adjust_and_select_again_leaves() {
        let (menu, step) = ready_at(Fuel).step(frame(select()));
        assert_eq!(step, OutfitStep::EnterFuelAdjust);
        assert_eq!(menu.state(), DoFuel);
        let (menu, step) = menu.step(frame(select()));
        assert_eq!(step, OutfitStep::LeaveFuelAdjust);
        assert_eq!(menu.state(), Fuel);
    }

    #[test]
    fn cancel_in_fuel_adjust_returns_to_the_menu_without_leaving() {
        let (menu, step) = ready_at(DoFuel).step(frame(cancel()));
        assert_eq!(step, OutfitStep::LeaveFuelAdjust);
        assert_eq!(menu.state(), Fuel);
    }

    #[test]
    fn cancel_outside_fuel_adjust_leaves_the_screen() {
        for state in [Fuel, Modules, SaveLoad, Exit] {
            let (_, step) = ready_at(state).step(frame(cancel()));
            assert_eq!(step, OutfitStep::Exit, "{state:?}");
        }
    }

    #[test]
    fn select_on_exit_leaves_the_screen() {
        let (_, step) = ready_at(Exit).step(frame(select()));
        assert_eq!(step, OutfitStep::Exit);
    }

    #[test]
    fn select_on_modules_and_save_load_returns_typed_routes() {
        let (menu, step) = ready_at(Modules).step(frame(select()));
        assert_eq!(step, OutfitStep::EnterModules);
        assert_eq!(menu.state(), Modules);
        assert!(
            menu.initialized(),
            "the module chooser returns to the same screen"
        );
        let (menu, step) = ready_at(SaveLoad).step(frame(select()));
        assert_eq!(step, OutfitStep::OpenSaveLoad);
        assert_eq!(menu.state(), SaveLoad);
    }

    #[test]
    fn save_load_return_follows_the_game_options_result() {
        assert_eq!(
            SaveLoadReturn::from_game_options(false),
            SaveLoadReturn::ExitOutfit
        );
        assert_eq!(
            SaveLoadReturn::from_game_options(true),
            SaveLoadReturn::RedrawAndContinue
        );
    }

    #[test]
    fn fuel_keys_in_fuel_adjust_become_fuel_steps() {
        for (set, key) in [
            (
                (|p: &mut MenuPulses| p.up = true) as fn(&mut MenuPulses),
                FuelKey::UnitUp,
            ),
            (|p| p.down = true, FuelKey::UnitDown),
            (|p| p.page_up = true, FuelKey::BarUp),
            (|p| p.page_down = true, FuelKey::BarDown),
        ] {
            let before = ready_at(DoFuel);
            let (menu, step) = before.step(frame(pressed(set)));
            assert_eq!(step, OutfitStep::AdjustFuel(key));
            assert_eq!(menu, before);
        }
    }

    #[test]
    fn left_and_right_do_not_move_the_menu_in_fuel_adjust() {
        let both = pressed(|p| {
            p.left = true;
            p.right = true;
        });
        let before = ready_at(DoFuel);
        let (menu, step) = before.step(frame(both));
        assert_eq!(step, OutfitStep::Idle);
        assert_eq!(menu, before);
    }

    #[test]
    fn page_keys_do_nothing_in_the_menu_chooser() {
        let before = ready_at(Modules);
        let (menu, step) = before.step(frame(pressed(|p| p.page_up = true)));
        assert_eq!(step, OutfitStep::Idle);
        assert_eq!(menu, before);
    }

    #[test]
    fn abort_exits_before_anything_else() {
        for menu in [OutfitMenu::new(), ready_at(DoFuel)] {
            let input = OutfitFrameInput {
                pulses: select(),
                activity: ActivityValue::new(activity_flags::CHECK_ABORT),
            };
            assert_eq!(menu.step(input).1, OutfitStep::Exit);
        }
    }

    #[test]
    fn a_pending_load_does_not_exit_the_outfit_screen() {
        // DoOutfit tests CHECK_ABORT only (outfit.c:607).
        let input = OutfitFrameInput {
            pulses: MenuPulses::default(),
            activity: ActivityValue::new(activity_flags::CHECK_LOAD),
        };
        assert_eq!(ready_at(Fuel).step(input).1, OutfitStep::Idle);
    }
}
