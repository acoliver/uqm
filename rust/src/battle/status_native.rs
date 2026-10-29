//! Live captain-window rendering and crew/energy bookkeeping from status.c.

use crate::ships::native::abi::*;

#[no_mangle]
pub static mut status_y_offsets: [COORD; 2] = [0; 2];

fn status_offsets(height_pixels: i32) -> [COORD; 2] {
    [(height_pixels >> 1) as COORD, 0]
}

fn crew_text(value: COUNT) -> [u8; 40] {
    let mut buffer = [0; 40];
    let mut digits = [0; 5];
    let mut remaining = value;
    let mut count = 0;
    loop {
        digits[count] = b'0' + (remaining % 10) as u8;
        count += 1;
        remaining /= 10;
        if remaining == 0 {
            break;
        }
    }
    for index in 0..count {
        buffer[index] = digits[count - index - 1];
    }
    buffer
}

fn energy_delta(current: BYTE, maximum: BYTE, delta: SIZE) -> (bool, SIZE) {
    if delta >= 0 {
        if current.wrapping_add(delta as BYTE) > maximum {
            (true, SIZE::from(maximum) - SIZE::from(current))
        } else {
            (true, delta)
        }
    } else {
        ((-i32::from(delta)) as BYTE <= current, delta)
    }
}

fn crew_delta(current: COUNT, original: COUNT, maximum: COUNT, delta: SIZE) -> (bool, COUNT, SIZE) {
    if delta > 0 {
        let updated = current.wrapping_add(delta as COUNT);
        if updated > maximum {
            (true, maximum, maximum.wrapping_sub(original) as SIZE)
        } else {
            (true, updated, delta)
        }
    } else if delta < 0 {
        if current > (-i32::from(delta)) as COUNT {
            (true, current.wrapping_add(delta as COUNT), delta)
        } else {
            (false, 0, (-i32::from(current as SIZE)) as SIZE)
        }
    } else {
        (true, current, 0)
    }
}

// BUILD_COLOR ignores the palette index; every MAKE_RGB15 color is opaque.
fn rgb15(r: u8, g: u8, b: u8) -> Color {
    Color {
        r: ((u16::from(r) << 3) | u16::from(r >> 2)) as u8,
        g: ((u16::from(g) << 3) | u16::from(g >> 2)) as u8,
        b: ((u16::from(b) << 3) | u16::from(b >> 2)) as u8,
        a: 255,
    }
}

fn rect(x: COORD, y: COORD, width: SIZE, height: SIZE) -> RECT {
    RECT {
        corner: POINT { x, y },
        extent: EXTENT { width, height },
    }
}

fn turn_frames(delta: STATUS_FLAGS, current: STATUS_FLAGS, pass: COUNT) -> ([i16; 2], usize) {
    if delta & LEFT as u16 != 0 {
        if delta & RIGHT as u16 == 0 {
            (
                [
                    if pass == 2 {
                        if current & LEFT as u16 != 0 {
                            4
                        } else {
                            2
                        }
                    } else {
                        3
                    },
                    0,
                ],
                1,
            )
        } else if current & RIGHT as u16 != 0 {
            if pass == 1 {
                ([3, 2], 2)
            } else {
                ([1, 0], 2)
            }
        } else if pass == 1 {
            ([1, 2], 2)
        } else {
            ([3, 4], 2)
        }
    } else if delta & RIGHT as u16 != 0 {
        (
            [
                if pass == 2 {
                    if current & RIGHT as u16 != 0 {
                        0
                    } else {
                        2
                    }
                } else {
                    1
                },
                0,
            ],
            1,
        )
    } else {
        ([0, 0], 0)
    }
}

