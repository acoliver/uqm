use super::script::LiveScript;
use crate::comm::races::starbas::{api::StarbaseApi, Handler};
use crate::comm::Comm;
use crate::engine::{battle::Battle, battle::Cx, Game};

#[test]
#[serial_test::serial]
fn starbase_adapter_uses_the_supplied_inventory_and_calendar() {
    let mut first = Game::new();
    let mut second = Game::new();
    first.world.data.flagship.crew_enlisted = 12;
    first.world.data.flagship.res_units = 34;
    first.world.data.flagship.element_amounts[2] = 12;
    first.world.data.flagship.total_element_mass = 12;
    first.world.data.game.game_clock.year_index = 2156;
    first.world.data.game.game_clock.month_index = 4;
    first.world.data.game.game_clock.day_index = 17;
    second.world.data.flagship.crew_enlisted = 99;
    second.world.data.flagship.element_amounts[2] = 25;
    second.world.data.flagship.total_element_mass = 25;
    let mut comm = Comm::new();
    let mut battle = Battle::new();
    {
        let mut cx = Cx::new(&mut battle, &mut first.world, &mut first.kernel, || {
            panic!("inventory query must not consume RNG")
        });
        let mut script = LiveScript::<Handler>::new(&mut comm, &mut cx, |_| {
            panic!("inventory query must not register a response")
        });
        let ship = script.ship();
        assert_eq!((ship.crew, ship.units), (12, 34));
        let date = script.date();
        assert_eq!((date.year, date.month, date.day), (2156, 4, 17));
        assert_eq!(script.mineral_amount(2), 12);
        script.remove_minerals(2, 5);
        assert_eq!(script.mineral_amount(2), 7);
        assert_eq!(script.ship().mineral_mass, 7);
    }
    assert_eq!(first.world.data.flagship.element_amounts[2], 7);
    assert_eq!(second.world.data.flagship.crew_enlisted, 99);
    assert_eq!(second.world.data.flagship.element_amounts[2], 25);
    assert_eq!(second.world.data.flagship.total_element_mass, 25);
}
