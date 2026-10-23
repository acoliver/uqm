//! Battle entry, ordered input processing, and frame scheduling from `battle.c`.

use crate::ships::native::abi;

const BATTLE_LEFT: u8 = 1 << 0;
const BATTLE_RIGHT: u8 = 1 << 1;
const BATTLE_THRUST: u8 = 1 << 2;
const BATTLE_WEAPON: u8 = 1 << 3;
const BATTLE_SPECIAL: u8 = 1 << 4;

/// The C interval is truncated to engine ticks before applying battle speed.
pub const fn frame_interval(ticks_per_second: u32, battle_speed: u8) -> Option<u32> {
    if battle_speed == u8::MAX {
        None
    } else {
        Some((ticks_per_second / 24) / (battle_speed as u32 + 1))
    }
}

pub fn battle_input_order(player_control: &[u8; 2], network_control: u8) -> [usize; 2] {
    let mut order = [0; 2];
    let mut i = 0;
    for (side, control) in player_control.iter().enumerate() {
        if control & network_control == 0 {
            order[i] = side;
            i += 1;
        }
    }
    for (side, control) in player_control.iter().enumerate() {
        if control & network_control != 0 {
            order[i] = side;
            i += 1;
        }
    }
    order
}

pub fn ship_input(input: u8, crew: u16) -> u8 {
    if crew == 0 {
        return 0;
    }
    let mut state = 0;
    if input & BATTLE_LEFT != 0 {
        state |= abi::LEFT as u8;
    } else if input & BATTLE_RIGHT != 0 {
        state |= abi::RIGHT as u8;
    }
    for (battle, ship) in [
        (BATTLE_THRUST, abi::THRUST),
        (BATTLE_WEAPON, abi::WEAPON),
        (BATTLE_SPECIAL, abi::SPECIAL),
    ] {
        if input & battle != 0 {
            state |= ship as u8;
        }
    }
    state
}

#[cfg(not(test))]
#[allow(non_snake_case)]
pub(crate) mod engine {
    use super::*;
    use crate::battle::tactrans_native::{flee_preprocess, ResetWinnerStarShip, StopDitty};
    use abi::*;
    use std::{mem, ptr};

    const BATTLE_ESCAPE: u8 = 1 << 5;

    pub const NETWORK_CONTROL: u8 = 1 << 3;
    pub const HUMAN_CONTROL: u8 = 1;
    pub const CYBORG_CONTROL: u8 = 1 << 1;
    pub const STANDARD_RATING: u8 = 1 << 4;
    pub const AWESOME_RATING: u8 = 1 << 6;
    // gfx_common.h SCALE enum starts with the stepped scaler.
    pub const SCALE_STEP: i32 = 0;

    #[no_mangle]
    pub static mut battle_counter: [BYTE; 2] = [0; 2];
    #[no_mangle]
    pub static mut instantVictory: BOOLEAN = 0;
    pub static mut BATTLE_INPUT_ORDER: [usize; 2] = [0; 2];
    #[no_mangle]
    pub static mut battleFrameCount: BattleFrameCounter = 0;
    static mut BATTLE_REF: MUSIC_REF = ptr::null_mut();

    extern "C" {
        pub static mut nth_frame: UWORD;
        fn rust_battle_frame_reset();
        fn rust_battle_frame_advance();
        fn rust_automation_seed_value(domain: DWORD, fallback: DWORD) -> DWORD;
        fn RedrawQueue(clear: BOOLEAN);
        fn getGameState(state: *const BYTE, start: i32, end: i32) -> BYTE;
        fn setGameState(state: *mut BYTE, start: i32, end: i32, value: BYTE);

    }

