use crate::comm::host::Host;
use crate::comm::races::{alliance::AllianceApi, encounter::EncounterApi};
use crate::comm::Comm;
use crate::engine::{
    battle::{Battle, Cx},
    Game, StoryKey,
};
use crate::navigation::abi::{ARILOU_UMGAH_CHECK, YEHAT_REBEL_SHIP, YEHAT_SHIP};

#[test]
#[serial_test::serial]
fn clock_adapters_schedule_and_dispatch_on_the_supplied_campaign() {
    if !crate::native_test_process::own_graphics_process(concat!(
        module_path!(),
        "::clock_adapters_schedule_and_dispatch_on_the_supplied_campaign"
    )) {
        return;
    }
    let mut game = Game::new();
    let display = crate::campaign_display_test::CampaignDisplay::install(&mut game).unwrap();
    game.world.story = Some(crate::engine::world::story::StoryState::new());
    crate::time::clock_bridge::rust_clock_init(&mut game.world);
    for _ in 0..=YEHAT_REBEL_SHIP {
        let mut fleets = game.world.avail_races();
        let id = fleets.alloc().unwrap();
        fleets.put(id);
    }
    let royal = game
        .world
        .avail_races_ref()
        .at(YEHAT_SHIP as usize)
        .unwrap();
    game.world.avail_races().get_mut(royal).actual_strength = 100;
    let mut comm = Comm::new();
    let mut battle = Battle::new();
    {
        let mut cx = Cx::new(&mut battle, &mut game.world, &mut game.kernel, || {
            panic!("adapter RNG")
        });
        // SAFETY: this fixture owns initialized display resources and exclusive campaign loans.
        let shofixt = super::shofixt::init_shofixti_comm(&mut comm, &mut cx);
        assert_eq!(cx.world.story_state(StoryKey::BATTLE_SEGUE), 0);
        assert!(std::ptr::fn_addr_eq(
            shofixt.init_encounter_func.unwrap(),
            super::shofixt::intro as fn(&mut Comm, &mut Cx<'_>)
        ));
        assert!(shofixt.post_encounter_func.is_some());
        assert!(shofixt.uninit_encounter_func.is_some());
        // SAFETY: the same retained display and exclusive loans remain live.
        let yehat = super::yehat::init_yehat_comm(&mut comm, &mut cx);
        assert_eq!(cx.world.story_state(StoryKey::BATTLE_SEGUE), 1);
        assert!(std::ptr::fn_addr_eq(
            yehat.init_encounter_func.unwrap(),
            super::yehat::intro as fn(&mut Comm, &mut Cx<'_>)
        ));
        assert!(yehat.post_encounter_func.is_some());
        assert!(yehat.uninit_encounter_func.is_some());
        {
            let mut script = Host::new(&mut comm, &mut cx, |_: ()| panic!("response callback"));
            script.event(1, 0, 0, 0, ARILOU_UMGAH_CHECK as u8);
            script.advance_ilwrath();
        }
        assert_eq!(cx.world.story_state(StoryKey::ARILOU_CHECKED_UMGAH), 2);
        assert_eq!(cx.world.clock.events_ref().count(), 0);
        {
            use crate::comm::races::shofixt::{api::ShofixtApi, Handler};
            let mut script =
                Host::new(&mut comm, &mut cx, |_: Handler| panic!("response callback"));
            script.schedule_return();
        }
        let events = cx.world.clock.events_ref();
        let event = events.get(events.head().unwrap());
        assert_eq!(
            (
                event.year_index,
                event.month_index,
                event.day_index,
                event.func_index
            ),
            (2155, 4, 17, 8)
        );
        {
            use crate::comm::races::yehat::{api::YehatApi, Handler};
            let mut script =
                Host::new(&mut comm, &mut cx, |_: Handler| panic!("response callback"));
            script.schedule_rebel_event();
        }
        let fleets = cx.world.avail_races_ref();
        let rebel = fleets.get(fleets.at(YEHAT_REBEL_SHIP as usize).unwrap());
        assert_eq!(fleets.get(royal).actual_strength, 66);
        assert_eq!(
            (rebel.actual_strength, rebel.loc.x, rebel.loc.y),
            (66, 5150, 0)
        );
        assert_eq!(cx.world.clock.events_ref().count(), 1);
    }
    drop(comm);
    display.finish(game);
}
