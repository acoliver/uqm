#[path = "support/battle.rs"]
mod battle;
#[path = "support/battle_context.rs"]
mod battle_context;

use battle::{Battle, FrameSet, Object};
use uqm_rust::engine::battle::{ElementHook, ElementId};
use uqm_rust::ships::native::abi::*;

fn transition_hook() -> ElementHook {
    ElementHook::Native(uqm_rust::navigation::hyper_encounters::transition)
}

fn call_transition(id: ElementId) {
    // SAFETY: the test owns the serialized element pool and retains no record borrow.
    unsafe {
        uqm_rust::app::bridge::enter(|cx| {
            uqm_rust::battle::process_native::invoke_element::invoke_element(
                cx,
                transition_hook(),
                id,
            );
        });
    }
}

#[test]
fn encounter_transition_preserves_frame_cadence_and_completion() {
    battle_context::run(|| {
        let owner = Battle::new(8);
        let frames = FrameSet::new(12);
        let mut frame_set = [frames.frame(0)];
        let mut element = Object::new(&owner);
        // SAFETY: this test owns the allocated element and its frame set.
        unsafe {
            let record = element.element_mut();
            record.state_flags = DISAPPEARING as u16;
            record.current.image.farray = frame_set.as_mut_ptr();
            record.current.image.frame = frames.frame(10);
            record.next = record.current;
            record.set_turn_wait(1);
            record.death_func = Some(transition_hook());
        }
        call_transition(element.id);
        // SAFETY: the callback has returned; the fixture still owns this element.
        unsafe {
            let record = element.element_mut();
            assert_eq!(record.state_flags & DISAPPEARING as u16, 0);
            assert_eq!(record.life_span(), 1);
            assert_eq!(record.turn_wait(), 0);
            assert_eq!(record.next.image.frame, frames.frame(10));
        }
        call_transition(element.id);
        // SAFETY: the callback has returned and no borrow crosses the next call.
        unsafe {
            let record = element.element_mut();
            assert_eq!(record.turn_wait(), 1);
            assert_eq!(record.next.image.frame, frames.frame(11));
            record.current = record.next;
            record.set_turn_wait(0);
        }
        call_transition(element.id);
        // SAFETY: the callback has returned; the fixture retains the frame set.
        unsafe {
            let record = element.element_mut();
            assert!(record.death_func.is_none());
            record.set_hit_points(1);
            record.set_turn_wait(0);
            record.current.image.frame = frames.frame(0);
        }
        call_transition(element.id);
        // SAFETY: the fixture retains its live element and frame set.
        unsafe {
            assert_eq!(element.element_mut().next.image.frame, frames.frame(11));
        }
    });
}

#[test]
fn interplanetary_flagship_keeps_motion_and_native_hooks() {
    battle_context::run(|| {
        let _owner = Battle::new(8);
        let frames = FrameSet::new(16);
        // SAFETY: the serialized fixture owns and restores navigation's live projections.
        unsafe {
            use uqm_rust::navigation::abi as nav;
            let mut solar: Box<nav::SOLARSYS_STATE> = Box::new(std::mem::zeroed());
            solar.SunDesc[0].radius = 128 << 6;
            solar.pBaseDesc = solar.PlanetDesc.as_mut_ptr();
            let old_solar = nav::pSolarSysState;
            nav::pSolarSysState = &mut *solar;
            let old_race = nav::EncounterRace;
            nav::EncounterRace = -1;
            let old_stamp = GlobData.Game_state.ShipStamp;
            let old_location = GlobData.Game_state.ip_location;
            let old_velocity = GlobData.Game_state.velocity;
            GlobData.Game_state.ShipStamp.frame = frames.frame(0);
            GlobData.Game_state.ShipStamp.origin = POINT { x: 80, y: 60 };
            GlobData.Game_state.ip_location = POINT { x: 100, y: 200 };
            let mut velocity = old_velocity;
            velocity.set_components(20, -30);
            GlobData.Game_state.velocity = velocity;
            nav::DoMissions();
            uqm_rust::app::bridge::enter(|cx| {
                let flagship = cx.elements().head().expect("spawned flagship");
                let state = cx.element(flagship);
                let Some(ElementHook::Native(pre)) = state.pre else {
                    panic!("flagship preprocess must be Native");
                };
                assert!(matches!(
                    state.collide,
                    Some(uqm_rust::engine::battle::CollisionHook::Native(_))
                ));
                uqm_rust::battle::process_native::invoke_element::invoke_element(
                    cx,
                    ElementHook::Native(pre),
                    flagship,
                );
                assert_eq!(cx.element(flagship).thrust, 41);
            });
            let location = GlobData.Game_state.ip_location;
            assert_eq!(location.x, 120);
            assert_eq!(location.y, 170);
            GlobData.Game_state.ShipStamp = old_stamp;
            GlobData.Game_state.ip_location = old_location;
            GlobData.Game_state.velocity = old_velocity;
            nav::pSolarSysState = old_solar;
            nav::EncounterRace = old_race;
        }
    });
}