    /// # Safety
    /// Game state must be initialized and accessed only on the game thread.
    pub unsafe fn activity() -> u16 {
        GlobData.Game_state.CurrentActivity
    }
    /// # Safety
    /// The caller must exclusively own game-thread activity state.
    pub unsafe fn set_activity(value: u16) {
        GlobData.Game_state.CurrentActivity = value;
    }
    /// # Safety
    /// `start..=end` must name a valid game-state bit field on the game thread.
    pub unsafe fn game_state(start: u32, end: u32) -> u8 {
        getGameState(
            (&raw const GlobData.Game_state.GameState).cast(),
            start as i32,
            end as i32,
        )
    }
    /// # Safety
    /// `start..=end` must name a valid game-state bit field owned by the game thread.
    pub unsafe fn set_game_state(start: u32, end: u32, value: u8) {
        setGameState(
            (&raw mut GlobData.Game_state.GameState).cast(),
            start as i32,
            end as i32,
            value,
        );
    }
    /// # Safety
    /// `side` must be a valid player index with initialized input state.
    pub unsafe fn player_input(side: usize) -> *mut InputContext {
        *(&raw const PlayerInput)
            .cast::<*mut InputContext>()
            .add(side)
    }
    /// # Safety
    /// `side` must be valid and `input` must remain live while installed, or be null during input replacement.
    pub unsafe fn set_player_input(side: usize, input: *mut InputContext) {
        *(&raw mut PlayerInput).cast::<*mut InputContext>().add(side) = input;
    }
    /// # Safety
    /// `side` must be a valid player index on the game thread.
    pub unsafe fn player_control(side: usize) -> u8 {
        *(&raw const PlayerControl).cast::<BYTE>().add(side)
    }
    /// # Safety
    /// `side` must be a valid player index owned by the game thread.
    pub unsafe fn set_player_control(side: usize, value: u8) {
        *(&raw mut PlayerControl).cast::<BYTE>().add(side) = value;
    }
    /// # Safety
    /// `side` must be a valid player index; queue access must remain on the game thread.
    pub unsafe fn race_queue(side: usize) -> *mut QUEUE {
        (&raw mut race_q).cast::<QUEUE>().add(side)
    }
    /// # Safety
    /// `side` must be a valid player index with initialized network state.
    pub unsafe fn connection(side: usize) -> *mut NetConnection {
        *(&raw const netConnections)
            .cast::<*mut NetConnection>()
            .add(side)
    }
    /// # Safety
    /// `element` must be live and its primitive index must address DisplayArray.
    pub unsafe fn primitive(element: *mut ELEMENT) -> *mut PRIMITIVE {
        (&raw mut DisplayArray)
            .cast::<PRIMITIVE>()
            .add((*element).PrimIndex as usize)
    }
    pub const fn rgb15(r: u8, g: u8, b: u8) -> Color {
        Color {
            r: (r << 3) | (r >> 2),
            g: (g << 3) | (g >> 2),
            b: (b << 3) | (b >> 2),
            a: 255,
        }
    }
    /// # Safety
    /// Screen dimensions must be initialized and accessed on the game thread.
    pub unsafe fn sis_width() -> i32 {
        ScreenWidth - 64 - 14
    }
    /// # Safety
    /// Screen dimensions must be initialized and accessed on the game thread.
    pub unsafe fn sis_height() -> i32 {
        ScreenHeight - 13
    }

    unsafe fn RunAwayAllowed() -> bool {
        (activity() as u8 == IN_ENCOUNTER as u8 || activity() as u8 == IN_LAST_BATTLE as u8)
            && game_state(STARBASE_AVAILABLE, END_STARBASE_AVAILABLE) != 0
            && game_state(BOMB_CARRIER, END_BOMB_CARRIER) == 0
    }

    unsafe fn DoRunAway(ship: *mut STARSHIP) {
        let e = (*ship).hShip.cast::<ELEMENT>();
        if (*primitive(e)).Type == gfx_object_STAMP_PRIM as u8
            && (*e).__bindgen_anon_1.life_span == NORMAL_LIFE as u16
            && (*e).state_flags & FINITE_LIFE as u16 == 0
            && (*e).__bindgen_anon_3.mass_points != (MAX_SHIP_MASS * 10) as u8
            && (*e).state_flags & APPEARING as u16 == 0
        {
            battle_counter[0] = battle_counter[0].wrapping_sub(1);
            (*e).__bindgen_anon_4.turn_wait = 3;
            (*e).__bindgen_anon_5.thrust_wait = 4;
            (*e).colorCycleIndex = 0;
            (*e).preprocess_func = Some(flee_preprocess);
            (*e).__bindgen_anon_3.mass_points = (MAX_SHIP_MASS * 10) as u8;
            (*e).velocity = mem::zeroed();
            (*ship).cur_status_flags &= !((SHIP_AT_MAX_SPEED | SHIP_BEYOND_MAX_SPEED) as u16);
            (*primitive(e)).color = rgb15(0x0b, 0, 0);
            (*primitive(e)).Type = gfx_object_STAMPFILL_PRIM as u8;
            (*ship).ship_input_state = 0;
        }
    }

