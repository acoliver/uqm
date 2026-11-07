//! Encounter fleet construction, decision screen, combat entry, and salvage.

pub const ENCOUNTER_DISPLAY_POINTS: [(i16, i16); 10] = [
    (10, 51),
    (-10, 51),
    (33, 40),
    (-33, 40),
    (49, 18),
    (-49, 18),
    (52, -6),
    (-52, -6),
    (44, -27),
    (-44, -27),
];
const RACE_BOUNTY: [u16; 23] = [
    1600, 3000, 1100, 2300, 2000, 500, 1800, 1600, 1000, 2200, 1200, 2300, 3600, 1700, 1000, 2100,
    4400, 700, 3000, 600, 1300, 3000, 2300,
];

pub const fn victory_state(opponent: u8, player: u8, protecting_samatra: bool) -> i16 {
    if opponent != 0 || player == 0 || protecting_samatra {
        0
    } else {
        1
    }
}
pub const fn fragment_is_removed(crew: u16, victory: bool, npc: bool) -> bool {
    crew == 0 || (victory && npc)
}
pub const fn thraddash_body_count(killed: u16, previous: u16, threshold: u16) -> u16 {
    let sum = killed.wrapping_add(previous);
    if sum > threshold {
        threshold
    } else {
        sum
    }
}
pub const fn combat_speed(flags: u8) -> u8 {
    match (flags & 0xc0) >> 6 {
        1 => 0,
        2 => 3,
        _ => 255,
    }
}

mod formation;
use formation::encounter_icon_origin;

struct EncounterSelection {
    initialized: bool,
    choice: u8,
}

#[derive(Debug, PartialEq)]
enum SelectionAction {
    Continue,
    Finish,
    Options,
    Unknown(u8),
}

impl EncounterSelection {
    fn new() -> Self {
        Self {
            initialized: false,
            choice: 0,
        }
    }

    fn advance(&mut self, aborted: bool, selected: bool, last_battle: bool) -> SelectionAction {
        if aborted {
            self.choice = 2;
            return SelectionAction::Finish;
        }
        if !self.initialized {
            self.initialized = true;
        } else if selected {
            return match self.choice {
                0 | 1 => {
                    if last_battle {
                        self.choice = 0;
                    }
                    SelectionAction::Finish
                }
                2 => SelectionAction::Options,
                choice => SelectionAction::Unknown(choice),
            };
        }
        SelectionAction::Continue
    }
}

fn encounter_has_no_salvage(world: &crate::engine::World) -> bool {
    use crate::ships::native::abi::{
        CHECK_ABORT, CHECK_LOAD, COUNT, IN_LAST_BATTLE, WON_LAST_BATTLE,
    };
    world.activity() & (CHECK_ABORT | CHECK_LOAD) as u16 != 0
        || world.data.flagship.crew_enlisted == COUNT::MAX
        || world.activity() as u8 == IN_LAST_BATTLE as u8
        || world.activity() as u8 == WON_LAST_BATTLE as u8
}

fn apply_thraddash_body_count(world: &mut crate::engine::World, killed: u16) -> u16 {
    use crate::engine::StoryKey;
    use crate::ships::native::abi::{THRADDASH_BODY_THRESHOLD, THRADDASH_SHIP};
    if killed != 0
        && world.encounter_race == THRADDASH_SHIP as i16
        && world.story_state(StoryKey::THRADD_MANNER) == 0
    {
        let killed = thraddash_body_count(
            killed,
            world.story_state(StoryKey::THRADDASH_BODY_COUNT) as u16,
            THRADDASH_BODY_THRESHOLD as u16,
        );
        world.set_story_state(StoryKey::THRADDASH_BODY_COUNT, u32::from(killed as u8));
        return killed;
    }
    killed
}

