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
    type Init = unsafe fn(&mut Comm, &mut Cx<'_>) -> crate::comm::locdata::CLocData;
    type Hook = fn(&mut Comm, &mut Cx<'_>);
    type Uninit = fn(&mut Comm, &mut Cx<'_>) -> u16;
    macro_rules! adapter {
        ($module:ident, $init:ident) => {
            (
                stringify!($module),
                super::$module::$init,
                super::$module::intro,
                super::$module::post,
                super::$module::uninit,
            )
        };
    }
    // Dialogue callbacks need loaded conversation resources; their supplied-owner
    // signatures are checked here while the real initializers and cleanup run.
    let adapters: [(&str, Init, Hook, Hook, Uninit); 22] = [
        adapter!(arilou, init_arilou_comm),
        adapter!(blackur, init_blackurq_comm),
        adapter!(chmmr, init_chmmr_comm),
        adapter!(comandr, init_commander_comm),
        adapter!(ilwrath, init_ilwrath_comm),
        adapter!(mycon, init_mycon_comm),
        adapter!(orz, init_orz_comm),
        adapter!(pkunk, init_pkunk_comm),
        adapter!(rebel, init_rebel_yehat_comm),
        adapter!(slyhome, init_slylandro_comm),
        adapter!(spahome, init_spahome_comm),
        adapter!(spathi, init_spathi_comm),
        adapter!(starbas, init_starbase_comm),
        adapter!(supox, init_supox_comm),
        adapter!(syreen, init_syreen_comm),
        adapter!(talkpet, init_talkpet_comm),
        adapter!(thradd, init_thradd_comm),
        adapter!(umgah, init_umgah_comm),
        adapter!(urquan, init_urquan_comm),
        adapter!(utwig, init_utwig_comm),
        adapter!(vux, init_vux_comm),
        adapter!(zoqfot, init_zoqfot_comm),
    ];
    let mut game = Game::new();
    let display = crate::campaign_display_test::CampaignDisplay::install(&mut game)?;
    for (name, initialize, _intro, _post, uninit) in adapters {
        game.world = crate::engine::World::new();
        game.world.story = Some(crate::engine::world::story::StoryState::new());
        crate::time::clock_bridge::rust_clock_init(&mut game.world);
        let mut comm = Comm::new();
        let mut battle = Battle::new();
        let mut cx = Cx::new(&mut battle, &mut game.world, &mut game.kernel, || {
            panic!("adapter initialization must not consume RNG")
        });
        // SAFETY: the serialized process owns initialized display dimensions and
        // each initializer receives exclusive loans of its live campaign/session.
        let descriptor = unsafe { initialize(&mut comm, &mut cx) };
        assert!(!descriptor.alien_frame_res.is_null(), "{name}");
        assert!(!descriptor.alien_font_res.is_null(), "{name}");
        assert!(!descriptor.conversation_phrases_res.is_null(), "{name}");
        assert!(descriptor.alien_text_width > 0, "{name}");
        assert_eq!(uninit(&mut comm, &mut cx), 0, "{name}");
        crate::time::clock_bridge::rust_clock_uninit(cx.world);
    }
    display.finish(game);
    Ok(())
}
