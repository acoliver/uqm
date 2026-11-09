//! Bidirectional credits scrolling and communication outtakes, ported from uqm/credits.c.

use crate::navigation::abi::*;
use std::ffi::c_int;

#[cfg(not(test))]
const CREDITS_BASE_RATE: c_int = 9;
const CREDITS_MAX_RATE: c_int = 130;
const CREDITS_FRAME_RATE: c_int = 36;
const MAX_CREDIT_FRAMES: usize = 32;

fn frame_index(index: c_int) -> usize {
    ((index + MAX_CREDIT_FRAMES as c_int) % MAX_CREDIT_FRAMES as c_int) as usize
}

fn scroll_step(rate: c_int) -> (c_int, c_int, TimePeriod) {
    let speed = rate.abs();
    if speed != 0 {
        let direction = rate / speed;
        let step = (speed + CREDITS_FRAME_RATE - 1) / CREDITS_FRAME_RATE;
        let delay = ONE_SECOND as c_int * step / speed;
        (direction, step * direction, delay as TimePeriod)
    } else {
        (0, 0, ONE_SECOND)
    }
}

#[cfg(not(test))]
pub(crate) mod runtime {
    use super::super::intro::{parse_text_lines, BLACK_COLOR, WANT_PIXMAP};
    use super::*;
    use crate::ui::rgb15;

    use crate::engine::{
        battle::Cx,
        observe::{CheckpointView, Io},
    };
    use std::ffi::c_char;
    use std::ptr;

    // uqm/comm.h is not included in navigation's generated ABI yet.

    use crate::comm::comm_abi::SetCommIntroMode;

    extern "C" {
        static mut CommWndRect: crate::comm::locdata::CRect;
    }
    const CIM_CROSSFADE_WINDOW: c_int = 1;
    const TRANS_COLOR: Color = rgb15(0, 0, 31);
    const CREDITS_TEXT_COLOR: Color = rgb15(31, 31, 31);
    const CREDITS_TIMEOUT: TimePeriod = ONE_SECOND * 5;
    const MAX_TEXT_LINES: usize = 50;
    const MAX_TEXT_COLS: usize = 5;

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct CreditTextFrame {
        frame: FRAME,
        str_index: c_int,
    }
    const EMPTY_FRAME: CreditTextFrame = CreditTextFrame {
        frame: ptr::null_mut(),
        str_index: 0,
    };

    #[derive(Clone, Copy)]
    struct ColumnFormat {
        align: TEXT_ALIGN,
        basex: COORD,
    }

    struct CreditsInputState {
        allow_cancel: BOOLEAN,
        allow_speed_change: BOOLEAN,
        close_when_done: BOOLEAN,
        close_time_out: DWORD,
    }

    static mut CREDITS_RATE: c_int = 0;
    static mut OUT_TAKES_RUNNING: BOOLEAN = 0;
    static mut CREDITS_RUNNING: BOOLEAN = 0;
    static mut CREDITS_TAB: STRING = ptr::null_mut();
    static mut CREDITS_BACK: FRAME = ptr::null_mut();
    static mut DRAW_CONTEXT: CONTEXT = ptr::null_mut();
    static mut LOCAL_CONTEXT: CONTEXT = ptr::null_mut();
    static mut CREDITS_FRAME: FRAME = ptr::null_mut();
    static mut CREDITS_EXTENT: EXTENT = EXTENT {
        width: 0,
        height: 0,
    };
    static mut TEXT_FRAMES: [CreditTextFrame; MAX_CREDIT_FRAMES] = [EMPTY_FRAME; MAX_CREDIT_FRAMES];
    static mut FIRST_FRAME: usize = 0;
    static mut LAST_FRAME: usize = 0;
    static mut TOTAL_HEIGHT: c_int = 0;
    static mut CUR_FRAME_OFS: c_int = 0;
    static mut NEXT_TIME: TimeCount = 0;
    static mut CREDITS_FONTS: [FONT; 3] = [ptr::null_mut(); 3];

    unsafe fn transparent_frame(w: c_int, h: c_int, color: Color) -> FRAME {
        let frame = CaptureDrawable(CreateDrawable(WANT_PIXMAP, w as SIZE, h as SIZE, 1));
        SetFrameTransparentColor(frame, color);
        let old_frame = SetContextFGFrame(frame);
        SetContextBackGroundColor(color);
        ClearDrawable();
        SetContextFGFrame(old_frame);
        frame
    }