    unsafe fn setupBattleInputOrder() {
        let controls = [player_control(0), player_control(1)];
        for (i, side) in battle_input_order(&controls, NETWORK_CONTROL)
            .into_iter()
            .enumerate()
        {
            BATTLE_INPUT_ORDER[i] = side;
        }
    }

    /// # Safety
    /// `context` must be a live human input context on the game thread.
    #[no_mangle]
    pub unsafe extern "C" fn frameInputHuman(
        context: *mut HumanInputContext,
        _ship: *mut STARSHIP,
    ) -> BATTLE_INPUT_STATE {
        CurrentInputToBattleInput((*context).playerNr)
    }

    unsafe fn ProcessInput() {
        netInput();
        let can_run = RunAwayAllowed();
        for side in BATTLE_INPUT_ORDER {
            let mut h = (*race_queue(side)).head;
            while !h.is_null() {
                let ship = h.cast::<STARSHIP>();
                let next = (*ship).succ;
                if !(*ship).hShip.is_null() {
                    (*ship).control = player_control(side);
                    let ctx = player_input(side);
                    let mut input = ((*(*ctx).handlers).frameInput.unwrap())(ctx, ship);
                    if player_control(side) & NETWORK_CONTROL == 0 {
                        let bib = getBattleInputBuffer(side);
                        Netplay_NotifyAll_battleInput(input);
                        flushPacketQueues();
                        BattleInputBuffer_push(bib, input);
                        BattleInputBuffer_pop(bib, &mut input);
                    }
                    (*ship).ship_input_state =
                        ship_input(input, (*(*ship).RaceDescPtr).ship_info.crew_level);
                    if (*(*ship).RaceDescPtr).ship_info.crew_level != 0
                        && can_run
                        && side == 0
                        && input & BATTLE_ESCAPE != 0
                    {
                        DoRunAway(ship);
                    }
                }
                h = next;
            }
        }
        flushPacketQueues();
        if activity() & (CHECK_LOAD | CHECK_ABORT) as u16 != 0 {
            set_activity(activity() & !(IN_BATTLE as u16));
        }
    }

    /// # Safety
    /// The game thread must own initialized audio, activity, and resource state.
    #[no_mangle]
    pub unsafe extern "C" fn BattleSong(play: BOOLEAN) {
        if BATTLE_REF.is_null() {
            let resource = if inHyperSpace() != 0 {
                HYPERSPACE_MUSIC.as_ptr()
            } else if inQuasiSpace() != 0 {
                QUASISPACE_MUSIC.as_ptr()
            } else {
                BATTLE_MUSIC.as_ptr()
            };
            BATTLE_REF = crate::sound::resource_types::LoadMusicInstance(resource.cast()).cast();
        }
        if play != 0 {
            PlayMusic(BATTLE_REF, 1, 1);
        }
    }

    /// # Safety
    /// The game thread must own the battle music handle and initialized audio state.
    #[no_mangle]
    pub unsafe extern "C" fn FreeBattleSong() {
        DestroyMusic(BATTLE_REF);
        BATTLE_REF = ptr::null_mut();
    }

