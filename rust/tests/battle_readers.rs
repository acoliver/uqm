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
