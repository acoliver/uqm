//! Battle entry, ordered input processing, and frame scheduling from `battle.c`.

use crate::battle::context_access::ContextOps;

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
        state |= uqm_engine::battle::ai_constants::LEFT as u8;
    } else if input & BATTLE_RIGHT != 0 {
        state |= uqm_engine::battle::ai_constants::RIGHT as u8;
    }
    for (battle, ship) in [
        (BATTLE_THRUST, uqm_engine::battle::ai_constants::THRUST),
        (BATTLE_WEAPON, uqm_engine::battle::ai_constants::WEAPON),
        (BATTLE_SPECIAL, uqm_engine::battle::ai_constants::SPECIAL),
    ] {
        if input & battle != 0 {
            state |= ship as u8;
        }
    }
    state
}

fn run_away_allowed(world: &uqm_engine::World) -> bool {
    use uqm_engine::StoryKey;
    (world.activity() as u8 == uqm_engine::world::activity::activity_kinds::IN_ENCOUNTER as u8
        || world.activity() as u8 == uqm_engine::battle::flags::IN_LAST_BATTLE as u8)
        && world.story_state(StoryKey::STARBASE_AVAILABLE) != 0
        && world.story_state(StoryKey::BOMB_CARRIER) == 0
}

fn seed(
    cx: &mut uqm_engine::battle::Cx<'_>,
    io: &mut uqm_engine::observe::Io<'_>,
    domain: u32,
    fallback: u32,
) -> u32 {
    io.observer
        .input_admitted(
            &uqm_engine::observe::InputView::Seed { domain, fallback },
            cx,
        )
        .seed
        .unwrap_or(fallback)
}

fn reset_observation(io: &mut uqm_engine::observe::Io<'_>) {
    uqm_engine::collections::observation::reset();
    io.observer
        .checkpoint(&uqm_engine::observe::CheckpointView::BattleReset);
}

fn advance_observation(cx: &uqm_engine::battle::Cx<'_>, io: &mut uqm_engine::observe::Io<'_>) {
    uqm_engine::collections::observation::advance();
    io.observer
        .checkpoint(&uqm_engine::observe::CheckpointView::BattleAdvanced);
    io.observer.simulation(cx, "battle");
}

fn initialize_battle(
    cx: &mut uqm_engine::battle::Cx<'_>,
) -> (std::rc::Rc<dyn uqm_engine::battle::net::BattleNet>, bool) {
    cx.b.initialize(cx.kernel);
    let had_net = cx.b.net.is_some();
    cx.b.net
        .get_or_insert_with(|| crate::supermelee::netplay::NetplaySession::default().battle_net());
    let net = cx.b.network();
    for side in 0..2 {
        if cx.kernel.player_control[side] & engine::NETWORK_CONTROL != 0 {
            cx.kernel.player_input[side]
                .as_mut()
                .unwrap_or_else(|| panic!("missing player input"))
                .connect(net.clone());
        }
    }
    (net, had_net)
}

pub(crate) mod engine {
    use super::*;
    use crate::battle::netplay::CrcState as crc_State;
    use crate::battle::tactrans_native::{flee_preprocess, reset_winner, stop_ditty};
    use uqm_engine::battle::ai_constants::MAX_SHIP_MASS;
    use uqm_engine::battle::flags::{
        APPEARING, FINITE_LIFE, NORMAL_LIFE, SHIP_AT_MAX_SPEED, SHIP_BEYOND_MAX_SPEED,
        STAMPFILL_PRIMITIVE,
    };
    use uqm_engine::battle::net::NetState;

    use crate::supermelee::netplay::crc::{crc_finish, crc_init};
    use crate::supermelee::netplay::{ResetReason, CHECKSUM_INTERVAL};
    use uqm_engine::battle::ElementHook;
    use uqm_engine::battle::STAMP_PRIMITIVE;
    use uqm_engine::graphics::geometry::Color;
    use uqm_engine::graphics::geometry::Extent;
    use uqm_engine::graphics::geometry::Point;
    use uqm_engine::graphics::geometry::Rect;
    use uqm_engine::sound::heart_ffi::stop_sound;
    use uqm_engine::time::counter_abi::get_time_counter;
    use uqm_engine::time::counter_abi::ONE_SECOND;
    use uqm_engine::world::activity::activity_flags::{CHECK_ABORT, CHECK_LOAD, IN_BATTLE};
    use uqm_engine::world::activity::activity_kinds::SUPER_MELEE;
    const SIS_ORG_X: u32 = 7;
    const SIS_ORG_Y: u32 = 10;