    unsafe fn render_text_frame(
        temp_context: CONTEXT,
        index: &mut c_int,
        direction: c_int,
        back: Color,
        fore: Color,
    ) -> FRAME {
        let mut line = ptr::null_mut();
        let count = GetStringTableCount(CREDITS_TAB) as c_int;
        if *index < 0 || *index >= count {
            let next = *index + direction;
            if next < 0 || next >= count {
                return ptr::null_mut();
            }
            *index = next;
        }
        while *index >= 0 && *index < GetStringTableCount(CREDITS_TAB) as c_int {
            line = GetStringAddress(SetAbsStringTableIndex(CREDITS_TAB, *index as COUNT));
            *index += direction;
            if !line.is_null() && *line != 0 {
                break;
            }
        }
        if line.is_null() || *line == 0 {
            return ptr::null_mut();
        }
        let mut size = 0;
        let mut alignment = [0 as c_char; 32];
        let mut scanned = 0;
        if libc::sscanf(
            line,
            c"%d %31s %n".as_ptr(),
            ptr::addr_of_mut!(size),
            alignment.as_mut_ptr(),
            ptr::addr_of_mut!(scanned),
        ) != 2
            || size <= 0
        {
            return ptr::null_mut();
        }
        line = line.add(scanned as usize);
        let mut buffer = [0 as c_char; 2048];
        utf8StringCopy((buffer.as_mut_ptr()).cast(), buffer.len(), (line).cast());
        let mut lines: [TEXT; MAX_TEXT_LINES] = std::mem::zeroed();
        let rows = parse_text_lines(lines.as_mut_ptr(), MAX_TEXT_LINES, buffer.as_mut_ptr());
        if rows == 0 {
            return ptr::null_mut();
        }
        let mut count = rows;
        for row in 0..rows {
            lines[row].baseline.x = 0;
            lines[row].baseline.y = (row + 1) as COORD;
            let mut next_col = libc::strchr(lines[row].pStr, b'\t' as c_int);
            let mut col = 1;
            while col < MAX_TEXT_COLS && !next_col.is_null() {
                *next_col = 0;
                next_col = next_col.add(1);
                if count < MAX_TEXT_LINES {
                    lines[count].pStr = next_col;
                    lines[count].CharCount = COUNT::MAX;
                    lines[count].baseline.x = col as COORD;
                    lines[count].baseline.y = (row + 1) as COORD;
                    count += 1;
                }
                col += 1;
                next_col = libc::strchr(next_col, b'\t' as c_int);
            }
        }
        let mut formats = [ColumnFormat {
            align: TEXT_ALIGN_ALIGN_LEFT,
            basex: CREDITS_EXTENT.width / 64,
        }; MAX_TEXT_COLS];
        let mut font_index = 0;
        while font_index < 3 && size > [13, 17, 45][font_index] {
            font_index += 1;
        }
        if font_index == 3 {
            return ptr::null_mut();
        }
        let mut sample = TEXT {
            align: TEXT_ALIGN_ALIGN_LEFT,
            baseline: POINT { x: 100, y: 100 },
            pStr: c" ".as_ptr(),
            CharCount: 1,
        };
        let old_context = SetContext(temp_context);
        SetContextFont(CREDITS_FONTS[font_index]);
        let mut leading: SIZE = 0;
        GetContextFontLeading(&mut leading);
        let mut margin: RECT = std::mem::zeroed();
        TextRect(&mut sample, &mut margin, ptr::null_mut());
        let mut col = libc::strtok(alignment.as_mut_ptr(), c",".as_ptr());
        let mut i = 0;
        while !col.is_null() && i < MAX_TEXT_COLS {
            formats[i].align = TEXT_ALIGN_ALIGN_LEFT;
            formats[i].basex = margin.extent.width;
            let mut align = 0 as c_char;
            let mut x = 0;
            let n = libc::sscanf(
                col,
                c"%c/%d".as_ptr(),
                ptr::addr_of_mut!(align),
                ptr::addr_of_mut!(x),
            );
            if n >= 1 {
                match align as u8 {
                    b'L' => {
                        formats[i].align = TEXT_ALIGN_ALIGN_LEFT;
                        if n >= 2 {
                            formats[i].basex = x as COORD;
                        }
                    }
                    b'C' => {
                        formats[i].align = TEXT_ALIGN_ALIGN_CENTER;
                        formats[i].basex = if n >= 2 {
                            x as COORD
                        } else {
                            CREDITS_EXTENT.width / 2
                        };
                    }
                    b'R' => {
                        formats[i].align = TEXT_ALIGN_ALIGN_RIGHT;
                        formats[i].basex = if n >= 2 {
                            x as COORD
                        } else {
                            (CREDITS_EXTENT.width as c_int - margin.extent.width as c_int) as COORD
                        };
                    }
                    _ => (),
                }
            }
            i += 1;
            col = libc::strtok(ptr::null_mut(), c",".as_ptr());
        }
        for text in &mut lines[..count] {
            let format = formats[text.baseline.x as usize];
            text.align = format.align;
            text.baseline.x = format.basex;
            text.baseline.y = (text.baseline.y as c_int * leading as c_int) as COORD;
        }
        let frame = transparent_frame(
            CREDITS_EXTENT.width as c_int,
            leading as c_int * rows as c_int + (leading as c_int >> 1),
            back,
        );
        let old_frame = SetContextFGFrame(frame);
        SetContextForeGroundColor(fore);
        for text in &mut lines[..count] {
            font_DrawText(text);
        }
        SetContextFGFrame(old_frame);
        SetContext(old_context);
        frame
    }

