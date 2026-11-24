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
pub mod live {
    use super::move_cursor;
    use crate::battle::context_access::ContextOps;
    use crate::battle::{NETWORK_CONTROL, PSYTRON_CONTROL};
    use uqm_engine::graphics::{context::ContextLease, drawable::FrameId, geometry::TextAlign};

    #[derive(Default)]
    pub struct FleetSelection {
        initialized: bool,
        pub player: [PlayerSelection; 2],
        net: Option<std::rc::Rc<dyn uqm_engine::battle::net::BattleNet>>,
    }

    #[derive(Default)]
    pub struct PlayerSelection {
        pub entered_at: u32,
        pub ship: Option<uqm_engine::battle::ShipId>,
        pub row: u16,
        pub col: u16,
        random_index: u16,
        pub selecting: bool,
        pub done: bool,
        highlight: Option<uqm_engine::graphics::highlight::Highlight>,
    }

    const FADE_ALL_TO_COLOR: u32 = 253;

    use uqm_engine::battle::net::NetState;

    use crate::supermelee::native::MELEE_PICK_MASK_PMAP_ANIM;
    use std::ffi::CString;
    use uqm_engine::battle::ShipId;
    use uqm_engine::graphics::geometry::Color;
    use uqm_engine::input_port::frames::{KEY_MENU_CANCEL, KEY_MENU_SELECT};
    use uqm_engine::sound::heart_ffi::wait_for_sound_end;
    use uqm_engine::time::counter_abi::get_time_counter;
    use uqm_engine::world::activity::activity_flags::{CHECK_ABORT, IN_BATTLE};
    fn by_index(
        cx: &mut uqm_engine::battle::Cx<'_>,
        player: usize,
        index: u16,
        used: bool,
    ) -> Option<ShipId> {
        cx.kernel.ships(player).by_index(index, used)
    }
    fn changed(state: &mut FleetSelection, player: usize) {
        let s = &mut state.player[player];
        let r = uqm_engine::graphics::geometry::Rect {
            corner: uqm_engine::graphics::geometry::Point {
                x: 57 + 18 * s.col as i16,
                y: 24 + 18 * s.row as i16 + (1 - player as i16) * 100,
            },
            extent: uqm_engine::graphics::geometry::Extent {
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
        cx: &mut uqm_engine::battle::Cx<'_>,
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
                net.ship_selected(choice);
            }
        }
        s.done = true;
        true
    }
    fn process(
        cx: &mut uqm_engine::battle::Cx<'_>,
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
                changed(state, player);
            }
        }
        1
    }
    pub fn select_ship(
        cx: &mut uqm_engine::battle::Cx<'_>,
        io: &mut uqm_engine::observe::Io<'_>,
        input: &uqm_engine::input::InputSession,
        context: &uqm_engine::battle::input::InputContext,
        state: &mut FleetSelection,
    ) -> u32 {
        use uqm_engine::battle::input::InputSource;
        let player = context.player();
        match context.source() {
            InputSource::Human => process(
                cx,
                state,
                usize::from(player),
                input.pulsed_battle_input(player),
            ),
            InputSource::Computer => {
                if get_time_counter()
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
        cx: &mut uqm_engine::battle::Cx<'_>,
        io: &mut uqm_engine::observe::Io<'_>,
        input: &uqm_engine::input::InputSession,
        state: &mut FleetSelection,
        inputs: &[Option<uqm_engine::battle::input::InputContext>; 2],
    ) -> u32 {
        input.set_menu_sounds(0, 0);
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
            || cx.world.activity() & CHECK_ABORT != 0;
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
            cx.world.set_activity(cx.world.activity() & !CHECK_ABORT);
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
        drawable: uqm_engine::graphics::drawable::LoadedDrawable,
    }
    impl FleetPicker {
        pub(crate) fn new(content: &uqm_engine::resource::Content, context: &ContextLease) -> Self {
            let drawable = uqm_engine::graphics::drawable::LoadedDrawable::create(2, 149, 55, 2)
                .unwrap_or_else(|| panic!("fleet picker allocation"));
            let mask = uqm_engine::graphics::drawable::LoadedDrawable::load(
                content,
                std::ffi::CStr::from_bytes_with_nul(MELEE_PICK_MASK_PMAP_ANIM)
                    .unwrap_or_else(|error| panic!("fleet mask resource: {error}")),
            )
            .unwrap_or_else(|| panic!("fleet picker mask"));
            context.with_selected(|| {
                for player in 0..2 {
                    context.set_frame(drawable.lease().context_frame(player));
                    context.draw_stamp(
                        &mask.lease().context_frame(player),
                        uqm_engine::graphics::geometry::Point { x: 0, y: 0 },
                    );
                }
            });
            Self { drawable }
        }
        pub(crate) fn frames(&self) -> [uqm_engine::graphics::drawable::ContextFrame; 2] {
            std::array::from_fn(|player| self.drawable.lease().context_frame(player))
        }
    }
    fn context(raw: Option<uqm_engine::graphics::context::ContextId>) -> ContextLease {
        ContextLease::retain(raw.unwrap_or_else(|| panic!("picker context")))
    }
    fn font(
        raw: Option<uqm_engine::graphics::font::FontId>,
    ) -> uqm_engine::graphics::font::FontLease {
        uqm_engine::graphics::font::FontLease::retain(raw.unwrap_or_else(|| panic!("picker font")))
    }
    fn retained_frame(raw: FrameId, index: usize) -> uqm_engine::graphics::drawable::ContextFrame {
        let leases = uqm_engine::graphics::drawable::retain_frames(&[raw]);
        leases
            .first()
            .unwrap_or_else(|| panic!("picker drawable absent"))
            .context_frame(index)
    }
    fn frame(
        cx: &uqm_engine::battle::Cx<'_>,
        player: usize,
    ) -> uqm_engine::graphics::drawable::ContextFrame {
        cx.b.selection_frames
            .as_ref()
            .unwrap_or_else(|| panic!("battle selection frames not admitted"))[player]
            .clone()
    }
    fn value(
        cx: &mut uqm_engine::battle::Cx<'_>,
        target: &uqm_engine::graphics::drawable::ContextFrame,
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
                uqm_engine::graphics::geometry::Rect {
                    corner: uqm_engine::graphics::geometry::Point { x: x - 18, y: 2 },
                    extent: uqm_engine::graphics::geometry::Extent {
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
                uqm_engine::graphics::geometry::Point { x, y: 7 },
                TextAlign::Right,
                rgb(19, 0, 0),
            );
        });
    }
    fn draw(cx: &uqm_engine::battle::Cx<'_>, player: usize) {
        let context = context(cx.kernel.space_context);
        context.with_selected(|| {
            context.draw_stamp(
                &frame(cx, player),
                uqm_engine::graphics::geometry::Point {
                    x: 54,
                    y: 15 + (1 - player as i16) * 100,
                },
            )
        });
    }
    fn build_fleet_ship(
        cx: &mut uqm_engine::battle::Cx<'_>,
        master: &uqm_content::ships::master_native::engine::MasterShip,
        side: usize,
        index: u16,
        context: &uqm_engine::graphics::context::ContextLease,
    ) {
        let random = cx.rng;
        let mut queue = cx.kernel.ships(side);
        let captain = queue.captain_name(master.species, random);
        let built = queue
            .build(master.species)
            .unwrap_or_else(|| panic!("battle ship pool exhausted"));
        let icon = uqm_engine::graphics::drawable::ContextFrame::retain(master.info.icons);
        context.draw_stamp(
            &icon,
            uqm_engine::graphics::geometry::Point {
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
        cx: &mut uqm_engine::battle::Cx<'_>,
        setup: &crate::supermelee::setup::team::MeleeSetup,
    ) {
        let context = context(cx.kernel.off_screen_context);
        let font = font(cx.kernel.tiny_font);
        let order = [cx.kernel.player_order(0), cx.kernel.player_order(1)];
        let catalog = uqm_content::ships::master_native::engine::snapshot(cx.kernel);
        context.with_selected(|| {
            for side_i in order {
                let side = usize::from(side_i == 0);
                let frame = frame(cx, side);
                context.set_frame(frame.clone());
                let mut rect = frame.rect();
                let baseline = uqm_engine::graphics::geometry::Point {
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
                    TextAlign::Center,
                    rgb(10, 10, 31),
                );
                let text = CString::new(setup.fleet_value[side_i as usize].to_string())
                    .unwrap_or_else(|error| panic!("fleet value: {error}"));
                context.draw_text(
                    &font,
                    &text,
                    uqm_engine::graphics::geometry::Point { x: 4, y: 7 },
                    TextAlign::Left,
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
    pub(crate) fn ship_death(cx: &mut uqm_engine::battle::Cx<'_>, owner: ShipId) {
        cx.set_ship_species(owner, 0);
        let ship = cx.pick_ship(owner);
        let target = frame(cx, ship.player as usize);
        let context = context(cx.kernel.off_screen_context);
        let icon = retained_frame(cx.kernel.status_frame, 3);
        context.with_selected(|| {
            context.set_frame(target.clone());
            context.draw_stamp(
                &icon,
                uqm_engine::graphics::geometry::Point {
                    x: 3 + 18 * (ship.index % 7) as i16,
                    y: 9 + 18 * (ship.index / 7) as i16,
                },
            );
        });
        value(cx, &target, ship.player as usize);
    }

    use uqm_engine::observe::Io;
    /// Called at the end of a live melee battle on the game thread.
    pub fn melee_game_over(
        cx: &mut uqm_engine::battle::Cx<'_>,
        input: &uqm_engine::input::InputSession,
        io: &mut Io<'_>,
    ) {
        for player in 0..2 {
            draw(cx, player);
        }
        {
            let net = cx.b.network();
            cx.negotiate_battle(net.as_ref(), NetState::InSetup);
            let timeout = get_time_counter().wrapping_add(840 * 4);
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
                uqm_engine::host::process_async();
                cx.kernel.input_runtime().sleep(cx, 840 / 40);
                if cx.world.activity() & CHECK_ABORT != 0
                    || button
                    || (cx.kernel.player_control[0] & cx.kernel.player_control[1] & PSYTRON_CONTROL
                        != 0
                        && get_time_counter() >= timeout)
                {
                    break;
                }
            }
        }
    }
    fn initialize_selection(
        cx: &mut uqm_engine::battle::Cx<'_>,
        state: &mut FleetSelection,
        counters: [u8; 2],
        mask: u16,
        screen_context: Option<uqm_engine::graphics::context::ContextId>,
        controls: [u8; 2],
        order: [u16; 2],
    ) {
        let now = get_time_counter();
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
                screen_context.unwrap_or_else(|| panic!("picker screen context")),
            );
            let mut highlight = context.highlight(uqm_engine::graphics::geometry::Rect {
                corner: uqm_engine::graphics::geometry::Point { x: 0, y: 0 },
                extent: uqm_engine::graphics::geometry::Extent {
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
        cx: &mut uqm_engine::battle::Cx<'_>,
        input: &uqm_engine::input::InputSession,
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
            let mut ok = cx.negotiate_battle(net.as_ref(), NetState::SelectShip);
            {
                input.set_default_menu_repeat(get_time_counter());
            };
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
            {
                wait_for_sound_end(0);
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
            };
            if ok {
                ok = cx.negotiate_battle(net.as_ref(), NetState::InterBattle);
            } else {
                net.set_state(NetState::InterBattle);
            }
            if !ok {
                cx.world.set_activity(cx.world.activity() & !IN_BATTLE);
            }
            net.end_selection();
            ok.then_some(ships)
        }
    }
    /// Both battle fleets and their selection resources must be initialized.
    pub fn get_initial_melee_star_ships(
        cx: &mut uqm_engine::battle::Cx<'_>,
        input: &uqm_engine::input::InputSession,
    ) -> Option<[Option<ShipId>; 2]> {
        for player in 0..2 {
            value(cx, &frame(cx, player), player);
            draw(cx, player);
        }
        let screen = ContextLease::retain(
            cx.kernel
                .screen_context
                .unwrap_or_else(|| panic!("melee selection screen")),
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
        cx: &mut uqm_engine::battle::Cx<'_>,
        input: &uqm_engine::input::InputSession,
        player: u16,
    ) -> Option<ShipId> {
        draw(cx, player as usize);
        get_ships(cx, input, 1 << player)?[player as usize]
    }
}
