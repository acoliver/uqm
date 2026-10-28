//! Observation precedes invocation without retaining a borrow across a game hook.

use super::*;

pub(crate) fn invoke<R>(callback: usize, parent: usize, call: impl FnOnce() -> R) -> R {
    crate::collections::observation::observe(callback, parent);
    call()
}

/// # Safety
/// `call` may use only live legacy storage and must retain no record reference.
pub(crate) unsafe fn invoke_legacy<R>(
    cx: &mut crate::engine::battle::Cx<'_>,
    callback: usize,
    parent: usize,
    call: impl FnOnce() -> R,
) -> R {
    // SAFETY: publishing suspends context access until the legacy invocation returns.
    unsafe { invoke(callback, parent, || crate::app::bridge::publish(cx, call)) }
}

pub(crate) fn record_spawn(address: usize, side: i16, species: u32) {
    crate::collections::observation::record(crate::collections::observation::Event::Spawn {
        address,
        side,
        species,
    });
}

pub(crate) fn battle_input(state: u8) -> BATTLE_INPUT_STATE {
    let mut input = 0;
    if state & LEFT as u8 != 0 {
        input |= BATTLE_LEFT;
    } else if state & RIGHT as u8 != 0 {
        input |= BATTLE_RIGHT;
    }
    if state & THRUST as u8 != 0 {
        input |= BATTLE_THRUST;
    }
    if state & WEAPON as u8 != 0 {
        input |= BATTLE_WEAPON;
    }
    if state & SPECIAL as u8 != 0 {
        input |= BATTLE_SPECIAL;
    }
    input
}

pub(crate) fn collision_order(test_is_player: bool) -> [usize; 2] {
    if test_is_player {
        [1, 0]
    } else {
        [0, 1]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collections::observation::{self, Event};

    #[test]
    #[serial_test::serial]
    fn observes_before_call_and_preserves_result() {
        observation::enable();
        observation::drain();
        let result = invoke(123, 456, || {
            let events = observation::drain();
            assert_eq!(
                events
                    .iter()
                    .filter(|event| matches!(
                        event,
                        Event::Hook {
                            callback: 123,
                            parent: 456
                        }
                    ))
                    .count(),
                1
            );
            789
        });
        assert_eq!(result, 789);
        assert!(!observation::drain().iter().any(|event| matches!(
            event,
            Event::Hook {
                callback: 123,
                parent: 456
            }
        )));
    }

    #[test]
    fn intelligence_input_preserves_left_priority_and_independent_action_bits() {
        assert_eq!(battle_input((LEFT | RIGHT) as u8), BATTLE_LEFT);
        assert_eq!(battle_input(RIGHT as u8), BATTLE_RIGHT);
        assert_eq!(
            battle_input((LEFT | RIGHT | THRUST | WEAPON | SPECIAL) as u8),
            BATTLE_LEFT | BATTLE_THRUST | BATTLE_WEAPON | BATTLE_SPECIAL
        );
        assert_eq!(battle_input(0), 0);
    }

    #[test]
    #[serial_test::serial]
    fn collision_order_reloads_hook_and_parent_after_first_callback() {
        use std::cell::Cell;
        observation::enable();
        for test_is_player in [false, true] {
            observation::drain();
            let callbacks = [Cell::new(1000), Cell::new(2000)];
            let parents = [Cell::new(100), Cell::new(200)];
            let order = collision_order(test_is_player);
            assert_eq!(order, if test_is_player { [1, 0] } else { [0, 1] });
            let mut called = Vec::new();
            for index in order {
                let hook = callbacks[index].get();
                invoke(hook, parents[index].get(), || {
                    called.push(hook);
                    callbacks[1 - index].set(3000);
                    parents[1 - index].set(300);
                });
            }
            assert_eq!(called, [if test_is_player { 2000 } else { 1000 }, 3000]);
            let hooks: Vec<_> = observation::drain()
                .into_iter()
                .filter_map(|event| match event {
                    Event::Hook { parent, callback } => Some((parent, callback)),
                    _ => None,
                })
                .collect();
            assert_eq!(
                hooks,
                [
                    (if test_is_player { 200 } else { 100 }, called[0]),
                    (300, 3000)
                ]
            );
        }
    }

    #[test]
    #[serial_test::serial]
    fn spawn_record_preserves_owner_metadata() {
        observation::enable();
        observation::drain();
        record_spawn(42, 1, 17);
        let events = observation::drain();
        assert_eq!(
            events
                .iter()
                .filter(|event| matches!(
                    event,
                    Event::Spawn {
                        address: 42,
                        side: 1,
                        species: 17
                    }
                ))
                .count(),
            1
        );
    }
}