    unsafe fn render_screen(target: CONTEXT) {
        let old_context = SetContext(target);
        let mut stamp = STAMP {
            origin: POINT { x: 0, y: 0 },
            frame: CREDITS_BACK,
        };
        DrawStamp(&mut stamp);
        stamp.origin.y = (-CUR_FRAME_OFS) as COORD;
        let mut i = FIRST_FRAME;
        while i != LAST_FRAME {
            let mut rect: RECT = std::mem::zeroed();
            stamp.frame = TEXT_FRAMES[i].frame;
            DrawStamp(&mut stamp);
            GetFrameRect(stamp.frame, &mut rect);
            stamp.origin.y = (stamp.origin.y as c_int + rect.extent.height as c_int) as COORD;
            i = frame_index(i as c_int + 1);
        }
        if OUT_TAKES_RUNNING != 0 {
            SetContextForeGroundColor(TRANS_COLOR);
            DrawFilledRectangle(ptr::addr_of_mut!(CommWndRect).cast());
        }
        SetContext(old_context);
    }

    unsafe fn init_credits() {
        TEXT_FRAMES = [EMPTY_FRAME; MAX_CREDIT_FRAMES];
        #[cfg(feature = "debug-process")]
        {
            LOCAL_CONTEXT = CreateContextAux(c"Credits.LocalContext".as_ptr());
            DRAW_CONTEXT = CreateContextAux(c"Credits.DrawContext".as_ptr());
        }
        #[cfg(not(feature = "debug-process"))]
        {
            LOCAL_CONTEXT = CreateContextAux();
            DRAW_CONTEXT = CreateContextAux();
        }
        let target_frame = GetContextFGFrame();
        let mut rect: RECT = std::mem::zeroed();
        GetContextClipRect(&mut rect);
        CREDITS_EXTENT = rect.extent;
        let old_context = SetContext(LOCAL_CONTEXT);
        CREDITS_FRAME = transparent_frame(
            CREDITS_EXTENT.width as c_int,
            CREDITS_EXTENT.height as c_int,
            TRANS_COLOR,
        );
        SetContextFGFrame(CREDITS_FRAME);
        TEXT_FRAMES[0].frame = transparent_frame(1, CREDITS_EXTENT.height as c_int, TRANS_COLOR);
        TEXT_FRAMES[0].str_index = -1;
        FIRST_FRAME = 0;
        LAST_FRAME = FIRST_FRAME + 1;
        TOTAL_HEIGHT = (*TEXT_FRAMES[0].frame).Bounds.height as c_int;
        CUR_FRAME_OFS = 0;
        SetContext(DRAW_CONTEXT);
        SetContextFGFrame(target_frame);
        SetContext(old_context);
        render_screen(LOCAL_CONTEXT);
        CREDITS_RATE = CREDITS_BASE_RATE;
        CREDITS_RUNNING = 1;
    }

