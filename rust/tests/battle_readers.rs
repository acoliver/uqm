#[path = "support/battle.rs"]
mod battle;
#[path = "support/battle_context.rs"]
mod battle_context;

use battle::{Battle, Object, Ship};
use uqm_rust::ships::native::{abi::*, put_element};

#[test]
fn legacy_slylandro_receives_the_fifth_junk_concern() {
    battle_context::run(|| {
        let battle = Battle::new(16);
        // SAFETY: the fixture owns both pools, ship descriptor, frames and queued object.
        unsafe {
            let mut ship = Ship::new(&battle, SPECIES_ID_SLYLANDRO_ID, 0);
            let mut junk = Object::new(&battle);
            let location = ship.element_mut().current.location;
            {
                let e = junk.element_mut();
                e.playerNr = -1;
                e.set_mass_points(1);
                e.set_life_span(1);
                e.current.location = POINT {
                    x: location.x + 32,
                    y: location.y,
                };
                e.next = e.current;
            }
            put_element(junk.ptr().cast());
            ship.descriptor_mut().ship_info.energy_level = 0;
            ship.ship_mut().special_counter = 0;
            let owner = &raw mut *ship.ship_mut();
            tactical_intelligence(std::ptr::null_mut(), owner);
            assert_ne!(ship.ship_mut().ship_input_state & SPECIAL as u8, 0);
        }
    });
}

#[test]
fn typed_ai_uses_the_supplied_subject_projection() {
    battle_context::run(|| {
        let battle = Battle::new(32);
        // SAFETY: the shared fixture retains both pool-backed ships and their frames.
        unsafe {
            let mut ship = Ship::new(&battle, SPECIES_ID_ARILOU_ID, 0);
            let mut target = Ship::new(&battle, SPECIES_ID_ARILOU_ID, 1);
            ship.element_mut().current.location = POINT { x: 1000, y: 1000 };
            target.element_mut().current.location = POINT { x: 3000, y: 1000 };
            uqm_rust::app::bridge::enter(|cx| {
                let live = cx.weapon_origin(ship.id);
                let other = cx.weapon_origin(target.id);
                let mut projected = live;
                projected.current.location.x = 2900;
                projected.next.location = projected.current.location;
                assert_eq!(cx.plot_intercept(&live, &other, 1, 100), 0);
                assert_eq!(cx.plot_intercept(&projected, &other, 1, 100), 1);
                let before = battle::queued();
                assert_eq!(cx.ship_weapons(&live, &other, 100), 0);
                assert_ne!(cx.ship_weapons(&projected, &other, 100), 0);
                assert_eq!(
                    battle::queued(),
                    before,
                    "prediction disposes temporary weapons"
                );
                assert_eq!(cx.element(ship.id).current.location.x, 1000);
                assert_eq!(projected.current.location.x, 2900);
            });
        }
    });
}
