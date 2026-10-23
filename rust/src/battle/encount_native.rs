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

#[cfg(any(not(test), feature = "linked_c_archive"))]
#[allow(non_snake_case)]
mod engine {
    use super::*;
    use crate::battle::battle_native::engine::{
        activity, game_state, nth_frame, player_input, race_queue, rgb15, set_activity,
        set_game_state, set_player_control, set_player_input, sis_height, sis_width,
        AWESOME_RATING, CYBORG_CONTROL, HUMAN_CONTROL, STANDARD_RATING,
    };
    use crate::battle::battle_native::{battle_counter, Battle};
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

    // planets.h star_desc; only the starmap pointer is needed here.
    #[repr(C)]
    struct StarDesc {
        star_pt: POINT,
        Type: BYTE,
        Index: BYTE,
        Prefix: BYTE,
        Postfix: BYTE,
    }

    extern "C" {
        fn LoadGraphicInstance(resource: RESOURCE) -> *mut std::ffi::c_void;
        static mut planet: [FRAME; 3];
        static mut CurStarDescPtr: *mut StarDesc;
        fn GetClusterName(star: *const StarDesc, buf: *mut UNICODE);
        fn AnyButtonPress(special: BOOLEAN) -> BOOLEAN;
    }

    unsafe fn game_string(index: u32) -> *const UNICODE {
        GetStringAddress(SetAbsStringTableIndex(GameStrings, index as COUNT)).cast()
    }
    unsafe fn planet_name() -> *mut UNICODE {
        (&raw mut GlobData.SIS_state.PlanetName).cast()
    }
    unsafe fn escape_pressed() -> bool {
        let template = *(&raw const PlayerControls).cast::<CONTROL_TEMPLATE>();
        CurrentInputState.key[template as usize][KEY_ESCAPE as usize] != 0
    }
    fn text(x: i16, y: i16, align: TEXT_ALIGN, string: *const UNICODE) -> TEXT {
        TEXT {
            baseline: POINT { x, y },
            pStr: string,
            align,
            CharCount: COUNT::MAX,
        }
    }
    unsafe fn draw_text(t: &mut TEXT, string: *const UNICODE) {
        t.pStr = string;
        t.CharCount = COUNT::MAX;
        font_DrawText(t);
    }

    unsafe extern "C" fn DoSelectAction(ms: *mut MENU_STATE) -> BOOLEAN {
        SetMenuSounds(0x0f, 1 << 4);
        if activity() & CHECK_ABORT as u16 != 0 {
            (*ms).CurState = ATTACK as u8 + 1;
            return 0;
        }
        if (*ms).Initialized == 0 {
            (*ms).Initialized = 1;
            (*ms).InputFunc = Some(DoSelectAction);
        } else if PulsedInputState.menu[KEY_MENU_SELECT as usize] != 0 {
            match (*ms).CurState as u32 {
                HAIL | ATTACK => {
                    if activity() as u8 == IN_LAST_BATTLE as u8 {
                        (*ms).CurState = HAIL as u8;
                    }
                    return 0;
                }
                option if option == ATTACK + 1 => {
                    if GameOptions() == 0 {
                        return 0;
                    }
                    DrawMenuStateStrings(PM_CONVERSE as u8, (*ms).CurState as i16);
                    SetFlashRect(usize::MAX as *const RECT);
                }
                option => {
                    printf(c"Unknown option: %d\n".as_ptr(), option as i32);
                }
            }
        }
        DoMenuChooser(ms, PM_CONVERSE as u8);
        1
    }

    unsafe fn GetShipFragQueueForPlayer(player: COUNT) -> *mut QUEUE {
        if player == RPG_PLAYER_NUM as u16 {
            &raw mut GlobData.Game_state.built_ship_q
        } else {
            &raw mut GlobData.Game_state.npc_built_ship_q
        }
    }

