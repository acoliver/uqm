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

#[cfg(not(test))]
pub(crate) mod engine {
    use super::*;
    use crate::battle::battle_native::battle;
    use crate::battle::battle_native::engine::{
        rgb15, sis_height, sis_width, AWESOME_RATING, CYBORG_CONTROL, HUMAN_CONTROL,
        STANDARD_RATING,
    };
    use crate::engine::battle::link_queue;
    use crate::engine::StoryKey;
    use crate::ships::native::abi::*;
    use crate::ships::native::{cosine, sine};
    use std::{mem, ptr};

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

    use crate::graphics::resgfx::LoadGraphicInstance;

    use crate::planets::starmap::{current_catalog, naming::cluster_name};

    use crate::mainloop::input_abi::live::AnyButtonPress;

    unsafe fn escape_pressed(input: &crate::engine::input::InputSession) -> bool {
        let template = input.player_controls()[0] as usize;
        input.current().key[template][KEY_ESCAPE as usize] != 0
    }
    enum SalvageText {
        Caption,
        Amount,
        Total,
    }

    fn draw_salvage_line(
        kernel: &crate::engine::Kernel,
        string: &std::ffi::CStr,
        baseline: POINT,
        style: SalvageText,
    ) {
        let context = crate::graphics::context_abi::ContextLease::retain(
            std::ptr::NonNull::new(kernel.space_context)
                .unwrap_or_else(|| panic!("salvage requires a space context")),
        );
        let (font, align, color) = match style {
            SalvageText::Caption => (
                kernel.tiny_font_lease(),
                TEXT_ALIGN_ALIGN_CENTER,
                rgb15(8, 8, 8),
            ),
            SalvageText::Amount => (
                kernel.micro_font_lease(),
                TEXT_ALIGN_ALIGN_RIGHT,
                rgb15(0, 0, 24),
            ),
            SalvageText::Total => (
                kernel.micro_font_lease(),
                TEXT_ALIGN_ALIGN_CENTER,
                rgb15(0, 0, 24),
            ),
        };
        let font = font.unwrap_or_else(|| panic!("salvage requires its retained text font"));
        context.draw_text(&font, string, baseline, align, color);
    }

    fn draw_salvage_captions(kernel: &crate::engine::Kernel, name: &[UNICODE; 80], rect: &RECT) {
        let baseline = POINT {
            x: rect.corner.x.wrapping_add(100),
            y: rect.corner.y.wrapping_add(68),
        };
        let bytes = name.map(|byte| byte as u8);
        let name = std::ffi::CStr::from_bytes_until_nul(&bytes)
            .unwrap_or_else(|error| panic!("unterminated salvage name: {error}"));
        draw_salvage_line(kernel, name, baseline, SalvageText::Caption);
        draw_salvage_line(
            kernel,
            kernel.game_text((ENCOUNTER_STRING_BASE + 3) as usize),
            POINT {
                y: baseline.y.wrapping_add(6),
                ..baseline
            },
            SalvageText::Caption,
        );
    }