pub(crate) fn explosion_flash(life: COUNT, y: COORD, mut draw: impl FnMut(Color, RECT, bool)) {
    let black = rgb15(0, 0, 0);
    let mut i = (35u16.wrapping_sub(life)) as BYTE;
    let (color, r) = if i <= 4 {
        let colors = [
            (15, 0, 0),
            (31, 25, 25),
            (31, 17, 0),
            (31, 28, 0),
            (31, 31, 31),
        ];
        let (r, g, b) = colors[i as usize];
        (rgb15(r, g, b), rect(4, y.wrapping_add(69), 55, 30))
    } else {
        i = i.wrapping_sub(5);
        if i <= 14 {
            let colors = [
                (30, 31, 18),
                (31, 31, 10),
                (31, 31, 0),
                (31, 28, 0),
                (31, 24, 0),
                (31, 21, 0),
                (31, 17, 0),
                (31, 14, 0),
                (31, 10, 0),
                (31, 7, 0),
                (31, 3, 0),
                (27, 0, 0),
                (23, 0, 0),
                (19, 0, 0),
                (15, 0, 0),
            ];
            let (red, g, b) = colors[i as usize];
            let mut r = rect(
                4 + i as i16,
                y.wrapping_add(69 + i as i16),
                55 - ((i as i16) << 1),
                30 - ((i as i16) << 1),
            );
            if r.extent.height == 2 {
                r.extent.height += 1;
            }
            draw(black, r, false);
            r.corner.x += 1;
            r.corner.y = r.corner.y.wrapping_add(1);
            r.extent.width -= 2;
            r.extent.height -= 2;
            (rgb15(red, g, b), r)
        } else {
            i = i.wrapping_sub(15);
            if i <= 4 {
                let (x, gap, color) = match i {
                    0 => (19, 23, rgb15(19, 0, 0)),
                    1 => (20, 19, rgb15(7, 0, 0)),
                    2 => (22, 13, rgb15(27, 0, 0)),
                    3 => (25, 5, rgb15(31, 0, 0)),
                    4 => (29, 1, rgb15(31, 0x50, 5)),
                    _ => unreachable!(),
                };
                let mut r = rect(
                    x,
                    y.wrapping_add(84),
                    if i == 4 { 2 } else { i as i16 + 1 },
                    1,
                );
                draw(black, r, true);
                r.corner.x += gap + r.extent.width;
                draw(black, r, true);
                r.corner.x -= gap;
                r.extent.width = gap;
                (color, r)
            } else {
                i = i.wrapping_sub(5);
                let color = if i <= 2 {
                    rgb15([23, 15, 11][i as usize], 0, 0)
                } else {
                    black
                };
                (color, rect(31, y.wrapping_add(84), 1, 1))
            }
        }
    };
    draw(color, r, true);
}

#[cfg(not(test))]
#[allow(non_snake_case)]
pub(crate) mod engine {
    use super::*;
    use crate::engine::battle::{Cx, ElementId};
    use crate::ships::native::{rel_frame_index, starship};

    /// # Safety
    /// Status rendering runs on the game thread after screen initialization.
    #[no_mangle]
    pub unsafe extern "C" fn InitStatusOffsets() {
        status_y_offsets = status_offsets(ScreenHeight);
    }

    pub(crate) unsafe fn offset(ship: *const STARSHIP) -> COORD {
        assert!((*ship).playerNr >= 0);
        *(&raw const status_y_offsets)
            .cast::<COORD>()
            .add((*ship).playerNr as usize)
    }

    unsafe fn filled(color: Color, mut r: RECT) {
        SetContextForeGroundColor(color);
        DrawFilledRectangle(&mut r);
    }

    pub(crate) unsafe fn CaptainsWindow(
        captain: *mut CAPTAIN_STUFF,
        y: COORD,
        delta: STATUS_FLAGS,
        current: STATUS_FLAGS,
        pass: COUNT,
    ) {
        let mut stamp = STAMP {
            origin: POINT {
                x: 4,
                y: y.wrapping_add(69),
            },
            frame: std::ptr::null_mut(),
        };
        let (frames, count) = turn_frames(delta, current, pass);
        for &frame in &frames[..count] {
            stamp.frame = rel_frame_index((*captain).turn, frame);
            DrawStamp(&mut stamp);
        }
        for (flag, base) in [
            (THRUST, (*captain).thrust),
            (WEAPON, (*captain).weapon),
            (SPECIAL, (*captain).special),
        ] {
            if delta & flag as u16 != 0 {
                let frame = if pass == 1 {
                    1
                } else if current & flag as u16 != 0 {
                    2
                } else {
                    0
                };
                stamp.frame = rel_frame_index(base, frame);
                DrawStamp(&mut stamp);
            }
        }
    }