    /// # Safety
    /// `player` must be a valid side; fleet, ship queues, and encounter resources must be initialized on the game thread.
    #[no_mangle]
    pub unsafe extern "C" fn BuildBattle(player: COUNT) {
        EncounterRace = -1;
        if GlobData.Game_state.npc_built_ship_q.head.is_null() {
            set_game_state(BATTLE_SEGUE, END_BATTLE_SEGUE, 0);
            return;
        }
        if player != RPG_PLAYER_NUM as u16 {
            match activity() as u8 as u32 {
                IN_LAST_BATTLE => load_gravity_well(NUMBER_OF_PLANET_TYPES as u8),
                IN_HYPERSPACE => load_gravity_well(
                    ((TFB_Random() as COUNT) % NUMBER_OF_PLANET_TYPES as COUNT) as u8,
                ),
                _ => {
                    set_game_state(ESCAPE_COUNTER, END_ESCAPE_COUNTER, 110);
                    load_gravity_well(game_state(BATTLE_PLANET, END_BATTLE_PLANET));
                }
            }
        }
        let q = GetShipFragQueueForPlayer(player);
        let ships = race_queue(player as usize);
        ReinitQueue(ships);
        let mut h = (*q).head;
        while !h.is_null() {
            let frag = h.cast::<SHIP_FRAGMENT>();
            h = (*frag).succ;
            let built = Build(
                ships,
                if (*frag).race_id == SAMATRA_SHIP as u8 {
                    SPECIES_ID_SA_MATRA_ID
                } else {
                    (*frag).SpeciesID
                },
            );
            if !built.is_null() {
                let ship = built.cast::<STARSHIP>();
                (*ship).captains_name_index = (*frag).captains_name_index;
                (*ship).playerNr = player as i16;
                (*ship).crew_level = if (*frag).crew_level != COUNT::MAX {
                    (*frag).crew_level
                } else {
                    (*frag).max_crew
                };
                (*ship).max_crew = (*frag).max_crew;
                (*ship).race_strings = (*frag).race_strings;
                (*ship).icons = (*frag).icons;
                (*ship).index = (*frag).index as u16;
                (*ship).ship_cost = 0;
                (*ship).RaceDescPtr = ptr::null_mut();
            }
        }
        if player == RPG_PLAYER_NUM as u16 {
            let built = Build(race_queue(0), SPECIES_ID_SIS_SHIP_ID);
            if !built.is_null() {
                let ship = built.cast::<STARSHIP>();
                (*ship).captains_name_index = 0;
                (*ship).playerNr = RPG_PLAYER_NUM as i16;
                (*ship).crew_level = 0;
                (*ship).max_crew = 0;
                (*ship).race_strings = ptr::null_mut();
                (*ship).icons = ptr::null_mut();
                (*ship).index = COUNT::MAX;
                (*ship).ship_cost = 0;
                (*ship).energy_counter = MAX_ENERGY_SIZE as u8;
                (*ship).RaceDescPtr = ptr::null_mut();
            }
        }
    }

    /// # Safety
    /// `player` must be a valid side with an initialized ship-fragment queue on the game thread.
    #[no_mangle]
    pub unsafe extern "C" fn FleetIsInfinite(player: COUNT) -> BOOLEAN {
        let h = (*GetShipFragQueueForPlayer(player)).head;
        if h.is_null() {
            return 0;
        }
        ((*h.cast::<SHIP_FRAGMENT>()).crew_level == COUNT::MAX) as BOOLEAN
    }

    /// # Safety
    /// `owner` must be a live starship whose player fleet and battle queues have matching order.
    #[no_mangle]
    pub unsafe extern "C" fn UpdateShipFragCrew(owner: *mut STARSHIP) {
        let frag_q = GetShipFragQueueForPlayer((*owner).playerNr as COUNT);
        let ship_q = race_queue((*owner).playerNr as usize);
        let mut hf = (*frag_q).head;
        let mut hs = (*ship_q).head;
        while !hf.is_null() && !hs.is_null() {
            let ship = hs.cast::<STARSHIP>();
            let frag = hf.cast::<SHIP_FRAGMENT>();
            hs = (*ship).succ;
            hf = (*frag).succ;
            if ship == owner {
                assert_ne!((*frag).crew_level, COUNT::MAX);
                (*frag).crew_level = (*ship).crew_level;
                break;
            }
        }
    }