pub(crate) mod engine {
    mod display;
    mod salvage;
    use super::*;
    #[cfg(not(test))]
    use crate::battle::battle_native::battle;
    #[cfg(not(test))]
    use crate::battle::battle_native::engine::{
        AWESOME_RATING, CYBORG_CONTROL, HUMAN_CONTROL, STANDARD_RATING,
    };
    use crate::engine::battle::link_queue;
    #[cfg(not(test))]
    use crate::engine::StoryKey;
    #[cfg(not(test))]
    use crate::pickship::ffi::draw_armada_pick_ship_in as draw_armada;
    use crate::ships::native::abi::*;
    use crate::ui::rgb15;

    const TEXT_FADE: [u8; 7] = [10, 9, 8, 6, 5, 4, 3];
    const SHIP_FADE: [(u8, u8, u8); 10] = [
        (7, 0, 0),
        (15, 0, 0),
        (23, 0, 0),
        (31, 10, 10),
        (31, 20, 20),
        (31, 31, 31),
        (31, 20, 20),
        (31, 10, 10),
        (27, 0, 0),
        (23, 0, 0),
    ];

    use crate::engine::world::{CStarDesc, CATALOG_LEN};
    #[cfg(not(test))]
    use crate::planets::starmap::naming::cluster_name;

    fn escape_pressed(input: &crate::engine::input::InputSession) -> bool {
        let template = input.player_controls()[0] as usize;
        input.current().key[template][KEY_ESCAPE as usize] != 0
    }
    enum SalvageText {
        Caption,
        Amount,
        Total,
    }

    #[cfg(not(test))]
    fn do_select_action(
        cx: &mut crate::engine::battle::Cx<'_>,
        io: &mut crate::engine::observe::Io<'_>,
        menu: &mut EncounterSelection,
    ) -> bool {
        let runtime = cx.kernel.input_runtime();
        let input = runtime.input();
        SetMenuSounds(input, 0x0f, 1 << 4);
        match menu.advance(
            cx.world.activity() & CHECK_ABORT as u16 != 0,
            input.pulsed().menu[KEY_MENU_SELECT as usize] != 0,
            cx.world.activity() as u8 == IN_LAST_BATTLE as u8,
        ) {
            SelectionAction::Finish => return false,
            SelectionAction::Options => {
                if crate::front_end::gameopt::game_options(cx, io) == 0 {
                    return false;
                }
                crate::ui::menu::draw_menu_state(
                    cx.kernel,
                    cx.world,
                    cx.screen_dimensions(),
                    PM_CONVERSE as u8,
                    menu.choice as i16,
                );
                crate::ui::sis::set_menu_flash(cx.kernel, cx.screen_dimensions(), false);
            }
            SelectionAction::Unknown(choice) => {
                println!("Unknown option: {choice}");
            }
            SelectionAction::Continue => {}
        }
        crate::ui::menu::choose_menu_state(cx, input, &mut menu.choice, PM_CONVERSE as u8);
        true
    }

    fn npc_race(world: &crate::engine::World) -> i16 {
        let ships = world.npc_built_ships_ref();
        let id = ships
            .head()
            .unwrap_or_else(|| panic!("encounter has an NPC fleet"));
        i16::from(ships.get(id).race_id)
    }

    type FragmentId =
        crate::engine::battle::link_queue::LinkId<crate::engine::world::queues::Fragment>;

    fn fragment_snapshot(
        world: &crate::engine::World,
        player: bool,
    ) -> Vec<(FragmentId, SHIP_FRAGMENT)> {
        if player {
            copy_fragments(world.built_ships_ref())
        } else {
            copy_fragments(world.npc_built_ships_ref())
        }
    }

    fn copy_fragments<const N: usize>(
        queue: link_queue::LinkQueue<'_, crate::engine::world::queues::Fragment, N>,
    ) -> Vec<(FragmentId, SHIP_FRAGMENT)> {
        let mut records = Vec::new();
        let mut next = queue.head();
        while let Some(id) = next {
            records.push((id, *queue.get(id)));
            next = queue.succ(id);
        }
        records
    }

    fn remove_campaign_fragment(world: &mut crate::engine::World, escorts: bool, id: FragmentId) {
        if escorts {
            remove_fragment(world.built_ships(), id);
        } else {
            remove_fragment(world.npc_built_ships(), id);
        }
    }

