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
    use crate::graphics::{
        context_abi::{ContextFrame, ContextLease},
        native_abi as render,
        native_font::FontLease,
    };
    use crate::ships::master_native::ShipChoice;
    use crate::supermelee::native::menu::live::drawing::repair_frame;
    use crate::supermelee::session::MeleeSession;

    use crate::ships::native::abi::*;
    use crate::supermelee::setup::{
        browser::write_team,
        team::{ship_cost, MeleeTeam as OwnedTeam},
    };
    use crate::ui::save_problem::SaveProblem;
    use std::{ffi::CString, mem, ptr::NonNull};

    fn draw_entry(
        context: &ContextLease,
        font: &FontLease,
        catalog: &[ShipChoice],
        team: &OwnedTeam,
        origin: render::POINT,
        ships: bool,
        highlighted: bool,
    ) {
        let color = if highlighted {
            rgb(23, 24, 29)
        } else {
            rgb(15, 16, 27)
        };
        context.draw_text(
            font,
            team.c_name(),
            origin,
            render::TEXT_ALIGN_ALIGN_LEFT,
            color,
        );
        let value = CString::new(
            team.ships
                .iter()
                .copied()
                .map(ship_cost)
                .sum::<u16>()
                .to_string(),
        )
        .unwrap();
        context.draw_text(
            font,
            &value,
            render::POINT {
                x: origin.x + 7 * 35 - 1,
                y: origin.y,
            },
            render::TEXT_ALIGN_ALIGN_RIGHT,
            color,
        );
        if ships {
            let mut origin = render::POINT {
                x: origin.x + 1,
                y: origin.y + 4,
            };
            for ship in team.ships {
                if ship.is_valid() && ship as u8 != 255 {
                    let source = catalog
                        .get(ship as u8 as usize)
                        .and_then(|ship| NonNull::new(ship.icon.raw().cast()));
                    context.draw_stamp(&ContextFrame::retain(source), origin);
                    origin.x += 17;
                }
            }
        }
    }
    fn rgb(r: u8, g: u8, b: u8) -> render::Color {
        render::Color {
            r: (r << 3) | (r >> 2),
            g: (g << 3) | (g >> 2),
            b: (b << 3) | (b >> 2),
            a: 255,
        }
    }
    fn fill_view(state: &mut MeleeSession) -> bool {
        let files = state
            .files
            .as_ref()
            .unwrap_or_else(|| panic!("melee file resources are not admitted"));
        state.load.fill_view(&files.teams)
    }
    fn select(state: &MeleeSession, highlighted: bool) {
        let index = state.load.current - state.load.top;
        let context = state
            .menu_context
            .as_ref()
            .unwrap_or_else(|| panic!("menu context not admitted"));
        let font = state
            .menu_font
            .as_ref()
            .unwrap_or_else(|| panic!("menu font not admitted"));
        context.draw_batch(|| {
            draw_entry(
                context,
                font,
                &state.catalog,
                &state.load.view[index as usize],
                render::POINT {
                    x: 5,
                    y: 34 + index as i16 * 32,
                },
                false,
                highlighted,
            )
        });
    }
    fn draw(state: &mut MeleeSession) {
        let context = state
            .menu_context
            .as_ref()
            .unwrap_or_else(|| panic!("menu context not admitted"))
            .clone();
        let font = state
            .menu_font
            .as_ref()
            .unwrap_or_else(|| panic!("menu font not admitted"))
            .clone();
        context.draw_batch(|| {
            crate::supermelee::native::menu::live::drawing::draw_melee_icon(state, 28);
            if fill_view(state) {
                for index in state.load.top..state.load.bottom {
                    let relative = index - state.load.top;
                    draw_entry(
                        &context,
                        &font,
                        &state.catalog,
                        &state.load.view[relative as usize],
                        render::POINT {
                            x: 5,
                            y: 34 + relative as i16 * 32,
                        },
                        true,
                        false,
                    );
                }
            }
        });
    }
    fn flash(state: &mut MeleeSession) {
        let now = uqm_engine::time::counter_abi::GetTimeCounter();
        if now >= state.load.next_flash {
            state.load.next_flash = now.wrapping_add(840 / 9);
            state.load.highlighted = !state.load.highlighted;
            select(state, state.load.highlighted);
        }
    }
    /// # Safety
    /// State and its load view must be live on the game thread.
    pub unsafe fn do_load_team(
        state: &mut MeleeSession,
        cx: &mut crate::engine::battle::Cx<'_>,
        input: &crate::engine::input::InputSession,
    ) -> u32 {
        let time = GetTimeCounter();
        input.set_paused(false);
        if cx.world.activity() & CHECK_ABORT as u16 != 0 {
            return 0;
        }
        SetMenuSounds(input, 1 | 2 | 128 | 256, 16);
        if state.initialized == 0 {
            cx.with_legacy(|| {
                draw(state);
                select(state, true);
            });
            state.initialized = 1;
            state.phase = crate::supermelee::session::MenuPhase::Loading;
            return 1;
        }
        let keys = input.pulsed().menu;
        if keys[KEY_MENU_SELECT as usize] != 0 || keys[KEY_MENU_CANCEL as usize] != 0 {
            if keys[KEY_MENU_SELECT as usize] != 0 {
                let team = state.load.view[(state.load.current - state.load.top) as usize].clone();
                local_change_team(cx, state, state.side as usize, &team);
            }
            state.phase = crate::supermelee::session::MenuPhase::Menu;
            state.last_input_time = GetTimeCounter();
            {
                let bounds = state.menu_frames().frame_rect(28);
                let rect = RECT {
                    corner: POINT {
                        x: bounds.corner.x,
                        y: bounds.corner.y,
                    },
                    extent: EXTENT {
                        width: bounds.extent.width,
                        height: bounds.extent.height,
                    },
                };
                repair_frame(cx, state, &rect);
            };
            return 1;
        }
        let (top, index) = page(
            state.load.top,
            state.load.bottom,
            state.load.current,
            state.load.count(),
            [
                keys[KEY_MENU_UP as usize] != 0,
                keys[KEY_MENU_DOWN as usize] != 0,
                keys[KEY_MENU_PAGE_UP as usize] != 0,
                keys[KEY_MENU_PAGE_DOWN as usize] != 0,
            ],
        );
        if index != state.load.current {
            if top == state.load.top {
                cx.with_legacy(|| select(state, false));
            } else {
                state.load.top = top;
                cx.with_legacy(|| draw(state));
            }
            state.load.current = index;
        }
        cx.with_legacy(|| {
            flash(state);
            SleepThreadUntil(time.wrapping_add(840 / 30));
        });
        1
    }
    pub fn load_team_list(state: &mut MeleeSession) {
        let files = state
            .files
            .as_ref()
            .unwrap_or_else(|| panic!("melee file resources are not admitted"));
        state.load.refresh(&files.teams);
    }
    /// # Safety
    /// State must own the team being saved on the game thread.
    pub unsafe fn do_save_team(state: &mut MeleeSession, screen: &ContextLease) {
        let mut filename = state.melee_setup.teams[state.side as usize]
            .c_name()
            .to_bytes()
            .to_vec();
        filename.extend(b".mle");
        filename.truncate(254);
        let filename = CString::new(filename).unwrap();
        let ok = screen.with_selected(|| {
            let mut saved = mem::zeroed();
            ConfirmSaveLoad(&mut saved);
            let team = &state.melee_setup.teams[state.side as usize];
            let ok = write_team(&state.files().teams, &filename, team).is_ok();
            state.load.top = 0;
            state.load.current = 0;
            DrawStamp(&mut saved);
            DestroyDrawable(ReleaseDrawable(saved.frame));
            ok
        });
        if !ok {
            SaveProblem();
        }
        load_team_list(state);
        if let Some(index) = state.load.file_index(&filename) {
            assert!(index < state.load.count());
            state.load.current = index;
            state.load.top = index.saturating_sub(2);
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
