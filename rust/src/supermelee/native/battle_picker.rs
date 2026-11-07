//! Between-battle fleet selection from pickmele.c.
fn move_cursor(row: u16, col: u16, input: u8) -> (u16, u16) {
    let mut col = col;
    let mut row = row;
    if input & 1 != 0 {
        col = if col == 0 { 7 } else { col - 1 };
    } else if input & 2 != 0 {
        col = if col == 7 { 0 } else { col + 1 };
    }
    if input & 4 != 0 {
        row = if row == 0 { 1 } else { row - 1 };
    } else if input & 64 != 0 {
        row += 1;
        if row == 2 {
            row = 0;
        }
    }
    (row, col)
}
#[cfg(test)]
mod tests {
    #[test]
    fn selection_wraps_including_random_and_exit_slots() {
        assert_eq!(super::move_cursor(0, 0, 1 | 4), (1, 7));
        assert_eq!(super::move_cursor(1, 7, 2 | 64), (0, 0));
        assert_eq!(super::move_cursor(0, 0, 1 | 2 | 4 | 64), (1, 7));
    }
}
#[cfg(not(test))]
pub mod live {
    use super::move_cursor;
    #[cfg(not(test))]
    use crate::battle::context_access::ContextOps;
    use crate::battle::{NETWORK_CONTROL, PSYTRON_CONTROL};
    use crate::graphics::{context_abi::ContextLease, native_abi as render};

    #[derive(Default)]
    pub struct FleetSelection {
        initialized: bool,
        pub player: [PlayerSelection; 2],
        net: Option<std::rc::Rc<dyn crate::engine::battle::net::BattleNet>>,
    }

    #[derive(Default)]
    pub struct PlayerSelection {
        pub entered_at: u32,
        pub ship: Option<crate::engine::battle::ShipId>,
        pub row: u16,
        pub col: u16,
        random_index: u16,
        pub selecting: bool,
        pub done: bool,
        highlight: Option<crate::graphics::context_abi::Highlight>,
    }

    const FADE_ALL_TO_COLOR: u32 = 253;