    fn remove_fragment<const N: usize>(
        mut queue: link_queue::LinkQueue<'_, crate::engine::world::queues::Fragment, N>,
        id: FragmentId,
    ) {
        queue.remove(id);
        queue.free(id);
    }

    #[cfg(not(test))]
    pub fn build_battle(
        world: &mut crate::engine::World,
        kernel: &mut crate::engine::Kernel,
        player: COUNT,
    ) {
        world.encounter_race = -1;
        if world.npc_built_ships_ref().head().is_none() {
            world.set_story_state(StoryKey::BATTLE_SEGUE, 0);
            return;
        }
        if player != RPG_PLAYER_NUM as u16 {
            let selector = match world.activity() as u8 as u32 {
                IN_LAST_BATTLE => NUMBER_OF_PLANET_TYPES as u8,
                IN_HYPERSPACE => ((TFB_Random() as COUNT) % NUMBER_OF_PLANET_TYPES as COUNT) as u8,
                _ => {
                    world.set_story_state(StoryKey::ESCAPE_COUNTER, 110);
                    world.story_state(StoryKey::BATTLE_PLANET) as u8
                }
            };
            load_gravity_well(kernel, selector);
        }
        kernel.ships(player as usize).reinit();
        let fragments = fragment_snapshot(world, player == RPG_PLAYER_NUM as COUNT);
        for (_, frag) in fragments {
            let built =
                kernel
                    .ships(player as usize)
                    .build(if frag.race_id == SAMATRA_SHIP as u8 {
                        SPECIES_ID_SA_MATRA_ID
                    } else {
                        frag.species_id
                    });
            if let Some(id) = built {
                let mut ships = kernel.ships(player as usize);
                let mut ship = ships.get(id);
                ship.captain = frag.captains_name_index;
                ship.player = player as i16;
                ship.crew = if frag.crew_level != COUNT::MAX {
                    frag.crew_level
                } else {
                    frag.max_crew
                };
                ship.max_crew = frag.max_crew;
                ships.set_picker_resources(id, world.fleet_resources(frag.species_id));
                ship.index = frag.index as u16;
                ship.cost = 0;
                ships.set(id, ship);
                ships.clear_descriptor(id);
            }
        }
        if player == RPG_PLAYER_NUM as u16 {
            let built = kernel.ships(0).build(SPECIES_ID_SIS_SHIP_ID);
            if let Some(id) = built {
                let mut ships = kernel.ships(0);
                let mut ship = ships.get(id);
                ship.captain = 0;
                ship.player = RPG_PLAYER_NUM as i16;
                ship.crew = 0;
                ship.max_crew = 0;
                ship.index = COUNT::MAX;
                ship.cost = 0;
                ship.energy_counter = MAX_ENERGY_SIZE as u8;
                ships.set(id, ship);
                ships.clear_descriptor(id);
            }
        }
    }

    pub fn fleet_is_infinite(world: &crate::engine::World, player: COUNT) -> BOOLEAN {
        if player == RPG_PLAYER_NUM as COUNT {
            let queue = world.built_ships_ref();
            queue
                .head()
                .is_some_and(|id| queue.get(id).crew_level == COUNT::MAX) as BOOLEAN
        } else {
            let queue = world.npc_built_ships_ref();
            queue
                .head()
                .is_some_and(|id| queue.get(id).crew_level == COUNT::MAX) as BOOLEAN
        }
    }

    #[cfg(not(test))]
    pub(crate) use crate::battle::context_access::reader_legacy::update_ship_frag_crew;