    const BATTLE_ESCAPE: u8 = 1 << 5;

    pub const NETWORK_CONTROL: u8 = 1 << 3;
    pub const HUMAN_CONTROL: u8 = 1;
    pub const CYBORG_CONTROL: u8 = 1 << 1;
    pub const STANDARD_RATING: u8 = 1 << 4;
    pub const AWESOME_RATING: u8 = 1 << 6;
    // gfx_common.h SCALE enum starts with the stepped scaler.
    pub const SCALE_STEP: i32 = 0;

    use crate::battle::process_native::engine::redraw_queue;
    use uqm_engine::observe::Io;

    pub const fn rgb15(r: u8, g: u8, b: u8) -> Color {
        Color {
            r: (r << 3) | (r >> 2),
            g: (g << 3) | (g >> 2),
            b: (b << 3) | (b >> 2),
            a: 255,
        }
    }

    fn run_away(
        cx: &mut uqm_engine::battle::Cx<'_>,
        io: &mut Io<'_>,
        ship: uqm_engine::battle::ShipId,
    ) {
        let e = cx.ship_element(ship);
        let state = cx.element(e);
        if cx.primitive_style(e).kind == STAMP_PRIMITIVE
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
            cx.primitive_style_mut(e).kind = STAMPFILL_PRIMITIVE;
            cx.ship_state_mut(ship).input = 0;
            {
                io.observer
                    .checkpoint(&uqm_engine::observe::CheckpointView::ShipEscaped);
            };
        }
    }

    fn process_input(
        cx: &mut uqm_engine::battle::Cx<'_>,
        io: &mut Io<'_>,
        net: &dyn uqm_engine::battle::net::BattleNet,
        order: [usize; 2],
    ) {
        crate::supermelee::native::protocol::with_battle_driver(cx, io, |driver| net.poll(driver));
        let can_run = run_away_allowed(cx.world);
        for side in order {
            let mut next = cx.first_ship(side);
            while let Some(ship) = next {
                next = cx.next_ship(ship);
                if cx.optional_ship_element(ship).is_some() {
                    cx.ship_state_mut(ship).control = cx.kernel.player_control[side];
                    let context = cx.kernel.player_input[side]
                        .clone()
                        .unwrap_or_else(|| panic!("missing player input"));
                    let mut input = match context.source() {
                        uqm_engine::battle::input::InputSource::Human => {
                            cx.human_input(context.player())
                        }
                        uqm_engine::battle::input::InputSource::Computer => {
                            crate::battle::context_access::ai_legacy::computer_intelligence(
                                cx,
                                context.player(),
                                Some(ship),
                            )
                        }
                        uqm_engine::battle::input::InputSource::Network(_) => {
                            let activity = cx.world.activity_control();
                            crate::supermelee::native::protocol::with_battle_driver(
                                cx,
                                io,
                                |driver| context.network().remote_input(side, &activity, driver),
                            )
                        }
                    };
                    if cx.kernel.player_control[side] & NETWORK_CONTROL == 0 {
                        let activity = cx.world.activity_control();
                        input = crate::supermelee::native::protocol::with_battle_driver(
                            cx,
                            io,
                            |driver| net.local_input(side, input, &activity, driver),
                        );
                    }
                    cx.ship_state_mut(ship).input =
                        ship_input(input, cx.race_stats(ship).ship_info.crew_level);
                    if cx.race_stats(ship).ship_info.crew_level != 0
                        && can_run
                        && side == 0
                        && input & BATTLE_ESCAPE != 0
                    {
                        run_away(cx, io, ship);
                    }
                }
            }
        }
        crate::supermelee::native::protocol::with_battle_driver(cx, io, |driver| net.flush(driver));
        if cx.world.activity() & (CHECK_LOAD | CHECK_ABORT) != 0 {
            cx.world.set_activity(cx.world.activity() & !IN_BATTLE);
        }
    }

    pub(crate) fn battle_song(
        kernel: &mut uqm_engine::Kernel,
        world: &uqm_engine::World,
        play: bool,
    ) {
        if kernel.battle_music.is_none() {
            let resource = if world.in_hyper_space() {
                c"music.hyperspace"
            } else if world.in_quasi_space() {
                c"music.quasispace"
            } else {
                c"music.battle"
            };
            kernel.battle_music =
                uqm_engine::sound::resource_types::LoadedMusic::load(&kernel.content(), resource)
                    .ok();
        }
        if play {
            crate::ui::settings::play_loaded_music(world, kernel.battle_music.as_ref(), 1, 1);
        }
    }

    pub fn free_battle_song(kernel: &mut uqm_engine::Kernel) {
        kernel.battle_music = None;
    }

    struct BattleSession {
        first_time: bool,
        next_time: u32,
    }

    fn do_battle(
        cx: &mut uqm_engine::battle::Cx<'_>,
        io: &mut Io<'_>,
        session: &mut BattleSession,
        net: &dyn uqm_engine::battle::net::BattleNet,
        callback: Option<fn(&mut uqm_engine::battle::Cx<'_>, &mut Io<'_>)>,
    ) -> bool {
        cx.kernel.input_controls().set_menu_sounds(0, 0);
        let frame = net.frame();
        if net.peer_count() > 0 && frame.is_multiple_of(CHECKSUM_INTERVAL as u32) {
            let mut state = crc_State::new();
            crc_init(&mut state);
            crate::supermelee::netplay::checksum::crc_process_game_state(cx, &mut state);
            crate::supermelee::native::protocol::with_battle_driver(cx, io, |driver| {
                net.record_checksum(frame, crc_finish(&state), driver)
            });
        }
        process_input(
            cx,
            io,
            net,
            battle_input_order(&cx.kernel.player_control, NETWORK_CONTROL),
        );
        if net.peer_count() > 0 {
            let delay = net.input_delay();
            if frame as usize >= delay
                && (frame as usize - delay).is_multiple_of(CHECKSUM_INTERVAL)
                && cx.world.activity() & CHECK_ABORT == 0
                && !net.verify_checksum((frame as usize - delay) as u32)
            {
                cx.world.set_activity(cx.world.activity() | CHECK_ABORT);
                crate::supermelee::native::protocol::with_battle_driver(cx, io, |driver| {
                    net.reset(ResetReason::SyncLoss as u32, driver)
                });
            }
        }
        let context = uqm_engine::graphics::context::ContextLease::require(cx.kernel.space_context);
        let (width, height) = cx.screen_dimensions();
        let rect = Rect {
            corner: Point {
                x: SIS_ORG_X as i16,
                y: SIS_ORG_Y as i16,
            },
            extent: Extent {
                width: (width - 78) as i16,
                height: (height - 13) as i16,
            },
        };
        if session.first_time {
            context.transition_source(Some(rect));
        }
        context.with_batch(|| {
            if let Some(callback) = callback {
                callback(cx, io);
            }
            redraw_queue(cx, true);
            if session.first_time {
                session.first_time = false;
                context.transition(Some(rect));
            }
        });
        if cx.world.activity() & IN_BATTLE == 0
            || cx.world.activity() & (CHECK_ABORT | CHECK_LOAD) != 0
        {
            return false;
        }
        match frame_interval(ONE_SECOND, (cx.b.nth_frame >> 8) as u8) {
            None => cx.kernel.input_runtime().yield_poll(cx),
            Some(interval) => {
                cx.kernel
                    .input_runtime()
                    .sleep_until(cx, session.next_time.wrapping_add(interval));
                session.next_time = get_time_counter();
            }
        }
        if cx.world.activity() & IN_BATTLE == 0 {
            return false;
        }
        complete_observed_frame(cx, io);
        net.set_frame(frame.wrapping_add(1));
        true
    }

    fn complete_observed_frame(cx: &mut uqm_engine::battle::Cx<'_>, io: &mut Io<'_>) {
        uqm_engine::observe::battle::observe_tick(
            cx,
            io,
            uqm_engine::observe::battle::BattlePhase::Advanced,
        );
        advance_observation(cx, io);
    }

    fn select_all_ships(cx: &mut uqm_engine::battle::Cx<'_>, count: i16) -> u32 {
        let controls = cx.kernel.player_control;
        if count == 1 {
            return crate::battle::ship_native::engine::next_star_ship(cx, None, 0);
        }
        if controls[0] & NETWORK_CONTROL != 0 && controls[1] & NETWORK_CONTROL != 0 {
            uqm_engine::log_abi::log_message(
                uqm_engine::logging::LogLevel::Error,
                "Only one side at a time can be network controlled.\n",
            );
            return 0;
        }
        crate::battle::ship_native::initial_star_ships(cx)
    }

    fn finish_battle(cx: &mut uqm_engine::battle::Cx<'_>, stop_audio: bool) {
        if stop_audio {
            stop_ditty(cx.kernel);
            crate::ui::settings::stop_battle_music();
            stop_sound();
        }
        cx.finish_battle();
    }

    fn seed_battle_rng(
        activity: u8,
        campaign_domain: u32,
        seed_value: impl FnOnce(u32, u32) -> u32,
    ) {
        if activity != SUPER_MELEE as u8 {
            uqm_engine::math::seed_random(seed_value(campaign_domain, get_time_counter()));
        } else {
            let seed = seed_value(2, 0);
            if seed != 0 {
                uqm_engine::math::seed_random(seed);
            }
        }
    }

    pub fn battle(
        cx: &mut uqm_engine::battle::Cx<'_>,
        io: &mut Io<'_>,
        callback: Option<fn(&mut uqm_engine::battle::Cx<'_>, &mut Io<'_>)>,
    ) -> bool {
        let (net, had_net) = initialize_battle(cx);
        let activity = cx.world.activity() as u8;
        seed_battle_rng(activity, 4, |domain, fallback| {
            seed(cx, io, domain, fallback)
        });
        battle_song(cx.kernel, cx.world, false);
        let ships = cx.kernel.ship_lifecycle();
        let mut count = (ships.init)(cx);
        if cx.kernel.instant_victory != 0 {
            count = 0;
            cx.b.counters = [1, 0];
            cx.kernel.instant_victory = 0;
        }
        if count != 0 {
            cx.world.set_activity(cx.world.activity() | IN_BATTLE);
            cx.b.counters = std::array::from_fn(|side| cx.kernel.ships(side).count() as u8);
            reset_observation(io);
            if cx.kernel.configuration.preferences().melee_scale != SCALE_STEP {
                uqm_engine::graphics::context::ContextLease::require(cx.kernel.space_context)
                    .scale_mode(cx.kernel.configuration.preferences().melee_scale);
            }
            let mut session = BattleSession {
                first_time: false,
                next_time: 0,
            };
            net.begin();
            reset_winner(cx.kernel);
            if select_all_ships(cx, count) == 0 {
                cx.world.set_activity(cx.world.activity() | CHECK_ABORT);
            } else {
                battle_song(cx.kernel, cx.world, true);
                net.init_end_frames();
                if !crate::supermelee::native::protocol::with_battle_driver(cx, io, |driver| {
                    net.negotiate(true, NetState::InBattle, driver)
                }) {
                    cx.world.set_activity(cx.world.activity() | CHECK_ABORT);
                } else {
                    session.first_time = cx.world.in_hqspace();
                    cx.run_input(io, false, |cx, io| {
                        do_battle(cx, io, &mut session, &*net, callback)
                    });
                }
            }
            if cx.world.activity() as u8 == SUPER_MELEE as u8 {
                if cx.world.activity() & CHECK_ABORT != 0 {
                    crate::supermelee::native::protocol::with_battle_driver(cx, io, |driver| {
                        net.wait_reset(NetState::InSetup, driver)
                    });
                    cx.world.set_activity(cx.world.activity() & !CHECK_ABORT);
                } else {
                    let runtime = cx.kernel.input_runtime();
                    crate::supermelee::native::battle_picker::live::melee_game_over(
                        cx,
                        runtime.input(),
                        io,
                    );
                }
            }
            net.finish();
        }
        finish_battle(cx, count != 0);
        (ships.uninit)(cx);
        free_battle_song(cx.kernel);
        if !had_net {
            cx.b.net = None;
        }
        count < 0
    }
}

