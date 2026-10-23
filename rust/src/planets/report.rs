//! Discovery report paging and typewriter presentation from `planets/report.c`.

fn character_delay(character: u32, last_in_word: bool) -> u32 {
    let mut delay = 0;
    if character == u32::from(b',') {
        delay += 840 / 4;
    }
    if matches!(character, 46 | 33 | 63) {
        delay += 840 / 2;
    } else {
        delay += 840 / 20;
    }
    if last_in_word {
        delay += 840 / 20;
    }
    delay
}

fn needs_page_marker(row: i32, rows: i32, remaining: u16, columns: i32, lf_remaining: u16) -> bool {
    row == rows - 1 && (i32::from(remaining) > columns || lf_remaining > 1)
}

#[cfg(not(test))]
mod live {
    use super::{character_delay, needs_page_marker};
    use crate::c_bindings::solar_system;
    use crate::comm::locdata::{CGlobData, CRect, CStamp};
    use crate::comm::sis_graphics::CText;
    use crate::mainloop::restart_menu::c_extern::SetContext;
    use crate::sound::types::CSoundPosition;
    use crate::starbase::cffi::CffiPlatform;
    use crate::starbase::gfx::{Gfx, Point, Rgba, Stamp};
    use crate::strings::unicode::{
        getCharFromString, utf8StringCount, utf8StringCountN, UniChar_isGraph,
    };
    use std::ffi::{c_void, CString};

    extern "C" {
        static SpaceJunkFrame: *mut c_void;
        static optWhichFonts: i32;
        static ScreenWidth: i32;
        static GlobData: CGlobData;
        #[cfg(feature = "debug-process")]
        static disableInteractivity: i32;
        fn SetAbsFrameIndex(frame: *mut c_void, index: u16) -> *mut c_void;
        fn GetFrameRect(frame: *mut c_void, rect: *mut c_void);
        fn GetScanContext(owner: *mut i32) -> *mut c_void;
        fn DestroyScanContext();
        fn SaveContextFrame(rect: *mut c_void) -> CStamp;
        fn SetContextFont(font: *mut c_void) -> *mut c_void;
        fn SetContextFontEffect(frame: *mut c_void) -> *mut c_void;
        fn GetStringAddress(string: *mut c_void) -> *const std::ffi::c_char;
        fn GetStringLength(string: *mut c_void) -> u16;
        fn font_DrawText(text: *mut CText);
        fn FlushInput();
        fn GetTimeCounter() -> u32;
        fn WaitForAnyButtonUntil(new_button: u32, timeout: u32, reset: u32) -> u32;
        fn WaitForAnyButton(new_button: u32, duration: u32, reset: u32) -> u32;
        fn WaitForNoInput(duration: u32, reset: u32) -> u32;
        fn PlaySound(
            sound: *mut c_void,
            position: CSoundPosition,
            object: *mut c_void,
            priority: u8,
        );
        fn NotPositional() -> CSoundPosition;
        fn ReleaseDrawable(frame: *mut c_void) -> *mut c_void;
        fn DestroyDrawable(drawable: *mut c_void) -> u32;
    }

    unsafe fn clear() {
        let g = CffiPlatform;
        let frame = SetAbsFrameIndex(SpaceJunkFrame, if optWhichFonts == 2 { 21 } else { 18 });
        let mut r = CRect::default();
        GetFrameRect(frame, (&mut r as *mut CRect).cast());
        g.batch();
        g.set_bg_color(Rgba::BLACK);
        g.clear_drawable();
        g.set_fg_color(Rgba::rgb15(0, 7, 0));
        let start_x = r.width >> 1;
        let mut y = 1;
        for _ in 0..75 / 6 {
            let mut x = start_x;
            for _ in 0..(ScreenWidth - 78) / 6 {
                let stamp = Stamp {
                    origin: Point { x, y },
                    frame: frame as usize,
                };
                if optWhichFonts == 2 {
                    g.draw_stamp(stamp);
                } else {
                    g.draw_filled_stamp(stamp);
                }
                x += r.width + 1;
            }
            y += r.height + 1;
        }
        g.unbatch();
    }