    #[cfg(not(test))]
    fn draw_encounter_location(
        cx: &mut crate::engine::battle::Cx<'_>,
        cluster: Option<&std::ffi::CStr>,
    ) {
        let context = crate::graphics::context_abi::ContextLease::retain(
            std::ptr::NonNull::new(cx.kernel.space_context)
                .unwrap_or_else(|| panic!("encounter requires a space context")),
        );
        let font = cx
            .kernel
            .micro_font_lease()
            .unwrap_or_else(|| panic!("encounter requires the micro font"));
        let mut baseline = POINT {
            x: (((cx.screen_dimensions().0 - 78) >> 1) + 1) as i16,
            y: 10,
        };
        let mut line = |text: &std::ffi::CStr| {
            context.draw_text(
                &font,
                text,
                baseline,
                TEXT_ALIGN_ALIGN_CENTER,
                rgb15(0, 0, 20),
            );
            baseline.y += 12;
        };
        if let Some(cluster) = cluster {
            line(cx.kernel.game_text((ENCOUNTER_STRING_BASE + 2) as usize));
            line(cluster);
            line(
                std::ffi::CStr::from_bytes_until_nul(&cx.world.data.flagship.planet_name)
                    .unwrap_or_else(|error| panic!("planet name is not terminated: {error}")),
            );
        } else {
            line(cx.kernel.game_text(ENCOUNTER_STRING_BASE as usize));
            line(cx.kernel.game_text((ENCOUNTER_STRING_BASE + 1) as usize));
        }
    }

    fn draw_encounter_fleet(
        world: &crate::engine::World,
        context: &crate::graphics::context_abi::ContextLease,
        dimensions: (i32, i32),
    ) {
        if world.activity() as u8 == IN_LAST_BATTLE as u8 {
            return;
        }
        let fragments = fragment_snapshot(world, false);
        let mut index = 0;
        let mut i = 0;
        while index < fragments.len() && i < 60 {
            let frag = &fragments[index].1;
            if frag.crew_level != COUNT::MAX {
                index += 1;
            }
            let resources = world.fleet_resources(frag.species_id);
            if let Some(icon) = super::formation::encounter_icon(resources.icon()) {
                context.draw_stamp(
                    &icon,
                    encounter_icon_origin(i, icon.rect().extent, dimensions),
                );
            }
            i += 1;
        }
    }

    #[cfg(not(test))]
    fn prepare_encounter(cx: &mut crate::engine::battle::Cx<'_>) {
        let context = crate::graphics::context_abi::ContextLease::require(cx.kernel.space_context);
        let font = cx
            .kernel
            .tiny_font_lease()
            .unwrap_or_else(|| panic!("encounter requires the tiny font"));
        context.select();
        context.set_font(&font);
        let music = crate::sound::resource_types::LoadedMusic::load(c"music.redalert");
        crate::ui::settings::play_loaded_music(cx.world, music.as_ref(), 0, 1);
        let segue = crate::graphics::drawable_abi::LoadedDrawable::load(c"graphics.segue")
            .unwrap_or_else(|| panic!("encounter requires the red alert backdrop"));
        crate::sound::heart_ffi::wait_for_sound_end(COUNT::MAX);
        crate::ui::settings::stop_loaded_music(music.as_ref());
        drop(music);
        let (width, height) = cx.screen_dimensions();
        let dimensions = (width - 78, height - 13);
        context.transition_source(None);
        context.with_batch(|| {
            context.set_background(rgb15(0, 0, 0));
            context.clear();
            context.draw_stamp(&segue.lease().context_frame(0), POINT { x: 0, y: 0 });
            let cluster = if in_hqspace(cx.world) == 0 {
                let star = cx
                    .world
                    .selected_star()
                    .unwrap_or_else(|| panic!("encounter requires a selected star"));
                Some(cluster_name(cx.kernel, star, STAR_NUMBER_BASE as usize))
            } else {
                None
            };
            draw_encounter_location(cx, cluster.as_deref());
            let message = crate::ui::sis::default_message_text(cx.kernel, cx.world);
            crate::ui::sis::draw_message(cx.kernel, width, &message, -1, -1, DSME_NONE as u16);
            let planet = crate::graphics::context_abi::ContextFrame::retain(
                std::ptr::NonNull::new(cx.kernel.planet[0]),
            );
            context.draw_stamp(
                &planet,
                POINT {
                    x: (dimensions.0 >> 1) as i16,
                    y: (dimensions.1 >> 1) as i16,
                },
            );
            draw_encounter_fleet(cx.world, &context, dimensions);
        });
        drop(segue);
        cx.with_legacy(|| context.transition(None));
    }