    /// # Safety
    /// Ship info, fonts, and the active graphics context must be live.
    #[no_mangle]
    pub unsafe extern "C" fn DrawBattleCrewAmount(info: *mut SHIP_INFO, y: COORD) {
        let x = if optWhichMenu == OPT_PC as i32 { 4 } else { 12 };
        let bytes = crew_text((*info).crew_level);
        let mut text = TEXT {
            baseline: POINT {
                x,
                y: y.wrapping_add(64),
            },
            pStr: bytes.as_ptr().cast(),
            align: TEXT_ALIGN_ALIGN_LEFT,
            CharCount: COUNT::MAX,
        };
        SetContextFont(StarConFont);
        filled(
            rgb15(10, 10, 10),
            rect(x, text.baseline.y.wrapping_sub(5), 24, 5),
        );
        SetContextForeGroundColor(rgb15(0, 0, 0));
        font_DrawText(&mut text);
    }

    /// # Safety
    /// Ship, captain assets, and the active status context must be live.
    #[no_mangle]
    pub unsafe extern "C" fn DrawCaptainsWindow(ship: *mut STARSHIP) {
        let rd = (*ship).RaceDescPtr;
        let captain = &raw mut (*rd).ship_data.captain_control;
        let mut frame = (*captain).background;
        if !frame.is_null() {
            frame = SetAbsFrameIndex(frame, 0);
            (*captain).background = frame;
            frame = rel_frame_index(frame, 1);
            (*captain).turn = frame;
            frame = rel_frame_index(frame, 5);
            (*captain).thrust = frame;
            frame = rel_frame_index(frame, 3);
            (*captain).weapon = frame;
            frame = rel_frame_index(frame, 3);
            (*captain).special = frame;
        }
        BatchGraphics();
        let y = offset(ship);
        let height = (ScreenHeight >> 1) as SIZE;
        let dark = rgb15(8, 8, 8);
        let light = rgb15(16, 16, 16);
        filled(
            rgb15(10, 10, 10),
            rect(2, y.wrapping_add(65), 60, height.wrapping_sub(67)),
        );
        filled(
            dark,
            rect(1, y.wrapping_add(65), 1, height.wrapping_sub(67)),
        );
        filled(
            dark,
            rect(0, y.wrapping_add(65), 1, height.wrapping_sub(66)),
        );
        filled(
            light,
            rect(63, y.wrapping_add(65), 1, height.wrapping_sub(65)),
        );
        filled(
            light,
            rect(62, y.wrapping_add(65), 1, height.wrapping_sub(65)),
        );
        filled(
            light,
            rect(1, y.wrapping_add(height.wrapping_sub(2)), 62, 1),
        );
        filled(
            light,
            rect(0, y.wrapping_add(height.wrapping_sub(1)), 63, 1),
        );
        let cy = y.wrapping_add(69);
        filled(dark, rect(59, cy, 1, 30));
        filled(dark, rect(3, cy.wrapping_add(30), 57, 1));
        filled(light, rect(3, cy.wrapping_sub(1), 57, 1));
        filled(light, rect(3, cy, 1, 30));
        let mut stamp = STAMP {
            origin: POINT { x: 4, y: cy },
            frame: (*captain).background,
        };
        DrawStamp(&mut stamp);
        if (*ship).captains_name_index == 0 && (*ship).playerNr == RPG_PLAYER_NUM as SIZE {
            let mut text = TEXT {
                baseline: POINT {
                    x: 32,
                    y: cy.wrapping_add(6),
                },
                pStr: (&raw const GlobData.SIS_state.CommanderName).cast(),
                align: TEXT_ALIGN_ALIGN_CENTER,
                CharCount: COUNT::MAX,
            };
            SetContextForeGroundColor(rgb15(0, 20, 0));
            SetContextFont(TinyFont);
            font_DrawText(&mut text);
        }
        if (*rd).ship_info.max_crew > MAX_CREW_SIZE as COUNT
            || (*rd).ship_info.ship_flags & PLAYER_CAPTAIN as u16 != 0
        {
            DrawBattleCrewAmount(&raw mut (*rd).ship_info, y);
        }
        UnbatchGraphics();
    }

