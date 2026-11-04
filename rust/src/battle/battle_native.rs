//! Battle entry, ordered input processing, and frame scheduling from `battle.c`.

#[cfg(not(test))]
use crate::engine::battle::input::InputContext;
#[cfg(not(test))]
use crate::engine::World as WorldOwner;
#[cfg(not(test))]
use crate::engine::{Kernel as KernelOwner, WorldData as WorldFields};
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
type BattleNetwork = (
    *mut crate::engine::battle::Battle,
    std::rc::Rc<dyn crate::engine::battle::net::BattleNet>,
    bool,
);
#[cfg(not(test))]
fn initialize_battle(cx: &mut crate::engine::battle::Cx<'_>) -> BattleNetwork {
    cx.b.initialize(cx.kernel);
    let battle = &mut *cx.b;
    let had_net = battle.net.is_some();
    battle
        .net
        .get_or_insert_with(|| crate::supermelee::netplay::NetplaySession::default().battle_net());
    (battle, battle.network(), had_net)
}

#[cfg(not(test))]
fn finish_battle_network(battle: &mut crate::engine::battle::Battle, had_net: bool) {
    if !had_net {
        battle.net = None;
    }
}

#[cfg(not(test))]
unsafe fn battle_network() -> BattleNetwork {
    use engine::NETWORK_CONTROL;
    let (battle, net, had_net) = crate::app::bridge::enter(|cx| {
        let result = initialize_battle(cx);
        for side in 0..2 {
            if cx.kernel.player_control[side] & NETWORK_CONTROL != 0 {
                cx.kernel.player_input[side]
                    .as_mut()
                    .unwrap_or_else(|| panic!("missing player input"))
                    .connect(result.1.clone());
            }
        }
        result
    });
    (battle, net, had_net)
}

#[cfg(not(test))]
pub(crate) mod engine {
    use super::*;
    use crate::battle::tactrans_native::{flee_preprocess, reset_winner, StopDitty};
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

    use crate::automation::battle_observer::rust_battle_frame_reset;

    use crate::automation::battle_observer::rust_battle_frame_advance;

    use crate::automation::coordinator::rust_automation_seed_value;

    use crate::battle::process_native::engine::RedrawQueue;

    use crate::state::ffi::rust_get_game_state_bits as getGameState;

    use crate::state::ffi::rust_set_game_state_bits as setGameState;

