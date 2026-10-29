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
#[allow(non_snake_case)]
pub mod live {
    use super::move_cursor;
    use crate::battle::sounds::PlayMenuSound;
    use crate::battle::{NETWORK_CONTROL, PSYTRON_CONTROL};
    use crate::mainloop::restart_menu::c_extern as flash;
    use crate::supermelee::c_bridge::PlayerControl;
    const FADE_ALL_TO_COLOR: u32 = 253;
    unsafe fn Flash_process(context: *mut FlashContext) {
        flash::Flash_process(context.cast());
    }
    unsafe fn Flash_terminate(context: *mut FlashContext) {
        flash::Flash_terminate(context.cast());
    }
    unsafe fn Flash_setMergeFactors(context: *mut FlashContext, a: i32, b: i32, c: i32) {
        flash::Flash_setMergeFactors(context.cast(), a, b, c);
    }
    unsafe fn Flash_setFrameTime(context: *mut FlashContext, t: u32) {
        flash::Flash_setFrameTime(context.cast(), t);
    }
    unsafe fn Flash_setSpeed(context: *mut FlashContext, a: u32, b: u32, c: u32, d: u32) {
        flash::Flash_setSpeed(context.cast(), a, b, c, d);
    }
    unsafe fn Flash_start(context: *mut FlashContext) {
        flash::Flash_start(context.cast());
    }

