//! Starbase encounters, menus and retained flagship drawing.
use crate::engine::input::{frames::*, InputSession};
use crate::engine::world::activity::activity_flags;
use crate::engine::world::activity::ActivityKind;
use crate::engine::{
    battle::Cx,
    observe::{CheckpointView, Io},
    StoryKey, World,
};
use crate::graphics::{drawable_abi::LoadedDrawable, menu as draw, native_abi::*};
use crate::sound::resource_types::LoadedMusic;
use crate::starbase::platform::StatusMessageMode;
use crate::starbase::{
    draw::{draw_base_state_strings, rotate_starbase, StarbaseAnimation},
    graphics::Graphics,
    starbase_menu::StarbaseState,
};
use crate::ui::settings::stop_music;
use crate::ui::sis;
use uqm_engine::time::counter_abi::ONE_SECOND;
const CROSSFADE_SCREEN: u32 = 2;
const FADE_IN_SCREEN: u32 = 3;
const SOUND_ARROWS: u16 = 0x0f;
const SOUND_SELECT: u16 = 1 << 4;
const BLACK: Color = super::rgb15(0, 0, 0);
struct StarbaseSession {
    initialized: bool,
    state: StarbaseState,
    art: Option<LoadedDrawable>,
    music: Option<LoadedMusic>,
    animation: StarbaseAnimation,
}
fn finish_starbase_screen(world: &mut World, menu: &mut StarbaseSession) -> bool {
    menu.art = None;
    stop_music();
    menu.music = None;
    if menu.state == StarbaseState::DepartBase {
        if world.activity() & (activity_flags::CHECK_ABORT | activity_flags::CHECK_LOAD) == 0 {
            world.set_story_state(StoryKey::STARBASE_VISITED, 0);
        }
        return false;
    }
    menu.initialized = false;
    true
}
fn activate_starbase_selection(
    input: &InputSession,
    cx: &mut Cx<'_>,
    io: &mut Io<'_>,
    menu: &mut StarbaseSession,
    services: &super::MenuServices,
) -> bool {
    if !finish_starbase_screen(cx.world, menu) {
        return false;
    }
    match menu.state {
        StarbaseState::TalkCommander => {
            io.observer.checkpoint(&CheckpointView::StarbaseMenu(false));
            cx.kernel.input_runtime().flush();
            (services.communicate)(
                cx,
                io,
                crate::engine::species::conv::COMMANDER,
                Default::default(),
            );
            io.observer.checkpoint(&CheckpointView::StarbaseMenu(true));
            cx.world
                .set_story_state(StoryKey::GLOBAL_FLAGS_AND_DATA, u8::MAX as u32);
        }
        StarbaseState::OutfitStarship | StarbaseState::Shipyard => {
            input.set_menu_sounds(SOUND_ARROWS, SOUND_SELECT);
            menu.music = if menu.state == StarbaseState::OutfitStarship {
                super::outfit::run(cx, io, services)
            } else {
                super::shipyard::run(cx, io, services)
            };
            menu.initialized = false;
        }
        StarbaseState::DepartBase => unreachable!(),
    }
    true
}
fn update_starbase_screen(
    cx: &mut Cx<'_>,
    input: &InputSession,
    menu: &mut StarbaseSession,
) -> bool {
    if cx.world.activity() & (activity_flags::CHECK_ABORT | activity_flags::CHECK_LOAD) != 0 {
        menu.state = StarbaseState::DepartBase;
        return false;
    }
    input.set_menu_sounds(SOUND_ARROWS, SOUND_SELECT);
    if !menu.initialized {
        cx.kernel.last_activity &= !activity_flags::CHECK_LOAD;
        sis::clear_flash(cx.kernel);
        if menu.music.is_some() {
            stop_music();
            menu.music = None;
        }
        menu.initialized = true;
        menu.art = LoadedDrawable::load(c"graphics.starbase");
        menu.music = LoadedMusic::load(c"music.starbase");
        draw::select(cx.kernel.screen_context);
        draw::transition_source();
        draw::batch();
        draw::background(BLACK);
        draw::clear();
        let graphics = Graphics::new(cx.kernel, cx.screen_dimensions());
        let frame = menu
            .art
            .as_ref()
            .unwrap_or_else(|| panic!("starbase graphics missing"))
            .lease()
            .frame(0)
            .unwrap_or_else(|| panic!("starbase art has no frame"));
        rotate_starbase(
            &graphics,
            &mut menu.animation,
            Some(frame as usize),
            uqm_engine::time::source::now_ticks(),
        );
        draw_base_state_strings(&graphics, None, menu.state);
        draw::transition(3);
        crate::ui::settings::play_loaded_music(cx.world, menu.music.as_ref(), 1, 1);
        draw::unbatch();
    } else if input.pulsed().menu[KEY_MENU_SELECT as usize] != 0 {
        return false;
    } else {
        let mut state = menu.state;
        if input.pulsed().menu[KEY_MENU_LEFT as usize] != 0
            || input.pulsed().menu[KEY_MENU_UP as usize] != 0
        {
            state = state.previous();
        } else if input.pulsed().menu[KEY_MENU_RIGHT as usize] != 0
            || input.pulsed().menu[KEY_MENU_DOWN as usize] != 0
        {
            state = state.next();
        }
        draw::batch();
        draw::select(cx.kernel.screen_context);
        let graphics = Graphics::new(cx.kernel, cx.screen_dimensions());
        if state != menu.state {
            draw_base_state_strings(&graphics, Some(menu.state), state);
            menu.state = state;
        }
        rotate_starbase(
            &graphics,
            &mut menu.animation,
            None,
            uqm_engine::time::source::now_ticks(),
        );
        draw::unbatch();
        cx.kernel.input_runtime().sleep(cx, ONE_SECOND / 30);
    }
    true
}