    /// # Safety
    /// Element and attached ship/race data must be live on the battle thread.
    #[no_mangle]
    pub unsafe extern "C" fn DeltaEnergy(e: *mut ELEMENT, delta: SIZE) -> BOOLEAN {
        let ship = starship(e);
        let rd = (*ship).RaceDescPtr;
        let info = &raw mut (*rd).ship_info;
        let (success, adjusted) = energy_delta((*info).energy_level, (*info).max_energy, delta);
        if !success {
            (*ship).cur_status_flags |= LOW_ON_ENERGY as u16;
        } else {
            (*ship).cur_status_flags &= !(LOW_ON_ENERGY as u16);
            (*ship).energy_counter = (*rd).characteristics.energy_wait;
            DeltaStatistics(info, offset(ship), 0, adjusted);
        }
        success as BOOLEAN
    }

    /// # Safety
    /// Element and attached ship/race data must be live on the battle thread.
    #[no_mangle]
    pub unsafe extern "C" fn DeltaCrew(e: *mut ELEMENT, delta: SIZE) -> BOOLEAN {
        if GlobData.Game_state.CurrentActivity as BYTE == IN_LAST_BATTLE as BYTE
            && (*e).playerNr == NPC_PLAYER_NUM as SIZE
        {
            return 1;
        }
        let ship = starship(e);
        let info = &raw mut (*(*ship).RaceDescPtr).ship_info;
        let (success, updated, adjusted) = crew_delta(
            (*e).__bindgen_anon_2.crew_level,
            (*info).crew_level,
            (*info).max_crew,
            delta,
        );
        (*e).__bindgen_anon_2.crew_level = updated;
        DeltaStatistics(info, offset(ship), adjusted, 0);
        success as BOOLEAN
    }

    /// # Safety
    /// Element, owning ship, captain frames, and active status context must be live.
    #[no_mangle]
    pub unsafe extern "C" fn PreProcessStatus(e: *mut ELEMENT) {
        let ship = starship(e);
        if (*ship).captains_name_index != 0 || (*ship).playerNr == RPG_PLAYER_NUM as SIZE {
            let current = (*ship).cur_status_flags;
            let delta = ((*ship).old_status_flags ^ current)
                & (LEFT | RIGHT | THRUST | WEAPON | SPECIAL) as u16;
            if delta != 0 {
                CaptainsWindow(
                    &raw mut (*(*ship).RaceDescPtr).ship_data.captain_control,
                    offset(ship),
                    delta,
                    current,
                    1,
                );
            }
        }
    }

    pub fn PostProcessStatus(cx: &mut Cx<'_>, e: ElementId) {
        let ship = cx.owner(e);
        if cx.ship(ship).captains_name_index != 0
            || cx.ship(ship).playerNr == RPG_PLAYER_NUM as SIZE
        {
            let current = cx.ship(ship).cur_status_flags;
            let y = cx.status_offset(ship);
            if cx.element(e).hit_points() == 0 {
                cx.ship_state_mut(ship).status &=
                    !((LEFT | RIGHT | THRUST | WEAPON | SPECIAL) as u16);
                if cx.desc(ship).ship_info.crew_level == 0 {
                    cx.draw_explosion_flash(cx.element(e).life_span(), y);
                }
            }
            let mut delta = (cx.ship(ship).old_status_flags ^ current)
                & (LEFT | RIGHT | THRUST | WEAPON | SPECIAL | LOW_ON_ENERGY) as u16;
            if delta != 0 {
                if delta & LOW_ON_ENERGY as u16 != 0 {
                    cx.draw_energy_label(
                        y,
                        if current & LOW_ON_ENERGY as u16 == 0 {
                            1
                        } else {
                            -1
                        },
                    );
                }
                delta &= (LEFT | RIGHT | THRUST | WEAPON | SPECIAL) as u16;
                if delta != 0 {
                    cx.draw_captain_controls(ship, y, delta, current);
                }
            }
            cx.ship_state_mut(ship).old_status = current;
        }
    }
}