    use crate::engine::battle::{link_queue, ShipId};
    use crate::graphics::drawable_abi;
    use crate::ships::native::abi::*;
    use crate::supermelee::native::teams::*;
    use std::{ffi::CString, mem, ptr};
    #[no_mangle]
    pub static mut PickMeleeFrame: FRAME = ptr::null_mut();
    unsafe fn player_queue(player: usize) -> *mut QUEUE {
        crate::c_bindings::live_queues::player_ships(player)
            .cast_mut()
            .cast()
    }
    unsafe fn by_index(player: usize, mut index: u16, used: bool) -> Option<ShipId> {
        let queue = link_queue::ships(player);
        let mut link = queue.head();
        while let Some(id) = link {
            let ship = queue.get(id);
            if used {
                if ship.species != 0 {
                    if index == 0 {
                        return Some(queue.ship_id(id, player as u8));
                    }
                    index = index.wrapping_sub(1);
                }
            } else if ship.index == index {
                return if ship.species == 0 {
                    None
                } else {
                    Some(queue.ship_id(id, player as u8))
                };
            }
            link = queue.succ(id);
        }
        None
    }
    unsafe fn changed(state: *mut GETMELEE_STATE, player: usize) {
        let s = &(*state).player[player];
        let r = RECT {
            corner: POINT {
                x: 57 + 18 * s.col as i16,
                y: 24 + 18 * s.row as i16 + (1 - player as i16) * 100,
            },
            extent: EXTENT {
                width: 18,
                height: 18,
            },
        };
        Flash_setRect(s.flashContext, &r);
    }
    /// # Safety
    /// The selection and live ship queues belong to the game thread.
    pub unsafe extern "C" fn setShipSelected(
        state: *mut GETMELEE_STATE,
        player: u16,
        choice: u16,
        network: bool,
    ) -> bool {
        let s = &mut (*state).player[player as usize];
        assert!(s.done == 0);
        let ship = if choice == u16::MAX {
            by_index(player as usize, s.randomIndex, true)
        } else {
            by_index(player as usize, choice, false)
        };
        let Some(ship) = ship else {
            return false;
        };
        s.choice = choice;
        let queue = link_queue::ships(player as usize);
        s.hBattleShip = queue.raw(queue.link(ship));
        PlayMenuSound(1);
        if network {
            if let Some(net) = &(*state).net {
                net.ship_selected(choice);
            }
        }
        s.done = 1;
        true
    }
    unsafe fn process(state: *mut GETMELEE_STATE, player: usize, input: u8) -> u32 {
        if input & 8 != 0 {
            let s = &(*state).player[player];
            if s.col == 7 && s.row == 0 {
                setShipSelected(state, player as u16, u16::MAX, true);
            } else if s.col == 7 && s.row == 1 {
                if ConfirmExit() != 0 {
                    return 0;
                }
            } else {
                setShipSelected(state, player as u16, s.row * 7 + s.col, true);
            }
        } else {
            let s = &mut (*state).player[player];
            let (row, col) = move_cursor(s.row, s.col, input);
            if (row, col) != (s.row, s.col) {
                s.row = row;
                s.col = col;
                PlayMenuSound(0);
                changed(state, player);
            }
        }
        1
    }
    /// # Safety
    /// Live input context and fleet selection, game thread only.
    #[no_mangle]
    pub unsafe extern "C" fn selectShipHuman(
        context: *mut HumanInputContext,
        state: *mut GETMELEE_STATE,
    ) -> u32 {
        let player = (*context).playerNr;
        process(state, player as usize, PulsedInputToBattleInput(player))
    }
    /// # Safety
    /// Live input context and fleet selection, game thread only.
    #[no_mangle]
    pub unsafe extern "C" fn selectShipComputer(
        context: *mut ComputerInputContext,
        state: *mut GETMELEE_STATE,
    ) -> u32 {
        let player = (*context).playerNr as usize;
        if GetTimeCounter() < (*state).player[player].timeIn.wrapping_add(840 >> 1) {
            return 1;
        }
        process(state, player, 8)
    }
    /// # Safety
    /// Live network input context and fleet selection, game thread only.
    #[no_mangle]
    pub unsafe extern "C" fn selectShipNetwork(
        context: *mut NetworkInputContext,
        state: *mut GETMELEE_STATE,
    ) -> u32 {
        (*state)
            .net
            .as_ref()
            .unwrap_or_else(|| panic!("missing net"))
            .flush();
        let p = &mut (*state).player[(*context).playerNr as usize];
        if p.remoteSelected != 0 {
            p.done = 1;
        }
        1
    }
    unsafe extern "C" fn choose(state: *mut GETMELEE_STATE) -> u32 {
        SetMenuSounds(0, 0);
        if (*state).Initialized == 0 {
            (*state).Initialized = 1;
            return 1;
        }
        for p in &(*state).player {
            if p.selecting != 0 && p.done == 0 {
                Flash_process(p.flashContext);
            }
        }
        SleepThread(840 / 120);
        (*state)
            .net
            .as_ref()
            .unwrap_or_else(|| panic!("missing net"))
            .poll();
        let mut abort = !(*state)
            .net
            .as_ref()
            .unwrap_or_else(|| panic!("missing net"))
            .all_connected()
            || GlobData.Game_state.CurrentActivity & CHECK_ABORT as u16 != 0;
        let mut done = true;
        if !abort {
            for player in 0..2 {
                let input = crate::battle::battle_native::engine::player_input(player);
                if (*state).player[player].selecting == 0 || (*state).player[player].done != 0 {
                    continue;
                }
                if ((*(*input).handlers).selectShip.unwrap())(input, state) == 0 {
                    abort = true;
                    break;
                }
                let p = &mut (*state).player[player];
                if p.done != 0 {
                    Flash_terminate(p.flashContext);
                    p.flashContext = ptr::null_mut();
                } else {
                    done = false;
                }
            }
        }
        (*state)
            .net
            .as_ref()
            .unwrap_or_else(|| panic!("missing net"))
            .flush();
        if abort {
            for p in &mut (*state).player {
                if p.selecting != 0 {
                    p.choice = 0;
                    p.hBattleShip = ptr::null_mut();
                }
            }
            GlobData.Game_state.CurrentActivity &= !(CHECK_ABORT as u16);
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
        let old = SetContext(OffScreenContext);
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
        SetContextFont(TinyFont);
        SetContextForeGroundColor(rgb(19, 0, 0));
        font_DrawText(&mut t);
        SetContext(old);
    }
    unsafe fn draw(player: usize) {
        let old = SetContext(SpaceContext);
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
        let old = SetContext(OffScreenContext);
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
    pub unsafe extern "C" fn FillPickMeleeFrame(setup: *mut MeleeSetup) {
        let old = SetContext(OffScreenContext);
        for i in 0..2 {
            let side_i = GetPlayerOrder(i);
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
                pStr: MeleeSetup_getTeamName(setup, side_i as usize),
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
            SetContextFont(TinyFont);
            SetContextForeGroundColor(rgb(10, 10, 31));
            font_DrawText(&mut t);
            let text =
                CString::new(MeleeSetup_getFleetValue(setup, side_i as usize).to_string()).unwrap();
            t.baseline = POINT { x: 4, y: 7 };
            t.align = TEXT_ALIGN_ALIGN_LEFT;
            t.pStr = text.as_ptr();
            SetContextForeGroundColor(rgb(4, 5, 31));
            font_DrawText(&mut t);
            assert!(link_queue::ships(side).head().is_none());
            for index in 0..14 {
                let ship = MeleeSetup_getShip(setup, side_i as usize, index);
                if ship == 255 {
                    continue;
                }
                let master =
                    GetStarShipFromIndex(&raw mut master_q, ship as u16).cast::<MASTER_SHIP_INFO>();
                if master.is_null() {
                    eprintln!("[BUG] FillPickMeleeFrame: StarShip={ship} master_q missing ship");
                    continue;
                }
                let captain = NameCaptain(player_queue(side), (*master).SpeciesID);
                let built = link_queue::ships(side)
                    .build((*master).SpeciesID)
                    .unwrap_or_else(|| panic!("battle ship pool exhausted"));
                let mut s = STAMP {
                    origin: POINT {
                        x: 4 + 18 * (index % 7) as i16,
                        y: 10 + 18 * (index / 7) as i16,
                    },
                    frame: (*master).ShipInfo.icons,
                };
                DrawStamp(&mut s);
                let mut queue = link_queue::ships(side);
                let mut ship = queue.get(built);
                ship.index = index;
                ship.cost = (*master).ShipInfo.ship_cost;
                ship.player = side as i16;
                ship.captain = captain;
                ship.crew = 0;
                ship.max_crew = 0;
                ship.strings = 0;
                ship.icons = 0;
                queue.set(built, ship);
                queue.clear_descriptor(built);
            }
        }
        SetContext(old);
    }
    /// # Safety
    /// The live battle ship and pick frame belong to the game thread.
    #[no_mangle]
    pub unsafe extern "C" fn MeleeShipDeath(ship: *mut std::ffi::c_void) {
        let ship = {
            let id = link_queue::identify(ship as usize);
            let mut queue = link_queue::ships(id.side as usize);
            let id = queue.link(id);
            let mut value = queue.get(id);
            value.species = 0;
            queue.set(id, value);
            value
        };
        let frame = SetAbsFrameIndex(PickMeleeFrame, ship.player as u16);
        let old = SetContext(OffScreenContext);
        drawable_abi::SetContextFGFrame(frame.cast());
        let mut s = STAMP {
            origin: POINT {
                x: 3 + 18 * (ship.index % 7) as i16,
                y: 9 + 18 * (ship.index / 7) as i16,
            },
            frame: SetAbsFrameIndex(StatusFrame, 3),
        };
        DrawStamp(&mut s);
        SetContext(old);
        value(frame, ship.player as usize);
    }

    use crate::automation::input_ffi::nested_input;

    use crate::mainloop::input_abi::live::UpdateInputState;
    /// # Safety
    /// Called at the end of a live melee battle on the game thread.
    #[no_mangle]
    pub unsafe extern "C" fn MeleeGameOver() {
        for player in 0..2 {
            draw(player);
        }
        let net = crate::app::bridge::battle(|b| b.network());
        net.negotiate(true, NetState_NetState_inSetup);
        let timeout = GetTimeCounter().wrapping_add(840 * 4);
        let mut pressed = PulsedInputState.menu[KEY_MENU_SELECT as usize] != 0
            || PulsedInputState.menu[KEY_MENU_CANCEL as usize] != 0;
        loop {
            if nested_input(|| UpdateInputState()).is_none() {
                break;
            }
            let mut button = PulsedInputState.menu[KEY_MENU_SELECT as usize] != 0
                || PulsedInputState.menu[KEY_MENU_CANCEL as usize] != 0;
            if pressed {
                pressed = button;
                button = false;
            }
            Async_process();
            SleepThread(840 / 40);
            if GlobData.Game_state.CurrentActivity & CHECK_ABORT as u16 != 0
                || button
                || (PlayerControl[0] & PlayerControl[1] & PSYTRON_CONTROL != 0
                    && GetTimeCounter() >= timeout)
            {
                break;
            }
        }
    }
    unsafe fn get_ships(mask: u16, ships: *mut *mut std::ffi::c_void) -> u32 {
        let mut state: GETMELEE_STATE = mem::zeroed();
        let (counters, net) = crate::app::bridge::battle(|b| (b.counters, b.network()));
        state.net = Some(net.clone());
        net.bind_picker(mask, (&raw mut state).cast());
        let now = GetTimeCounter();
        state.InputFunc = Some(choose);
        for i in 0..2 {
            let player = GetPlayerOrder(i) as usize;
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
            p.flashContext = Flash_createHighlight(ScreenContext, ptr::null_mut());
            Flash_setMergeFactors(p.flashContext, 2, 3, 2);
            Flash_setFrameTime(p.flashContext, 840 / 16);
            if PlayerControl[player] & NETWORK_CONTROL != 0 {
                Flash_setSpeed(p.flashContext, 840 / 2, 0, 840 / 2, 0);
            } else {
                Flash_setSpeed(p.flashContext, 0, 840 / 16, 0, 840 / 16);
            }
            changed(&mut state, player);
            Flash_start(state.player[player].flashContext);
        }
        let mut ok = net.negotiate(true, NetState_NetState_selectShip);
        SetDefaultMenuRepeatDelay();
        SetContext(OffScreenContext);
        crate::mainloop::restart_menu::c_extern::DoInput(
            (&mut state as *mut GETMELEE_STATE).cast(),
            0,
        );
        WaitForSoundEnd(0);
        for (player, p) in state.player.iter_mut().enumerate() {
            if p.selecting == 0 {
                continue;
            }
            if p.done != 0 {
                *ships.add(player) = p.hBattleShip;
            } else {
                Flash_terminate(p.flashContext);
                p.flashContext = ptr::null_mut();
                ok = false;
            }
        }
        if ok {
            if !net.negotiate(true, NetState_NetState_interBattle) {
                ok = false;
            }
        } else {
            net.set_state(NetState_NetState_interBattle);
        }
        if !ok {
            GlobData.Game_state.CurrentActivity &= !(IN_BATTLE as u16);
        }
        net.bind_picker(mask, ptr::null_mut());
        u32::from(ok)
    }
    /// # Safety
    /// Output has two slots and both battle fleets are initialized.
    #[no_mangle]
    pub unsafe extern "C" fn GetInitialMeleeStarShips(ships: *mut *mut std::ffi::c_void) -> u32 {
        for player in 0..2 {
            value(SetAbsFrameIndex(PickMeleeFrame, player as u16), player);
            draw(player);
        }
        SleepThreadUntil(FadeScreen(FADE_ALL_TO_COLOR, 840 / 2).wrapping_add(840 / 60));
        FlushColorXForms();
        get_ships(3, ships)
    }
    /// # Safety
    /// Output has one slot and the selected player's battle fleet is live.
    #[no_mangle]
    pub unsafe extern "C" fn GetNextMeleeStarShip(
        player: u16,
        result: *mut *mut std::ffi::c_void,
    ) -> u32 {
        draw(player as usize);
        let mut ships = [ptr::null_mut(); 2];
        let ok = get_ships(1 << player, ships.as_mut_ptr());
        if ok != 0 {
            *result = ships[player as usize];
        }
        ok
    }
    /// # Safety
    /// State may be null; otherwise it is the active selection record.
    #[no_mangle]
    pub unsafe extern "C" fn updateMeleeSelection(
        state: *mut GETMELEE_STATE,
        player: u16,
        ship: u16,
    ) -> bool {
        if state.is_null()
            || (*state).player[player as usize].selecting == 0
            || (*state).player[player as usize].done != 0
        {
            log::warn!("Unexpected ship selection packet received");
            return false;
        }
        if !setShipSelected(state, player, ship, false) {
            log::warn!("Invalid ship selection received from remote party");
            return false;
        }
        (*state).player[player as usize].remoteSelected = 1;
        true
    }
}
