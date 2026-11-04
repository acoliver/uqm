//! Encounter fleet construction, decision screen, combat entry, and salvage.

#[cfg(not(test))]
use crate::engine::{Kernel as KernelOwner, World as WorldOwner, WorldData as WorldFields};
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

#[cfg(not(test))]
use crate::ships::native::abi::{COUNT, POINT, TEXT, TEXT_ALIGN, UNICODE};
#[cfg(not(test))]
fn text(x: i16, y: i16, align: TEXT_ALIGN, string: *const UNICODE) -> TEXT {
    TEXT {
        baseline: POINT { x, y },
        pStr: string,
        align,
        CharCount: COUNT::MAX,
    }
}

#[cfg(not(test))]
pub(crate) mod engine {
    use super::*;
    use crate::battle::battle_native::battle;
    use crate::battle::battle_native::engine::{
        activity, game_state, rgb15, set_activity, set_game_state, sis_height, sis_width,
        AWESOME_RATING, CYBORG_CONTROL, HUMAN_CONTROL, STANDARD_RATING,
    };
    use crate::engine::battle::link_queue;
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

    use crate::planets::starmap::naming::GetClusterName;

    use crate::mainloop::input_abi::live::AnyButtonPress;

    unsafe fn game_string(index: u32) -> *const UNICODE {
        GetStringAddress(SetAbsStringTableIndex(
            KernelOwner::current().read(|owner| owner.game_strings),
            index as COUNT,
        ))
        .cast()
    }
    unsafe fn planet_name() -> *mut UNICODE {
        let world = WorldFields::current();
        (&raw mut (**world).flagship.planet_name).cast()
    }
    unsafe fn escape_pressed() -> bool {
        let template = *(&raw const PlayerControls).cast::<CONTROL_TEMPLATE>();
        CurrentInputState.key[template as usize][KEY_ESCAPE as usize] != 0
    }
    unsafe fn draw_text(t: &mut TEXT, string: *const UNICODE) {
        t.pStr = string;
        t.CharCount = COUNT::MAX;
        font_DrawText(t);
    }