pub use engine::{battle, free_battle_song};

#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::RefCell, rc::Rc};
    use uqm_engine::battle::Cx;
    use uqm_engine::observe::{CheckpointView, Injected, InputView, Io, Observer, Presented};

    fn with_context(body: impl FnOnce(&mut Cx<'_>)) {
        let mut game = uqm_engine::Game::new();
        let mut battle = uqm_engine::battle::Battle::new();
        body(&mut Cx::new(
            &mut battle,
            &mut game.world,
            &mut game.kernel,
            uqm_engine::math::random,
        ));
    }

    #[derive(Debug, PartialEq)]
    enum Observed {
        Fallback,
        Seed(u32, u32),
        Reset(u64),
        Advanced(u64),
        Simulated(u64),
    }

    struct Recording {
        events: Rc<RefCell<Vec<Observed>>>,
        seed: Option<u32>,
    }

    impl Observer for Recording {
        fn input_admitted(&mut self, view: &InputView, _: &mut Cx<'_>) -> Injected {
            let InputView::Seed { domain, fallback } = *view else {
                panic!("unexpected input {view:?}");
            };
            self.events
                .borrow_mut()
                .push(Observed::Seed(domain, fallback));
            Injected {
                seed: self.seed,
                stop: false,
                planet_side: None,
                comm_response: None,
            }
        }

        fn simulation(&mut self, _: &Cx<'_>, boundary: &str) {
            assert_eq!(boundary, "battle");
            let frame = uqm_engine::collections::observation::current_frame();
            self.events.borrow_mut().push(Observed::Simulated(frame));
        }

        fn checkpoint(&mut self, view: &CheckpointView<'_>) {
            let frame = uqm_engine::collections::observation::current_frame();
            self.events.borrow_mut().push(match view {
                CheckpointView::BattleReset => Observed::Reset(frame),
                CheckpointView::BattleAdvanced => Observed::Advanced(frame),
                _ => panic!("unexpected checkpoint {view:?}"),
            });
        }

        fn presented(&mut self, _: &mut Presented) {
            panic!("unexpected presentation");
        }
    }

    #[test]
    fn supplied_seed_observer_keeps_domains_and_eager_fallback_order() {
        let events = Rc::new(RefCell::new(Vec::new()));
        let mut observer = Recording {
            events: events.clone(),
            seed: Some(17),
        };
        with_context(|cx| {
            let mut io = Io {
                observer: &mut observer,
            };
            assert_eq!(
                seed(cx, &mut io, 4, {
                    events.borrow_mut().push(Observed::Fallback);
                    123
                }),
                17
            );
            assert_eq!(seed(cx, &mut io, 2, 0), 17);
            observer.seed = None;
            let mut io = Io {
                observer: &mut observer,
            };
            assert_eq!(seed(cx, &mut io, 4, 456), 456);
            assert_eq!(seed(cx, &mut io, 2, 0), 0);
        });
        assert_eq!(
            *events.borrow(),
            [
                Observed::Fallback,
                Observed::Seed(4, 123),
                Observed::Seed(2, 0),
                Observed::Seed(4, 456),
                Observed::Seed(2, 0),
            ]
        );
    }

    #[test]
    #[serial_test::serial]
    fn retained_battle_observer_sees_counter_changes_before_checkpoints() {
        let events = Rc::new(RefCell::new(Vec::new()));
        let mut observer = Recording {
            events: events.clone(),
            seed: None,
        };
        uqm_engine::collections::observation::advance();
        with_context(|cx| {
            let mut io = Io {
                observer: &mut observer,
            };
            reset_observation(&mut io);
            advance_observation(cx, &mut io);
            advance_observation(cx, &mut io);
        });
        assert_eq!(
            *events.borrow(),
            [
                Observed::Reset(0),
                Observed::Advanced(1),
                Observed::Simulated(1),
                Observed::Advanced(2),
                Observed::Simulated(2)
            ]
        );
    }

    #[test]
    fn retreat_permission_uses_the_supplied_world_and_low_activity_byte() {
        use uqm_engine::{StoryKey, World};
        let mut world = World::new();
        world.story = Some(uqm_engine::state::game_state::StoryState::new());
        for activity in 0..=4 {
            for flags in [
                0,
                uqm_engine::world::activity::activity_flags::IN_BATTLE,
                uqm_engine::world::activity::activity_flags::CHECK_ABORT,
            ] {
                world.set_activity(activity | flags);
                for starbase in 0..=1 {
                    world.set_story_state(StoryKey::STARBASE_AVAILABLE, starbase);
                    for bomb in 0..=1 {
                        world.set_story_state(StoryKey::BOMB_CARRIER, bomb);
                        assert_eq!(
                            run_away_allowed(&world),
                            (activity == uqm_engine::world::activity::activity_kinds::IN_ENCOUNTER
                                || activity == uqm_engine::battle::flags::IN_LAST_BATTLE as u16)
                                && starbase != 0
                                && bomb == 0,
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn input_mapping_matches_all_byte_patterns() {
        for input in 0u8..=255 {
            let mut expected = 0;
            if input & 1 != 0 {
                expected |= uqm_engine::battle::ai_constants::LEFT as u8;
            } else if input & 2 != 0 {
                expected |= uqm_engine::battle::ai_constants::RIGHT as u8;
            }
            if input & 4 != 0 {
                expected |= uqm_engine::battle::ai_constants::THRUST as u8;
            }
            if input & 8 != 0 {
                expected |= uqm_engine::battle::ai_constants::WEAPON as u8;
            }
            if input & 16 != 0 {
                expected |= uqm_engine::battle::ai_constants::SPECIAL as u8;
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
            (uqm_engine::battle::ai_constants::LEFT
                | uqm_engine::battle::ai_constants::THRUST
                | uqm_engine::battle::ai_constants::WEAPON
                | uqm_engine::battle::ai_constants::SPECIAL) as u8
        );
        assert_eq!(ship_input(both, 0), 0);
    }
}