    unsafe extern "C" fn DoBattle(bs: *mut battlestate_struct) -> BOOLEAN {
        crate::log_abi::log_message(
            5,
            &format!(
                "BATTLE_DEBUG: DoBattle called, first_time={}, CHECK_ABORT={}",
                (*bs).first_time,
                (activity() & CHECK_ABORT as u16 != 0) as i32
            ),
        );
        SetMenuSounds(0, 0);
        if getNumNetConnections() > 0 && battleFrameCount.is_multiple_of(NETPLAY_CHECKSUM_INTERVAL)
        {
            let mut state = mem::zeroed();
            crc_init(&mut state);
            crc_processState(&mut state);
            let checksum = crc_finish(&state);
            Netplay_NotifyAll_checksum(battleFrameCount, checksum);
            flushPacketQueues();
            addLocalChecksum(battleFrameCount, checksum);
        }
        ProcessInput();
        if getNumNetConnections() > 0 {
            let delay = getBattleInputDelay();
            if battleFrameCount as usize >= delay
                && (battleFrameCount as usize - delay)
                    .is_multiple_of(NETPLAY_CHECKSUM_INTERVAL as usize)
                && activity() & CHECK_ABORT as u16 == 0
                && !verifyChecksums((battleFrameCount as usize - delay) as u32)
            {
                set_activity(activity() | CHECK_ABORT as u16);
                resetConnections(NetplayResetReason_ResetReason_syncLoss);
            }
        }
        let mut rect: RECT = mem::zeroed();
        if (*bs).first_time != 0 {
            rect.corner.x = SIS_ORG_X as i16;
            rect.corner.y = SIS_ORG_Y as i16;
            rect.extent.width = sis_width() as _;
            rect.extent.height = sis_height() as _;
            SetTransitionSource(&rect);
        }
        BatchGraphics();
        if let Some(callback) = (*bs).frame_cb {
            callback();
        }
        RedrawQueue(1);
        if (*bs).first_time != 0 {
            (*bs).first_time = 0;
            ScreenTransition(3, &rect);
        }
        UnbatchGraphics();
        if activity() & IN_BATTLE as u16 == 0 || activity() & (CHECK_ABORT | CHECK_LOAD) as u16 != 0
        {
            return 0;
        }
        match frame_interval(ONE_SECOND, (nth_frame >> 8) as u8) {
            None => {
                Async_process();
                TaskSwitch();
            }
            Some(interval) => {
                SleepThreadUntil((*bs).NextTime.wrapping_add(interval));
                (*bs).NextTime = GetTimeCounter();
            }
        }
        if activity() & IN_BATTLE as u16 == 0 {
            return 0;
        }
        rust_battle_frame_advance();
        battleFrameCount = battleFrameCount.wrapping_add(1);
        1
    }

    /// # Safety
    /// Player controls and any network-controlled side connections must be initialized.
    #[no_mangle]
    pub unsafe extern "C" fn GetPlayerOrder(i: COUNT) -> COUNT {
        if (player_control(0) & NETWORK_CONTROL != 0
            && !NetConnection_getDiscriminant(connection(0)))
            || (player_control(1) & NETWORK_CONTROL != 0
                && NetConnection_getDiscriminant(connection(1)))
        {
            i
        } else {
            1u16.wrapping_sub(i)
        }
    }

    unsafe fn selectAllShips(count: SIZE) -> BOOLEAN {
        if count == 1 {
            return GetNextStarShip(ptr::null_mut(), 0);
        }
        if player_control(0) & NETWORK_CONTROL != 0 && player_control(1) & NETWORK_CONTROL != 0 {
            crate::log_abi::log_message(2, "Only one side at a time can be network controlled.\n");
            return 0;
        }
        GetInitialStarShips()
    }

    /// # Safety
    /// The game thread must own initialized battle, input, audio, and display state; any callback must remain valid for the battle.
    #[no_mangle]
    pub unsafe extern "C" fn Battle(callback: BattleFrameCallback) -> BOOLEAN {
        if activity() as u8 != SUPER_MELEE as u8 {
            crate::math::seed_random(GetTimeCounter());
        } else {
            let seed = rust_automation_seed_value(2, 0);
            if seed != 0 {
                crate::math::seed_random(seed);
            }
        }
        BattleSong(0);
        let mut count = InitShips();
        if instantVictory != 0 {
            count = 0;
            battle_counter = [1, 0];
            instantVictory = 0;
        }
        if count != 0 {
            let mut bs: battlestate_struct = mem::zeroed();
            set_activity(activity() | IN_BATTLE as u16);
            battle_counter[0] = CountLinks(race_queue(0)) as u8;
            battle_counter[1] = CountLinks(race_queue(1)) as u8;
            rust_battle_frame_reset();
            if optMeleeScale != SCALE_STEP {
                SetGraphicScaleMode(optMeleeScale);
            }
            setupBattleInputOrder();
            initBattleInputBuffers();
            initChecksumBuffers();
            battleFrameCount = 0;
            ResetWinnerStarShip();
            setBattleStateConnections(&mut bs);
            crate::log_abi::log_message(
                5,
                &format!(
                    "BATTLE_DEBUG: About to call selectAllShips, CHECK_ABORT={}, IN_BATTLE={}",
                    (activity() & CHECK_ABORT as u16 != 0) as i32,
                    (activity() & IN_BATTLE as u16 != 0) as i32
                ),
            );
            if selectAllShips(count) == 0 {
                set_activity(activity() | CHECK_ABORT as u16);
            } else {
                crate::log_abi::log_message(
                    5,
                    "BATTLE_DEBUG: selectAllShips returned TRUE, calling BattleSong",
                );
                BattleSong(1);
                bs.NextTime = 0;
                initBattleStateDataConnections();
                if !negotiateReadyConnections(true, NetState_NetState_inBattle) {
                    set_activity(activity() | CHECK_ABORT as u16);
                } else {
                    bs.InputFunc = Some(DoBattle);
                    bs.frame_cb = callback;
                    bs.first_time = inHQSpace();
                    crate::log_abi::log_message(
                        5,
                        "BATTLE_DEBUG: About to call DoInput for battle loop",
                    );
                    DoInput((&mut bs as *mut battlestate_struct).cast(), 0);
                    crate::log_abi::log_message(
                        5,
                        &format!(
                            "BATTLE_DEBUG: DoInput returned, CHECK_ABORT={}",
                            (activity() & CHECK_ABORT as u16 != 0) as i32
                        ),
                    );
                }
            }
            if activity() as u8 == SUPER_MELEE as u8 {
                if activity() & CHECK_ABORT as u16 != 0 {
                    waitResetConnections(NetState_NetState_inSetup);
                    set_activity(activity() & !(CHECK_ABORT as u16));
                } else {
                    MeleeGameOver();
                }
            }
            uninitBattleInputBuffers();
            uninitChecksumBuffers();
            setBattleStateConnections(ptr::null_mut());
            StopDitty();
            StopMusic();
            StopSound();
        }
        UninitShips();
        FreeBattleSong();
        (count < 0) as BOOLEAN
    }
}

