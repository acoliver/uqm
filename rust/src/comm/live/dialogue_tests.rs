//! Every race conversation runs through its live host: the intro, a walk
//! through the offered responses, the post-encounter step and teardown.
//! The walks use a phrase table where every phrase has text, so each
//! response the scripts offer can be built and chosen.

use crate::comm::lifecycle::{self, DialogueKind};
use crate::comm::Comm;
use crate::engine::{
    battle::{input::InputContext, Battle, Cx},
    input::{Frame, InputRuntime, InputSession},
    observe::{CampaignView, InputView, Io, Observer, Unobserved},
    species::ship::YEHAT_REBEL_SHIP,
    world::story::StoryState,
    Game, StoryKey, World,
};
use crate::strings::{conversation::ConversationTable, owned_table::LoadedTextTable};

/// Runs every input frame to completion; time and presentation are free.
struct Pacing(InputSession);

impl InputRuntime for Pacing {
    fn input(&self) -> &InputSession {
        &self.0
    }
    fn observer(&self) -> Box<dyn Observer<Option<CampaignView>>> {
        Box::new(Unobserved)
    }
    fn yield_poll(&self, _: &mut Cx<'_>) {}
    fn yield_task(&self, _: &mut Cx<'_>) {}
    fn sleep(&self, _: &mut Cx<'_>, _: u32) {}
    fn sleep_until(&self, _: &mut Cx<'_>, _: u32) {}
    fn flush(&self) {}
    fn confirm_exit(&self, _: &mut Cx<'_>) -> bool {
        false
    }
    fn update(&self, _: &mut Cx<'_>) {}
    fn run(&self, cx: &mut Cx<'_>, io: &mut Io<'_>, _reset: bool, frame: &mut dyn Frame) {
        for _ in 0..64 {
            if frame.admit(cx, io, InputView::BeforeUpdate).stop || !frame.frame(cx, io) {
                return;
            }
        }
        panic!("conversation input loop did not finish");
    }
    fn battle_input(&self, _: &InputContext) -> u8 {
        0
    }
}

const KINDS: [DialogueKind; DialogueKind::COUNT] = {
    use DialogueKind::*;
    [
        Blackur, Chmmr, Yehat, Umgah, Mycon, Talkpet, Utwig, Druuge, Syreen, Orz, Spahome, Ilwrath,
        Starbas, Slyhome, Arilou, Supox, Shofixt, Pkunk, Rebel, Slyland, Vux, Melnorm, Zoqfot,
        Comandr, Urquan, Thradd, Spathi,
    ]
};

/// Story fields chosen by `pick(field, width_mask)`. Counters the scripts
/// advance from zero and cap (visits, counts, mineral loads) are held to
/// the range the scripts leave them in, and the starbase visit date is a
/// calendar date.
fn story_with(mut pick: impl FnMut(u32) -> u32) -> Vec<(StoryKey, u32)> {
    let mut chosen: Vec<_> = crate::engine::world::story::GAME_STATE_FIELDS
        .iter()
        .map(|&(name, start, end)| {
            let width = end - start + 1;
            let counter = name.ends_with("_VISITS") || name.ends_with("_COUNT");
            let mask = if counter {
                1
            } else if name.ends_with("_MINERAL_LOAD") {
                3
            } else if width >= 32 {
                u32::MAX
            } else {
                (1u32 << width) - 1
            };
            let key = StoryKey::from_name(name).unwrap_or_else(|| panic!("{name}"));
            (counter, key, pick(mask) & mask)
        })
        .collect();
    // Some fields alias the bits of others; the counters are written last.
    chosen.sort_by_key(|&(counter, ..)| counter);
    let mut story: Vec<_> = chosen
        .into_iter()
        .map(|(_, key, value)| (key, value))
        .collect();
    let month = story.len() as u32 % 12;
    story.push((StoryKey::STARBASE_MONTH, 1 + month));
    story.push((StoryKey::STARBASE_DAY, 1));
    story
}

/// Story fields drawn from a fixed xorshift seed.
fn drawn_story(seed: u64) -> Vec<(StoryKey, u32)> {
    let mut state = seed;
    story_with(|_| {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state as u32
    })
}

/// Every story field set to 1: each flag-guarded branch taken at once.
fn all_set_story() -> Vec<(StoryKey, u32)> {
    story_with(|_| 1)
}

fn campaign(world: &mut World, art: crate::graphics::native_abi::FRAME, story: &[(StoryKey, u32)]) {
    use crate::engine::battle::{frames::FrameSelection, link_queue::ShipPickerResources};
    *world = World::new();
    world.story = Some(StoryState::new());
    crate::time::clock_bridge::rust_clock_init(world);
    for species in 0..=u32::from(YEHAT_REBEL_SHIP) {
        let mut fleets = world.avail_races();
        let id = fleets.alloc().unwrap_or_else(|| panic!("fleet pool"));
        fleets.get_mut(id).actual_strength = 100;
        fleets.get_mut(id).species_id = species;
        fleets.put(id);
        world.retain_fleet_resources(
            species,
            ShipPickerResources::new(None, FrameSelection::retain(art)),
        );
    }
    let sis = &mut world.data.flagship;
    sis.crew_enlisted = 40;
    sis.fuel_on_board = 5000;
    sis.res_units = 2000;
    sis.num_landers = 2;
    sis.element_amounts = [40; 8];
    sis.total_element_mass = 320;
    // The Starbase conversation is only reached once the starbase has a visit date.
    world.set_story_state(StoryKey::STARBASE_MONTH, 2);
    world.set_story_state(StoryKey::STARBASE_DAY, 17);
    for &(key, value) in story {
        world.set_story_state(key, value);
    }
}

fn phrases() -> ConversationTable {
    ConversationTable::from(LoadedTextTable::fixture(vec![b"Phrase\0".to_vec(); 512]))
}

/// Hail `kind` with the fixture's art and an all-text phrase table, run
/// `body` on the open conversation, then close it. The Starbase and
/// Commander conversations run inside the starbase visit, which shows
/// resource units on the status line.
fn converse<R>(
    cx: &mut Cx<'_>,
    kind: DialogueKind,
    body: impl FnOnce(&mut Comm, &mut Cx<'_>) -> R,
) -> R {
    let status = if matches!(kind, DialogueKind::Starbas | DialogueKind::Comandr) {
        crate::starbase::platform::StatusMessageMode::ResUnits
    } else {
        crate::starbase::platform::StatusMessageMode::Credits
    };
    let previous = cx.kernel.set_status_message_mode(status as u32);
    let mut comm = Comm::new();
    let mut descriptor = lifecycle::session(&mut comm, cx, kind);
    descriptor.definition.alien_frame_res = Some(c"fixture.comm.graphics");
    descriptor.definition.alien_font_res = Some(c"fixture.comm.font");
    descriptor.definition.alien_colormap_res = None;
    descriptor.definition.alien_song_res = None;
    descriptor.definition.alien_alt_song_res = None;
    descriptor.definition.conversation_phrases_res = None;
    comm.install_descriptor(descriptor);
    crate::comm::hail::prepare(&mut comm, cx.world, cx.kernel, (320, 240))
        .unwrap_or_else(|error| panic!("{kind:?}: {error}"));
    drop(
        comm.display
            .assets
            .as_mut()
            .map(|assets| assets.intro_batch.take()),
    );
    comm.set_conversation_phrases(Some(phrases()));
    let result = body(&mut comm, cx);
    crate::comm::hail::finish(&mut comm, cx);
    cx.kernel.set_status_message_mode(previous);
    result
}

/// Run one conversation, choosing `choose(step, offered)` at each step.
fn walk(cx: &mut Cx<'_>, kind: DialogueKind, choose: fn(usize, usize) -> usize) -> usize {
    converse(cx, kind, |comm, cx| {
        comm.data.init_encounter_func.expect("intro")(comm, cx);
        let mut delivered = 0;
        for step in 0..24 {
            let offered = comm.responses().count();
            if offered == 0 || crate::comm::talk_segue::check_abort_external(cx.world) {
                break;
            }
            assert!(comm.responses_mut().select(choose(step, offered) as i32));
            let activity = cx.world.activity();
            let (func, reference) = crate::comm::talk_segue::select_response(activity, comm)
                .unwrap_or_else(|| panic!("{kind:?} offered a response without a handler"));
            func(comm, cx, reference);
            delivered += 1;
        }
        delivered
    })
}

const POLICIES: [fn(usize, usize) -> usize; 4] = [
    |_, _| 0,
    |_, offered| offered - 1,
    |step, offered| step % offered,
    |step, offered| (step * 7 + 3) % offered,
];

/// Run `body` once per campaign on the display fixture with the conversation
/// resources installed: fixture art, fonts, panels and a pacing runtime.
fn with_fixture(mut body: impl FnMut(&mut Game, crate::graphics::native_abi::FRAME)) {
    use crate::graphics::{
        context_abi::{ContextFrame, ContextLease},
        native_font::FontLease,
    };
    let mut game = Game::new();
    let display = crate::campaign_display_test::CampaignDisplay::install_prepared(
        &mut game,
        || {},
        |game| {
            let space = ContextLease::require(game.kernel.status_context);
            space.set_frame(ContextFrame::retain(std::ptr::NonNull::new(
                game.kernel.screen,
            )));
            game.kernel.space_context = space.as_raw();
            game.kernel.radar_context = space.as_raw();
            space.set_font(&FontLease::retain(
                std::ptr::NonNull::new(game.kernel.tiny_font)
                    .unwrap_or_else(|| panic!("tiny font fixture")),
            ));
            // Cargo and status panels the starbase and trader scripts redraw.
            let icons = crate::graphics::drawable_abi::LoadedDrawable::create(8, 8, 8, 128)
                .unwrap_or_else(|| panic!("icon fixture allocation failed"));
            let first = icons
                .lease()
                .frame(0)
                .unwrap_or_else(|| panic!("icon fixture has no frame"));
            game.kernel.misc_data_frame = first;
            game.kernel.flag_stat_frame = first;
            game.kernel.font_grad_frame = first;
            game.kernel.star_con_font = game.kernel.tiny_font;
            let input = InputSession::new(Default::default(), game.kernel.configuration.clone());
            game.kernel
                .install_input_runtime(std::rc::Rc::new(Pacing(input)));
            body(game, first);
            game.kernel.misc_data_frame = std::ptr::null_mut();
            game.kernel.flag_stat_frame = std::ptr::null_mut();
            game.kernel.font_grad_frame = std::ptr::null_mut();
            drop(icons);
        },
    )
    .unwrap_or_else(|error| panic!("display fixture: {error}"));
    display.finish(game);
}

fn run_all(story: &[(StoryKey, u32)]) {
    with_fixture(|game, art| {
        for kind in KINDS {
            for policy in POLICIES {
                campaign(&mut game.world, art, story);
                let mut battle = Battle::new();
                let mut cx = Cx::new(&mut battle, &mut game.world, &mut game.kernel, || 7);
                walk(&mut cx, kind, policy);
                crate::time::clock_bridge::rust_clock_uninit(cx.world);
            }
        }
    });
}

#[test]
#[serial_test::serial]
fn every_conversation_walks_its_responses_on_a_fresh_campaign() {
    if !crate::native_test_process::own_graphics_process(concat!(
        module_path!(),
        "::every_conversation_walks_its_responses_on_a_fresh_campaign"
    )) {
        return;
    }
    run_all(&[]);
}

#[test]
#[serial_test::serial]
fn every_conversation_walks_its_responses_in_drawn_story_states() {
    if !crate::native_test_process::own_graphics_process(concat!(
        module_path!(),
        "::every_conversation_walks_its_responses_in_drawn_story_states"
    )) {
        return;
    }
    for seed in [
        0x9e37_79b9_7f4a_7c15,
        0x0123_4567_89ab_cdef,
        0x5bd1_e995_c2b2_ae35,
    ] {
        run_all(&drawn_story(seed));
    }
}

/// Every response handler of every race, reached through the race's callback table.
fn handler_tables() -> Vec<(DialogueKind, Vec<crate::comm::response::ResponseFunc>)> {
    vec![
        {
            use crate::comm::races::arilou::Handler as H;
            let callbacks: Vec<_> = [
                H::ExitConversation,
                H::ArilouHome,
                H::AngryHomeArilou,
                H::AngrySpaceArilou,
                H::FriendlySpaceArilou,
            ]
            .into_iter()
            .map(super::arilou::callback)
            .collect();
            (DialogueKind::Arilou, callbacks)
        },
        {
            use crate::comm::races::blackur::Handler as H;
            let callbacks: Vec<_> = [H::Combat, H::Story]
                .into_iter()
                .map(super::blackur::callback)
                .collect();
            (DialogueKind::Blackur, callbacks)
        },
        {
            use crate::comm::races::chmmr::Handler as H;
            let callbacks: Vec<_> = [
                H::ExitConversation,
                H::NotReady,
                H::ImproveBomb,
                H::ChmmrFree,
                H::ChmmrAdvice,
                H::ChmmrShielded,
                H::AfterBomb,
            ]
            .into_iter()
            .map(super::chmmr::callback)
            .collect();
            (DialogueKind::Chmmr, callbacks)
        },
        {
            use crate::comm::races::druuge::Handler as H;
            let callbacks: Vec<_> = [
                H::ExitConversation,
                H::Buy,
                H::Trade,
                H::Sell,
                H::ExplainSlaveTrade,
                H::TradeWorld,
                H::BombAmbush,
                H::Space,
            ]
            .into_iter()
            .map(super::druuge::callback)
            .collect();
            (DialogueKind::Druuge, callbacks)
        },
        {
            use crate::comm::races::melnorm::Handler as H;
            let callbacks: Vec<_> = [
                H::ExitConversation,
                H::DoRescue,
                H::DoBuy,
                H::DoSell,
                H::NatureOfConversation,
                H::DoBluster,
                H::DoFirstMeeting,
                H::DoMelnormeMiffed,
                H::DoMelnormePissed,
            ]
            .into_iter()
            .map(super::melnorm::callback)
            .collect();
            (DialogueKind::Melnorm, callbacks)
        },
        {
            use crate::comm::races::mycon::Handler as H;
            let callbacks: Vec<_> = [
                H::CombatIsInevitable,
                H::SunDevice,
                H::TrickMycon,
                H::NormalMycon,
            ]
            .into_iter()
            .map(super::mycon::callback)
            .collect();
            (DialogueKind::Mycon, callbacks)
        },
        {
            use crate::comm::races::orz::Handler as H;
            let callbacks: Vec<_> = [
                H::ExitConversation,
                H::TaaloWorld,
                H::OrzAllied,
                H::OfferAlliance,
                H::OrzNeutral,
                H::OrzAngry,
            ]
            .into_iter()
            .map(super::orz::callback)
            .collect();
            (DialogueKind::Orz, callbacks)
        },
        {
            use crate::comm::races::pkunk::Handler as H;
            let callbacks: Vec<_> = [
                H::ExitConversation,
                H::PkunkAngry,
                H::DiscussConquer,
                H::OfferAlliance,
                H::AboutPkunk,
                H::AboutIlwrath,
                H::PkunkHome,
                H::PkunkFriendlySpace,
                H::PkunkNeutralSpace,
                H::PkunkMigrate,
            ]
            .into_iter()
            .map(super::pkunk::callback)
            .collect();
            (DialogueKind::Pkunk, callbacks)
        },
        {
            use crate::comm::races::rebel::Handler as H;
            let callbacks: Vec<_> = [H::Exit, H::Info, H::Rebels]
                .into_iter()
                .map(super::rebel::callback)
                .collect();
            (DialogueKind::Rebel, callbacks)
        },
        {
            use crate::comm::races::shofixt::Handler as H;
            let callbacks: Vec<_> = [
                H::Exit,
                H::GiveMaidens,
                H::Console,
                H::Explain,
                H::Realize,
                H::Friendly,
            ]
            .into_iter()
            .map(super::shofixt::callback)
            .collect();
            (DialogueKind::Shofixt, callbacks)
        },
        {
            use crate::comm::races::slyhome::Handler as H;
            let callbacks: Vec<_> = [
                H::Exit,
                H::Home,
                H::Human,
                H::Alien,
                H::Probe,
                H::Function,
                H::Bug,
                H::Fix,
            ]
            .into_iter()
            .map(super::slyhome::callback)
            .collect();
            (DialogueKind::Slyhome, callbacks)
        },
        {
            use crate::comm::races::spahome::Handler as H;
            let callbacks: Vec<_> = [
                H::Exit,
                H::Password,
                H::Council,
                H::Angry,
                H::Alliance,
                H::Learn,
                H::Quest,
                H::Party,
                H::Allies,
                H::Info,
            ]
            .into_iter()
            .map(super::spahome::callback)
            .collect();
            (DialogueKind::Spahome, callbacks)
        },
        {
            use crate::comm::races::supox::Handler as H;
            let callbacks: Vec<_> = [
                H::AlliedHome,
                H::ExitConversation,
                H::BeforeKohrAh,
                H::AfterKohrAh,
                H::NeutralSupox,
            ]
            .into_iter()
            .map(super::supox::callback)
            .collect();
            (DialogueKind::Supox, callbacks)
        },
        {
            use crate::comm::races::syreen::Handler as H;
            let callbacks: Vec<_> = [
                H::FriendlyExit,
                H::Sex,
                H::Foreplay,
                H::AfterAmbush,
                H::AmbushReady,
                H::SyreenShuttle,
                H::NormalSyreen,
                H::InitialSyreen,
                H::SyreenVault,
            ]
            .into_iter()
            .map(super::syreen::callback)
            .collect();
            (DialogueKind::Syreen, callbacks)
        },
        {
            use crate::comm::races::talkpet::Handler as H;
            let callbacks: Vec<_> = [
                H::ExitConversation,
                H::MindFuckUrquan,
                H::MindControl,
                H::PetInfo,
                H::PetDevice,
                H::CompelPlayer,
                H::KillPet,
                H::PetDeal,
            ]
            .into_iter()
            .map(super::talkpet::callback)
            .collect();
            (DialogueKind::Talkpet, callbacks)
        },
        {
            use crate::comm::races::thradd::Handler as H;
            let callbacks: Vec<_> = [
                H::ExitConversation,
                H::ThraddAllies,
                H::ThraddDemeanor,
                H::ThraddCulture,
                H::HelixWorld,
                H::ThraddHostile,
            ]
            .into_iter()
            .map(super::thradd::callback)
            .collect();
            (DialogueKind::Thradd, callbacks)
        },
        {
            use crate::comm::races::umgah::Handler as H;
            let callbacks: Vec<_> = [
                H::CombatIsInevitable,
                H::Zombies,
                H::NormalUmgah,
                H::UmgahReward,
            ]
            .into_iter()
            .map(super::umgah::callback)
            .collect();
            (DialogueKind::Umgah, callbacks)
        },
        {
            use crate::comm::races::utwig::Handler as H;
            let callbacks: Vec<_> = [
                H::AlliedHome,
                H::ExitConversation,
                H::BeforeKohrAh,
                H::AfterKohrAh,
                H::NeutralUtwig,
                H::BombWorld,
            ]
            .into_iter()
            .map(super::utwig::callback)
            .collect();
            (DialogueKind::Utwig, callbacks)
        },
        {
            use crate::comm::races::zoqfot::Handler as H;
            let callbacks: Vec<_> = [
                H::ExitConversation,
                H::ZoqFotIntro,
                H::FormAlliance,
                H::AquaintZoqFot,
                H::ZoqFotInfo,
                H::ZoqFotHome,
            ]
            .into_iter()
            .map(super::zoqfot::callback)
            .collect();
            (DialogueKind::Zoqfot, callbacks)
        },
        {
            use crate::comm::races::ilwrath::Handler as H;
            let callbacks: Vec<_> = [
                H::CombatIsInevitable,
                H::IlwrathGods,
                H::IlwrathInfo,
                H::IlwrathHome,
                H::GodsOrder,
                H::GodsSpeak,
            ]
            .into_iter()
            .map(super::ilwrath::callback)
            .collect();
            (DialogueKind::Ilwrath, callbacks)
        },
        {
            use crate::comm::races::spathi::Handler as H;
            let callbacks: Vec<_> = [
                H::ExitConversation,
                H::SpathiOnPluto,
                H::SpathiMustGrovel,
                H::SpathiAllies,
                H::SpathiFriendly,
                H::SpathiBefriend,
                H::SpathiAntagonize,
                H::SpathiNeutral,
            ]
            .into_iter()
            .map(super::spathi::callback)
            .collect();
            (DialogueKind::Spathi, callbacks)
        },
        {
            use crate::comm::races::starbas::Handler as H;
            let callbacks: Vec<_> = [
                H::ByeBye,
                H::AllianceInfo,
                H::HierarchyInfo,
                H::AlienRaces,
                H::WarInfo,
                H::AncientHistory,
                H::TellHistory,
                H::DefeatUrquan,
                H::TellMission,
                H::TellStarBase,
                H::NeedInfo,
                H::NormalStarbase,
                H::SellMinerals,
            ]
            .into_iter()
            .map(super::starbas::callback)
            .collect();
            (DialogueKind::Starbas, callbacks)
        },
        {
            use crate::comm::races::urquan::Handler as H;
            let callbacks: Vec<_> = [
                H::Combat,
                H::Normal,
                H::Hypno,
                H::MentionCompulsion,
                H::DescribePet,
                H::DescribeCompulsion,
                H::Loser,
            ]
            .into_iter()
            .map(super::urquan::callback)
            .collect();
            (DialogueKind::Urquan, callbacks)
        },
        {
            use crate::comm::races::vux::Handler as H;
            let callbacks: Vec<_> = [H::Combat, H::Menagerie, H::Normal]
                .into_iter()
                .map(super::vux::callback)
                .collect();
            (DialogueKind::Vux, callbacks)
        },
        {
            use crate::comm::races::yehat::Handler as H;
            let callbacks: Vec<_> = [H::Exit, H::Royalists, H::StartRevolt, H::Home, H::Space]
                .into_iter()
                .map(super::yehat::callback)
                .collect();
            (DialogueKind::Yehat, callbacks)
        },
    ]
}

#[test]
#[serial_test::serial]
fn every_response_handler_runs_from_its_callback_table() {
    if !crate::native_test_process::own_graphics_process(concat!(
        module_path!(),
        "::every_response_handler_runs_from_its_callback_table"
    )) {
        return;
    }
    let stories = [
        Vec::new(),
        all_set_story(),
        drawn_story(0x2545_f491_4f6c_dd1d),
        drawn_story(0x9e37_79b9_7f4a_7c15),
    ];
    with_fixture(|game, art| {
        for (kind, handlers) in handler_tables() {
            for handler in handlers {
                for story in &stories {
                    for reference in [0, 1, 2, 3] {
                        campaign(&mut game.world, art, story);
                        let mut battle = Battle::new();
                        let mut cx = Cx::new(&mut battle, &mut game.world, &mut game.kernel, || 7);
                        converse(&mut cx, kind, |comm, cx| handler(comm, cx, reference));
                        crate::time::clock_bridge::rust_clock_uninit(cx.world);
                    }
                }
            }
        }
    });
}