    unsafe fn free_text_frame(index: usize) {
        DestroyDrawable(ReleaseDrawable(TEXT_FRAMES[index].frame));
        TEXT_FRAMES[index].frame = ptr::null_mut();
    }

    unsafe fn uninit_credits() {
        DestroyContext(DRAW_CONTEXT);
        DRAW_CONTEXT = ptr::null_mut();
        DestroyContext(LOCAL_CONTEXT);
        LOCAL_CONTEXT = ptr::null_mut();
        DestroyDrawable(ReleaseDrawable(CREDITS_FRAME));
        CREDITS_FRAME = ptr::null_mut();
        while FIRST_FRAME != LAST_FRAME {
            free_text_frame(FIRST_FRAME);
            FIRST_FRAME = frame_index(FIRST_FRAME as c_int + 1);
        }
    }

    unsafe fn deficit_height() -> c_int {
        let mut max_pos = -CUR_FRAME_OFS;
        let mut i = FIRST_FRAME;
        while i != LAST_FRAME {
            let mut rect: RECT = std::mem::zeroed();
            GetFrameRect(TEXT_FRAMES[i].frame, &mut rect);
            max_pos += rect.extent.height as c_int;
            i = frame_index(i as c_int + 1);
        }
        CREDITS_EXTENT.height as c_int - max_pos
    }

    static mut VICTORY_CREDITS_ACTIVE: bool = false;