    use crate::engine::battle::ShipId;
    use crate::ships::native::abi::*;
    use std::ffi::CString;
    fn by_index(
        cx: &mut crate::engine::battle::Cx<'_>,
        player: usize,
        index: u16,
        used: bool,
    ) -> Option<ShipId> {
        cx.kernel.ships(player).by_index(index, used)
    }
    fn changed(state: &mut FleetSelection, player: usize) {
        let s = &mut state.player[player];
        let r = render::RECT {
            corner: render::POINT {
                x: 57 + 18 * s.col as i16,
                y: 24 + 18 * s.row as i16 + (1 - player as i16) * 100,
            },
            extent: render::EXTENT {
                width: 18,
                height: 18,
            },
        };
        s.highlight
            .as_mut()
            .unwrap_or_else(|| panic!("selecting player highlight"))
            .set_rect(r);
    }
    pub fn set_ship_selected(
        cx: &mut crate::engine::battle::Cx<'_>,
        state: &mut FleetSelection,
        player: u16,
        choice: u16,
        network: bool,
    ) -> bool {
        let s = &mut state.player[player as usize];
        assert!(!s.done);
        let ship = if choice == u16::MAX {
            by_index(cx, player as usize, s.random_index, true)
        } else {
            by_index(cx, player as usize, choice, false)
        };
        let Some(ship) = ship else {
            return false;
        };
        s.ship = Some(ship);
        if network {
            crate::ui::settings::play_menu_sound(cx.kernel, cx.world, 1);
            if let Some(net) = &state.net {
                cx.with_legacy(|| net.ship_selected(choice));
            }
        }
        s.done = true;
        true
    }
    fn process(
        cx: &mut crate::engine::battle::Cx<'_>,
        state: &mut FleetSelection,
        player: usize,
        keys: u8,
    ) -> u32 {
        if keys & 8 != 0 {
            let s = &state.player[player];
            if s.col == 7 && s.row == 0 {
                set_ship_selected(cx, state, player as u16, u16::MAX, true);
            } else if s.col == 7 && s.row == 1 {
                if cx.kernel.input_runtime().confirm_exit(cx) {
                    return 0;
                }
            } else {
                set_ship_selected(cx, state, player as u16, s.row * 7 + s.col, true);
            }
        } else {
            let s = &mut state.player[player];
            let (row, col) = move_cursor(s.row, s.col, keys);
            if (row, col) != (s.row, s.col) {
                s.row = row;
                s.col = col;
                crate::ui::settings::play_menu_sound(cx.kernel, cx.world, 0);
                cx.with_legacy(|| changed(state, player));
            }
        }
        1
    }
    pub fn select_ship(
        cx: &mut crate::engine::battle::Cx<'_>,
        io: &mut crate::engine::observe::Io<'_>,
        input: &crate::engine::input::InputSession,
        context: &crate::engine::battle::input::InputContext,
        state: &mut FleetSelection,
    ) -> u32 {
        use crate::engine::battle::input::InputSource;
        let player = context.player();
        match context.source() {
            InputSource::Human => process(cx, state, usize::from(player), PulsedInputToBattleInput(input, player)),
            InputSource::Computer => {
                if GetTimeCounter()
                    < state.player[usize::from(player)]
                        .entered_at
                        .wrapping_add(840 >> 1)
                {
                    return 1;
                }
                process(cx, state, usize::from(player), 8)
            }
            InputSource::Network(_) => {
                state
                    .net
                    .as_ref()
                    .unwrap_or_else(|| panic!("missing net"))
                    .flush(&mut crate::supermelee::native::protocol::Driver::new(
                        cx,
                        io,
                        &mut crate::supermelee::native::protocol::battle_notice,
                    ));
                if let Some(choice) = state
                    .net
                    .as_ref()
                    .unwrap_or_else(|| panic!("missing net"))
                    .selected(usize::from(player))
                {
                    // SAFETY: the protocol validated the retained fleet slot before recording it.
                    assert!(set_ship_selected(cx, state, player, choice, false));
                }
                1
            }
        }
    }
    fn choose(
        cx: &mut crate::engine::battle::Cx<'_>,
        io: &mut crate::engine::observe::Io<'_>,
        input: &crate::engine::input::InputSession,
        state: &mut FleetSelection,
        inputs: &[Option<crate::engine::battle::input::InputContext>; 2],
    ) -> u32 {
        SetMenuSounds(input, 0, 0);
        if !state.initialized {
            state.initialized = true;
            return 1;
        }
        for p in &mut state.player {
            if p.selecting && !p.done {
                p.highlight
                    .as_mut()
                    .unwrap_or_else(|| panic!("selecting player highlight"))
                    .process();
            }
        }
        cx.kernel.input_runtime().sleep(cx, 840 / 120);
        state
            .net
            .as_ref()
            .unwrap_or_else(|| panic!("missing net"))
            .poll(&mut crate::supermelee::native::protocol::Driver::new(
                cx,
                io,
                &mut crate::supermelee::native::protocol::battle_notice,
            ));
        let mut abort = !state
            .net
            .as_ref()
            .unwrap_or_else(|| panic!("missing net"))
            .all_connected()
            || cx.world.activity() & CHECK_ABORT as u16 != 0;
        let mut done = true;
        if !abort {
            for (player, context) in inputs.iter().enumerate() {
                if !state.player[player].selecting || state.player[player].done {
                    continue;
                }
                if select_ship(
                    cx,
                    io,
                    input,
                    context
                        .as_ref()
                        .unwrap_or_else(|| panic!("missing player input")),
                    state,
                ) == 0
                {
                    abort = true;
                    break;
                }
                let p = &mut state.player[player];
                if p.done {
                    p.highlight = None;
                } else {
                    done = false;
                }
            }
        }
        state
            .net
            .as_ref()
            .unwrap_or_else(|| panic!("missing net"))
            .flush(&mut crate::supermelee::native::protocol::Driver::new(
                cx,
                io,
                &mut crate::supermelee::native::protocol::battle_notice,
            ));
        if abort {
            for p in &mut state.player {
                if p.selecting {
                    p.ship = None;
                }
            }
            cx.world
                .set_activity(cx.world.activity() & !(CHECK_ABORT as u16));
            0
        } else {
            u32::from(!done)
        }
    }
    fn rgb(r: u8, g: u8, b: u8) -> Color {
        Color {
            r: (r << 3) | (r >> 2),
            g: (g << 3) | (g >> 2),
            b: (b << 3) | (b >> 2),
            a: 255,
        }
    }
    pub(crate) struct FleetPicker {
        drawable: crate::graphics::drawable_abi::LoadedDrawable,
    }
    impl FleetPicker {
        pub(crate) fn new(context: &ContextLease) -> Self {
            let drawable = crate::graphics::drawable_abi::LoadedDrawable::create(2, 149, 55, 2)
                .unwrap_or_else(|| panic!("fleet picker allocation"));
            let mask = crate::graphics::drawable_abi::LoadedDrawable::load(
                std::ffi::CStr::from_bytes_with_nul(MELEE_PICK_MASK_PMAP_ANIM)
                    .unwrap_or_else(|error| panic!("fleet mask resource: {error}")),
            )
            .unwrap_or_else(|| panic!("fleet picker mask"));
            context.with_selected(|| {
                for player in 0..2 {
                    context.set_frame(drawable.lease().context_frame(player));
                    context.draw_stamp(
                        &mask.lease().context_frame(player),
                        render::POINT { x: 0, y: 0 },
                    );
                }
            });
            Self { drawable }
        }
        pub(crate) fn frames(&self) -> [crate::graphics::context_abi::ContextFrame; 2] {
            std::array::from_fn(|player| self.drawable.lease().context_frame(player))
        }
    }
    fn context(raw: CONTEXT) -> ContextLease {
        ContextLease::retain(
            std::ptr::NonNull::new(raw.cast()).unwrap_or_else(|| panic!("picker context")),
        )
    }
    fn font(raw: FONT) -> crate::graphics::native_font::FontLease {
        crate::graphics::native_font::FontLease::retain(
            std::ptr::NonNull::new(raw.cast()).unwrap_or_else(|| panic!("picker font")),
        )
    }
    fn retained_frame(raw: FRAME, index: usize) -> crate::graphics::context_abi::ContextFrame {
        let leases = crate::graphics::drawable_abi::retain_frame_allocations(&[raw.cast()]);
        leases
            .first()
            .unwrap_or_else(|| panic!("picker drawable absent"))
            .context_frame(index)
    }
    fn frame(
        cx: &crate::engine::battle::Cx<'_>,
        player: usize,
    ) -> crate::graphics::context_abi::ContextFrame {
        cx.b.selection_frames
            .as_ref()
            .unwrap_or_else(|| panic!("battle selection frames not admitted"))[player]
            .clone()
    }
    fn value(
        cx: &mut crate::engine::battle::Cx<'_>,
        target: &crate::graphics::context_abi::ContextFrame,
        player: usize,
    ) {
        let mut worth = 0u16;
        {
            let queue = cx.kernel.ships(player);
            let mut link = queue.head();
            while let Some(id) = link {
                let ship = queue.get(id);
                if ship.species != 0 {
                    worth = worth.wrapping_add(ship.cost as u16);
                }
                link = queue.succ(id);
            }
        }
        let context = context(cx.kernel.off_screen_context);
        let font = font(cx.kernel.tiny_font);
        context.with_selected(|| {
            context.set_frame(target.clone());
            let x = target.rect().extent.width - 4;
            context.fill_rectangle(
                rgb(0, 1, 15),
                render::RECT {
                    corner: render::POINT { x: x - 18, y: 2 },
                    extent: render::EXTENT {
                        width: 18,
                        height: 5,
                    },
                },
            );
            let text = CString::new(worth.to_string())
                .unwrap_or_else(|error| panic!("fleet worth: {error}"));
            context.draw_text(
                &font,
                &text,
                render::POINT { x, y: 7 },
                TEXT_ALIGN_ALIGN_RIGHT,
                rgb(19, 0, 0),
            );
        });
    }
    fn draw(cx: &crate::engine::battle::Cx<'_>, player: usize) {
        let context = context(cx.kernel.space_context);
        context.with_selected(|| {
            context.draw_stamp(
                &frame(cx, player),
                render::POINT {
                    x: 54,
                    y: 15 + (1 - player as i16) * 100,
                },
            )
        });
    }
    fn build_fleet_ship(
        cx: &mut crate::engine::battle::Cx<'_>,
        master: &crate::ships::master_native::engine::MasterShip,
        side: usize,
        index: u16,
        context: &crate::graphics::context_abi::ContextLease,
    ) {
        let random = cx.rng;
        let mut queue = cx.kernel.ships(side);
        let captain = queue.captain_name(master.species, random);
        let built = queue
            .build(master.species)
            .unwrap_or_else(|| panic!("battle ship pool exhausted"));
        let icon = crate::graphics::context_abi::ContextFrame::retain(std::ptr::NonNull::new(
            master.info.icons.cast(),
        ));
        context.draw_stamp(
            &icon,
            render::POINT {
                x: 4 + 18 * (index % 7) as i16,
                y: 10 + 18 * (index / 7) as i16,
            },
        );
        queue.edit(built, |ship| {
            ship.index = index;
            ship.cost = master.info.ship_cost;
            ship.player = side as i16;
            ship.captain = captain;
            ship.crew = 0;
            ship.max_crew = 0;
        });
        queue.clear_descriptor(built);
    }
    pub(crate) fn fill_pick_melee_frame(
        cx: &mut crate::engine::battle::Cx<'_>,
        setup: &crate::supermelee::setup::team::MeleeSetup,
    ) {
        let context = context(cx.kernel.off_screen_context);
        let font = font(cx.kernel.tiny_font);
        let order = [cx.kernel.player_order(0), cx.kernel.player_order(1)];
        let catalog = crate::ships::master_native::engine::snapshot(cx.kernel);
        context.with_selected(|| {
            for side_i in order {
                let side = usize::from(side_i == 0);
                let frame = frame(cx, side);
                context.set_frame(frame.clone());
                let mut rect = frame.rect();
                let baseline = render::POINT {
                    x: rect.extent.width >> 1,
                    y: rect.extent.height - 7 + 4,
                };
                rect.corner.x += 2;
                rect.corner.y += 2;
                rect.extent.width -= 4 + 18 + 1;
                rect.extent.height -= 4 + 7;
                context.fill_rectangle(rgb(0, 1, 15), rect);
                rect.corner.x += 2;
                rect.extent.width += 18 - 4;
                rect.corner.y += rect.extent.height;
                rect.extent.height = 7;
                context.fill_rectangle(rgb(0, 1, 15), rect);
                context.draw_text(
                    &font,
                    setup.teams[side_i as usize].c_name(),
                    baseline,
                    TEXT_ALIGN_ALIGN_CENTER,
                    rgb(10, 10, 31),
                );
                let text = CString::new(setup.fleet_value[side_i as usize].to_string())
                    .unwrap_or_else(|error| panic!("fleet value: {error}"));
                context.draw_text(
                    &font,
                    &text,
                    render::POINT { x: 4, y: 7 },
                    TEXT_ALIGN_ALIGN_LEFT,
                    rgb(4, 5, 31),
                );
                assert!(cx.kernel.ships(side).head().is_none());
                for index in 0..14 {
                    let ship = setup.teams[side_i as usize].ships[index as usize] as u32;
                    if ship == 255 {
                        continue;
                    }
                    let master = catalog
                        .get(ship as usize)
                        .unwrap_or_else(|| panic!("melee master ship {ship}"));
                    build_fleet_ship(cx, master, side, index, &context);
                }
            }
        });
    }
    pub(crate) fn ship_death(cx: &mut crate::engine::battle::Cx<'_>, owner: ShipId) {
        cx.set_ship_species(owner, 0);
        let ship = cx.pick_ship(owner);
        let target = frame(cx, ship.player as usize);
        let context = context(cx.kernel.off_screen_context);
        let icon = retained_frame(cx.kernel.status_frame, 3);
        context.with_selected(|| {
            context.set_frame(target.clone());
            context.draw_stamp(
                &icon,
                render::POINT {
                    x: 3 + 18 * (ship.index % 7) as i16,
                    y: 9 + 18 * (ship.index / 7) as i16,
                },
            );
        });
        value(cx, &target, ship.player as usize);
    }