#[cfg(not(test))]
pub use engine::{
    DeltaCrew, DeltaEnergy, DrawBattleCrewAmount, DrawCaptainsWindow, InitStatusOffsets,
    PostProcessStatus, PreProcessStatus,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offsets_put_player_zero_at_bottom() {
        assert_eq!(status_offsets(240), [120, 0]);
        assert_eq!(status_offsets(481), [240, 0]);
    }

    #[test]
    fn energy_checks_narrow_but_do_not_replace_the_accepted_delta() {
        assert_eq!(energy_delta(250, 255, 10), (true, 10));
        assert_eq!(energy_delta(200, 255, 256), (true, 256));
        assert_eq!(energy_delta(12, 20, 10), (true, 8));
        assert_eq!(energy_delta(2, 255, -3), (false, -3));
        assert_eq!(energy_delta(12, 20, -12), (true, -12));
        assert_eq!(energy_delta(0, 255, i16::MIN), (true, i16::MIN));
    }

    #[test]
    fn crew_clamp_uses_ship_info_before_update() {
        assert_eq!(crew_delta(9, 7, 10, 4), (true, 10, 3));
        assert_eq!(crew_delta(3, 3, 10, -3), (false, 0, -3));
        assert_eq!(crew_delta(4, 4, 10, -3), (true, 1, -3));
        assert_eq!(crew_delta(65535, 65535, 65535, 1), (true, 0, 1));
        assert_eq!(
            crew_delta(32768, 32768, 65535, i16::MIN),
            (false, 0, i16::MIN)
        );
    }

    #[test]
    fn crew_text_is_unsigned_decimal_and_terminated() {
        assert_eq!(&crew_text(0)[..2], b"0\0");
        assert_eq!(&crew_text(65535)[..6], b"65535\0");
    }

    #[test]
    fn captain_turn_animation_orders_both_draws() {
        let both = (LEFT | RIGHT) as u16;
        assert_eq!(turn_frames(both, RIGHT as u16, 1), ([3, 2], 2));
        assert_eq!(turn_frames(both, RIGHT as u16, 2), ([1, 0], 2));
        assert_eq!(turn_frames(both, LEFT as u16, 1), ([1, 2], 2));
        assert_eq!(turn_frames(both, LEFT as u16, 2), ([3, 4], 2));
        assert_eq!(turn_frames(LEFT as u16, LEFT as u16, 2), ([4, 0], 1));
        assert_eq!(turn_frames(RIGHT as u16, 0, 2), ([2, 0], 1));
    }

    #[test]
    fn explosion_flash_renders_each_phase_and_the_two_pixel_center() {
        let capture = |life| {
            let mut ops = Vec::new();
            explosion_flash(life, 120, |c, r, fill| ops.push((c, r, fill)));
            ops
        };
        let ops = capture(35);
        assert_eq!(ops.len(), 1);
        assert_eq!(ops[0].0.a, 255);
        assert_eq!(ops[0].1.corner.y, 189);
        assert_eq!(ops[0].1.extent.width, 55);
        let ops = capture(16); // shrinking rectangle, i=14
        assert_eq!(ops.len(), 2);
        assert!(!ops[0].2);
        assert_eq!(ops[0].1.extent.height, 3);
        assert_eq!(ops[1].1.extent.height, 1);
        let ops = capture(11); // side flashes, i=4
        assert_eq!(ops.len(), 3);
        assert_eq!(ops[0].1.extent.width, 2);
        assert_eq!(ops[2].1.extent.width, 1);
        assert_eq!(ops[2].0.g, 148); // C BYTE truncation of CC5TO8(0x50)
        let ops = capture(7);
        assert_eq!(ops.len(), 1);
        assert_eq!(ops[0].1.corner.x, 31);
        assert_eq!(ops[0].0.r, 0);
    }
}
