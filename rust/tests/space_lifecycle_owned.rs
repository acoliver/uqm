#[path = "support/campaign_display.rs"]
mod campaign_display;

use uqm_rust::engine::{
    battle::{Battle, Cx},
    Game,
};
use uqm_rust::game_init::space_resources::{init_battle_ships, uninit_ships, uninit_space};
use uqm_rust::ships::native::abi::{IN_BATTLE, SUPER_MELEE};

#[test]
fn battle_entry_has_no_application_lifecycle_dependency() {
    let source = include_str!("../src/battle/battle_native.rs");
    assert!(
        !source.contains("crate::game_init"),
        "battle must receive lifecycle operations from application composition"
    );
}

#[test]
fn battle_ship_lifecycle_uses_the_unpublished_owner_even_when_resources_fail() {
    let mut first = Game::new();
    let mut second = Game::new();
    let display = campaign_display::CampaignDisplay::install(&mut first).unwrap();
    first
        .kernel
        .install_ship_lifecycle(uqm_rust::engine::battle::services::ShipLifecycle {
            init: init_battle_ships,
            uninit: uninit_ships,
        });
    let mut first_battle = Battle::new();
    let mut second_battle = Battle::new();
    first.world.set_activity(SUPER_MELEE as u16);
    second
        .world
        .set_activity(SUPER_MELEE as u16 | IN_BATTLE as u16);
    second.kernel.ships_initialized = true;
    {
        let mut first = Cx::new(
            &mut first_battle,
            &mut first.world,
            &mut first.kernel,
            uqm_rust::math::random,
        );
        let second = Cx::new(
            &mut second_battle,
            &mut second.world,
            &mut second.kernel,
            uqm_rust::math::random,
        );
        let element = first.alloc_element().unwrap();
        first.put(element);

        let ships = first.kernel.ship_lifecycle();
        assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            second.kernel.ship_lifecycle()
        }))
        .is_err());

        // The mounted fixture has no graphics.stars entry, exercising real load failure.
        assert_eq!((ships.init)(&mut first), 0);
        assert_eq!(first.kernel.space_init_count, 1);
        assert!(!first.kernel.ships_initialized);
        assert_eq!(first.elements().head(), Some(element));
        assert_eq!(second.kernel.space_init_count, 0);
        assert!(second.kernel.ships_initialized);
        uninit_space(first.kernel);

        (ships.uninit)(&mut first);
        assert_eq!(first.elements().head(), Some(element));
        first
            .world
            .set_activity(SUPER_MELEE as u16 | IN_BATTLE as u16);
        first.kernel.ships_initialized = true;
        (ships.uninit)(&mut first);
        assert!(!first.kernel.ships_initialized);
        assert_eq!(first.world.activity(), SUPER_MELEE as u16);
        assert_eq!(
            second.world.activity(),
            SUPER_MELEE as u16 | IN_BATTLE as u16
        );
        assert!(second.kernel.ships_initialized);
        (ships.uninit)(&mut first);
        assert!(!first.kernel.ships_initialized);
    }
    display.finish(first);
}