    unsafe fn do_select_action(
        cx: &mut crate::engine::battle::Cx<'_>,
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
                if cx.with_legacy(|| GameOptions()) == 0 {
                    return false;
                }
                cx.with_legacy(|| {
                    DrawMenuStateStrings(PM_CONVERSE as u8, menu.choice as i16);
                    SetFlashRect(usize::MAX as *const RECT);
                });
            }
            SelectionAction::Unknown(choice) => {
                printf(c"Unknown option: %d\n".as_ptr(), i32::from(choice));
            }
            SelectionAction::Continue => {}
        }
        let flags = cx.world.data.game.glob_flags;
        cx.with_legacy(|| {
            crate::ui::menu::choose_menu_state(input, &mut menu.choice, PM_CONVERSE as u8, flags);
        });
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

    /// # Safety
    /// `player` must be a valid side; fleet, ship queues, and encounter resources must be initialized on the game thread.
    pub unsafe fn build_battle(
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
                ships.set_picker_resources(id, frag.race_strings, frag.icons);
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

    pub(crate) use crate::battle::context_access::reader_legacy::update_ship_frag_crew;

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

    /// # Safety
    /// The game thread must own initialized encounter, display, input, resource, and audio state.
    unsafe fn prepare_encounter(cx: &mut crate::engine::battle::Cx<'_>) {
        SetContext(cx.kernel.space_context);
        SetContextFont(cx.kernel.tiny_font);
        let music = crate::sound::resource_types::LoadedMusic::load(c"music.redalert");
        crate::ui::settings::play_loaded_music(cx.world, music.as_ref(), 0, 1);
        let segue = CaptureDrawable(LoadGraphicInstance(SEGUE_PMAP_ANIM.as_ptr().cast()).cast());
        cx.with_legacy(|| WaitForSoundEnd(COUNT::MAX));
        stop_music();
        drop(music);
        let mut stamp = STAMP {
            origin: POINT { x: 0, y: 0 },
            frame: segue,
        };
        SetTransitionSource(ptr::null());
        BatchGraphics();
        SetContextBackGroundColor(rgb15(0, 0, 0));
        ClearDrawable();
        DrawStamp(&mut stamp);
        let cluster = if in_hqspace(cx.world) == 0 {
            let star = cx
                .world
                .selected_star(current_catalog())
                .unwrap_or_else(|| panic!("encounter requires a selected star"));
            Some(cluster_name(cx.kernel, star, STAR_NUMBER_BASE as usize))
        } else {
            None
        };
        draw_encounter_location(cx, cluster.as_deref());
        cx.with_legacy(|| DrawSISMessage(ptr::null()));
        stamp.origin = POINT {
            x: (sis_width() >> 1) as i16,
            y: (sis_height() >> 1) as i16,
        };
        stamp.frame = cx.kernel.planet[0];
        DrawStamp(&mut stamp);
        if cx.world.activity() as u8 != IN_LAST_BATTLE as u8 {
            let fragments = fragment_snapshot(cx.world, false);
            let mut index = 0;
            let mut i = 0usize;
            while index < fragments.len() && i < 60 {
                let frag = &fragments[index].1;
                if frag.crew_level != COUNT::MAX {
                    index += 1;
                }
                let (x, y) = ENCOUNTER_DISPLAY_POINTS[i % ENCOUNTER_DISPLAY_POINTS.len()];
                stamp.origin = POINT { x, y };
                if i >= ENCOUNTER_DISPLAY_POINTS.len() {
                    let radius = (square_root((x as i64 * x as i64 + y as i64 * y as i64) as u32)
                        as usize
                        + (i / ENCOUNTER_DISPLAY_POINTS.len()) * 18)
                        as COUNT;
                    let angle = ARCTAN(x, y);
                    stamp.origin.x = cosine(angle as i32, radius as i32);
                    stamp.origin.y = sine(angle as i32, radius as i32);
                }
                stamp.frame = SetAbsFrameIndex(frag.icons, 0);
                let mut rect: RECT = mem::zeroed();
                GetFrameRect(stamp.frame, &mut rect);
                stamp.origin.x = (stamp.origin.x as i32 + (sis_width() >> 1)
                    - (rect.extent.width as i32 >> 1)) as i16;
                stamp.origin.y = (stamp.origin.y as i32 + (sis_height() >> 1)
                    - (rect.extent.height as i32 >> 1)) as i16;
                DrawStamp(&mut stamp);
                i += 1;
            }
        }
        UnbatchGraphics();
        DestroyDrawable(ReleaseDrawable(segue));
        cx.with_legacy(|| ScreenTransition(3, ptr::null()));
    }

    /// # Safety
    /// Native encounter resources and input are initialized on the game thread.
    pub unsafe fn init_encounter(
        cx: &mut crate::engine::battle::Cx<'_>,
        io: &mut crate::engine::observe::Io<'_>,
    ) -> COUNT {
        prepare_encounter(cx);
        let mut menu = EncounterSelection::new();
        cx.with_legacy(|| {
            DrawMenuStateStrings(PM_CONVERSE as u8, menu.choice as i16);
            SetFlashRect(usize::MAX as *const RECT);
        });
        cx.run_input(io, true, |cx, _| do_select_action(cx, &mut menu));
        cx.with_legacy(|| SetFlashRect(ptr::null()));
        COUNT::from(menu.choice)
    }

    unsafe fn draw_fade_text(
        cx: &mut crate::engine::battle::Cx<'_>,
        input: &crate::engine::input::InputSession,
        str1: &std::ffi::CStr,
        str2: &std::ffi::CStr,
        fade_in: BOOLEAN,
        rect: &RECT,
    ) {
        let context = crate::graphics::context_abi::ContextLease::retain(
            std::ptr::NonNull::new(cx.kernel.space_context)
                .unwrap_or_else(|| panic!("salvage requires a space context")),
        );
        let font = cx
            .kernel
            .micro_font_lease()
            .unwrap_or_else(|| panic!("salvage requires the micro font"));
        let baseline = POINT {
            x: rect.corner.x.wrapping_add(100),
            y: rect.corner.y.wrapping_add(45),
        };
        let render = |color| {
            context.draw_text(&font, str1, baseline, TEXT_ALIGN_ALIGN_CENTER, color);
            context.draw_text(
                &font,
                str2,
                POINT {
                    y: baseline.y.wrapping_add(11),
                    ..baseline
                },
                TEXT_ALIGN_ALIGN_CENTER,
                color,
            );
        };
        FlushInput(input);
        let mut time = GetTimeCounter();
        if fade_in != 0 {
            let mut i = 0usize;
            while i < TEXT_FADE.len() {
                if AnyButtonPress(input, 1) != 0 {
                    i = TEXT_FADE.len() - 1;
                }
                let v = TEXT_FADE[i];
                render(rgb15(v, v, v));
                cx.with_legacy(|| SleepThreadUntil(time.wrapping_add(ONE_SECOND / 20)));
                time = GetTimeCounter();
                i += 1;
            }
        } else {
            let mut i = TEXT_FADE.len() as i16 - 1;
            while i >= 0 {
                if AnyButtonPress(input, 1) != 0 {
                    i = 0;
                }
                let v = TEXT_FADE[i as usize];
                render(rgb15(v, v, v));
                cx.with_legacy(|| SleepThreadUntil(time.wrapping_add(ONE_SECOND / 20)));
                time = GetTimeCounter();
                i -= 1;
            }
            render(rgb15(10, 10, 10));
        }
    }

    /// # Safety
    /// The game thread must own initialized encounter fleet queues, game state, display, and salvage resources.
    pub unsafe fn uninit_encounter(cx: &mut crate::engine::battle::Cx<'_>, input: &crate::engine::input::InputSession) -> COUNT {
        let mut killed = 0u16;
        free_gravity_well(cx.kernel);
        if encounter_has_no_salvage(cx.world) {
            return killed;
        }
        if cx.world.story_state(StoryKey::BATTLE_SEGUE) == 0 {
            cx.kernel.ships(0).reinit();
            cx.kernel.ships(1).reinit();
            return killed;
        }
        let mut recycle = 0u16;
        let mut rect: RECT = mem::zeroed();
        let mut salvage_rect: RECT = mem::zeroed();
        let mut stamp: STAMP = mem::zeroed();
        let mut str1 = std::ffi::CString::default();
        let mut str2 = std::ffi::CString::default();
        let mut buf = [0 as UNICODE; 80];
        cx.world.set_story_state(StoryKey::BATTLE_SEGUE, 0);
        cx.world.set_story_state(StoryKey::BOMB_CARRIER, 0);
        let counters = cx.b.counters;
        let mut victory = victory_state(
            counters[1],
            counters[0],
            cx.world.story_state(StoryKey::URQUAN_PROTECTING_SAMATRA) != 0,
        );
        cx.world.encounter_race = npc_race(cx.world);
        let race_index = cx.world.encounter_race as usize;
        if cx.world.avail_races_ref().at(race_index).is_none() {
            victory = -1;
            init_siscontexts(cx.kernel);
        }
        let previous_mode = SetStatusMessageMode(StatMsgMode_SMM_RES_UNITS);
        let mut sleepy = true;
        for side in 0..NUM_SIDES as usize {
            if side != 0 {
                if victory < 0 {
                    victory = 0;
                } else {
                    cx.with_legacy(|| {
                        draw_sisframe();
                        DrawSISMessage(ptr::null());
                    });
                    if in_hqspace(cx.world) != 0 {
                        let point = cx.world.data.game.ship_stamp.origin;
                        cx.with_legacy(|| DrawHyperCoords(point));
                    } else if cx.world.data.game.ip_planet == 0 {
                        let point = (*cx.world.star_pointer(current_catalog())).star_pt;
                        cx.with_legacy(|| DrawHyperCoords(point));
                    } else {
                        let name = cx.world.data.flagship.planet_name.as_mut_ptr().cast();
                        cx.with_legacy(|| DrawSISTitle(name));
                    }
                    SetContext(cx.kernel.space_context);
                    if victory != 0 {
                        cx.with_legacy(|| DrawArmadaPickShip(1, &mut salvage_rect));
                    }
                }
            }
            cx.kernel.ships(NUM_SIDES as usize - 1 - side).reinit();
            let fragments = fragment_snapshot(cx.world, side == 0);
            for (id, frag) in fragments {
                if fragment_is_removed(
                    frag.crew_level,
                    victory != 0,
                    side == NUM_SIDES as usize - 1,
                ) {
                    if side == NUM_SIDES as usize - 1 {
                        killed = killed.wrapping_add(1);
                        if victory != 0 {
                            if killed == 1 {
                                recycle = 0;
                                cx.with_legacy(|| DrawStatusMessage(ptr::null()));
                                let kernel = &*cx.kernel;
                                stamp.origin.x = salvage_rect.corner.x.wrapping_add(32);
                                stamp.origin.y = salvage_rect.corner.y.wrapping_add(56);
                                stamp.frame = IncFrameIndex(frag.icons);
                                DrawStamp(&mut stamp);
                                SetContextForeGroundColor(rgb15(8, 8, 8));
                                SetContextFont(kernel.tiny_font);
                                let name = std::ffi::CStr::from_ptr(
                                    GetStringAddress(frag.race_strings).cast(),
                                );
                                crate::ships::native::copy_display_name(&mut buf, name.to_bytes());
                                strupr(buf.as_mut_ptr());
                                draw_salvage_captions(kernel, &buf, &salvage_rect);

                                stamp.frame = frag.icons;
                                SetContextFont(kernel.micro_font);
                                str1 = kernel
                                    .game_text((ENCOUNTER_STRING_BASE + 4) as usize)
                                    .to_owned();
                                str2 = kernel
                                    .game_text((ENCOUNTER_STRING_BASE + 5) as usize)
                                    .to_owned();
                                draw_fade_text(cx, input, &str1, &str2, 1, &salvage_rect);
                            }
                            rect.corner.y = salvage_rect.corner.y.wrapping_add(9);
                            rect.extent.height = 22;
                            SetContextForeGroundColor(rgb15(0, 0, 0));
                            rect.extent.width = 34;
                            rect.corner.x = (salvage_rect.corner.x as i32
                                + salvage_rect.extent.width as i32
                                - (10 + rect.extent.width as i32))
                                as i16;
                            DrawFilledRectangle(&mut rect);
                            let bounty = RACE_BOUNTY[cx.world.encounter_race as usize] >> 3;
                            recycle = recycle.wrapping_add(bounty);
                            let amount = std::ffi::CString::new(recycle.to_string())
                                .unwrap_or_else(|error| panic!("invalid salvage amount: {error}"));
                            SetContextForeGroundColor(rgb15(0, 0, 24));
                            draw_salvage_line(
                                cx.kernel,
                                &amount,
                                POINT {
                                    x: (rect.corner.x as i32 + rect.extent.width as i32 - 1) as i16,
                                    y: rect.corner.y.wrapping_add(14),
                                },
                                SalvageText::Amount,
                            );
                            cx.with_legacy(|| DeltaSISGauges(0, 0, bounty as i32));
                            let before = victory;
                            victory = victory.wrapping_add(1);
                            if (before - 1) % 5 != 0 {
                                stamp.origin.x = stamp.origin.x.wrapping_add(17);
                            } else {
                                SetContextForeGroundColor(rgb15(0, 0, 0));
                                rect.corner.x = salvage_rect.corner.x.wrapping_add(10);
                                rect.extent.width = 104;
                                DrawFilledRectangle(&mut rect);
                                stamp.origin.x = rect.corner.x.wrapping_add(2);
                                stamp.origin.y = salvage_rect.corner.y.wrapping_add(12);
                            }
                            if sleepy {
                                let mut time = GetTimeCounter();
                                for (r, g, b) in SHIP_FADE {
                                    sleepy = AnyButtonPress(input, 1) == 0
                                        && cx.world.activity() & CHECK_ABORT as u16 == 0;
                                    if !sleepy {
                                        break;
                                    }
                                    SetContextForeGroundColor(rgb15(r, g, b));
                                    DrawFilledStamp(&mut stamp);
                                    cx.with_legacy(|| {
                                        SleepThreadUntil(time.wrapping_add(ONE_SECOND / 15))
                                    });
                                    time = GetTimeCounter();
                                }
                            }
                            DrawStamp(&mut stamp);
                        }
                    }
                    remove_campaign_fragment(cx.world, side == 0, id);
                }
            }
        }
        SetStatusMessageMode(previous_mode);
        if victory != 0 {
            cx.with_legacy(|| WaitForAnyButton(1, ONE_SECOND * 3, 0));
            if !escape_pressed(input) {
                draw_fade_text(cx, input, &str1, &str2, 0, &salvage_rect);
                if !escape_pressed(input) {
                    SetContextForeGroundColor(rgb15(0, 0, 0));
                    rect.corner.x = salvage_rect.corner.x.wrapping_add(10);
                    rect.extent.width = 132;
                    DrawFilledRectangle(&mut rect);
                    let mut total = format!("{recycle} ").into_bytes();
                    total.extend_from_slice(
                        cx.kernel
                            .game_text((STATUS_STRING_BASE + 1) as usize)
                            .to_bytes(),
                    );
                    let total = std::ffi::CString::new(total)
                        .unwrap_or_else(|error| panic!("invalid salvage total: {error}"));
                    SetContextForeGroundColor(rgb15(0, 0, 24));
                    draw_salvage_line(
                        cx.kernel,
                        &total,
                        POINT {
                            x: (rect.corner.x as i32 + (rect.extent.width as i32 >> 1)) as i16,
                            y: rect.corner.y.wrapping_add(14),
                        },
                        SalvageText::Total,
                    );
                    str1 = cx
                        .kernel
                        .game_text((ENCOUNTER_STRING_BASE + 6) as usize)
                        .to_owned();
                    str2 = cx
                        .kernel
                        .game_text((ENCOUNTER_STRING_BASE + 7) as usize)
                        .to_owned();
                    draw_fade_text(cx, input, &str1, &str2, 1, &salvage_rect);
                    cx.with_legacy(|| WaitForAnyButton(1, ONE_SECOND * 2, 0));
                    if !escape_pressed(input) {
                        draw_fade_text(cx, input, &str1, &str2, 0, &salvage_rect);
                    }
                }
            }
            cx.with_legacy(|| DrawStatusMessage(ptr::null()));
        }
        killed = apply_thraddash_body_count(cx.world, killed);
        killed
    }

    /// # Safety
    /// The game thread must retain initialized native encounter audio resources.
    pub unsafe fn encounter_battle(
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
            if !SetPlayerInput(cx.kernel, 0) {
                crate::log_abi::log_message(1, "Could not set cyborg player input.");
                if cfg!(feature = "debug-process") {
                    std::process::abort();
                } else {
                    std::process::exit(1);
                }
            }
        }
        cx.kernel.game_sounds = cx.with_legacy(|| {
            crate::strings::string_table::CaptureStringTable(
                crate::sound::resource_types::LoadSoundInstance(GAME_SOUNDS.as_ptr().cast()).cast(),
            )
            .cast()
        });
        battle(cx, io, None);
        DestroySound(ReleaseStringTable(cx.kernel.game_sounds));
        cx.kernel.game_sounds = ptr::null_mut();
        if cx.world.activity() & CHECK_ABORT as u16 != 0 {
            cx.world.data.flagship.crew_enlisted = COUNT::MAX;
        }
        if cx.world.data.game.glob_flags & CYBORG_ENABLED as u8 != 0 {
            cx.b.nth_frame = 0;
            cx.kernel.player_control[0] = HUMAN_CONTROL | STANDARD_RATING;
            ClearPlayerInput(cx.kernel, 0);
            cx.kernel.replace_player_input(0, saved);
        }
        cx.world.set_activity(old);
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
        world.story = Some(crate::engine::world::story::GameState::new());
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