    /// # Safety
    /// Game state must be initialized and accessed only on the game thread.
    pub fn activity() -> u16 {
        WorldFields::current().read(|owner| owner.game.current_activity)
    }
    /// # Safety
    /// The caller must exclusively own game-thread activity state.
    pub unsafe fn set_activity(value: u16) {
        (**WorldFields::current()).game.current_activity = value;
    }
    /// # Safety
    /// `start..=end` must name a valid game-state bit field on the game thread.
    pub fn game_state(start: u32, end: u32) -> u8 {
        WorldOwner::current().read(|world| getGameState(world, start as i32, end as i32))
    }
    /// # Safety
    /// `start..=end` must name a valid game-state bit field owned by the game thread.
    pub fn set_game_state(start: u32, end: u32, value: u8) {
        WorldOwner::current().write(|world| setGameState(world, start as i32, end as i32, value));
    }
    /// # Safety
    /// `side` must be a valid player index with initialized input state.
    pub fn player_input(side: usize) -> Option<InputContext> {
        KernelOwner::current().read(|owner| owner.player_input[side].clone())
    }
    pub fn set_player_input(side: usize, input: Option<InputContext>) {
        KernelOwner::current().write(|owner| owner.player_input[side] = input);
    }
    /// # Safety
    /// `side` must be a valid player index on the game thread.
    pub unsafe fn player_control(side: usize) -> u8 {
        let kernel = KernelOwner::current();
        *(&raw const (**kernel).player_control)
            .cast::<BYTE>()
            .add(side)
    }
    /// # Safety
    /// `side` must be a valid player index owned by the game thread.
    pub unsafe fn set_player_control(side: usize, value: u8) {
        let kernel = KernelOwner::current();
        *(&raw mut (**kernel).player_control)
            .cast::<BYTE>()
            .add(side) = value;
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

    fn run_away_allowed() -> bool {
        (activity() as u8 == IN_ENCOUNTER as u8 || activity() as u8 == IN_LAST_BATTLE as u8)
            && game_state(STARBASE_AVAILABLE, END_STARBASE_AVAILABLE) != 0
            && game_state(BOMB_CARRIER, END_BOMB_CARRIER) == 0
    }

    fn run_away(cx: &mut crate::engine::battle::Cx<'_>, ship: crate::engine::battle::ShipId) {
        let e = cx.ship_element(ship);
        let state = cx.element(e);
        if cx.primitive_style(e).kind == gfx_object_STAMP_PRIM as u8
            && state.life == NORMAL_LIFE as u16
            && state.flags & FINITE_LIFE as u16 == 0
            && state.mass != (MAX_SHIP_MASS * 10) as u8
            && state.flags & APPEARING as u16 == 0
        {
            cx.b.counters[0] = cx.b.counters[0].wrapping_sub(1);
            cx.element_mut(e).turn = 3;
            cx.element_mut(e).thrust = 4;
            cx.element_mut(e).color_cycle = 0;
            cx.element_mut(e).pre = Some(ElementHook::Native(flee_preprocess));
            cx.element_mut(e).mass = (MAX_SHIP_MASS * 10) as u8;
            // SAFETY: the legacy velocity record admits the zero state.
            cx.element_mut(e).velocity = crate::battle::velocity::VelocityDesc::default();
            cx.ship_state_mut(ship).status &= !((SHIP_AT_MAX_SPEED | SHIP_BEYOND_MAX_SPEED) as u16);
            cx.primitive_style_mut(e).color = rgb15(0x0b, 0, 0);
            cx.primitive_style_mut(e).kind = gfx_object_STAMPFILL_PRIM as u8;
            cx.ship_state_mut(ship).input = 0;
            crate::automation::ui_observation::record_ship_escape();
        }
    }

    unsafe fn process_input(
        cx: &mut crate::engine::battle::Cx<'_>,
        net: &dyn crate::engine::battle::net::BattleNet,
        order: [usize; 2],
    ) {
        net.poll();
        let can_run = run_away_allowed();
        for side in order {
            let mut next = cx.first_ship(side);
            while let Some(ship) = next {
                next = cx.next_ship(ship);
                if cx.optional_ship_element(ship).is_some() {
                    cx.ship_state_mut(ship).control = player_control(side);
                    let context = cx.kernel.player_input[side]
                        .clone()
                        .unwrap_or_else(|| panic!("missing player input"));
                    let mut input = match context.source() {
                        crate::engine::battle::input::InputSource::Human => {
                            cx.human_input(context.player())
                        }
                        crate::engine::battle::input::InputSource::Computer => {
                            crate::battle::context_access::ai_legacy::computer_intelligence(
                                cx,
                                context.player(),
                                Some(ship),
                            )
                        }
                        crate::engine::battle::input::InputSource::Network(_) => {
                            context.network().remote_input(side)
                        }
                    };
                    if player_control(side) & NETWORK_CONTROL == 0 {
                        input = net.local_input(side, input);
                    }
                    cx.ship_state_mut(ship).input =
                        ship_input(input, cx.race_stats(ship).ship_info.crew_level);
                    if cx.race_stats(ship).ship_info.crew_level != 0
                        && can_run
                        && side == 0
                        && input & BATTLE_ESCAPE != 0
                    {
                        run_away(cx, ship);
                    }
                }
            }
        }
        net.flush();
        if activity() & (CHECK_LOAD | CHECK_ABORT) as u16 != 0 {
            set_activity(activity() & !(IN_BATTLE as u16));
        }
    }

    /// # Safety
    /// The game thread must own initialized audio, activity, and resource state.
    #[no_mangle]
    pub unsafe extern "C" fn BattleSong(play: BOOLEAN) {
        if KernelOwner::current()
            .read(|owner| owner.battle_music)
            .is_null()
        {
            let resource = if WorldOwner::current().read(in_hyper_space) != 0 {
                HYPERSPACE_MUSIC.as_ptr()
            } else if WorldOwner::current().read(in_quasi_space) != 0 {
                QUASISPACE_MUSIC.as_ptr()
            } else {
                BATTLE_MUSIC.as_ptr()
            };
            (**KernelOwner::current()).battle_music =
                crate::sound::resource_types::LoadMusicInstance(resource.cast()).cast();
        }
        if play != 0 {
            PlayMusic(
                KernelOwner::current().read(|owner| owner.battle_music),
                1,
                1,
            );
        }
    }

    /// # Safety
    /// The game thread must own the battle music handle and initialized audio state.
    #[no_mangle]
    pub unsafe extern "C" fn FreeBattleSong() {
        DestroyMusic(KernelOwner::current().read(|owner| owner.battle_music));
        (**KernelOwner::current()).battle_music = ptr::null_mut();
    }

    unsafe extern "C" fn do_battle(bs: *mut battlestate_struct) -> BOOLEAN {
        crate::log_abi::log_message(
            5,
            &format!(
                "BATTLE_DEBUG: DoBattle called, first_time={}, CHECK_ABORT={}",
                (*bs).first_time,
                (activity() & CHECK_ABORT as u16 != 0) as i32
            ),
        );
        SetMenuSounds(0, 0);
        let net = (*bs).net.clone().unwrap_or_else(|| panic!("missing net"));
        let frame = net.frame();
        if net.peer_count() > 0 && frame.is_multiple_of(NETPLAY_CHECKSUM_INTERVAL) {
            let mut state = mem::zeroed();
            crc_init(&mut state);
            crc_processState(&mut state);
            net.record_checksum(frame, crc_finish(&state));
        }
        crate::app::bridge::enter(|cx| {
            process_input(
                cx,
                &*net,
                battle_input_order(&[player_control(0), player_control(1)], NETWORK_CONTROL),
            )
        });
        if net.peer_count() > 0 {
            let delay = net.input_delay();
            if frame as usize >= delay
                && (frame as usize - delay).is_multiple_of(NETPLAY_CHECKSUM_INTERVAL as usize)
                && activity() & CHECK_ABORT as u16 == 0
                && !net.verify_checksum((frame as usize - delay) as u32)
            {
                set_activity(activity() | CHECK_ABORT as u16);
                net.reset(NetplayResetReason_ResetReason_syncLoss);
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
        match frame_interval(ONE_SECOND, ((*(*bs).battle).nth_frame >> 8) as u8) {
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
        net.set_frame(frame.wrapping_add(1));
        1
    }

    /// # Safety
    /// Player controls and any network-controlled side connections must be initialized.
    #[no_mangle]
    pub unsafe extern "C" fn GetPlayerOrder(i: COUNT) -> COUNT {
        for side in 0..2 {
            if player_control(side) & NETWORK_CONTROL != 0 {
                return player_input(side)
                    .unwrap_or_else(|| panic!("missing player input"))
                    .network()
                    .player_order(i);
            }
        }
        1u16.wrapping_sub(i)
    }

    unsafe fn select_all_ships(count: SIZE) -> BOOLEAN {
        if count == 1 {
            return crate::app::bridge::enter(|cx| {
                crate::battle::ship_native::engine::next_star_ship(cx, None, 0)
            });
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
        let (battle, net, had_net) = battle_network();
        if activity() as u8 != SUPER_MELEE as u8 {
            crate::math::seed_random(rust_automation_seed_value(
                crate::automation::trace::SeedDomain::CAMPAIGN_BATTLE_ID,
                GetTimeCounter(),
            ));
        } else {
            let seed = rust_automation_seed_value(2, 0);
            if seed != 0 {
                crate::math::seed_random(seed);
            }
        }
        BattleSong(0);
        let mut count = InitShips();
        if KernelOwner::current().read(|owner| owner.instant_victory) != 0 {
            count = 0;
            (*battle).counters = [1, 0];
            (**KernelOwner::current()).instant_victory = 0;
        }
        if count != 0 {
            let mut bs: battlestate_struct = mem::zeroed();
            set_activity(activity() | IN_BATTLE as u16);
            (*battle).counters[0] = crate::engine::battle::link_queue::ships(0).count() as u8;
            (*battle).counters[1] = crate::engine::battle::link_queue::ships(1).count() as u8;
            rust_battle_frame_reset();
            if optMeleeScale != SCALE_STEP {
                SetGraphicScaleMode(optMeleeScale);
            }
            bs.battle = battle;
            bs.net = Some(net.clone());
            net.begin((&raw mut bs).cast());
            crate::app::bridge::enter(|cx| reset_winner(cx.kernel));
            crate::log_abi::log_message(
                5,
                &format!(
                    "BATTLE_DEBUG: About to call selectAllShips, CHECK_ABORT={}, IN_BATTLE={}",
                    (activity() & CHECK_ABORT as u16 != 0) as i32,
                    (activity() & IN_BATTLE as u16 != 0) as i32
                ),
            );
            if select_all_ships(count) == 0 {
                set_activity(activity() | CHECK_ABORT as u16);
            } else {
                crate::log_abi::log_message(
                    5,
                    "BATTLE_DEBUG: selectAllShips returned TRUE, calling BattleSong",
                );
                BattleSong(1);
                bs.NextTime = 0;
                net.init_end_frames();
                if !net.negotiate(true, NetState_NetState_inBattle) {
                    set_activity(activity() | CHECK_ABORT as u16);
                } else {
                    bs.InputFunc = Some(do_battle);
                    bs.frame_cb = callback;
                    bs.first_time = WorldOwner::current().read(in_hqspace);
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
                    net.wait_reset(NetState_NetState_inSetup);
                    set_activity(activity() & !(CHECK_ABORT as u16));
                } else {
                    MeleeGameOver();
                }
            }
            net.finish();
            StopDitty();
            stop_music();
            StopSound();
        }
        crate::app::bridge::enter(|cx| cx.finish_battle());
        UninitShips();
        FreeBattleSong();
        finish_battle_network(&mut *battle, had_net);
        (count < 0) as BOOLEAN
    }
}

#[cfg(not(test))]
pub use engine::{
    player_input, set_player_input, Battle, BattleSong, FreeBattleSong, GetPlayerOrder,
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
