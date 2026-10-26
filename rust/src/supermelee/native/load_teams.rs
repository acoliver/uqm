//! Saved-fleet browser and persistence UI from loadmele.c.
fn page(top: u16, bottom: u16, current: u16, count: u16, keys: [bool; 4]) -> (u16, u16) {
    let (mut next_top, mut next) = (top, current);
    if keys[0] {
        if next > 0 {
            next -= 1;
            if next < next_top {
                next_top = next_top.saturating_sub(5);
            }
        }
    } else if keys[1] {
        if next + 1 < count {
            next += 1;
            if next >= bottom {
                next_top = bottom;
            }
        }
    } else if keys[2] {
        next = next.saturating_sub(5);
        next_top = next_top.saturating_sub(5);
    } else if keys[3] {
        if next + 5 < count {
            next += 5;
            next_top += 5;
        } else {
            next = count - 1;
            if next_top + 5 < count && count > 5 {
                next_top = count - 5;
            }
        }
    }
    (next_top, next)
}

#[cfg(not(test))]
#[allow(non_snake_case)]
pub(crate) mod live {
    use super::page;

    use crate::io::uio_bridge::{uio_fclose, uio_fopen, uio_unlink};
    use crate::memory::{rust_hfree, rust_hmalloc};
    use crate::savegame::problem::SaveProblem;
    use crate::ships::native::abi::*;
    use crate::supermelee::native::teams::*;
    use std::{
        ffi::{c_char, CStr, CString},
        mem, ptr,
    };