    /// # Safety
    /// The game thread must own initialized encounter, display, input, resource, and audio state.
    #[no_mangle]
    pub unsafe extern "C" fn InitEncounter() -> COUNT {
        SetContext(SpaceContext);
        SetContextFont(TinyFont);
        let music =
            crate::sound::resource_types::LoadMusicInstance(REDALERT_MUSIC.as_ptr().cast()).cast();
        PlayMusic(music, 0, 1);
        let segue = CaptureDrawable(LoadGraphicInstance(SEGUE_PMAP_ANIM.as_ptr().cast()).cast());
        WaitForSoundEnd(COUNT::MAX);
        StopMusic();
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
        SetContextFont(MicroFont);
        SetContextForeGroundColor(rgb15(0, 0, 20));
        if inHQSpace() != 0 {
            draw_text(&mut t, game_string(ENCOUNTER_STRING_BASE));
            t.baseline.y += 12;
            draw_text(&mut t, game_string(ENCOUNTER_STRING_BASE + 1));
        } else {
            let mut buf = [0 as UNICODE; 256];
            draw_text(&mut t, game_string(ENCOUNTER_STRING_BASE + 2));
            t.baseline.y += 12;
            GetClusterName(CurStarDescPtr, buf.as_mut_ptr());
            draw_text(&mut t, buf.as_ptr());
            t.baseline.y += 12;
            draw_text(&mut t, planet_name());
        }
        DrawSISMessage(ptr::null());
        stamp.origin = POINT {
            x: (sis_width() >> 1) as i16,
            y: (sis_height() >> 1) as i16,
        };
        stamp.frame = planet[0];
        DrawStamp(&mut stamp);
        if activity() as u8 != IN_LAST_BATTLE as u8 {
            let mut h = GlobData.Game_state.npc_built_ship_q.head;
            let mut i = 0usize;
            while !h.is_null() && i < 60 {
                let frag = h.cast::<SHIP_FRAGMENT>();
                let next = if (*frag).crew_level != COUNT::MAX {
                    (*frag).succ
                } else {
                    h
                };
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
                stamp.frame = SetAbsFrameIndex((*frag).icons, 0);
                let mut rect: RECT = mem::zeroed();
                GetFrameRect(stamp.frame, &mut rect);
                stamp.origin.x = (stamp.origin.x as i32 + (sis_width() >> 1)
                    - (rect.extent.width as i32 >> 1)) as i16;
                stamp.origin.y = (stamp.origin.y as i32 + (sis_height() >> 1)
                    - (rect.extent.height as i32 >> 1)) as i16;
                DrawStamp(&mut stamp);
                h = next;
                i += 1;
            }
        }
        UnbatchGraphics();
        DestroyDrawable(ReleaseDrawable(segue));
        ScreenTransition(3, ptr::null());
        let mut menu: MENU_STATE = mem::zeroed();
        menu.InputFunc = Some(DoSelectAction);
        menu.Initialized = 0;
        menu.CurState = HAIL as u8;
        DrawMenuStateStrings(PM_CONVERSE as u8, menu.CurState as i16);
        SetFlashRect(usize::MAX as *const RECT);
        DoInput((&mut menu as *mut MENU_STATE).cast(), 1);
        SetFlashRect(ptr::null());
        menu.CurState as COUNT
    }

