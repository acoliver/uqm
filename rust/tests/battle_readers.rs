#![forbid(unsafe_code)]
#[path = "support/battle.rs"]
mod battle;
#[path = "support/battle_context.rs"]
mod battle_context;

use battle::{Battle, Object, Ship};
use uqm_engine::battle::ai_constants::SPECIAL;
use uqm_engine::graphics::geometry::Point;
use uqm_engine::species::species_id::{SPECIES_ID_ARILOU_ID, SPECIES_ID_SLYLANDRO_ID};

#[test]
fn legacy_slylandro_receives_the_fifth_junk_concern() {
    battle_context::run(|_| {
        let battle = Battle::new(16);
        {
            let mut ship = Ship::new(&battle, SPECIES_ID_SLYLANDRO_ID, 0);
            let mut junk = Object::new(&battle);
            let location = ship.element_mut().current.location;
            {
                let mut e = junk.element_mut();
                e.player_nr = -1;
                e.set_mass_points(1);
                e.set_life_span(1);
                e.current.location = Point {
                    x: location.x + 32,
                    y: location.y,
                };
                e.next = e.current.clone();
            }
            battle::put(junk.id);
            ship.descriptor_mut().ship_info.energy_level = 0;
            ship.ship_mut().special_counter = 0;
            crate::battle_context::with(|cx| {
                uqm_engine::battle::cyborg::tactical(cx, ship.ship_id)
            });
            assert_ne!(ship.ship_mut().ship_input_state & SPECIAL as u8, 0);
        }
    });
}

#[test]
fn typed_ai_uses_the_supplied_subject_projection() {
    battle_context::run(|_| {
        let battle = Battle::new(32);
        {
            let mut ship = Ship::new(&battle, SPECIES_ID_ARILOU_ID, 0);
            let mut target = Ship::new(&battle, SPECIES_ID_ARILOU_ID, 1);
            ship.element_mut().current.location = Point { x: 1000, y: 1000 };
            target.element_mut().current.location = Point { x: 3000, y: 1000 };
            crate::battle_context::with(|cx| {
                let live = cx.weapon_origin(ship.id);
                let other = cx.weapon_origin(target.id);
                let mut projected = live.clone();
                projected.current.location.x = 2900;
                projected.next.location = projected.current.location;
                assert_eq!(cx.plot_intercept(&live, &other, 1, 100), 0);
                assert_eq!(cx.plot_intercept(&projected, &other, 1, 100), 1);
                let before = battle::queued_in(cx);
                assert_eq!(cx.ship_weapons(&live, &other, 100), 0);
                assert_ne!(cx.ship_weapons(&projected, &other, 100), 0);
                assert_eq!(
                    battle::queued_in(cx),
                    before,
                    "prediction disposes temporary weapons"
                );
                assert_eq!(cx.element(ship.id).current.location.x, 1000);
                assert_eq!(projected.current.location.x, 2900);
            });
        }
    });
}