    unsafe fn load_image(entry: STRING, team: *mut MeleeTeam) -> bool {
        let stream = uio_fopen(meleeDir.cast(), GetStringAddress(entry), c"rb".as_ptr());
        if stream.is_null() {
            return false;
        }
        if MeleeTeam_deserialize(team, stream) == -1 {
            return false;
        }
        uio_fclose(stream);
        true
    }
    unsafe fn fleet(state: *mut MELEE_STATE, mut index: u16, result: *mut MeleeTeam) -> bool {
        let load = &mut (*state).load;
        if index < load.preBuiltCount {
            MeleeTeam_copy(result, *load.preBuiltList.add(index as usize));
            return true;
        }
        index -= load.preBuiltCount;
        let first = index;
        while index < load.numIndices {
            let entry =
                SetAbsStringTableIndex(load.dirEntries, *load.entryIndices.add(index as usize));
            if load_image(entry, result) {
                break;
            }
            log::warn!(
                "File '{}' is not a valid SuperMelee team",
                CStr::from_ptr(GetStringAddress(entry)).to_string_lossy()
            );
            index += 1;
        }
        if index != first {
            let count = index - first;
            assert!(first + count <= load.numIndices);
            load.numIndices -= count;
            ptr::copy(
                load.entryIndices.add(index as usize),
                load.entryIndices.add(first as usize),
                (load.numIndices - first) as usize,
            );
        }
        index < load.numIndices
    }
    unsafe fn file_index(state: *mut MELEE_STATE, name: *const c_char) -> u16 {
        let load = &(*state).load;
        for index in 0..load.numIndices {
            let entry =
                SetAbsStringTableIndex(load.dirEntries, *load.entryIndices.add(index as usize));
            if std::ffi::CStr::from_ptr(GetStringAddress(entry))
                .to_bytes()
                .eq_ignore_ascii_case(std::ffi::CStr::from_ptr(name).to_bytes())
            {
                return load.preBuiltCount + index;
            }
        }
        u16::MAX
    }
    unsafe fn draw_entry(team: *const MeleeTeam, origin: POINT, ships: bool, highlighted: bool) {
        let color = if highlighted {
            rgb(23, 24, 29)
        } else {
            rgb(15, 16, 27)
        };
        SetContextForeGroundColor(color);
        let mut text = TEXT {
            baseline: origin,
            align: TEXT_ALIGN_ALIGN_LEFT as _,
            pStr: MeleeTeam_getTeamName(team),
            CharCount: u16::MAX,
        };
        font_DrawText(&mut text);
        let value = CString::new(MeleeTeam_getValue(team).to_string()).unwrap();
        text.baseline.x += 7 * 35 - 1;
        text.align = TEXT_ALIGN_ALIGN_RIGHT as _;
        text.pStr = value.as_ptr();
        font_DrawText(&mut text);
        if ships {
            let mut stamp = STAMP {
                origin: POINT {
                    x: origin.x + 1,
                    y: origin.y + 4,
                },
                frame: ptr::null_mut(),
            };
            for ship in (*team).ships {
                if ship != 255 {
                    stamp.frame =
                        crate::ships::master_native::GetShipIconsFromIndex(ship as u8 as u32);
                    DrawStamp(&mut stamp);
                    stamp.origin.x += 17;
                }
            }
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
    unsafe fn fill_view(state: *mut MELEE_STATE) -> bool {
        let mut n = 0;
        while n < 5 {
            if !fleet(state, (*state).load.top + n, (*state).load.view[n as usize]) {
                break;
            }
            n += 1;
        }
        if n == 0 {
            return false;
        }
        (*state).load.bot = (*state).load.top + n;
        true
    }
    unsafe fn select(state: *mut MELEE_STATE, highlighted: bool) {
        let index = (*state).load.cur - (*state).load.top;
        let old = SetContext(SpaceContext);
        SetContextFont(MicroFont);
        BatchGraphics();
        draw_entry(
            (*state).load.view[index as usize],
            POINT {
                x: 5,
                y: 34 + index as i16 * 32,
            },
            false,
            highlighted,
        );
        UnbatchGraphics();
        SetContext(old);
    }
    unsafe fn draw(state: *mut MELEE_STATE) {
        let old = SetContext(SpaceContext);
        SetContextFont(MicroFont);
        BatchGraphics();
        DrawMeleeIcon(28);
        if fill_view(state) {
            for index in (*state).load.top..(*state).load.bot {
                let relative = index - (*state).load.top;
                draw_entry(
                    (*state).load.view[relative as usize],
                    POINT {
                        x: 5,
                        y: 34 + relative as i16 * 32,
                    },
                    true,
                    false,
                );
            }
        }
        UnbatchGraphics();
        SetContext(old);
    }
    static mut NEXT_FLASH: u32 = 0;
    static mut HIGHLIGHTED: bool = false;
    unsafe fn flash(state: *mut MELEE_STATE) {
        let now = GetTimeCounter();
        if now >= NEXT_FLASH {
            NEXT_FLASH = now.wrapping_add(840 / 9);
            HIGHLIGHTED = !HIGHLIGHTED;
            let old = SetContext(SpaceContext);
            select(state, HIGHLIGHTED);
            SetContext(old);
        }
    }
    /// # Safety
    /// State and its load view must be live on the game thread.
    #[no_mangle]
    pub unsafe extern "C" fn DoLoadTeam(state: *mut MELEE_STATE) -> u32 {
        let time = GetTimeCounter();
        GamePaused = 0;
        if GlobData.Game_state.CurrentActivity & CHECK_ABORT as u16 != 0 {
            return 0;
        }
        SetMenuSounds(1 | 2 | 128 | 256, 16);
        if (*state).Initialized == 0 {
            draw(state);
            select(state, true);
            (*state).Initialized = 1;
            (*state).InputFunc = Some(DoLoadTeam);
            return 1;
        }
        let keys = PulsedInputState.menu;
        if keys[KEY_MENU_SELECT as usize] != 0 || keys[KEY_MENU_CANCEL as usize] != 0 {
            if keys[KEY_MENU_SELECT as usize] != 0 {
                Melee_LocalChange_team(
                    state,
                    (*state).side as usize,
                    (*state).load.view[((*state).load.cur - (*state).load.top) as usize],
                );
            }
            (*state).InputFunc = Some(DoMelee);
            (*state).LastInputTime = GetTimeCounter();
            let mut rect = mem::zeroed();
            GetFrameRect(SetAbsFrameIndex(MeleeFrame, 28), &mut rect);
            RepairMeleeFrame(&rect);
            return 1;
        }
        let (top, index) = page(
            (*state).load.top,
            (*state).load.bot,
            (*state).load.cur,
            (*state).load.numIndices + (*state).load.preBuiltCount,
            [
                keys[KEY_MENU_UP as usize] != 0,
                keys[KEY_MENU_DOWN as usize] != 0,
                keys[KEY_MENU_PAGE_UP as usize] != 0,
                keys[KEY_MENU_PAGE_DOWN as usize] != 0,
            ],
        );
        if index != (*state).load.cur {
            if top == (*state).load.top {
                select(state, false);
            } else {
                (*state).load.top = top;
                draw(state);
            }
            (*state).load.cur = index;
        }
        flash(state);
        SleepThreadUntil(time.wrapping_add(840 / 30));
        1
    }
    /// # Safety
    /// State must own a live load view on the game thread.
    #[no_mangle]
    pub unsafe extern "C" fn LoadTeamList(state: *mut MELEE_STATE) {
        let load = &mut (*state).load;
        DestroyStringTable(ReleaseStringTable(load.dirEntries));
        load.dirEntries = crate::strings::string_table::CaptureStringTable(
            LoadDirEntryTable(
                meleeDir.cast(),
                c"".as_ptr(),
                c".mle".as_ptr(),
                match_MatchType_match_MATCH_SUFFIX,
            )
            .cast(),
        )
        .cast();
        if !load.entryIndices.is_null() {
            rust_hfree(load.entryIndices.cast());
        }
        load.numIndices = crate::strings::string_table::GetStringTableCount(load.dirEntries.cast());
        load.entryIndices = rust_hmalloc(load.numIndices as usize * mem::size_of::<u16>()).cast();
        for index in 0..load.numIndices {
            *load.entryIndices.add(index as usize) = index;
        }
    }
    /// # Safety
    /// State must own the team being saved on the game thread.
    #[no_mangle]
    pub unsafe extern "C" fn DoSaveTeam(state: *mut MELEE_STATE) -> u32 {
        let mut filename = CStr::from_ptr(MeleeSetup_getTeamName(
            (*state).meleeSetup,
            (*state).side as usize,
        ))
        .to_bytes()
        .to_vec();
        filename.extend(b".mle");
        filename.truncate(254);
        let filename = CString::new(filename).unwrap();
        let old = SetContext(ScreenContext);
        let mut saved = mem::zeroed();
        ConfirmSaveLoad(&mut saved);
        let stream = uio_fopen(meleeDir.cast(), filename.as_ptr(), c"wb".as_ptr());
        let mut ok = false;
        if !stream.is_null() {
            ok = MeleeTeam_serialize(
                &(*(*state).meleeSetup).teams[(*state).side as usize],
                stream,
            ) == 0;
            uio_fclose(stream);
            if !ok {
                uio_unlink(meleeDir.cast(), filename.as_ptr());
            }
        }
        (*state).load.top = 0;
        (*state).load.cur = 0;
        DrawStamp(&mut saved);
        DestroyDrawable(ReleaseDrawable(saved.frame));
        SetContext(old);
        if !ok {
            SaveProblem();
        }
        LoadTeamList(state);
        let index = file_index(state, filename.as_ptr());
        if index != u16::MAX {
            assert!(index < (*state).load.preBuiltCount + (*state).load.numIndices);
            (*state).load.cur = index;
            (*state).load.top = index.saturating_sub(2);
        }
        u32::from(!stream.is_null())
    }
    /// # Safety
    /// State must be allocated and initialized on the game thread.
    #[no_mangle]
    pub unsafe extern "C" fn InitMeleeLoadState(state: *mut MELEE_STATE) {
        let load = &mut (*state).load;
        load.entryIndices = ptr::null_mut();
        let catalog = crate::supermelee::setup::persistence::builtin_teams();
        load.preBuiltCount = catalog.len() as u16;
        load.preBuiltList = rust_hmalloc(catalog.len() * mem::size_of::<*mut MeleeTeam>()).cast();
        for (index, original) in catalog.iter().enumerate() {
            let team = MeleeTeam_new();
            *load.preBuiltList.add(index) = team;
            (*team).ships = original.ships.map(|s| s as u32);
            if index < 5 {
                MeleeTeam_setName(
                    team,
                    crate::comm::hail::game_string(MELEE_STRING_BASE as i32 + 4 + index as i32),
                );
            } else {
                ptr::copy_nonoverlapping(
                    original.name.as_ptr().cast(),
                    (*team).name.as_mut_ptr(),
                    55,
                );
            }
        }
        for team in &mut load.view {
            *team = MeleeTeam_new();
        }
    }
    /// # Safety
    /// State must own its load view and prebuilt catalog.
    #[no_mangle]
    pub unsafe extern "C" fn UninitMeleeLoadState(state: *mut MELEE_STATE) {
        let load = &mut (*state).load;
        for team in load.view {
            MeleeTeam_delete(team);
        }
        for index in 0..load.preBuiltCount {
            MeleeTeam_delete(*load.preBuiltList.add(index as usize));
        }
        rust_hfree(load.preBuiltList.cast());
        load.preBuiltCount = 0;
        if !load.entryIndices.is_null() {
            rust_hfree(load.entryIndices.cast());
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn paging_preserves_cursor_and_view_rules() {
        assert_eq!(page(5, 10, 5, 15, [true, false, false, false]), (0, 4));
        assert_eq!(page(0, 5, 4, 15, [false, true, false, false]), (5, 5));
        assert_eq!(page(5, 10, 7, 15, [false, false, true, false]), (0, 2));
        assert_eq!(page(10, 15, 13, 17, [false, false, false, true]), (12, 16));
        assert_eq!(page(0, 4, 0, 4, [false, false, false, true]), (0, 3));
    }
}
