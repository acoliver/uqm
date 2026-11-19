//! Controller storage retained by the native input runtime and its observers.

use super::frames::{
    InputFrames, InputState, FLIGHT_KEYS, KEY_DOWN, KEY_ESCAPE, KEY_LEFT, KEY_RIGHT, KEY_SPECIAL,
    KEY_UP, KEY_WEAPON, TEMPLATES,
};
use std::cell::UnsafeCell;
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread::ThreadId;

#[derive(Default)]
struct ControllerState {
    immediate: Box<UnsafeCell<InputState>>,
    frames: InputFrames,
    player_controls: [u32; 2],
    paused: bool,
    exit_requested: bool,
    menu_sounds: (u16, u16),
    injection_rejected: bool,
}

/// `BATTLE_INPUT_STATE` bits of the flight keys, in the order of `controls.h`.
fn battle_input(keys: &[i32; FLIGHT_KEYS]) -> u8 {
    [
        (KEY_UP, 2),
        (KEY_LEFT, 0),
        (KEY_RIGHT, 1),
        (KEY_WEAPON, 3),
        (KEY_SPECIAL, 4),
        (KEY_ESCAPE, 5),
        (KEY_DOWN, 6),
    ]
    .into_iter()
    .fold(0, |flags, (key, bit)| {
        flags | (u8::from(keys[key as usize] != 0) << bit)
    })
}

/// Clones retain the same allocation; access stays on the provider's owner thread.
#[derive(Clone)]
pub struct InputControls {
    state: Arc<Mutex<ControllerState>>,
    owner: ThreadId,
}

impl Default for InputControls {
    fn default() -> Self {
        Self {
            state: Arc::new(Mutex::new(ControllerState::default())),
            owner: std::thread::current().id(),
        }
    }
}

impl InputControls {
    fn state(&self) -> MutexGuard<'_, ControllerState> {
        assert_eq!(
            self.owner,
            std::thread::current().id(),
            "input used off its owner thread"
        );
        self.state
            .lock()
            .unwrap_or_else(|_| panic!("input state poisoned"))
    }

    /// The provider may write this stable buffer only on the owner thread,
    /// between control operations. No state guard crosses a provider call.
    pub fn immediate_ptr(&self) -> *mut InputState {
        self.state().immediate.get()
    }

    pub fn immediate(&self) -> InputState {
        // SAFETY: the thread check excludes concurrent provider access; no borrowed view escapes.
        unsafe { self.state().immediate.get().read() }
    }

    pub fn current(&self) -> InputState {
        self.state().frames.current
    }

    pub fn pulsed(&self) -> InputState {
        self.state().frames.pulsed
    }

    pub fn advance(&self, before: InputState, after: InputState, now: u32) {
        self.state().frames.advance(before, after, now);
    }

    pub fn clear_menu_state(&self) {
        self.state().frames.clear_menu_state();
    }

    pub fn reset_repeat(&self, now: u32) {
        self.state().frames.reset_repeat(now);
    }

    pub fn configure_repeat(&self, minimum: u32, maximum: u32, step: u32, gestalt: bool, now: u32) {
        self.state()
            .frames
            .configure_repeat(minimum, maximum, step, gestalt, now);
    }

    /// `SetDefaultMenuRepeatDelay`: the first repeat after half a second, then
    /// one every twelfth of a second.
    pub fn set_default_menu_repeat(&self, now: u32) {
        self.configure_repeat(840 / 12, 840 / 2, 840 / 12, false, now);
    }

    /// One player's held flight keys as battle input flags.
    pub fn current_battle_input(&self, player: u16) -> u8 {
        battle_input(&self.current().key[self.player_controls()[usize::from(player)] as usize])
    }

    /// One player's newly pressed flight keys as battle input flags.
    pub fn pulsed_battle_input(&self, player: u16) -> u8 {
        battle_input(&self.pulsed().key[self.player_controls()[usize::from(player)] as usize])
    }

    pub fn repeat_config(&self) -> (u32, u32, u32, bool) {
        self.state().frames.repeat_config()
    }

    pub fn set_menu_key(&self, index: usize, pressed: bool) {
        // SAFETY: only the provider thread accesses this buffer, through its retained raw pointer.
        unsafe {
            std::ptr::addr_of_mut!((*self.state().immediate.get()).menu[index])
                .write(i32::from(pressed))
        };
    }

    pub fn clear_menu_pulse(&self, index: usize) {
        self.state().frames.pulsed.menu[index] = 0;
    }

    pub fn set_player_key(&self, player: usize, key: usize, pressed: bool) {
        let state = self.state();
        let template = state.player_controls[player] as usize;
        // SAFETY: the owner thread serializes these writes with the provider.
        unsafe {
            std::ptr::addr_of_mut!((*state.immediate.get()).key[template][key])
                .write(i32::from(pressed))
        };
    }

    pub fn player_controls(&self) -> [u32; 2] {
        self.state().player_controls
    }

    pub fn set_player_control(&self, player: usize, template: u32) {
        assert!((template as usize) < TEMPLATES, "invalid control template");
        self.state().player_controls[player] = template;
    }

    pub fn paused(&self) -> bool {
        self.state().paused
    }

    pub fn set_paused(&self, paused: bool) {
        self.state().paused = paused;
    }

    pub fn exit_requested(&self) -> bool {
        self.state().exit_requested
    }

    pub fn set_exit_requested(&self, requested: bool) {
        self.state().exit_requested = requested;
    }

    pub fn menu_sounds(&self) -> (u16, u16) {
        self.state().menu_sounds
    }

    pub fn set_menu_sounds(&self, first: u16, second: u16) {
        self.state().menu_sounds = (first, second);
    }

    pub fn reject_injection(&self) {
        self.state().injection_rejected = true;
    }

    pub fn injection_rejected(&self) -> bool {
        self.state().injection_rejected
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn battle_input_maps_the_players_template_keys_to_battle_flags() {
        let controls = InputControls::default();
        controls.set_player_control(1, 2);
        let mut state = InputState::default();
        state.key[2][KEY_UP as usize] = 1;
        state.key[2][KEY_WEAPON as usize] = 1;
        state.key[2][KEY_DOWN as usize] = 1;
        state.key[0][KEY_LEFT as usize] = 1;
        controls.advance(state, state, 0);
        assert_eq!(
            controls.current_battle_input(1),
            (1 << 2) | (1 << 3) | (1 << 6)
        );
        assert_eq!(controls.current_battle_input(0), 1 << 0);
        // The first frame with the keys down pulses all of them.
        assert_eq!(
            controls.pulsed_battle_input(1),
            (1 << 2) | (1 << 3) | (1 << 6)
        );
    }

    #[test]
    fn default_menu_repeat_is_a_half_second_delay_then_a_twelfth() {
        let controls = InputControls::default();
        controls.set_default_menu_repeat(0);
        assert_eq!(controls.repeat_config(), (70, 420, 70, false));
    }
}