    unsafe fn process_frame(checkpoint: &mut dyn FnMut(CheckpointView<'_>)) {
        let now = GetTimeCounter();
        if now < NEXT_TIME {
            return;
        }
        let (direction, step, delay) = scroll_step(CREDITS_RATE);
        NEXT_TIME = GetTimeCounter().wrapping_add(delay);
        let mut stamp = STAMP {
            origin: POINT { x: 0, y: 0 },
            frame: CREDITS_FRAME,
        };
        let old_context = SetContext(DRAW_CONTEXT);
        DrawStamp(&mut stamp);
        SetContext(old_context);
        FlushGraphics();
        let deficit = deficit_height();
        CUR_FRAME_OFS += step;
        if CUR_FRAME_OFS < -(CREDITS_EXTENT.height as c_int / 20) {
            if CREDITS_RATE < 0 {
                CREDITS_RATE -= CREDITS_RATE / 10 - 1;
            }
        } else if deficit > CREDITS_EXTENT.height as c_int / 25 {
            if CREDITS_RATE > 0 {
                CREDITS_RATE -= CREDITS_RATE / 10 + 1;
            }
            CREDITS_RUNNING = (CREDITS_RATE != 0) as BOOLEAN;
            if CREDITS_RUNNING == 0 && VICTORY_CREDITS_ACTIVE {
                checkpoint(CheckpointView::CreditsCompleted);
            }
        } else if CREDITS_RUNNING == 0 {
            CREDITS_RUNNING = 1;
        }
        let mut rect: RECT = std::mem::zeroed();
        if FIRST_FRAME != LAST_FRAME {
            if direction > 0 {
                GetFrameRect(TEXT_FRAMES[FIRST_FRAME].frame, &mut rect);
                if CUR_FRAME_OFS >= rect.extent.height as c_int {
                    TOTAL_HEIGHT -= rect.extent.height as c_int;
                    free_text_frame(FIRST_FRAME);
                    FIRST_FRAME = frame_index(FIRST_FRAME as c_int + 1);
                    CUR_FRAME_OFS -= rect.extent.height as c_int;
                }
            } else if direction < 0 {
                let index = frame_index(LAST_FRAME as c_int - 1);
                GetFrameRect(TEXT_FRAMES[index].frame, &mut rect);
                let frame_pos = TOTAL_HEIGHT - CUR_FRAME_OFS - rect.extent.height as c_int;
                if frame_pos >= CREDITS_EXTENT.height as c_int {
                    LAST_FRAME = index;
                    TOTAL_HEIGHT -= rect.extent.height as c_int;
                    free_text_frame(LAST_FRAME);
                }
            }
        }
        if direction > 0 {
            let mut next = 0;
            if FIRST_FRAME != LAST_FRAME {
                next = TEXT_FRAMES[frame_index(LAST_FRAME as c_int - 1)].str_index + 1;
            }
            while TOTAL_HEIGHT - CUR_FRAME_OFS < CREDITS_EXTENT.height as c_int
                && next < GetStringTableCount(CREDITS_TAB) as c_int
            {
                let index = LAST_FRAME;
                TEXT_FRAMES[index].frame = render_text_frame(
                    LOCAL_CONTEXT,
                    &mut next,
                    direction,
                    BLACK_COLOR,
                    CREDITS_TEXT_COLOR,
                );
                TEXT_FRAMES[index].str_index = next - 1;
                if !TEXT_FRAMES[index].frame.is_null() {
                    GetFrameRect(TEXT_FRAMES[index].frame, &mut rect);
                    TOTAL_HEIGHT += rect.extent.height as c_int;
                    LAST_FRAME = frame_index(LAST_FRAME as c_int + 1);
                }
            }
        } else if direction < 0 {
            let mut next = GetStringTableCount(CREDITS_TAB) as c_int - 1;
            if FIRST_FRAME != LAST_FRAME {
                next = TEXT_FRAMES[FIRST_FRAME].str_index - 1;
            }
            while CUR_FRAME_OFS < 0 && next >= 0 {
                let index = frame_index(FIRST_FRAME as c_int - 1);
                TEXT_FRAMES[index].frame = render_text_frame(
                    LOCAL_CONTEXT,
                    &mut next,
                    direction,
                    BLACK_COLOR,
                    CREDITS_TEXT_COLOR,
                );
                TEXT_FRAMES[index].str_index = next + 1;
                if !TEXT_FRAMES[index].frame.is_null() {
                    GetFrameRect(TEXT_FRAMES[index].frame, &mut rect);
                    TOTAL_HEIGHT += rect.extent.height as c_int;
                    FIRST_FRAME = index;
                    CUR_FRAME_OFS += rect.extent.height as c_int;
                }
            }
        }
        render_screen(LOCAL_CONTEXT);
    }

    unsafe fn load_credits() -> BOOLEAN {
        CREDITS_TAB = CaptureStringTable(LoadStringTableInstance(CREDITS_STRTAB.as_ptr().cast()));
        if CREDITS_TAB.is_null() {
            return 0;
        }
        CREDITS_BACK =
            CaptureDrawable(LoadGraphicInstance(CREDITS_BACK_ANIM.as_ptr().cast()).cast());
        for (i, resource) in [
            PT13AA_FONT.as_ptr(),
            PT17AA_FONT.as_ptr(),
            PT45AA_FONT.as_ptr(),
        ]
        .into_iter()
        .enumerate()
        {
            CREDITS_FONTS[i] = LoadGraphicInstance(resource.cast()).cast();
        }
        1
    }

    unsafe fn free_credits() {
        DestroyStringTable(ReleaseStringTable(CREDITS_TAB));
        CREDITS_TAB = ptr::null_mut();
        DestroyDrawable(ReleaseDrawable(CREDITS_BACK));
        CREDITS_BACK = ptr::null_mut();
        let fonts =
            std::slice::from_raw_parts_mut(ptr::addr_of_mut!(CREDITS_FONTS).cast::<FONT>(), 3);
        for font in fonts {
            DestroyFont(*font);
            *font = ptr::null_mut();
        }
    }

    unsafe fn outtakes(cx: &mut Cx<'_>, io: &mut Io<'_>) {
        const CONVERSATIONS: [CONVERSATION; 15] = [
            CONVERSATION_ZOQFOTPIK_CONVERSATION,
            CONVERSATION_TALKING_PET_CONVERSATION,
            CONVERSATION_ORZ_CONVERSATION,
            CONVERSATION_UTWIG_CONVERSATION,
            CONVERSATION_THRADD_CONVERSATION,
            CONVERSATION_SUPOX_CONVERSATION,
            CONVERSATION_SYREEN_CONVERSATION,
            CONVERSATION_SHOFIXTI_CONVERSATION,
            CONVERSATION_PKUNK_CONVERSATION,
            CONVERSATION_YEHAT_CONVERSATION,
            CONVERSATION_DRUUGE_CONVERSATION,
            CONVERSATION_URQUAN_CONVERSATION,
            CONVERSATION_VUX_CONVERSATION,
            CONVERSATION_BLACKURQ_CONVERSATION,
            CONVERSATION_ARILOU_CONVERSATION,
        ];
        let configuration = cx.kernel.configuration.clone();
        let old_subtitles = cx.with_legacy(|| {
            let old = configuration.preferences().subtitles;
            configuration.update_preferences(|options| options.subtitles = 1);
            sliderDisabled = 1;
            oscillDisabled = 1;
            old
        });
        let mut i = 0;
        while i < CONVERSATIONS.len() && cx.world.activity() & CHECK_ABORT as u16 == 0 {
            cx.with_legacy(|| SetCommIntroMode((CIM_CROSSFADE_WINDOW) as u32, 0));
            crate::comm::dispatch::init_communication(cx, io, CONVERSATIONS[i]);
            i += 1;
        }
        cx.with_legacy(|| {
            configuration.update_preferences(|options| options.subtitles = old_subtitles);
            sliderDisabled = 0;
            oscillDisabled = 0;
        });
    }

    unsafe fn menu(input: &crate::engine::input::InputSession, key: u32) -> bool {
        input.pulsed().menu[key as usize] != 0
    }

    unsafe fn credits_input(
        input: &crate::engine::input::InputSession,
        state: &mut CreditsInputState,
        activity: u16,
    ) -> BOOLEAN {
        if CREDITS_RUNNING != 0 {
            state.close_time_out = 0;
        }
        if activity & CHECK_ABORT as u16 != 0
            || (state.allow_cancel != 0
                && (menu(input, KEY_MENU_SELECT) || menu(input, KEY_MENU_CANCEL)))
        {
            return 0;
        }
        if state.allow_speed_change != 0 && (menu(input, KEY_MENU_UP) || menu(input, KEY_MENU_DOWN))
        {
            let mut new_rate = CREDITS_RATE;
            let step = CREDITS_RATE.abs() / 5 + 1;
            if menu(input, KEY_MENU_DOWN) {
                new_rate += step;
            } else if menu(input, KEY_MENU_UP) {
                new_rate -= step;
            }
            new_rate = new_rate.clamp(-CREDITS_MAX_RATE, CREDITS_MAX_RATE);
            CREDITS_RATE = new_rate;
        }
        if CREDITS_RUNNING == 0 {
            state.allow_cancel = 1;
        }
        if CREDITS_RUNNING == 0 && state.close_when_done != 0 {
            if state.close_time_out == 0 {
                state.close_time_out = GetTimeCounter().wrapping_add(CREDITS_TIMEOUT);
            } else if GetTimeCounter() > state.close_time_out {
                return 0;
            }
        }
        if CREDITS_RUNNING == 0 && (menu(input, KEY_MENU_SELECT) || menu(input, KEY_MENU_CANCEL)) {
            return 0;
        }
        SleepThread(ONE_SECOND / CREDITS_FRAME_RATE as u32);
        1
    }

    fn credits_frame_hook() -> crate::engine::input::InputFrameHook {
        // SAFETY: The credits owner removes this hook before releasing its published drawing state.
        crate::engine::input::InputFrameHook::new(|checkpoint, _| unsafe {
            process_frame(checkpoint);
        })
    }

    unsafe fn prepare_credits(
        input: &crate::engine::input::InputSession,
        context: CONTEXT,
        with_outtakes: bool,
        io: &mut Io<'_>,
    ) -> Option<MUSIC_REF> {
        let music = LoadMusicInstance(CREDITS_MUSIC.as_ptr().cast()).cast();
        SetContext(context);
        SetContextClipRect(ptr::null_mut());
        let mut screen_rect: RECT = std::mem::zeroed();
        GetContextClipRect(&mut screen_rect);
        SetContextBackGroundColor(BLACK_COLOR);
        ClearDrawable();
        if load_credits() == 0 {
            return None;
        }
        let mut stamp = STAMP {
            origin: POINT { x: 0, y: 0 },
            frame: CREDITS_BACK,
        };
        DrawStamp(&mut stamp);
        FadeScreen(ScreenFadeType_FadeAllToColor, (ONE_SECOND / 2) as SIZE);
        if with_outtakes {
            io.observer.checkpoint(&CheckpointView::CreditsStarted);
        }
        CommWndRect.corner.x =
            ((screen_rect.extent.width as c_int - CommWndRect.width as c_int) / 2) as COORD;
        CommWndRect.corner.y = 5;
        init_credits();
        VICTORY_CREDITS_ACTIVE = with_outtakes;
        SetInputCallback(input, Some(credits_frame_hook()));
        if with_outtakes {
            OUT_TAKES_RUNNING = 1;
        }
        Some(music)
    }

    unsafe fn finish_credits(
        input: &crate::engine::input::InputSession,
        context: CONTEXT,
        music: MUSIC_REF,
    ) {
        SetInputCallback(input, None);
        FadeMusic(0, (ONE_SECOND / 2) as SIZE);
        uninit_credits();
        SetContext(context);
        SleepThreadUntil(FadeScreen(
            ScreenFadeType_FadeAllToBlack,
            (ONE_SECOND / 2) as SIZE,
        ));
        FlushColorXForms();
        if !music.is_null() {
            stop_music();
            DestroyMusic(music);
        }
        FadeMusic(NORMAL_VOLUME as BYTE, 0);
        free_credits();
        VICTORY_CREDITS_ACTIVE = false;
    }

    /// # Safety
    /// Graphics, audio, communication and input must be initialized on the game thread.
    pub unsafe fn credits(cx: &mut Cx<'_>, io: &mut Io<'_>, with_outtakes: bool) {
        let runtime = cx.kernel.input_runtime();
        let input = runtime.input();
        let context = cx.kernel.screen_context;
        let Some(music) = cx.with_legacy(|| prepare_credits(input, context, with_outtakes, io))
        else {
            return;
        };
        if with_outtakes {
            outtakes(cx, io);
            cx.with_legacy(|| OUT_TAKES_RUNNING = 0);
        }
        if cx.world.activity() & CHECK_ABORT as u16 == 0 {
            if with_outtakes {
                cx.with_legacy(|| io.observer.checkpoint(&CheckpointView::CreditsReady));
            }
            let music_enabled = cx.world.data.game.glob_flags & 8 == 0;
            cx.with_legacy(|| {
                if !music.is_null() {
                    crate::sound::music::remember_native_music(
                        std::ptr::NonNull::new(music.cast()),
                        true,
                        1,
                        music_enabled,
                    );
                }
            });
            let mut state = CreditsInputState {
                allow_cancel: (!with_outtakes) as BOOLEAN,
                allow_speed_change: 1,
                close_when_done: (!with_outtakes) as BOOLEAN,
                close_time_out: 0,
            };
            cx.with_legacy(|| SetMenuSounds(input, 0, 0));
            cx.run_input(io, true, |cx, _| {
                let activity = cx.world.activity();
                cx.with_legacy(|| credits_input(input, &mut state, activity) != 0)
            });
        }
        let context = cx.kernel.screen_context;
        cx.with_legacy(|| finish_credits(input, context, music));
    }
}

#[cfg(not(test))]
pub use runtime::credits;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ring_indices_wrap_in_both_directions() {
        assert_eq!(frame_index(-1), 31);
        assert_eq!(frame_index(0), 0);
        assert_eq!(frame_index(31), 31);
        assert_eq!(frame_index(32), 0);
        for index in 0..32 {
            assert_eq!(
                frame_index(frame_index(index + 1) as c_int - 1),
                index as usize
            );
        }
    }

    #[test]
    fn scrolling_uses_integer_frame_delay_and_directional_steps() {
        assert_eq!(scroll_step(0), (0, 0, 840));
        assert_eq!(scroll_step(9), (1, 1, 93));
        assert_eq!(scroll_step(-9), (-1, -1, 93));
        assert_eq!(scroll_step(36), (1, 1, 23));
        assert_eq!(scroll_step(37), (1, 2, 45));
        assert_eq!(scroll_step(130), (1, 4, 25));
        assert_eq!(scroll_step(-130), (-1, -4, 25));
        for rate in 1..=CREDITS_MAX_RATE {
            let (direction, step, delay) = scroll_step(rate);
            assert_eq!(direction, 1);
            assert_eq!(step, (rate + 35) / 36);
            assert_eq!(delay, (840 * step / rate) as u32);
            assert_eq!(scroll_step(-rate), (-1, -step, delay));
        }
    }
}