#[cfg(not(test))]
pub use engine::{
    battleFrameCount, battle_counter, frameInputHuman, instantVictory, Battle, BattleSong,
    FreeBattleSong, GetPlayerOrder, BATTLE_INPUT_ORDER,
};

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn c_boolean_and_input_widths_are_distinct() {
        assert_eq!(std::mem::size_of::<abi::BOOLEAN>(), 4);
        assert_eq!(std::mem::size_of::<bool>(), 1);
        assert_eq!(std::mem::size_of::<abi::BATTLE_INPUT_STATE>(), 1);
        assert_eq!(std::mem::size_of::<abi::COUNT>(), 2);
        assert_eq!(std::mem::size_of::<abi::SIZE>(), 2);
    }

    #[test]
    fn input_mapping_matches_all_byte_patterns() {
        for input in 0u8..=255 {
            let mut expected = 0;
            if input & 1 != 0 {
                expected |= abi::LEFT as u8;
            } else if input & 2 != 0 {
                expected |= abi::RIGHT as u8;
            }
            if input & 4 != 0 {
                expected |= abi::THRUST as u8;
            }
            if input & 8 != 0 {
                expected |= abi::WEAPON as u8;
            }
            if input & 16 != 0 {
                expected |= abi::SPECIAL as u8;
            }
            assert_eq!(ship_input(input, 1), expected);
            assert_eq!(ship_input(input, u16::MAX), expected);
            assert_eq!(ship_input(input, 0), 0);
        }
        for speed in 0..=254 {
            assert_eq!(frame_interval(840, speed), Some(35 / (speed as u32 + 1)));
        }
    }

    #[test]
    fn frame_pacing_is_24hz_before_speed_division() {
        assert_eq!(frame_interval(840, 0), Some(35));
        assert_eq!(frame_interval(840, 1), Some(17));
        assert_eq!(frame_interval(840, 3), Some(8));
        assert_eq!(frame_interval(840, 255), None);
        assert_eq!(frame_interval(1000, 2), Some(13));
    }
    #[test]
    fn local_sides_precede_network_sides() {
        assert_eq!(battle_input_order(&[8, 1], 8), [1, 0]);
        assert_eq!(battle_input_order(&[1, 1], 8), [0, 1]);
        assert_eq!(battle_input_order(&[8, 8], 8), [0, 1]);
    }
    #[test]
    fn input_left_wins_and_dead_crew_cannot_act() {
        let both = BATTLE_LEFT | BATTLE_RIGHT | BATTLE_THRUST | BATTLE_WEAPON | BATTLE_SPECIAL;
        assert_eq!(
            ship_input(both, 1),
            (abi::LEFT | abi::THRUST | abi::WEAPON | abi::SPECIAL) as u8
        );
        assert_eq!(ship_input(both, 0), 0);
    }
}
