use crate::comm::host::Host;
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
        let mut script = Host::<Handler>::new(&mut comm, &mut cx, |_| {
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

#[test]
#[serial_test::serial]
fn unblocked_ui_adapters_initialize_with_supplied_campaign_owners(
) -> Result<(), Box<dyn std::error::Error>> {
    if !crate::native_test_process::own_graphics_process(concat!(
        module_path!(),
        "::unblocked_ui_adapters_initialize_with_supplied_campaign_owners"
    )) {
        return Ok(());
    }
    use crate::comm::lifecycle::{self, DialogueKind::*};
    let adapters = [
        Arilou, Blackur, Chmmr, Comandr, Ilwrath, Mycon, Orz, Pkunk, Rebel, Slyhome, Spahome,
        Spathi, Starbas, Supox, Syreen, Talkpet, Thradd, Umgah, Urquan, Utwig, Vux, Zoqfot, Druuge,
        Melnorm, Slyland,
    ];
    let mut game = Game::new();
    let display = crate::campaign_display_test::CampaignDisplay::install(&mut game)?;
    for kind in adapters {
        let name = format!("{kind:?}");
        game.world = crate::engine::World::new();
        game.world.story = Some(crate::engine::world::story::StoryState::new());
        crate::time::clock_bridge::rust_clock_init(&mut game.world);
        let mut comm = Comm::new();
        let mut battle = Battle::new();
        let mut cx = Cx::new(&mut battle, &mut game.world, &mut game.kernel, || {
            panic!("adapter initialization must not consume RNG")
        });
        let descriptor = lifecycle::session(&mut comm, &mut cx, kind);
        assert!(descriptor.definition.alien_frame_res.is_some(), "{name}");
        assert!(descriptor.definition.alien_font_res.is_some(), "{name}");
        assert!(
            descriptor.definition.conversation_phrases_res.is_some(),
            "{name}"
        );
        assert!(descriptor.definition.alien_text_width > 0, "{name}");
        assert!(descriptor.init_encounter_func.is_some(), "{name}");
        assert!(descriptor.post_encounter_func.is_some(), "{name}");
        assert_eq!(
            descriptor.uninit_encounter_func.unwrap()(&mut comm, &mut cx),
            0,
            "{name}"
        );
        crate::time::clock_bridge::rust_clock_uninit(cx.world);
    }
    display.finish(game);
    Ok(())
}
