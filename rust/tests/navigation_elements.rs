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

#[test]
fn interplanetary_group_preserves_parent_and_updates_its_own_record() {
    battle_context::run(|| {
        let _owner = Battle::new(8);
        let frames = FrameSet::new(16);
        // SAFETY: this serialized fixture owns the group pool, solar descriptors and frames.
        unsafe {
            use uqm_rust::collections::queue::{AllocLink, InitQueue, PutQueue, UninitQueue};
            use uqm_rust::navigation::abi as nav;
            let mut solar: Box<nav::SOLARSYS_STATE> = Box::new(std::mem::zeroed());
            solar.SunDesc[0].radius = 128 << 6;
            solar.SunDesc[0].NumPlanets = 1;
            solar.PlanetDesc[0].image.origin = POINT { x: 80, y: 60 };
            solar.PlanetDesc[0].pPrevDesc = solar.SunDesc.as_mut_ptr();
            solar.MoonDesc[0].pPrevDesc = solar.PlanetDesc.as_mut_ptr();
            solar.pBaseDesc = solar.MoonDesc.as_mut_ptr();
            solar.pOrbitalDesc = solar.MoonDesc.as_mut_ptr();
            let old_solar = nav::pSolarSysState;
            let old_race = nav::EncounterRace;
            let old_stamp = GlobData.Game_state.ShipStamp;
            let old_location = GlobData.Game_state.ip_location;
            let old_groups = GlobData.Game_state.ip_group_q;
            let old_stars = nav::stars_in_space;
            let queue = &raw mut GlobData.Game_state.ip_group_q;
            queue.write(std::mem::zeroed());
            assert!(InitQueue(
                queue.cast(),
                2,
                size_of::<nav::IP_GROUP>() as u16
            ));
            let group = AllocLink(queue.cast()).cast::<nav::IP_GROUP>();
            assert!(!group.is_null());
            group.write(std::mem::zeroed());
            (*group).race_id = 0;
            (*group).group_id = 7;
            (*group).in_system = 1;
            (*group).dest_loc = 1;
            (*group).task = (nav::ON_STATION | nav::IGNORE_FLAGSHIP) as u8;
            (*group).loc = POINT { x: 300, y: 400 };
            (*group).melee_icon = frames.frame(0);
            PutQueue(queue.cast(), group.cast());
            nav::pSolarSysState = &mut *solar;
            nav::EncounterRace = -1;
            nav::stars_in_space = frames.frame(0);
            GlobData.Game_state.ShipStamp.frame = frames.frame(0);
            GlobData.Game_state.ip_location = POINT { x: 100, y: 200 };
            nav::DoMissions();
            uqm_rust::app::bridge::enter(|cx| {
                let flagship = cx.elements().head().expect("spawned flagship");
                assert_eq!(cx.element(flagship).turn, 1);
                let id = cx.elements().succ(flagship).expect("spawned group");
                assert_eq!(cx.parent(id), uqm_rust::engine::battle::Parent::IpGroup(0));
                let state = cx.element(id);
                assert_eq!(cx.frame_number(state.current.image.frame), 1);
                let hook = state.death.expect("group movement hook");
                assert!(matches!(hook, ElementHook::Native(_)));
                uqm_rust::battle::process_native::invoke_element::invoke_element(cx, hook, id);
                assert_eq!(cx.element(id).flags & CHANGING as u16, CHANGING as u16);
                assert_eq!(cx.element(id).flags & NONSOLID as u16, NONSOLID as u16);
                assert_eq!(cx.parent(id), uqm_rust::engine::battle::Parent::IpGroup(0));
            });
            assert_ne!(((*group).loc.x, (*group).loc.y), (300, 400));
            assert_eq!((*group).sys_loc, 0);
            assert_eq!((*group).group_id, 7);
            nav::NotifyOthers(0, 2);
            assert_eq!((*group).dest_loc, 2);
            assert_ne!((*group).task & nav::IGNORE_FLAGSHIP as u8, 0);
            UninitQueue(queue.cast());
            GlobData.Game_state.ip_group_q = old_groups;
            GlobData.Game_state.ShipStamp = old_stamp;
            GlobData.Game_state.ip_location = old_location;
            nav::stars_in_space = old_stars;
            nav::pSolarSysState = old_solar;
            nav::EncounterRace = old_race;
        }
    });
}