    #[cfg(not(test))]
    pub fn init_encounter(
        cx: &mut crate::engine::battle::Cx<'_>,
        io: &mut crate::engine::observe::Io<'_>,
    ) -> COUNT {
        prepare_encounter(cx);
        let mut menu = EncounterSelection::new();
        crate::ui::menu::draw_menu_state(
            cx.kernel,
            cx.world,
            cx.screen_dimensions(),
            PM_CONVERSE as u8,
            menu.choice as i16,
        );
        crate::ui::sis::set_menu_flash(cx.kernel, cx.screen_dimensions(), false);
        cx.run_input(io, true, |cx, io| do_select_action(cx, io, &mut menu));
        cx.kernel.sis_highlight.context.borrow_mut().take();
        COUNT::from(menu.choice)
    }

    #[cfg(not(test))]
    pub fn uninit_encounter(
        cx: &mut crate::engine::battle::Cx<'_>,
        io: &mut crate::engine::observe::Io<'_>,
        input: &crate::engine::input::InputSession,
    ) -> COUNT {
        free_gravity_well(cx.kernel);
        if encounter_has_no_salvage(cx.world) {
            return 0;
        }
        if cx.world.story_state(StoryKey::BATTLE_SEGUE) == 0 {
            cx.kernel.ships(0).reinit();
            cx.kernel.ships(1).reinit();
            return 0;
        }
        cx.world.set_story_state(StoryKey::BATTLE_SEGUE, 0);
        cx.world.set_story_state(StoryKey::BOMB_CARRIER, 0);
        let mut victory = victory_state(
            cx.b.counters[1],
            cx.b.counters[0],
            cx.world.story_state(StoryKey::URQUAN_PROTECTING_SAMATRA) != 0,
        );
        cx.world.encounter_race = npc_race(cx.world);
        if cx
            .world
            .avail_races_ref()
            .at(cx.world.encounter_race as usize)
            .is_none()
        {
            victory = -1;
            init_siscontexts(cx.kernel, cx.screen_dimensions());
        }
        let mut salvage = salvage::Salvage::new(cx.kernel, victory);
        let previous_mode = cx.kernel.set_status_message_mode(StatMsgMode_SMM_RES_UNITS);
        let mut killed = 0u16;
        for side in 0..NUM_SIDES as usize {
            if side != 0 {
                if salvage.victory < 0 {
                    salvage.victory = 0;
                } else {
                    salvage.panel(cx, draw_armada);
                }
            }
            cx.kernel.ships(NUM_SIDES as usize - 1 - side).reinit();
            for (id, frag) in fragment_snapshot(cx.world, side == 0) {
                if fragment_is_removed(
                    frag.crew_level,
                    salvage.victory != 0,
                    side == NUM_SIDES as usize - 1,
                ) {
                    if side == NUM_SIDES as usize - 1 {
                        killed = killed.wrapping_add(1);
                        if salvage.victory != 0 {
                            if killed == 1 {
                                salvage.start(cx, input, &frag);
                            }
                            salvage.reward(cx, input);
                        }
                    }
                    remove_campaign_fragment(cx.world, side == 0, id);
                }
            }
        }
        cx.kernel.set_status_message_mode(previous_mode);
        salvage.finish(cx, io, input, |cx, io, duration| {
            crate::ui::util::live::wait_for_any_button(cx, io, 1, duration, 0);
        });
        apply_thraddash_body_count(cx.world, killed)
    }

