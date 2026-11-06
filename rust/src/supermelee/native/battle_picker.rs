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
    use crate::battle::{NETWORK_CONTROL, PSYTRON_CONTROL};
    use crate::engine::battle::schema::GETMELEE_STATE;
    use crate::engine::Kernel as KernelOwner;
    use crate::graphics::{context_abi::ContextLease, native_abi as render};

    const FADE_ALL_TO_COLOR: u32 = 253;

    use crate::engine::battle::{
        link_queue::{self, LinkQueue, Starship},
        ShipId,
    };
    use crate::graphics::drawable_abi;
    use crate::ships::native::abi::*;
    use std::{ffi::CString, mem, ptr};
    #[no_mangle]
    pub static mut PickMeleeFrame: FRAME = ptr::null_mut();
    unsafe fn by_index(player: usize, index: u16, used: bool) -> Option<ShipId> {
        link_queue::ships(player).by_index(index, used)
    }
    fn changed(state: &mut GETMELEE_STATE, player: usize) {
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
    /// # Safety
    /// The selection and live ship queues belong to the game thread.
    pub unsafe fn set_ship_selected(
        cx: &mut crate::engine::battle::Cx<'_>,
        state: *mut GETMELEE_STATE,
        player: u16,
        choice: u16,
        network: bool,
    ) -> bool {
        let s = &mut (*state).player[player as usize];
        assert!(s.done == 0);
        let ship = if choice == u16::MAX {
            cx.with_legacy(|| by_index(player as usize, s.randomIndex, true))
        } else {
            cx.with_legacy(|| by_index(player as usize, choice, false))
        };
        let Some(ship) = ship else {
            return false;
        };
        s.choice = choice;
        s.hBattleShip = Some(ship);
        if network {
            crate::ui::settings::play_menu_sound(cx.kernel, cx.world, 1);
            if let Some(net) = &(*state).net {
                cx.with_legacy(|| net.ship_selected(choice));
            }
        }
        s.done = 1;
        true
    }
    unsafe fn process(
        cx: &mut crate::engine::battle::Cx<'_>,
        input: &crate::engine::input::InputSession,
        state: *mut GETMELEE_STATE,
        player: usize,
        keys: u8,
    ) -> u32 {
        if keys & 8 != 0 {
            let s = &(*state).player[player];
            if s.col == 7 && s.row == 0 {
                set_ship_selected(cx, state, player as u16, u16::MAX, true);
            } else if s.col == 7 && s.row == 1 {
                if ConfirmExit(cx, input) != 0 {
                    return 0;
                }
            } else {
                set_ship_selected(cx, state, player as u16, s.row * 7 + s.col, true);
            }
        } else {
            let s = &mut (*state).player[player];
            let (row, col) = move_cursor(s.row, s.col, keys);
            if (row, col) != (s.row, s.col) {
                s.row = row;
                s.col = col;
                crate::ui::settings::play_menu_sound(cx.kernel, cx.world, 0);
                cx.with_legacy(|| changed(&mut *state, player));
            }
        }
        1
    }
    pub fn select_ship(
        cx: &mut crate::engine::battle::Cx<'_>,
        io: &mut crate::engine::observe::Io<'_>,
        input: &crate::engine::input::InputSession,
        context: &crate::engine::battle::input::InputContext,
        state: &mut GETMELEE_STATE,
    ) -> u32 {
        use crate::engine::battle::input::InputSource;
        let player = context.player();
        match context.source() {
            InputSource::Human => {
                // SAFETY: the picker owns its selection state and reads game-thread input.
                unsafe {
                    process(
                        cx,
                        input,
                        state,
                        usize::from(player),
                        PulsedInputToBattleInput(input, player),
                    )
                }
            }
            InputSource::Computer => {
                if GetTimeCounter()
                    < state.player[usize::from(player)]
                        .timeIn
                        .wrapping_add(840 >> 1)
                {
                    return 1;
                }
                // SAFETY: the picker owns the initialized fleet selection for this player.
                unsafe { process(cx, input, state, usize::from(player), 8) }
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
                    assert!(unsafe { set_ship_selected(cx, state, player, choice, false) });
                    state.player[usize::from(player)].remoteSelected = 1;
                }
                1
            }
        }
    }
    unsafe fn choose(
        cx: &mut crate::engine::battle::Cx<'_>,
        io: &mut crate::engine::observe::Io<'_>,
        input: &crate::engine::input::InputSession,
        state: *mut GETMELEE_STATE,
        inputs: &[Option<crate::engine::battle::input::InputContext>; 2],
    ) -> u32 {
        SetMenuSounds(input, 0, 0);
        if (*state).Initialized == 0 {
            (*state).Initialized = 1;
            return 1;
        }
        for p in &mut (*state).player {
            if p.selecting != 0 && p.done == 0 {
                p.highlight
                    .as_mut()
                    .unwrap_or_else(|| panic!("selecting player highlight"))
                    .process();
            }
        }
        cx.with_legacy(|| SleepThread(840 / 120));
        (*state)
            .net
            .as_ref()
            .unwrap_or_else(|| panic!("missing net"))
            .poll(&mut crate::supermelee::native::protocol::Driver::new(
                cx,
                io,
                &mut crate::supermelee::native::protocol::battle_notice,
            ));
        let mut abort = !(*state)
            .net
            .as_ref()
            .unwrap_or_else(|| panic!("missing net"))
            .all_connected()
            || cx.world.activity() & CHECK_ABORT as u16 != 0;
        let mut done = true;
        if !abort {
            for (player, context) in inputs.iter().enumerate() {
                if (*state).player[player].selecting == 0 || (*state).player[player].done != 0 {
                    continue;
                }
                if select_ship(
                    cx,
                    io,
                    input,
                    context
                        .as_ref()
                        .unwrap_or_else(|| panic!("missing player input")),
                    &mut *state,
                ) == 0
                {
                    abort = true;
                    break;
                }
                let p = &mut (*state).player[player];
                if p.done != 0 {
                    p.highlight = None;
                } else {
                    done = false;
                }
            }
        }
        (*state)
            .net
            .as_ref()
            .unwrap_or_else(|| panic!("missing net"))
            .flush(&mut crate::supermelee::native::protocol::Driver::new(
                cx,
                io,
                &mut crate::supermelee::native::protocol::battle_notice,
            ));
        if abort {
            for p in &mut (*state).player {
                if p.selecting != 0 {
                    p.choice = 0;
                    p.hBattleShip = None;
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
    unsafe fn value(frame: FRAME, player: usize) {
        let mut worth = 0u16;
        {
            let queue = link_queue::ships(player);
            let mut link = queue.head();
            while let Some(id) = link {
                let ship = queue.get(id);
                if ship.species != 0 {
                    worth = worth.wrapping_add(ship.cost as u16);
                }
                link = queue.succ(id);
            }
        }
        let old = SetContext(KernelOwner::current().read(|owner| owner.off_screen_context));
        drawable_abi::SetContextFGFrame(frame.cast());
        let mut r = mem::zeroed();
        GetFrameRect(frame, &mut r);
        let x = r.extent.width - 4;
        r.corner = POINT { x: x - 18, y: 2 };
        r.extent = EXTENT {
            width: 18,
            height: 5,
        };
        SetContextForeGroundColor(rgb(0, 1, 15));
        DrawFilledRectangle(&mut r);
        let buf = CString::new(worth.to_string()).unwrap();
        let mut t = TEXT {
            baseline: POINT { x, y: 7 },
            align: TEXT_ALIGN_ALIGN_RIGHT,
            pStr: buf.as_ptr(),
            CharCount: u16::MAX,
        };
        SetContextFont(KernelOwner::current().read(|owner| owner.tiny_font));
        SetContextForeGroundColor(rgb(19, 0, 0));
        font_DrawText(&mut t);
        SetContext(old);
    }
    unsafe fn draw(player: usize) {
        let old = SetContext(KernelOwner::current().read(|owner| owner.space_context));
        let mut s = STAMP {
            origin: POINT {
                x: 54,
                y: 15 + (1 - player as i16) * 100,
            },
            frame: SetAbsFrameIndex(PickMeleeFrame, player as u16),
        };
        DrawStamp(&mut s);
        SetContext(old);
    }
    /// # Safety
    /// Melee graphics and master ship list must be loaded.
    #[no_mangle]
    pub unsafe extern "C" fn BuildPickMeleeFrame() {
        let old = SetContext(KernelOwner::current().read(|owner| owner.off_screen_context));
        PickMeleeFrame = CaptureDrawable(drawable_abi::CreateDrawable(2, 149, 55, 2).cast());
        let mut s = STAMP {
            origin: POINT { x: 0, y: 0 },
            frame: CaptureDrawable(
                crate::graphics::resgfx::LoadGraphicInstance(
                    MELEE_PICK_MASK_PMAP_ANIM.as_ptr().cast(),
                )
                .cast(),
            ),
        };
        drawable_abi::SetContextFGFrame(PickMeleeFrame.cast());
        DrawStamp(&mut s);
        s.frame = IncFrameIndex(s.frame);
        drawable_abi::SetContextFGFrame(IncFrameIndex(PickMeleeFrame).cast());
        DrawStamp(&mut s);
        DestroyDrawable(ReleaseDrawable(s.frame));
        SetContext(old);
    }
    /// # Safety
    /// Frame must be live, game thread only.
    #[no_mangle]
    pub unsafe extern "C" fn DestroyPickMeleeFrame() {
        DestroyDrawable(ReleaseDrawable(PickMeleeFrame));
        PickMeleeFrame = ptr::null_mut();
    }
    /// # Safety
    /// The setup, master queue and empty battle queues must be initialized.
    #[no_mangle]
    pub unsafe fn fill_pick_melee_frame(setup: &crate::supermelee::setup::team::MeleeSetup) {
        let (context, order, catalog) = KernelOwner::current().read(|owner| {
            (
                owner.off_screen_context,
                [owner.player_order(0), owner.player_order(1)],
                crate::ships::master_native::engine::snapshot(owner),
            )
        });
        let old = SetContext(context);
        for side_i in order {
            let side = usize::from(side_i == 0);
            let frame = SetAbsFrameIndex(PickMeleeFrame, side as u16);
            drawable_abi::SetContextFGFrame(frame.cast());
            let mut r = mem::zeroed();
            GetFrameRect(frame, &mut r);
            let mut t = TEXT {
                baseline: POINT {
                    x: r.extent.width >> 1,
                    y: r.extent.height - 7 + 4,
                },
                align: TEXT_ALIGN_ALIGN_CENTER,
                pStr: setup.teams[side_i as usize].c_name().as_ptr(),
                CharCount: u16::MAX,
            };
            r.corner.x += 2;
            r.corner.y += 2;
            r.extent.width -= 4 + 18 + 1;
            r.extent.height -= 4 + 7;
            SetContextForeGroundColor(rgb(0, 1, 15));
            DrawFilledRectangle(&mut r);
            r.corner.x += 2;
            r.extent.width += 18 - 4;
            r.corner.y += r.extent.height;
            r.extent.height = 7;
            DrawFilledRectangle(&mut r);
            SetContextFont(KernelOwner::current().read(|owner| owner.tiny_font));
            SetContextForeGroundColor(rgb(10, 10, 31));
            font_DrawText(&mut t);
            let text = CString::new(setup.fleet_value[side_i as usize].to_string()).unwrap();
            t.baseline = POINT { x: 4, y: 7 };
            t.align = TEXT_ALIGN_ALIGN_LEFT;
            t.pStr = text.as_ptr();
            SetContextForeGroundColor(rgb(4, 5, 31));
            font_DrawText(&mut t);
            assert!(link_queue::ships(side).head().is_none());
            for index in 0..14 {
                let ship = setup.teams[side_i as usize].ships[index as usize] as u32;
                if ship == 255 {
                    continue;
                }
                let master = catalog
                    .get(ship as usize)
                    .unwrap_or_else(|| panic!("melee master ship {ship}"));
                let captain = link_queue::ships(side).captain_name(master.species, TFB_Random);
                let built = link_queue::ships(side)
                    .build(master.species)
                    .unwrap_or_else(|| panic!("battle ship pool exhausted"));
                let mut s = STAMP {
                    origin: POINT {
                        x: 4 + 18 * (index % 7) as i16,
                        y: 10 + 18 * (index / 7) as i16,
                    },
                    frame: master.info.icons,
                };
                DrawStamp(&mut s);
                let mut queue = link_queue::ships(side);
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
        }
        SetContext(old);
    }
    /// # Safety
    /// The live battle ship and pick frame belong to the game thread.
    #[no_mangle]
    pub unsafe extern "C" fn MeleeShipDeath(ship: *mut std::ffi::c_void) {
        let ship = LinkQueue::<Starship>::mark_dead(link_queue::identify(ship as usize));
        let frame = SetAbsFrameIndex(PickMeleeFrame, ship.player as u16);
        let old = SetContext(KernelOwner::current().read(|owner| owner.off_screen_context));
        drawable_abi::SetContextFGFrame(frame.cast());
        let mut s = STAMP {
            origin: POINT {
                x: 3 + 18 * (ship.index % 7) as i16,
                y: 9 + 18 * (ship.index / 7) as i16,
            },
            frame: SetAbsFrameIndex(KernelOwner::current().read(|owner| owner.status_frame), 3),
        };
        DrawStamp(&mut s);
        SetContext(old);
        value(frame, ship.player as usize);
    }

    use crate::engine::observe::Io;
    use crate::mainloop::input_abi::live::UpdateInputState;
    /// # Safety
    /// Called at the end of a live melee battle on the game thread.
    pub unsafe fn melee_game_over(input: &crate::engine::input::InputSession, io: &mut Io<'_>) {
        for player in 0..2 {
            draw(player);
        }
        crate::app::bridge::enter(|cx| {
            let net = cx.b.network();
            cx.negotiate_battle(net.as_ref(), NetState_NetState_inSetup);
            let timeout = GetTimeCounter().wrapping_add(840 * 4);
            let mut pressed = input.pulsed().menu[KEY_MENU_SELECT as usize] != 0
                || input.pulsed().menu[KEY_MENU_CANCEL as usize] != 0;
            loop {
                if cx
                    .nested_input(io, |cx| UpdateInputState(cx, input))
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
                    SleepThread(840 / 40);
                });
                if cx.world.activity() & CHECK_ABORT as u16 != 0
                    || button
                    || (cx.kernel.player_control[0] & cx.kernel.player_control[1] & PSYTRON_CONTROL
                        != 0
                        && GetTimeCounter() >= timeout)
                {
                    break;
                }
            }
        });
    }
    unsafe fn initialize_selection(
        state: &mut GETMELEE_STATE,
        counters: [u8; 2],
        mask: u16,
        screen_context: CONTEXT,
        controls: [u8; 2],
        order: [u16; 2],
    ) {
        let now = GetTimeCounter();
        for player in order.map(usize::from) {
            let p = &mut state.player[player];
            p.selecting = u32::from(mask & (1 << player) != 0);
            p.ships_left = u16::from(counters[player]);
            p.randomIndex = TFB_Random() as u16 % p.ships_left;
            p.done = 0;
            if p.selecting == 0 {
                continue;
            }
            p.timeIn = now;
            p.row = 0;
            p.col = 7;
            p.remoteSelected = 0;
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
    unsafe fn get_ships(
        input: &crate::engine::input::InputSession,
        mask: u16,
    ) -> Option<[Option<ShipId>; 2]> {
        crate::app::bridge::enter(|cx| {
            let mut state = GETMELEE_STATE::default();
            let counters = cx.b.counters;
            let net = cx.b.network();
            state.net = Some(net.clone());
            let screen_context = cx.kernel.screen_context;
            let controls = cx.kernel.player_control;
            let order = [cx.kernel.player_order(0), cx.kernel.player_order(1)];
            cx.with_legacy(|| {
                let available = std::array::from_fn(|player| {
                    std::array::from_fn(|slot| by_index(player, slot as u16, false).is_some())
                });
                net.begin_selection(mask, available);
                initialize_selection(&mut state, counters, mask, screen_context, controls, order);
            });
            let mut ok = cx.negotiate_battle(net.as_ref(), NetState_NetState_selectShip);
            cx.with_legacy(|| {
                SetDefaultMenuRepeatDelay(input);
                SetContext(KernelOwner::current().read(|owner| owner.off_screen_context));
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
                    if p.selecting == 0 {
                        continue;
                    }
                    if p.done != 0 {
                        ships[player] = p.hBattleShip;
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
        })
    }
    /// # Safety
    /// Both battle fleets and their selection resources must be initialized.
    pub unsafe fn get_initial_melee_star_ships(
        input: &crate::engine::input::InputSession,
    ) -> Option<[Option<ShipId>; 2]> {
        for player in 0..2 {
            value(SetAbsFrameIndex(PickMeleeFrame, player as u16), player);
            draw(player);
        }
        SleepThreadUntil(FadeScreen(FADE_ALL_TO_COLOR, 840 / 2).wrapping_add(840 / 60));
        FlushColorXForms();
        get_ships(input, 3)
    }
    /// # Safety
    /// The selected player's battle fleet and selection resources must be initialized.
    pub unsafe fn get_next_melee_star_ship(
        input: &crate::engine::input::InputSession,
        player: u16,
    ) -> Option<ShipId> {
        draw(player as usize);
        get_ships(input, 1 << player)?[player as usize]
    }
}