    unsafe fn do_select_action(
        cx: &mut crate::engine::battle::Cx<'_>,
        menu: &mut EncounterSelection,
    ) -> bool {
        SetMenuSounds(0x0f, 1 << 4);
        match menu.advance(
            cx.world.activity() & CHECK_ABORT as u16 != 0,
            PulsedInputState.menu[KEY_MENU_SELECT as usize] != 0,
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
            crate::ui::menu::choose_menu_state(&mut menu.choice, PM_CONVERSE as u8, flags);
        });
        true
    }

    unsafe fn encounter_star() -> *mut crate::engine::world::CStarDesc {
        WorldOwner::current()
            .read(|world| world.star_pointer(crate::planets::starmap::current_catalog()))
    }

    unsafe fn encounter_race_exists(race: COUNT) -> bool {
        WorldOwner::current().read(|world| world.avail_races_ref().at(usize::from(race)).is_some())
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
    #[no_mangle]
    pub unsafe extern "C" fn BuildBattle(player: COUNT) {
        EncounterRace = -1;
        if WorldOwner::current().read(|world| world.npc_built_ships_ref().head().is_none()) {
            set_game_state(BATTLE_SEGUE, END_BATTLE_SEGUE, 0);
            return;
        }
        if player != RPG_PLAYER_NUM as u16 {
            match activity() as u8 as u32 {
                IN_LAST_BATTLE => KernelOwner::current()
                    .write(|kernel| load_gravity_well(kernel, NUMBER_OF_PLANET_TYPES as u8)),
                IN_HYPERSPACE => KernelOwner::current().write(|kernel| {
                    load_gravity_well(
                        kernel,
                        ((TFB_Random() as COUNT) % NUMBER_OF_PLANET_TYPES as COUNT) as u8,
                    )
                }),
                _ => {
                    set_game_state(ESCAPE_COUNTER, END_ESCAPE_COUNTER, 110);
                    KernelOwner::current().write(|kernel| {
                        load_gravity_well(kernel, game_state(BATTLE_PLANET, END_BATTLE_PLANET))
                    });
                }
            }
        }
        link_queue::ships(player as usize).reinit();
        let fragments = WorldOwner::current()
            .read(|world| fragment_snapshot(world, player == RPG_PLAYER_NUM as COUNT));
        for (_, frag) in fragments {
            let built =
                link_queue::ships(player as usize).build(if frag.race_id == SAMATRA_SHIP as u8 {
                    SPECIES_ID_SA_MATRA_ID
                } else {
                    frag.species_id
                });
            if let Some(id) = built {
                let mut ships = link_queue::ships(player as usize);
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
            let built = link_queue::ships(0).build(SPECIES_ID_SIS_SHIP_ID);
            if let Some(id) = built {
                let mut ships = link_queue::ships(0);
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

    /// # Safety
    /// The game thread must own initialized encounter, display, input, resource, and audio state.
    unsafe fn prepare_encounter() {
        SetContext(KernelOwner::current().read(|owner| owner.space_context));
        SetContextFont(KernelOwner::current().read(|owner| owner.tiny_font));
        let music =
            crate::sound::resource_types::LoadMusicInstance(REDALERT_MUSIC.as_ptr().cast()).cast();
        PlayMusic(music, 0, 1);
        let segue = CaptureDrawable(LoadGraphicInstance(SEGUE_PMAP_ANIM.as_ptr().cast()).cast());
        WaitForSoundEnd(COUNT::MAX);
        stop_music();
        DestroyMusic(music);
        let mut stamp = STAMP {
            origin: POINT { x: 0, y: 0 },
            frame: segue,
        };
        SetTransitionSource(ptr::null());
        BatchGraphics();
        SetContextBackGroundColor(rgb15(0, 0, 0));
        ClearDrawable();
        DrawStamp(&mut stamp);
        let mut t = text(
            ((sis_width() >> 1) + 1) as i16,
            10,
            TEXT_ALIGN_ALIGN_CENTER,
            ptr::null(),
        );
        SetContextFont(KernelOwner::current().read(|owner| owner.micro_font));
        SetContextForeGroundColor(rgb15(0, 0, 20));
        if WorldOwner::current().read(in_hqspace) != 0 {
            draw_text(&mut t, game_string(ENCOUNTER_STRING_BASE));
            t.baseline.y += 12;
            draw_text(&mut t, game_string(ENCOUNTER_STRING_BASE + 1));
        } else {
            let mut buf = [0 as UNICODE; 256];
            draw_text(&mut t, game_string(ENCOUNTER_STRING_BASE + 2));
            t.baseline.y += 12;
            GetClusterName(encounter_star(), buf.as_mut_ptr());
            draw_text(&mut t, buf.as_ptr());
            t.baseline.y += 12;
            draw_text(&mut t, planet_name());
        }
        DrawSISMessage(ptr::null());
        stamp.origin = POINT {
            x: (sis_width() >> 1) as i16,
            y: (sis_height() >> 1) as i16,
        };
        stamp.frame = KernelOwner::current().read(|owner| owner.planet[0]);
        DrawStamp(&mut stamp);
        if activity() as u8 != IN_LAST_BATTLE as u8 {
            let fragments = WorldOwner::current().read(|world| fragment_snapshot(world, false));
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
        ScreenTransition(3, ptr::null());
    }

    /// # Safety
    /// Native encounter resources and input are initialized on the game thread.
    pub unsafe fn init_encounter(
        cx: &mut crate::engine::battle::Cx<'_>,
        io: &mut crate::engine::observe::Io<'_>,
    ) -> COUNT {
        cx.with_legacy(|| prepare_encounter());
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
        str1: *const UNICODE,
        str2: *const UNICODE,
        fade_in: BOOLEAN,
        rect: *mut RECT,
    ) {
        let mut t1 = text(
            (*rect).corner.x.wrapping_add(100),
            (*rect).corner.y.wrapping_add(45),
            TEXT_ALIGN_ALIGN_CENTER,
            str1,
        );
        let mut t2 = t1;
        t2.baseline.y = t2.baseline.y.wrapping_add(11);
        t2.pStr = str2;
        FlushInput();
        let mut time = GetTimeCounter();
        if fade_in != 0 {
            let mut i = 0usize;
            while i < TEXT_FADE.len() {
                if AnyButtonPress(1) != 0 {
                    i = TEXT_FADE.len() - 1;
                }
                let v = TEXT_FADE[i];
                SetContextForeGroundColor(rgb15(v, v, v));
                font_DrawText(&mut t1);
                font_DrawText(&mut t2);
                SleepThreadUntil(time.wrapping_add(ONE_SECOND / 20));
                time = GetTimeCounter();
                i += 1;
            }
        } else {
            let mut i = TEXT_FADE.len() as i16 - 1;
            while i >= 0 {
                if AnyButtonPress(1) != 0 {
                    i = 0;
                }
                let v = TEXT_FADE[i as usize];
                SetContextForeGroundColor(rgb15(v, v, v));
                font_DrawText(&mut t1);
                font_DrawText(&mut t2);
                SleepThreadUntil(time.wrapping_add(ONE_SECOND / 20));
                time = GetTimeCounter();
                i -= 1;
            }
            SetContextForeGroundColor(rgb15(10, 10, 10));
            font_DrawText(&mut t1);
            font_DrawText(&mut t2);
        }
    }

    /// # Safety
    /// The game thread must own initialized encounter fleet queues, game state, display, and salvage resources.
    #[no_mangle]
    pub unsafe extern "C" fn UninitEncounter() -> COUNT {
        let mut killed = 0u16;
        KernelOwner::current().write(free_gravity_well);
        if activity() & (CHECK_ABORT | CHECK_LOAD) as u16 != 0
            || (**WorldFields::current()).flagship.crew_enlisted == COUNT::MAX
            || activity() as u8 == IN_LAST_BATTLE as u8
            || activity() as u8 == WON_LAST_BATTLE as u8
        {
            return killed;
        }
        if game_state(BATTLE_SEGUE, END_BATTLE_SEGUE) == 0 {
            link_queue::ships(0).reinit();
            link_queue::ships(1).reinit();
            return killed;
        }
        let mut recycle = 0u16;
        let mut rect: RECT = mem::zeroed();
        let mut salvage_rect: RECT = mem::zeroed();
        let mut t: TEXT;
        let mut stamp: STAMP = mem::zeroed();
        let mut str1 = ptr::null();
        let mut str2 = ptr::null();
        let mut buf = [0 as UNICODE; 80];
        set_game_state(BATTLE_SEGUE, END_BATTLE_SEGUE, 0);
        set_game_state(BOMB_CARRIER, END_BOMB_CARRIER, 0);
        let counters = crate::app::bridge::battle(|b| b.counters);
        let mut victory = victory_state(
            counters[1],
            counters[0],
            game_state(URQUAN_PROTECTING_SAMATRA, END_URQUAN_PROTECTING_SAMATRA) != 0,
        );
        EncounterRace = WorldOwner::current().read(npc_race);
        if !encounter_race_exists(EncounterRace as COUNT) {
            victory = -1;
            KernelOwner::current().write(|kernel| init_siscontexts(kernel));
        }
        let previous_mode = SetStatusMessageMode(StatMsgMode_SMM_RES_UNITS);
        let mut sleepy = true;
        for side in 0..NUM_SIDES as usize {
            if side != 0 {
                if victory < 0 {
                    victory = 0;
                } else {
                    draw_sisframe();
                    DrawSISMessage(ptr::null());
                    if WorldOwner::current().read(in_hqspace) != 0 {
                        DrawHyperCoords((**WorldFields::current()).game.ship_stamp.origin);
                    } else if (**WorldFields::current()).game.ip_planet == 0 {
                        DrawHyperCoords((*encounter_star()).star_pt);
                    } else {
                        DrawSISTitle(planet_name());
                    }
                    SetContext(KernelOwner::current().read(|owner| owner.space_context));
                    if victory != 0 {
                        DrawArmadaPickShip(1, &mut salvage_rect);
                    }
                }
            }
            link_queue::ships(NUM_SIDES as usize - 1 - side).reinit();
            let fragments = WorldOwner::current().read(|world| fragment_snapshot(world, side == 0));
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
                                DrawStatusMessage(ptr::null());
                                stamp.origin.x = salvage_rect.corner.x.wrapping_add(32);
                                stamp.origin.y = salvage_rect.corner.y.wrapping_add(56);
                                stamp.frame = IncFrameIndex(frag.icons);
                                DrawStamp(&mut stamp);
                                SetContextForeGroundColor(rgb15(8, 8, 8));
                                SetContextFont(
                                    KernelOwner::current().read(|owner| owner.tiny_font),
                                );
                                let name = std::ffi::CStr::from_ptr(
                                    GetStringAddress(frag.race_strings).cast(),
                                );
                                crate::ships::native::copy_display_name(&mut buf, name.to_bytes());
                                strupr(buf.as_mut_ptr());
                                t = text(
                                    salvage_rect.corner.x.wrapping_add(100),
                                    salvage_rect.corner.y.wrapping_add(68),
                                    TEXT_ALIGN_ALIGN_CENTER,
                                    buf.as_ptr(),
                                );
                                font_DrawText(&mut t);
                                t.baseline.y = t.baseline.y.wrapping_add(6);
                                draw_text(&mut t, game_string(ENCOUNTER_STRING_BASE + 3));
                                stamp.frame = frag.icons;
                                SetContextFont(
                                    KernelOwner::current().read(|owner| owner.micro_font),
                                );
                                str1 = game_string(ENCOUNTER_STRING_BASE + 4);
                                str2 = game_string(ENCOUNTER_STRING_BASE + 5);
                                draw_fade_text(str1, str2, 1, &mut salvage_rect);
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
                            let bounty = RACE_BOUNTY[EncounterRace as usize] >> 3;
                            recycle = recycle.wrapping_add(bounty);
                            sprintf(buf.as_mut_ptr(), c"%u".as_ptr(), recycle as u32);
                            t = text(
                                (rect.corner.x as i32 + rect.extent.width as i32 - 1) as i16,
                                rect.corner.y.wrapping_add(14),
                                TEXT_ALIGN_ALIGN_RIGHT,
                                buf.as_ptr(),
                            );
                            SetContextForeGroundColor(rgb15(0, 0, 24));
                            font_DrawText(&mut t);
                            DeltaSISGauges(0, 0, bounty as i32);
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
                                    sleepy = AnyButtonPress(1) == 0
                                        && activity() & CHECK_ABORT as u16 == 0;
                                    if !sleepy {
                                        break;
                                    }
                                    SetContextForeGroundColor(rgb15(r, g, b));
                                    DrawFilledStamp(&mut stamp);
                                    SleepThreadUntil(time.wrapping_add(ONE_SECOND / 15));
                                    time = GetTimeCounter();
                                }
                            }
                            DrawStamp(&mut stamp);
                        }
                    }
                    WorldOwner::current()
                        .write(|world| remove_campaign_fragment(world, side == 0, id));
                }
            }
        }
        SetStatusMessageMode(previous_mode);
        if victory != 0 {
            WaitForAnyButton(1, ONE_SECOND * 3, 0);
            if !escape_pressed() {
                draw_fade_text(str1, str2, 0, &mut salvage_rect);
                if !escape_pressed() {
                    SetContextForeGroundColor(rgb15(0, 0, 0));
                    rect.corner.x = salvage_rect.corner.x.wrapping_add(10);
                    rect.extent.width = 132;
                    DrawFilledRectangle(&mut rect);
                    sprintf(
                        buf.as_mut_ptr(),
                        c"%u %s".as_ptr(),
                        recycle as u32,
                        game_string(STATUS_STRING_BASE + 1),
                    );
                    t = text(
                        (rect.corner.x as i32 + (rect.extent.width as i32 >> 1)) as i16,
                        rect.corner.y.wrapping_add(14),
                        TEXT_ALIGN_ALIGN_CENTER,
                        buf.as_ptr(),
                    );
                    SetContextForeGroundColor(rgb15(0, 0, 24));
                    font_DrawText(&mut t);
                    str1 = game_string(ENCOUNTER_STRING_BASE + 6);
                    str2 = game_string(ENCOUNTER_STRING_BASE + 7);
                    draw_fade_text(str1, str2, 1, &mut salvage_rect);
                    WaitForAnyButton(1, ONE_SECOND * 2, 0);
                    if !escape_pressed() {
                        draw_fade_text(str1, str2, 0, &mut salvage_rect);
                    }
                }
            }
            DrawStatusMessage(ptr::null());
        }
        if killed != 0
            && EncounterRace == THRADDASH_SHIP as i16
            && game_state(THRADD_MANNER, END_THRADD_MANNER) == 0
        {
            killed = thraddash_body_count(
                killed,
                game_state(THRADDASH_BODY_COUNT, END_THRADDASH_BODY_COUNT) as u16,
                THRADDASH_BODY_THRESHOLD as u16,
            );
            set_game_state(THRADDASH_BODY_COUNT, END_THRADDASH_BODY_COUNT, killed as u8);
        }
        killed
    }

    /// # Safety
    /// The game thread must own initialized encounter, fleet, input, audio, and activity state.
    #[no_mangle]
    pub unsafe extern "C" fn EncounterBattle() {
        set_game_state(BATTLE_SEGUE, END_BATTLE_SEGUE, 1);
        let old = activity();
        set_activity(if old as u8 == IN_LAST_BATTLE as u8 {
            IN_LAST_BATTLE as u16
        } else {
            IN_ENCOUNTER as u16
        });
        WaitForSoundEnd(COUNT::MAX);
        let mut saved = None;
        if WorldFields::current().read(|owner| owner.game.glob_flags) & CYBORG_ENABLED as u8 != 0 {
            let speed = 1
                | ((combat_speed(WorldFields::current().read(|owner| owner.game.glob_flags))
                    as u16)
                    << 8);
            crate::app::bridge::battle(|b| b.nth_frame = speed);
            let installed = KernelOwner::current().write(|kernel| {
                kernel.player_control[0] = CYBORG_CONTROL | AWESOME_RATING;
                saved = kernel.replace_player_input(0, None);
                SetPlayerInput(kernel, 0)
            });
            if !installed {
                crate::log_abi::log_message(1, "Could not set cyborg player input.");
                // misc.h explode(): abort in DEBUG, exit(EXIT_FAILURE) otherwise.
                if cfg!(feature = "debug-process") {
                    std::process::abort();
                } else {
                    std::process::exit(1);
                }
            }
        }
        (**KernelOwner::current()).game_sounds = crate::strings::string_table::CaptureStringTable(
            crate::sound::resource_types::LoadSoundInstance(GAME_SOUNDS.as_ptr().cast()).cast(),
        )
        .cast();
        crate::app::bridge::enter(|cx| {
            let mut observer = cx.kernel.input_runtime().observer();
            let mut io = crate::engine::observe::Io {
                observer: observer.as_mut(),
            };
            battle(cx, &mut io, None)
        });
        DestroySound(ReleaseStringTable(
            KernelOwner::current().read(|owner| owner.game_sounds),
        ));
        (**KernelOwner::current()).game_sounds = ptr::null_mut();
        if activity() & CHECK_ABORT as u16 != 0 {
            (**WorldFields::current()).flagship.crew_enlisted = COUNT::MAX;
        }
        if WorldFields::current().read(|owner| owner.game.glob_flags) & CYBORG_ENABLED as u8 != 0 {
            crate::app::bridge::battle(|b| b.nth_frame = 0);
            KernelOwner::current().write(|kernel| {
                kernel.player_control[0] = HUMAN_CONTROL | STANDARD_RATING;
                ClearPlayerInput(kernel, 0);
                kernel.replace_player_input(0, saved);
            });
        }
        set_activity(old);
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