    #[cfg(not(test))]
    pub fn encounter_battle(
        cx: &mut crate::engine::battle::Cx<'_>,
        io: &mut crate::engine::observe::Io<'_>,
    ) {
        cx.world.set_story_state(StoryKey::BATTLE_SEGUE, 1);
        let old = cx.world.activity();
        cx.world.set_activity(if old as u8 == IN_LAST_BATTLE as u8 {
            IN_LAST_BATTLE as u16
        } else {
            IN_ENCOUNTER as u16
        });
        cx.with_legacy(|| WaitForSoundEnd(COUNT::MAX));
        let mut saved = None;
        if cx.world.data.game.glob_flags & CYBORG_ENABLED as u8 != 0 {
            cx.b.nth_frame = 1 | ((combat_speed(cx.world.data.game.glob_flags) as u16) << 8);
            cx.kernel.player_control[0] = CYBORG_CONTROL | AWESOME_RATING;
            saved = cx.kernel.replace_player_input(0, None);
            cx.kernel.install_player_input(0);
        }
        cx.kernel.game_sounds = crate::sound::owned_bank::LoadedSoundBank::load(c"sounds.battle");
        battle(cx, io, None);
        cx.kernel.game_sounds = None;
        if cx.world.activity() & CHECK_ABORT as u16 != 0 {
            cx.world.data.flagship.crew_enlisted = COUNT::MAX;
        }
        if cx.world.data.game.glob_flags & CYBORG_ENABLED as u8 != 0 {
            cx.b.nth_frame = 0;
            cx.kernel.player_control[0] = HUMAN_CONTROL | STANDARD_RATING;
            cx.kernel.clear_player_input(0);
            cx.kernel.replace_player_input(0, saved);
        }
        cx.world.set_activity(old);
    }
    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn fragment_snapshots_keep_order_and_removal_selects_the_supplied_fleet() {
            let mut world = crate::engine::World::new();
            let npc = {
                let mut queue = world.npc_built_ships();
                let id = queue
                    .alloc()
                    .unwrap_or_else(|| panic!("fragment fixture queue is full"));
                queue.get_mut(id).race_id = 9;
                queue.get_mut(id).crew_level = COUNT::MAX;
                queue.put(id);
                id
            };
            let escort = {
                let mut queue = world.built_ships();
                let id = queue
                    .alloc()
                    .unwrap_or_else(|| panic!("fragment fixture queue is full"));
                queue.get_mut(id).race_id = 4;
                queue.get_mut(id).crew_level = 3;
                queue.put(id);
                id
            };
            assert_eq!(npc_race(&world), 9);
            assert_eq!(fragment_snapshot(&world, false)[0].0, npc);
            assert_eq!(fragment_snapshot(&world, true)[0].0, escort);
            assert_eq!(fragment_snapshot(&world, true)[0].1.crew_level, 3);
            assert_ne!(fleet_is_infinite(&world, 1), 0);
            assert_eq!(fleet_is_infinite(&world, RPG_PLAYER_NUM as COUNT), 0);
            assert!(fragment_is_removed(0, false, false));
            remove_campaign_fragment(&mut world, false, npc);
            assert!(fragment_snapshot(&world, false).is_empty());
            assert_eq!(fragment_snapshot(&world, true)[0].0, escort);
            assert_eq!(fleet_is_infinite(&world, 1), 0);
            remove_campaign_fragment(&mut world, true, escort);
            assert!(fragment_snapshot(&world, true).is_empty());
        }

        #[test]
        fn encounter_escape_reads_the_selected_player_template() {
            let input = crate::engine::input::InputSession::default();
            input.set_player_control(0, 2);
            assert!(!escape_pressed(&input));
            input.set_player_key(0, KEY_ESCAPE as usize, true);
            input.advance(input.immediate(), input.immediate(), 1);
            assert!(escape_pressed(&input));
            input.set_player_control(0, 0);
            assert!(!escape_pressed(&input));
        }

        #[test]
        fn salvage_fades_keep_their_authored_envelopes() {
            assert_eq!(TEXT_FADE, [10, 9, 8, 6, 5, 4, 3]);
            assert_eq!(SHIP_FADE.len(), 10);
            assert_eq!(SHIP_FADE[0], (7, 0, 0));
            assert_eq!(SHIP_FADE[5], (31, 31, 31));
            assert_eq!(SHIP_FADE[9], (23, 0, 0));
        }
    }
}

