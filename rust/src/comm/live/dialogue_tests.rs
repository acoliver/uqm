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

/// Story fields drawn from a fixed xorshift seed, each within its own width.
/// Counters the scripts advance from zero and cap (visits, counts, mineral
/// loads) are drawn within the range the scripts leave them in.
fn drawn_story(seed: u64) -> Vec<(StoryKey, u32)> {
    let mut state = seed;
    let mut drawn: Vec<_> = crate::engine::world::story::GAME_STATE_FIELDS
        .iter()
        .map(|&(name, start, end)| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            let width = end - start + 1;
            let visits = name.ends_with("_VISITS") || name.ends_with("_COUNT");
            let mask = if visits {
                1
            } else if name.ends_with("_MINERAL_LOAD") {
                3
            } else if width >= 32 {
                u32::MAX
            } else {
                (1u32 << width) - 1
            };
            let key = StoryKey::from_name(name).unwrap_or_else(|| panic!("{name}"));
            (visits, key, state as u32 & mask)
        })
        .collect();
    // Some fields alias the bits of others; the visit counters are written last.
    drawn.sort_by_key(|&(visits, ..)| visits);
    let mut story: Vec<_> = drawn
        .into_iter()
        .map(|(_, key, value)| (key, value))
        .collect();
    // The starbase visit date is a calendar date.
    story.push((StoryKey::STARBASE_MONTH, 1 + (seed % 12) as u32));
    story.push((StoryKey::STARBASE_DAY, 1 + (seed % 28) as u32));
    story
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

/// Run one conversation, choosing `choose(step, offered)` at each step.
/// The descriptor's art is swapped for the fixture's, and every phrase has
/// text, so each response the script offers can be built and chosen.
fn walk(cx: &mut Cx<'_>, kind: DialogueKind, choose: fn(usize, usize) -> usize) -> usize {
    // The Starbase conversation runs inside the starbase visit, which shows
    // resource units on the status line.
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
    comm.data.init_encounter_func.expect("intro")(&mut comm, cx);
    let mut delivered = 0;
    for step in 0..24 {
        let offered = comm.responses().count();
        if offered == 0 || crate::comm::talk_segue::check_abort_external(cx.world) {
            break;
        }
        assert!(comm.responses_mut().select(choose(step, offered) as i32));
        let activity = cx.world.activity();
        let (func, reference) = crate::comm::talk_segue::select_response(activity, &mut comm)
            .unwrap_or_else(|| panic!("{kind:?} offered a response without a handler"));
        func(&mut comm, cx, reference);
        delivered += 1;
    }
    crate::comm::hail::finish(&mut comm, cx);
    cx.kernel.set_status_message_mode(previous);
    delivered
}

const POLICIES: [fn(usize, usize) -> usize; 4] = [
    |_, _| 0,
    |_, offered| offered - 1,
    |step, offered| step % offered,
    |step, offered| (step * 7 + 3) % offered,
];

fn run_all(story: &[(StoryKey, u32)]) {
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
            for kind in KINDS {
                for policy in POLICIES {
                    campaign(&mut game.world, first, story);
                    let mut battle = Battle::new();
                    let mut cx = Cx::new(&mut battle, &mut game.world, &mut game.kernel, || 7);
                    walk(&mut cx, kind, policy);
                    crate::time::clock_bridge::rust_clock_uninit(cx.world);
                }
            }
            game.kernel.misc_data_frame = std::ptr::null_mut();
            game.kernel.flag_stat_frame = std::ptr::null_mut();
            game.kernel.font_grad_frame = std::ptr::null_mut();
            drop(icons);
        },
    )
    .unwrap_or_else(|error| panic!("display fixture: {error}"));
    display.finish(game);
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