    unsafe fn DrawFadeText(
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
        free_gravity_well();
        if activity() & (CHECK_ABORT | CHECK_LOAD) as u16 != 0
            || GlobData.SIS_state.CrewEnlisted == COUNT::MAX
            || activity() as u8 == IN_LAST_BATTLE as u8
            || activity() as u8 == WON_LAST_BATTLE as u8
        {
            return killed;
        }
        if game_state(BATTLE_SEGUE, END_BATTLE_SEGUE) == 0 {
            ReinitQueue(race_queue(0));
            ReinitQueue(race_queue(1));
            return killed;
        }
        let mut recycle = 0u16;
        let mut rect: RECT = mem::zeroed();
        let mut salvage_rect: RECT = mem::zeroed();
        let mut t: TEXT;
        let mut stamp = STAMP {
            origin: POINT { x: 0, y: 0 },
            frame: ptr::null_mut(),
        };
        let mut str1 = ptr::null();
        let mut str2 = ptr::null();
        let mut buf = [0 as UNICODE; 80];
        set_game_state(BATTLE_SEGUE, END_BATTLE_SEGUE, 0);
        set_game_state(BOMB_CARRIER, END_BOMB_CARRIER, 0);
        let mut victory = victory_state(
            battle_counter[1],
            battle_counter[0],
            game_state(URQUAN_PROTECTING_SAMATRA, END_URQUAN_PROTECTING_SAMATRA) != 0,
        );
        let frag = GlobData
            .Game_state
            .npc_built_ship_q
            .head
            .cast::<SHIP_FRAGMENT>();
        EncounterRace = (*frag).race_id as i16;
        if GetStarShipFromIndex(
            &raw mut GlobData.Game_state.avail_race_q,
            EncounterRace as COUNT,
        )
        .is_null()
        {
            victory = -1;
            InitSISContexts();
        }
        let previous_mode = SetStatusMessageMode(StatMsgMode_SMM_RES_UNITS);
        let mut sleepy = true;
        for side in 0..NUM_SIDES as usize {
            let q = if side == 0 {
                &raw mut GlobData.Game_state.built_ship_q
            } else {
                if victory < 0 {
                    victory = 0;
                } else {
                    DrawSISFrame();
                    DrawSISMessage(ptr::null());
                    if inHQSpace() != 0 {
                        DrawHyperCoords(GlobData.Game_state.ShipStamp.origin);
                    } else if GlobData.Game_state.ip_planet == 0 {
                        DrawHyperCoords((*CurStarDescPtr).star_pt);
                    } else {
                        DrawSISTitle(planet_name());
                    }
                    SetContext(SpaceContext);
                    if victory != 0 {
                        DrawArmadaPickShip(1, &mut salvage_rect);
                    }
                }
                &raw mut GlobData.Game_state.npc_built_ship_q
            };
            ReinitQueue(race_queue(NUM_SIDES as usize - 1 - side));
            let mut h = (*q).head;
            while !h.is_null() {
                let frag = h.cast::<SHIP_FRAGMENT>();
                let next = (*frag).succ;
                if fragment_is_removed(
                    (*frag).crew_level,
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
                                stamp.frame = IncFrameIndex((*frag).icons);
                                DrawStamp(&mut stamp);
                                SetContextForeGroundColor(rgb15(8, 8, 8));
                                SetContextFont(TinyFont);
                                utf8StringCopy(
                                    buf.as_mut_ptr(),
                                    buf.len(),
                                    GetStringAddress((*frag).race_strings).cast(),
                                );
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
                                stamp.frame = (*frag).icons;
                                SetContextFont(MicroFont);
                                str1 = game_string(ENCOUNTER_STRING_BASE + 4);
                                str2 = game_string(ENCOUNTER_STRING_BASE + 5);
                                DrawFadeText(str1, str2, 1, &mut salvage_rect);
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
                    RemoveQueue(q, h);
                    FreeLink(q, h);
                    h = next;
                    continue;
                }
                h = next;
            }
        }
        SetStatusMessageMode(previous_mode);
        if victory != 0 {
            WaitForAnyButton(1, ONE_SECOND * 3, 0);
            if !escape_pressed() {
                DrawFadeText(str1, str2, 0, &mut salvage_rect);
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
                    DrawFadeText(str1, str2, 1, &mut salvage_rect);
                    WaitForAnyButton(1, ONE_SECOND * 2, 0);
                    if !escape_pressed() {
                        DrawFadeText(str1, str2, 0, &mut salvage_rect);
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
        let mut saved = ptr::null_mut();
        if GlobData.Game_state.glob_flags & CYBORG_ENABLED as u8 != 0 {
            nth_frame = 1 | ((combat_speed(GlobData.Game_state.glob_flags) as u16) << 8);
            set_player_control(0, CYBORG_CONTROL | AWESOME_RATING);
            saved = player_input(0);
            set_player_input(0, ptr::null_mut());
            if !SetPlayerInput(0) {
                crate::log_abi::log_message(1, "Could not set cyborg player input.");
                // misc.h explode(): abort in DEBUG, exit(EXIT_FAILURE) otherwise.
                if cfg!(feature = "debug-process") {
                    std::process::abort();
                } else {
                    std::process::exit(1);
                }
            }
        }
        GameSounds = crate::strings::string_table::CaptureStringTable(
            crate::sound::resource_types::LoadSoundInstance(GAME_SOUNDS.as_ptr().cast()).cast(),
        )
        .cast();
        Battle(None);
        DestroySound(ReleaseStringTable(GameSounds));
        GameSounds = ptr::null_mut();
        if activity() & CHECK_ABORT as u16 != 0 {
            GlobData.SIS_state.CrewEnlisted = COUNT::MAX;
        }
        if GlobData.Game_state.glob_flags & CYBORG_ENABLED as u8 != 0 {
            nth_frame = 0;
            set_player_control(0, HUMAN_CONTROL | STANDARD_RATING);
            ClearPlayerInput(0);
            set_player_input(0, saved);
        }
        set_activity(old);
    }
}

#[cfg(any(not(test), feature = "linked_c_archive"))]
pub use engine::*;

#[cfg(test)]
mod tests {
    use super::*;
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