#[cfg(not(test))]
pub use engine::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encounter_selection_ignores_initial_select_then_preserves_hail_attack_and_options() {
        for choice in 0..=2 {
            let mut menu = EncounterSelection::new();
            menu.choice = choice;
            assert_eq!(menu.advance(false, true, false), SelectionAction::Continue);
            assert_eq!(menu.choice, choice);
            assert_eq!(
                menu.advance(false, true, false),
                if choice == 2 {
                    SelectionAction::Options
                } else {
                    SelectionAction::Finish
                }
            );
        }
        for choice in 0..=1 {
            let mut menu = EncounterSelection::new();
            menu.choice = choice;
            menu.advance(false, false, true);
            assert_eq!(menu.advance(false, true, true), SelectionAction::Finish);
            assert_eq!(menu.choice, 0);
        }
        let mut menu = EncounterSelection::new();
        assert_eq!(menu.advance(true, false, false), SelectionAction::Finish);
        assert_eq!(menu.choice, 2);
    }

    #[test]
    fn salvage_requires_surviving_player_without_samatra_protection() {
        assert_eq!(victory_state(0, 1, false), 1);
        assert_eq!(victory_state(0, 0, false), 0);
        assert_eq!(victory_state(1, 1, false), 0);
        assert_eq!(victory_state(0, 1, true), 0);
        assert!(fragment_is_removed(0, false, false));
        assert!(fragment_is_removed(3, true, true));
        assert!(!fragment_is_removed(3, true, false));
    }
    #[test]
    fn retained_world_controls_salvage_and_thraddash_accounting() {
        use crate::engine::StoryKey;
        use crate::ships::native::abi::{CHECK_ABORT, IN_LAST_BATTLE, THRADDASH_SHIP};
        let mut world = crate::engine::World::new();
        assert!(!encounter_has_no_salvage(&world));
        world.set_activity(CHECK_ABORT as u16);
        assert!(encounter_has_no_salvage(&world));
        world.set_activity(IN_LAST_BATTLE as u16);
        assert!(encounter_has_no_salvage(&world));
        world.set_activity(0);
        world.data.flagship.crew_enlisted = u16::MAX;
        assert!(encounter_has_no_salvage(&world));
        world.story = Some(crate::engine::world::story::StoryState::new());
        world.encounter_race = THRADDASH_SHIP as i16;
        world.set_story_state(StoryKey::THRADDASH_BODY_COUNT, 23);
        assert_eq!(apply_thraddash_body_count(&mut world, 4), 25);
        assert_eq!(world.story_state(StoryKey::THRADDASH_BODY_COUNT), 25);
        world.set_story_state(StoryKey::THRADD_MANNER, 1);
        assert_eq!(apply_thraddash_body_count(&mut world, 2), 2);
        assert_eq!(world.story_state(StoryKey::THRADDASH_BODY_COUNT), 25);
    }
    #[test]
    fn body_count_preserves_count_assignment_before_threshold() {
        assert_eq!(thraddash_body_count(4, 23, 25), 25);
        assert_eq!(thraddash_body_count(2, 4, 25), 6);
        assert_eq!(thraddash_body_count(65535, 2, 25), 1);
    }
    #[test]
    fn cyborg_speed_has_exact_skip_values() {
        assert_eq!(combat_speed(0x40), 0);
        assert_eq!(combat_speed(0x80), 3);
        assert_eq!(combat_speed(0xc0), 255);
        assert_eq!(combat_speed(0), 255);
    }
    #[test]
    fn bounty_table_and_authored_icon_locations_are_unchanged() {
        assert_eq!(RACE_BOUNTY[8] >> 3, 125);
        assert_eq!(RACE_BOUNTY[16] >> 3, 550);
        assert_eq!(ENCOUNTER_DISPLAY_POINTS[9], (-44, -27));
    }
}
