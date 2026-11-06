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
pub(crate) mod live {
    use super::page;
    use crate::engine::Kernel as KernelOwner;

    use crate::io::uio_owned::Directory;
    use crate::ships::native::abi::*;
    use crate::supermelee::native::teams::*;
    use crate::supermelee::setup::{
        browser::write_team,
        team::{ship_cost, MeleeTeam as OwnedTeam},
    };
    use crate::ui::save_problem::SaveProblem;
    use std::{
        ffi::{c_char, CStr, CString},
        mem, ptr,
    };

    unsafe fn draw_entry(
        catalog: &[crate::ships::master_native::engine::MasterShip],
        team: &OwnedTeam,
        origin: POINT,
        ships: bool,
        highlighted: bool,
    ) {
        let color = if highlighted {
            rgb(23, 24, 29)
        } else {
            rgb(15, 16, 27)
        };
        SetContextForeGroundColor(color);
        let mut text = TEXT {
            baseline: origin,
            align: TEXT_ALIGN_ALIGN_LEFT as _,
            pStr: team.c_name().as_ptr(),
            CharCount: u16::MAX,
        };
        font_DrawText(&mut text);
        let value = CString::new(
            team.ships
                .iter()
                .copied()
                .map(ship_cost)
                .sum::<u16>()
                .to_string(),
        )
        .unwrap();
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
            for ship in team.ships {
                if ship.is_valid() && ship as u8 != 255 {
                    stamp.frame = catalog
                        .get(ship as u8 as usize)
                        .map_or(ptr::null_mut(), |ship| ship.info.icons);
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
        let directory =
            Directory::retain(std::ptr::NonNull::new(meleeDir.cast()).expect("melee directory"));
        (*state).load.fill_view(&directory)
    }
    unsafe fn select(state: *mut MELEE_STATE, highlighted: bool) {
        let index = (*state).load.current - (*state).load.top;
        let old = SetContext(KernelOwner::current().read(|owner| owner.space_context));
        SetContextFont(KernelOwner::current().read(|owner| owner.micro_font));
        BatchGraphics();
        draw_entry(
            &[],
            &(*state).load.view[index as usize],
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
        let (context, catalog) = KernelOwner::current().read(|owner| {
            (
                owner.space_context,
                crate::ships::master_native::engine::snapshot(owner),
            )
        });
        let old = SetContext(context);
        SetContextFont(KernelOwner::current().read(|owner| owner.micro_font));
        BatchGraphics();
        DrawMeleeIcon(28);
        if fill_view(state) {
            for index in (*state).load.top..(*state).load.bottom {
                let relative = index - (*state).load.top;
                draw_entry(
                    &catalog,
                    &(*state).load.view[relative as usize],
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
    unsafe fn flash(state: *mut MELEE_STATE) {
        let now = GetTimeCounter();
        if now >= (*state).load.next_flash {
            (*state).load.next_flash = now.wrapping_add(840 / 9);
            (*state).load.highlighted = !(*state).load.highlighted;
            let old = SetContext(KernelOwner::current().read(|owner| owner.space_context));
            select(state, (*state).load.highlighted);
            SetContext(old);
        }
    }
    /// # Safety
    /// State and its load view must be live on the game thread.
    pub unsafe fn do_load_team(
        state: *mut MELEE_STATE,
        input: &crate::engine::input::InputSession,
    ) -> u32 {
        let time = GetTimeCounter();
        input.set_paused(false);
        if crate::engine::World::current().read(|world| world.activity()) & CHECK_ABORT as u16 != 0
        {
            return 0;
        }
        SetMenuSounds(input, 1 | 2 | 128 | 256, 16);
        if (*state).initialized == 0 {
            draw(state);
            select(state, true);
            (*state).initialized = 1;
            (*state).phase = crate::supermelee::netplay::session::MenuPhase::Loading;
            return 1;
        }
        let keys = input.pulsed().menu;
        if keys[KEY_MENU_SELECT as usize] != 0 || keys[KEY_MENU_CANCEL as usize] != 0 {
            if keys[KEY_MENU_SELECT as usize] != 0 {
                let team = (*state).load.view[((*state).load.current - (*state).load.top) as usize]
                    .clone();
                Melee_LocalChange_team(state, (*state).side as usize, &team);
            }
            (*state).phase = crate::supermelee::netplay::session::MenuPhase::Menu;
            (*state).last_input_time = GetTimeCounter();
            let mut rect = mem::zeroed();
            GetFrameRect(SetAbsFrameIndex(MeleeFrame, 28), &mut rect);
            RepairMeleeFrame(&rect);
            return 1;
        }
        let (top, index) = page(
            (*state).load.top,
            (*state).load.bottom,
            (*state).load.current,
            (*state).load.count(),
            [
                keys[KEY_MENU_UP as usize] != 0,
                keys[KEY_MENU_DOWN as usize] != 0,
                keys[KEY_MENU_PAGE_UP as usize] != 0,
                keys[KEY_MENU_PAGE_DOWN as usize] != 0,
            ],
        );
        if index != (*state).load.current {
            if top == (*state).load.top {
                select(state, false);
            } else {
                (*state).load.top = top;
                draw(state);
            }
            (*state).load.current = index;
        }
        flash(state);
        SleepThreadUntil(time.wrapping_add(840 / 30));
        1
    }
    /// # Safety
    /// State must own a live load view on the game thread.
    pub unsafe fn LoadTeamList(state: *mut MELEE_STATE) {
        let directory =
            Directory::retain(std::ptr::NonNull::new(meleeDir.cast()).expect("melee directory"));
        (*state).load.refresh(&directory);
    }
    /// # Safety
    /// State must own the team being saved on the game thread.
    pub unsafe fn DoSaveTeam(state: *mut MELEE_STATE) {
        let mut filename = CStr::from_ptr(MeleeSetup_getTeamName(
            (*state).melee_setup,
            (*state).side as usize,
        ))
        .to_bytes()
        .to_vec();
        filename.extend(b".mle");
        filename.truncate(254);
        let filename = CString::new(filename).unwrap();
        let old = SetContext(KernelOwner::current().read(|owner| owner.screen_context));
        let mut saved = mem::zeroed();
        ConfirmSaveLoad(&mut saved);
        let directory =
            Directory::retain(std::ptr::NonNull::new(meleeDir.cast()).expect("melee directory"));
        let original = &(*(*state).melee_setup).teams[(*state).side as usize];
        let team = OwnedTeam {
            ships: original.ships.map(|ship| {
                crate::supermelee::types::MeleeShip::from_u8(ship as u8)
                    .unwrap_or(crate::supermelee::types::MeleeShip::MeleeNone)
            }),
            name: original.name.map(|byte| byte as u8),
        };
        let ok = write_team(&directory, &filename, &team).is_ok();
        (*state).load.top = 0;
        (*state).load.current = 0;
        DrawStamp(&mut saved);
        DestroyDrawable(ReleaseDrawable(saved.frame));
        SetContext(old);
        if !ok {
            SaveProblem();
        }
        LoadTeamList(state);
        if let Some(index) = (*state).load.file_index(&filename) {
            assert!(index < (*state).load.count());
            (*state).load.current = index;
            (*state).load.top = index.saturating_sub(2);
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