    use crate::engine::observe::Io;
    /// Called at the end of a live melee battle on the game thread.
    pub fn melee_game_over(
        cx: &mut crate::engine::battle::Cx<'_>,
        input: &crate::engine::input::InputSession,
        io: &mut Io<'_>,
    ) {
        for player in 0..2 {
            draw(cx, player);
        }
        {
            let net = cx.b.network();
            cx.negotiate_battle(net.as_ref(), NetState_NetState_inSetup);
            let timeout = GetTimeCounter().wrapping_add(840 * 4);
            let mut pressed = input.pulsed().menu[KEY_MENU_SELECT as usize] != 0
                || input.pulsed().menu[KEY_MENU_CANCEL as usize] != 0;
            loop {
                if cx
                    .nested_input(io, |cx| cx.kernel.input_runtime().update(cx))
                    .is_none()
                {
                    break;
                }
                let mut button = input.pulsed().menu[KEY_MENU_SELECT as usize] != 0
                    || input.pulsed().menu[KEY_MENU_CANCEL as usize] != 0;
                if pressed {
                    pressed = button;
                    button = false;
                }
                cx.with_legacy(|| {
                    Async_process();
                });
                cx.kernel.input_runtime().sleep(cx, 840 / 40);
                if cx.world.activity() & CHECK_ABORT as u16 != 0
                    || button
                    || (cx.kernel.player_control[0] & cx.kernel.player_control[1] & PSYTRON_CONTROL
                        != 0
                        && GetTimeCounter() >= timeout)
                {
                    break;
                }
            }
        }
    }
    fn initialize_selection(
        cx: &mut crate::engine::battle::Cx<'_>,
        state: &mut FleetSelection,
        counters: [u8; 2],
        mask: u16,
        screen_context: CONTEXT,
        controls: [u8; 2],
        order: [u16; 2],
    ) {
        let now = GetTimeCounter();
        for player in order.map(usize::from) {
            let p = &mut state.player[player];
            p.selecting = mask & (1 << player) != 0;
            p.random_index = cx.random() as u16 % u16::from(counters[player]);
            p.done = false;
            if !p.selecting {
                continue;
            }
            p.entered_at = now;
            p.row = 0;
            p.col = 7;
            let context = ContextLease::retain(
                std::ptr::NonNull::new(screen_context.cast())
                    .unwrap_or_else(|| panic!("picker screen context")),
            );
            let mut highlight = context.highlight(render::RECT {
                corner: render::POINT { x: 0, y: 0 },
                extent: render::EXTENT {
                    width: 0,
                    height: 0,
                },
            });
            highlight.set_merge_factors(2, 3, 2);
            highlight.set_frame_time(840 / 16);
            if controls[player] & NETWORK_CONTROL != 0 {
                highlight.set_speed(840 / 2, 0, 840 / 2, 0);
            } else {
                highlight.set_speed(0, 840 / 16, 0, 840 / 16);
            }
            p.highlight = Some(highlight);
            changed(state, player);
            state.player[player]
                .highlight
                .as_mut()
                .unwrap_or_else(|| panic!("selecting player highlight"))
                .start();
        }
    }
    fn get_ships(
        cx: &mut crate::engine::battle::Cx<'_>,
        input: &crate::engine::input::InputSession,
        mask: u16,
    ) -> Option<[Option<ShipId>; 2]> {
        {
            let mut state = FleetSelection::default();
            let counters = cx.b.counters;
            let net = cx.b.network();
            state.net = Some(net.clone());
            let screen_context = cx.kernel.screen_context;
            let controls = cx.kernel.player_control;
            let order = [cx.kernel.player_order(0), cx.kernel.player_order(1)];
            {
                let available = std::array::from_fn(|player| {
                    std::array::from_fn(|slot| by_index(cx, player, slot as u16, false).is_some())
                });
                net.begin_selection(mask, available);
                initialize_selection(
                    cx,
                    &mut state,
                    counters,
                    mask,
                    screen_context,
                    controls,
                    order,
                );
            }
            let mut ok = cx.negotiate_battle(net.as_ref(), NetState_NetState_selectShip);
            cx.with_legacy(|| {
                SetDefaultMenuRepeatDelay(input);
            });
            let runtime = cx.kernel.input_runtime();
            let mut observer = runtime.observer();
            let mut io = Io {
                observer: observer.as_mut(),
            };
            cx.run_input(&mut io, false, |cx, io| {
                let inputs = cx.kernel.player_input.clone();
                choose(cx, io, input, &mut state, &inputs) != 0
            });
            let mut ships = [None; 2];
            cx.with_legacy(|| {
                WaitForSoundEnd(0);
                for (player, p) in state.player.iter_mut().enumerate() {
                    if !p.selecting {
                        continue;
                    }
                    if p.done {
                        ships[player] = p.ship;
                    } else {
                        p.highlight = None;
                        ok = false;
                    }
                }
            });
            if ok {
                ok = cx.negotiate_battle(net.as_ref(), NetState_NetState_interBattle);
            } else {
                net.set_state(NetState_NetState_interBattle);
            }
            if !ok {
                cx.world
                    .set_activity(cx.world.activity() & !(IN_BATTLE as u16));
            }
            net.end_selection();
            ok.then_some(ships)
        }
    }
    /// Both battle fleets and their selection resources must be initialized.
    pub fn get_initial_melee_star_ships(
        cx: &mut crate::engine::battle::Cx<'_>,
        input: &crate::engine::input::InputSession,
    ) -> Option<[Option<ShipId>; 2]> {
        for player in 0..2 {
            value(cx, &frame(cx, player), player);
            draw(cx, player);
        }
        let screen = ContextLease::retain(
            std::ptr::NonNull::new(cx.kernel.screen_context).expect("melee selection screen"),
        );
        let deadline = screen
            .fade_screen(FADE_ALL_TO_COLOR, 840 / 2)
            .wrapping_add(840 / 60);
        cx.kernel.input_runtime().sleep_until(cx, deadline);
        screen.finish_color_transforms();
        get_ships(cx, input, 3)
    }
    /// The selected player's battle fleet and selection resources must be initialized.
    pub fn get_next_melee_star_ship(
        cx: &mut crate::engine::battle::Cx<'_>,
        input: &crate::engine::input::InputSession,
        player: u16,
    ) -> Option<ShipId> {
        draw(cx, player as usize);
        get_ships(cx, input, 1 << player)?[player as usize]
    }
}
