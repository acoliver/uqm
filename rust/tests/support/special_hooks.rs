#![allow(dead_code)]
use uqm_rust::collections::observation::Event;
use uqm_rust::ships::hook_registry;

/// Counts the `special` activations of `race` among observed events. The
/// callback is resolved through the registry name the parity trace records.
pub fn count(events: &[Event], race: &str) -> usize {
    let name = format!("ships::races::{race}_native::special");
    let address = hook_registry::hooks()
        .into_iter()
        .find(|(_, registered)| *registered == name)
        .unwrap_or_else(|| panic!("{name} is not in the hook registry"))
        .0;
    events
        .iter()
        .filter(|event| matches!(event, Event::Hook { callback, .. } if *callback == address))
        .count()
}