    unsafe fn make_report(sounds: *mut c_void, text: *const u8, mut remaining: u16) {
        let g = CffiPlatform;
        let end_page = CString::new(format!("{}\n", g.game_string(305 + 3))).unwrap();
        let end_len = utf8StringCount(end_page.as_ptr().cast()) as u16;
        let mut r = CRect::default();
        GetFrameRect(
            SetAbsFrameIndex(SpaceJunkFrame, 18),
            (&mut r as *mut CRect).cast(),
        );
        let columns = (ScreenWidth - 78) / 6;
        let rows = 75 / 6;
        let mut t = CText::default();
        t.p_str = text.cast();
        t.char_count = 1;
        let mut sleepy = true;
        let mut last_c = 0;
        let mut row = 0;
        FlushInput();
        t.baseline.y = r.height + 1;
        if remaining != 0 {
            clear();
            g.set_fg_color(Rgba::rgb15(0, 31, 0));
        }
        while remaining != 0 {
            let last_string = t.p_str;
            let mut next = t.p_str.cast::<u8>();
            let mut lf_remaining = remaining;
            while lf_remaining != 0 && getCharFromString(&mut next) != u32::from(b'\n') {
                lf_remaining -= 1;
            }
            let mut col: u16 = 0;
            if needs_page_marker(row, rows, remaining, columns, lf_remaining) {
                col = ((columns >> 1) - (i32::from(end_len) >> 1)) as u16;
                t.p_str = end_page.as_ptr();
                remaining = remaining.wrapping_add(end_len);
            }
            t.baseline.x = ((r.width >> 1) as i32 + i32::from(col) * i32::from(r.width + 1)) as i16;
            loop {
                let mut end = t.p_str.cast::<u8>();
                next = end;
                while UniChar_isGraph(getCharFromString(&mut next)) != 0 {
                    end = next;
                }
                let mut word_chars = utf8StringCountN(t.p_str.cast(), end) as u16;
                col = col.wrapping_add(word_chars);
                if i32::from(col) <= columns {
                    remaining = remaining.wrapping_sub(word_chars);
                    remaining = remaining.saturating_sub(1);
                    let mut timeout = GetTimeCounter();
                    while word_chars != 0 {
                        word_chars -= 1;
                        next = t.p_str.cast();
                        let c = getCharFromString(&mut next);
                        font_DrawText(&mut t);
                        if sleepy && GlobData.game_state.current_activity & (1 << 14) == 0 {
                            PlaySound(sounds, NotPositional(), std::ptr::null_mut(), 2);
                            timeout = timeout.wrapping_add(character_delay(c, word_chars == 0));
                            if WaitForAnyButtonUntil(1, timeout, 0) != 0 {
                                sleepy = false;
                                g.batch();
                            }
                        }
                        t.p_str = next.cast();
                        t.baseline.x += r.width + 1;
                    }
                    col += 1;
                    next = t.p_str.cast();
                    last_c = getCharFromString(&mut next);
                    t.p_str = next.cast();
                    t.baseline.x += r.width + 1;
                }
                if i32::from(col) > columns || last_c == u32::from(b'\n') || remaining == 0 {
                    break;
                }
            }
            t.baseline.y += r.height + 1;
            row += 1;
            if row == rows || remaining == 0 {
                t.p_str = last_string;
                if !sleepy {
                    g.unbatch();
                }
                if WaitForAnyButton(1, u32::MAX, 0) == 0 {
                    break;
                }
                t.baseline.y = r.height + 1;
                row = 0;
                if remaining != 0 {
                    if !sleepy {
                        g.batch();
                    }
                    clear();
                    g.set_fg_color(Rgba::rgb15(0, 31, 0));
                }
            }
        }
    }

    #[no_mangle]
    unsafe extern "C" fn DoDiscoveryReport(sounds: *mut c_void) {
        #[cfg(feature = "debug-process")]
        if disableInteractivity != 0 {
            return;
        }
        let mut own_context = 0;
        let context = GetScanContext(&mut own_context);
        let old_context = SetContext(context);
        let saved = SaveContextFrame(std::ptr::null_mut());
        let info = std::ptr::addr_of!((*solar_system::current()).SysInfo.PlanetInfo);
        let old_font = SetContextFont((*info).LanderFont.cast());
        let old_effect = SetContextFontEffect(if optWhichFonts == 2 {
            (*info).LanderFontEff.cast()
        } else {
            std::ptr::null_mut()
        });
        make_report(
            sounds,
            GetStringAddress((*info).DiscoveryString.cast()).cast(),
            GetStringLength((*info).DiscoveryString.cast()),
        );
        SetContextFontEffect(old_effect);
        SetContextFont(old_font);
        CffiPlatform.draw_stamp(Stamp {
            origin: Point {
                x: saved.origin.x,
                y: saved.origin.y,
            },
            frame: saved.frame as usize,
        });
        SetContext(old_context);
        if own_context != 0 {
            DestroyScanContext();
        }
        DestroyDrawable(ReleaseDrawable(saved.frame));
        WaitForNoInput(u32::MAX, 1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn punctuation_and_word_end_delays_match_the_readout() {
        assert_eq!(character_delay(u32::from(b'a'), false), 42);
        assert_eq!(character_delay(u32::from(b'a'), true), 84);
        assert_eq!(character_delay(u32::from(b','), false), 252);
        assert_eq!(character_delay(u32::from(b'.'), true), 462);
        assert_eq!(character_delay(u32::from(b'!'), false), 420);
        assert_eq!(character_delay(u32::from(b'?'), false), 420);
    }
    #[test]
    fn page_marker_reserves_last_row_only_when_text_does_not_fit() {
        assert!(!needs_page_marker(10, 12, 100, 40, 100));
        assert!(!needs_page_marker(11, 12, 40, 40, 0));
        assert!(!needs_page_marker(11, 12, 40, 40, 1));
        assert!(needs_page_marker(11, 12, 40, 40, 2));
        assert!(needs_page_marker(11, 12, 41, 40, 0));
    }
}
