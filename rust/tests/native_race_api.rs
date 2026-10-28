#[path = "support/battle.rs"]
mod battle;
#[path = "support/battle_context.rs"]
mod battle_context;

use battle::{Battle, Ship};

use uqm_rust::collections::observation::{self, Event};
use uqm_rust::engine::battle::{Cx, Selection, ShipId, WeaponOrigin, WeaponSpawn};
use uqm_rust::ships::native::abi::*;

fn alternate_pre(cx: &mut Cx<'_>, ship: ShipId) {
    cx.ship_state_mut(ship).energy_counter = 71;
}
fn alternate_post(cx: &mut Cx<'_>, ship: ShipId) {
    cx.ship_state_mut(ship).energy_counter = 39;
}
fn alternate_weapon(cx: &mut Cx<'_>, ship: ShipId, subject: WeaponOrigin) -> WeaponSpawn {
    cx.ship_state_mut(ship).weapon_counter = subject.turn_wait();
    vec![cx.alloc_element()]
}

#[test]
fn callback_modes_control_dispatch_metadata_and_observations() {
    battle_context::run(|| {
        let owner = Battle::new(8);
        let ship = Ship::bare(&owner, SPECIES_ID_VUX_ID, 0);
        // SAFETY: owner keeps the serialized test battle published until all fixtures drop.
        unsafe {
            uqm_rust::app::bridge::enter(|cx| {
                observation::enable();
                cx.set_preprocess(ship.ship_id, Selection::Disabled);
                observation::drain();
                assert!(!cx.invoke_ship_process(ship.ship_id, false));
                assert!(!observation::drain()
                    .iter()
                    .any(|e| matches!(e, Event::Hook { .. })));
                cx.set_preprocess(ship.ship_id, Selection::Override(alternate_pre));
                cx.set_postprocess(ship.ship_id, Selection::Override(alternate_post));
                assert!(cx.invoke_ship_process(ship.ship_id, false));
                assert_eq!(cx.ship_state(ship.ship_id).energy_counter, 71);
                assert!(cx.invoke_ship_process(ship.ship_id, true));
                assert_eq!(cx.ship_state(ship.ship_id).energy_counter, 39);
                cx.set_weapon(ship.ship_id, Selection::Override(alternate_weapon));
                let mut projected = cx.weapon_origin(ship.id);
                projected.set_turn_wait(23);
                let outputs = cx.spawn_weapons(ship.ship_id, projected);
                assert_eq!(cx.ship_state(ship.ship_id).weapon_counter, 23);
                cx.dispose(outputs[0].unwrap());
                cx.set_weapon(ship.ship_id, Selection::Disabled);
                assert!(cx.spawn_weapons(ship.ship_id, projected).is_empty());
                cx.set_preprocess(ship.ship_id, Selection::Default);
            });
            assert_ne!(ship.snapshot().flags & PLAYER_SHIP as u16, 0);
        }
    });
}

#[test]
fn ordered_lifecycle_preserves_reverse_order_exhaustion_and_primitive_reuse() {
    battle_context::run(|| {
        let _owner = Battle::new(3);
        // SAFETY: this test owns all element slots and the game-thread context.
        unsafe {
            uqm_rust::app::bridge::enter(|cx| {
                let a = cx.alloc_element().unwrap();
                let b = cx.alloc_element().unwrap();
                let c = cx.alloc_element().unwrap();
                assert!(cx.alloc_element().is_none());
                cx.put(a);
                cx.insert_before(b, Some(a));
                cx.insert_before(c, Some(a));
                assert_eq!(cx.elements().head(), Some(b));
                assert_eq!(cx.elements().succ(b), Some(c));
                assert_eq!(cx.elements().succ(c), Some(a));
                assert_eq!(cx.elements().tail(), Some(a));
                assert_eq!(cx.elements().pred(a), Some(c));
                assert_eq!(cx.elements().pred(c), Some(b));
                for id in [b, c, a] {
                    cx.unlink(id);
                    cx.dispose(id);
                    assert!(!cx.elements().contains(id));
                }
                assert!(cx.elements().head().is_none());
                for _ in 0..400 {
                    let id = cx.alloc_element().unwrap();
                    assert_ne!(id, a);
                    cx.dispose(id);
                }
            });
        }
    });
}

#[test]
fn projected_autoaim_uses_both_positions_without_mutating_live_target() {
    battle_context::run(|| {
        let owner = Battle::new(8);
        // SAFETY: fixtures retain the frame and descriptor resources for these hook calls.
        unsafe {
            let ship = Ship::new(&owner, SPECIES_ID_ARILOU_ID, 0);
            let enemy = Ship::new(&owner, SPECIES_ID_VUX_ID, 1);
            uqm_rust::app::bridge::enter(|cx| {
                cx.element_mut(enemy.id).current.location = POINT { x: 4000, y: 4300 };
                cx.element_mut(enemy.id).next.location = POINT { x: 4000, y: 4300 };
                cx.set_target(ship.id, Some(enemy.id));
                let live = cx.element(ship.id);
                let mut subject = cx.weapon_origin(ship.id);
                subject.current.location = POINT { x: 3800, y: 4300 };
                subject.next.location = POINT { x: 3800, y: 4300 };
                let mut facing = 0;
                assert_eq!(cx.track_subject(&mut subject, &mut facing), 4);
                assert_eq!(facing, 1);
                let weapons = cx.spawn_weapons(ship.ship_id, subject);
                let laser = cx.element(weapons[0].unwrap());
                let (dx, dy) = laser.velocity.get_current_components();
                assert!(dx > 0);
                assert_eq!(dy, 0);
                assert_eq!(laser.current.location.y, 4300);
                assert_eq!(cx.target(ship.id), Some(enemy.id));
                assert_eq!(
                    cx.element(ship.id).current.location.x,
                    live.current.location.x
                );
                assert_eq!(
                    cx.element(ship.id).current.location.y,
                    live.current.location.y
                );
                assert_eq!(cx.element(ship.id).next.location.x, live.next.location.x);
                assert_eq!(cx.element(ship.id).next.location.y, live.next.location.y);
                cx.dispose(weapons[0].unwrap());
            });
        }
    });
}