fn do_time_passage(cx: &mut Cx<'_>) {
    let interval = (ONE_SECOND * 2) as SIZE;
    let deadline = draw::fade_screen(draw::ScreenFade::Black, interval);
    cx.kernel.input_runtime().sleep_until(cx, deadline);
    crate::time::clock_abi::advance_days(cx, 14);
}

fn cleanup_after_star_base(world: &mut World) {
    if world.activity() & (activity_flags::CHECK_LOAD | activity_flags::CHECK_ABORT) == 0 {
        world.set_story_state(StoryKey::GLOBAL_FLAGS_AND_DATA, 0);
        world.set_activity(activity_flags::CHECK_LOAD);
        world.next_activity =
            (ActivityKind::InInterplanetary as u32) as u16 | activity_flags::START_INTERPLANETARY;
    }
}

fn talk_to_commander(
    cx: &mut Cx<'_>,
    io: &mut Io<'_>,
    services: &super::MenuServices,
    intro: (u32, u32),
) {
    (services.communicate)(
        cx,
        io,
        crate::engine::species::conv::COMMANDER,
        crate::comm::CommOptions {
            intro,
            ..Default::default()
        },
    );
}

fn prepare_starbase_visit(
    cx: &mut Cx<'_>,
    io: &mut Io<'_>,
    services: &super::MenuServices,
) -> Option<u32> {
    if cx.world.story_state(StoryKey::CHMMR_BOMB_STATE) == 2 {
        cx.world.select_star(None);
        cx.world
            .set_story_state(StoryKey::GLOBAL_FLAGS_AND_DATA, BYTE::MAX as u32);
    }
    if cx.world.story_state(StoryKey::STARBASE_AVAILABLE) == 0 {
        io.observer.checkpoint(&CheckpointView::EncounterDispatch(
            crate::engine::species::conv::COMMANDER,
        ));
        talk_to_commander(cx, io, services, (CROSSFADE_SCREEN, 0));
        if cx.world.story_state(StoryKey::PROBE_ILWRATH_ENCOUNTER) == 0
            || cx.world.activity() & activity_flags::CHECK_ABORT != 0
        {
            cleanup_after_star_base(cx.world);
            return None;
        }
        let ship = crate::comm::dispatch::clone_npc_fragment(
            cx.world,
            crate::engine::species::ship::ILWRATH_SHIP,
            7,
        )
        .unwrap_or_else(|| panic!("starbase Ilwrath fragment capacity"));
        cx.world.npc_built_ships().get_mut(ship).race_id = BYTE::MAX;
        (services.communicate)(
            cx,
            io,
            crate::engine::species::conv::ILWRATH,
            Default::default(),
        );
        if cx.world.data.flagship.crew_enlisted == COUNT::MAX
            || cx.world.activity() & activity_flags::CHECK_ABORT != 0
        {
            return None;
        }
        talk_to_commander(cx, io, services, (CROSSFADE_SCREEN, 0));
        if cx.world.activity() & activity_flags::CHECK_ABORT != 0 {
            return None;
        }
        cx.world
            .set_story_state(StoryKey::GLOBAL_FLAGS_AND_DATA, BYTE::MAX as u32);
    }
    let previous = cx
        .kernel
        .set_status_message_mode(StatusMessageMode::ResUnits as u32);
    if cx.world.story_state(StoryKey::MOONBASE_ON_SHIP) != 0
        || cx.world.story_state(StoryKey::CHMMR_BOMB_STATE) == 2
    {
        do_time_passage(cx);
        if cx.world.data.flagship.crew_enlisted == COUNT::MAX {
            return None;
        }
        io.observer.checkpoint(&CheckpointView::EncounterDispatch(
            crate::engine::species::conv::COMMANDER,
        ));
        talk_to_commander(cx, io, services, (FADE_IN_SCREEN, ONE_SECOND * 2));
        if cx.world.activity() & activity_flags::CHECK_ABORT != 0 {
            return None;
        }
        cx.world
            .set_story_state(StoryKey::GLOBAL_FLAGS_AND_DATA, BYTE::MAX as u32);
    }
    Some(previous)
}

/// Run the retained visit and its synchronous campaign submodes.
pub fn run_starbase_visit(cx: &mut Cx<'_>, io: &mut Io<'_>, services: &super::MenuServices) {
    let Some(previous) = prepare_starbase_visit(cx, io, services) else {
        return;
    };
    let runtime = cx.kernel.input_runtime();
    let input = runtime.input();
    let mut menu = StarbaseSession {
        initialized: false,
        state: StarbaseState::TalkCommander,
        art: None,
        music: None,
        animation: StarbaseAnimation::default(),
    };
    let old = draw::select(cx.kernel.screen_context);
    io.observer.checkpoint(&CheckpointView::StarbaseMenu(true));
    cx.run_input(io, true, |cx, io| {
        if update_starbase_screen(cx, input, &mut menu) {
            true
        } else {
            activate_starbase_selection(input, cx, io, &mut menu, services)
        }
    });
    io.observer.checkpoint(&CheckpointView::StarbaseMenu(false));
    draw::select(old);
    cx.kernel.set_status_message_mode(previous);
    cleanup_after_star_base(cx.world);
}
/// Advance fourteen days, finish bomb installation and force a SolarSys reload.
pub fn install_bomb_at_earth(cx: &mut Cx<'_>) {
    let screen_context = cx.kernel.screen_context;
    do_time_passage(cx);
    draw::select(screen_context);
    draw::transition_source();
    draw::background(BLACK);
    draw::clear();
    let deadline = draw::fade_screen(draw::ScreenFade::Color, 0);
    cx.kernel.input_runtime().sleep_until(cx, deadline);
    cx.world.set_story_state(StoryKey::CHMMR_BOMB_STATE, 3);
    cx.world.set_activity(activity_flags::CHECK_LOAD);
    cx.world.next_activity =
        (ActivityKind::InInterplanetary as u32) as u16 | activity_flags::START_INTERPLANETARY;
    cx.world.select_star(None);
}
