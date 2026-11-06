//! Presentation scripts and legacy-video input, ported from uqm/intro.c.

mod text;

#[cfg(not(test))]
use crate::engine::{Kernel, WorldData};
use crate::navigation::abi::*;
use std::{ffi::c_char, ptr};

#[cfg(not(test))]
pub(super) const WANT_PIXMAP: CREATE_FLAGS = 1 << 1;
pub(super) const BLACK_COLOR: Color = rgb15(0, 0, 0);

pub(super) const fn rgb15(r: u8, g: u8, b: u8) -> Color {
    Color {
        r: (r << 3) | (r >> 2),
        g: (g << 3) | (g >> 2),
        b: (b << 3) | (b >> 2),
        a: 0xff,
    }
}

unsafe fn parse_color(src: *const c_char, color: &mut Color) -> BOOLEAN {
    let mut value = 0u32;
    if libc::sscanf(src, c"%x".as_ptr(), ptr::addr_of_mut!(value)) != 1 {
        return 0;
    }
    *color = Color {
        r: ((value >> 16) & 0xff) as u8,
        g: ((value >> 8) & 0xff) as u8,
        b: (value & 0xff) as u8,
        a: 0,
    };
    1
}

pub(super) unsafe fn parse_text_lines(lines: *mut TEXT, max: usize, buffer: *mut c_char) -> usize {
    let buffer = std::slice::from_raw_parts_mut(buffer.cast::<u8>(), libc::strlen(buffer) + 1);
    let lines = std::slice::from_raw_parts_mut(lines, max);
    text::split_lines(lines, buffer)
}

#[cfg(not(test))]
pub(crate) mod runtime {
    use super::*;

    use crate::video::{
        control::{VidGetPosition, VidProcessFrame, VidSeek},
        legacy::{
            DestroyLegacyVideo, LegacyVideo, LegacyVideoRef, LoadLegacyVideoInstance,
            PlayLegacyVideo, PlayingLegacyVideo, StopLegacyVideo,
        },
    };

    use crate::engine::{
        battle::Cx,
        observe::{CheckpointView, Io},
    };
    use std::ffi::{c_int, CStr};

    const LOG_WARNING: c_int = 3;
    const MAX_FONTS: usize = 5;
    const MAX_TEXT_LINES: usize = 15;

    unsafe fn menu(key: u32) -> bool {
        PulsedInputState.menu[key as usize] != 0
    }

    unsafe fn unbatch(state: &mut PresentationState, yield_task: bool) {
        if state.batched != 0 {
            UnbatchGraphics();
            state.batched = 0;
            if yield_task {
                TaskSwitch();
            }
        }
    }

    unsafe fn draw_text(text: *mut TEXT, fore: Color, back: Color, effect: c_int) {
        if effect == b'T' as c_int {
            font_DrawTracedText(text, fore, back);
        } else {
            SetContextForeGroundColor(fore);
            font_DrawText(text);
        }
    }

    unsafe fn generate_sis(state: &mut PresentationState) {
        let world = WorldData::current();
        let old_context = SetContext(Kernel::current().read(|owner| owner.off_screen_context));
        let skeleton =
            CaptureDrawable(LoadGraphicInstance(SISSKEL_MASK_PMAP_ANIM.as_ptr().cast()).cast());
        let modules =
            CaptureDrawable(LoadGraphicInstance(SISMODS_MASK_PMAP_ANIM.as_ptr().cast()).cast());
        let mut bounds: RECT = std::mem::zeroed();
        GetFrameRect(skeleton, &mut bounds);
        let sis = CaptureDrawable(CreateDrawable(
            WANT_PIXMAP,
            bounds.extent.width,
            bounds.extent.height,
            1,
        ));
        SetContextFGFrame(sis);
        SetContextClipRect(ptr::null_mut());
        let background = rgb15(1, 1, 1);
        SetContextBackGroundColor(background);
        ClearDrawable();
        SetFrameTransparentColor(sis, background);
        let mut stamp = STAMP {
            origin: POINT { x: 0, y: 0 },
            frame: SetAbsFrameIndex(skeleton, 0),
        };
        DrawStamp(&mut stamp);
        let sis_state = ptr::addr_of!((**world).flagship);
        for (pieces, x, y, slots) in [
            (
                ptr::addr_of!((*sis_state).drive_slots).cast::<BYTE>(),
                DRIVE_TOP_X,
                DRIVE_TOP_Y,
                NUM_DRIVE_SLOTS,
            ),
            (
                ptr::addr_of!((*sis_state).jet_slots).cast::<BYTE>(),
                JET_TOP_X,
                JET_TOP_Y,
                NUM_JET_SLOTS,
            ),
            (
                ptr::addr_of!((*sis_state).module_slots).cast::<BYTE>(),
                MODULE_TOP_X,
                MODULE_TOP_Y,
                NUM_MODULE_SLOTS,
            ),
        ] {
            for slot in 0..slots {
                let piece = *pieces.add(slot as usize) as COUNT;
                if piece < NUM_MODULES as COUNT {
                    stamp.origin.x = (x + slot * SHIP_PIECE_OFFSET) as COORD;
                    stamp.origin.y = (y as c_int - 79) as COORD;
                    stamp.frame = SetAbsFrameIndex(modules, piece);
                    DrawStamp(&mut stamp);
                }
            }
        }
        DestroyDrawable(ReleaseDrawable(skeleton));
        DestroyDrawable(ReleaseDrawable(modules));
        SetFrameHot(
            sis,
            POINT {
                x: bounds.extent.width / 2,
                y: bounds.extent.height / 2,
            },
        );
        SetContext(old_context);
        FlushGraphics();
        state.sis_frame = sis;
    }

    unsafe fn draw_movie_frame(state: &PresentationState) {
        let mut stamp = STAMP {
            origin: POINT { x: 0, y: 0 },
            frame: SetAbsFrameIndex(state.frame, state.movie_frame as COUNT),
        };
        DrawStamp(&mut stamp);
    }

    unsafe fn fade_screen(
        state: &mut PresentationState,
        src: *const c_char,
        kind: ScreenFadeType,
    ) -> BOOLEAN {
        let mut msecs = 0;
        if libc::sscanf(src, c"%d".as_ptr(), ptr::addr_of_mut!(msecs)) == 1 {
            state.time_out = FadeScreen(kind, (msecs * ONE_SECOND as c_int / 1000) as SIZE)
                .wrapping_add(ONE_SECOND / 10);
            state.time_out_on_skip = 0;
        }
        1
    }

    unsafe fn show_file(cx: &mut Cx<'_>, io: &mut Io<'_>, name: *const c_char) -> BOOLEAN {
        let table =
            cx.with_legacy(|| CaptureStringTable(LoadStringTableFile((contentDir).cast(), name)));
        let result = show_slides(cx, io, table, false);
        cx.with_legacy(|| DestroyStringTable(ReleaseStringTable(table)));
        result
    }

    mod records;
    use records::PresentationState;

    enum PresentationStep {
        Input(bool),
        Call,
    }

    unsafe fn admit_presentation(
        state: &mut PresentationState,
        io: &mut Io<'_>,
        activity: u16,
    ) -> Option<bool> {
        if menu(KEY_MENU_CANCEL) || activity & CHECK_ABORT as u16 != 0 {
            io.observer
                .checkpoint(&CheckpointView::PresentationCancelled);
            return Some(false);
        }
        if state.time_out != 0 {
            let mut delay = ONE_SECOND / 84;
            if GetTimeCounter() >= state.time_out {
                if state.movie_frame >= 0 {
                    draw_movie_frame(state);
                    state.movie_frame += 1;
                    if state.movie_frame > state.movie_end_frame {
                        state.movie_frame = -1;
                    }
                    delay = state.interframe_delay as TimePeriod;
                } else {
                    state.time_out = 0;
                    return Some(true);
                }
            }
            if state.time_out_on_skip != 0
                && (menu(KEY_MENU_SELECT) || menu(KEY_MENU_SPECIAL) || menu(KEY_MENU_RIGHT))
            {
                state.time_out = 0;
                state.movie_frame = -1;
                return Some(true);
            }
            SleepThread(delay);
            return Some(true);
        }
        None
    }

    unsafe fn presentation_commands(
        state: &mut PresentationState,
        io: &mut Io<'_>,
    ) -> PresentationStep {
        while state.oper_index < GetStringTableCount(state.slide_show) {
            let mut opcode = [0 as c_char; 16];
            let mut src = GetStringAddress(state.slide_show);
            state.oper_index = state.oper_index.wrapping_add(1);
            state.slide_show = SetRelStringTableIndex(state.slide_show, 1);
            if src.is_null() || libc::sscanf(src, c"%15s".as_ptr(), opcode.as_mut_ptr()) != 1 {
                continue;
            }
            src = src.add(libc::strlen(opcode.as_ptr()));
            if *src != 0 {
                src = src.add(1);
            }
            for ch in &mut opcode {
                *ch = libc::toupper(*ch as u8 as c_int) as c_char;
            }
            match CStr::from_ptr(opcode.as_ptr()).to_bytes() {
                b"DIMS" => {
                    let (mut w, mut h) = (0, 0);
                    if libc::sscanf(
                        src,
                        c"%d %d".as_ptr(),
                        ptr::addr_of_mut!(w),
                        ptr::addr_of_mut!(h),
                    ) == 2
                    {
                        state.clip_r.extent.width = w as COORD;
                        state.clip_r.extent.height = h as COORD;
                        state.clip_r.corner.x = ((ScreenWidth - w) / 2) as COORD;
                        state.clip_r.corner.y = ((ScreenHeight - h) / 2) as COORD;
                        SetContextClipRect(&mut state.clip_r);
                    }
                }
                b"FONT" => {
                    let mut index = 0;
                    state.buffer[0] = 0;
                    if libc::sscanf(
                        src,
                        c"%d %255[^\n]".as_ptr(),
                        ptr::addr_of_mut!(index),
                        state.buffer.as_mut_ptr(),
                    ) < 1
                        || index < 0
                        || index >= MAX_FONTS as c_int
                    {
                        crate::log_abi::log_message(
                            LOG_WARNING,
                            &format!(
                                "Bad FONT command '{}'",
                                std::ffi::CStr::from_ptr(src).to_string_lossy()
                            ),
                        );
                        continue;
                    }
                    let font = &mut state.fonts[index as usize];
                    if state.buffer[0] != 0 {
                        if !font.is_null() {
                            DestroyFont(*font);
                        }
                        *font = LoadFontFile(state.buffer.as_ptr());
                    }
                    SetContextFont(*font);
                }
                b"ANI" => {
                    utf8StringCopy(
                        (state.buffer.as_mut_ptr()).cast(),
                        state.buffer.len(),
                        (src).cast(),
                    );
                    if !state.frame.is_null() {
                        DestroyDrawable(ReleaseDrawable(state.frame));
                    }
                    state.frame = CaptureDrawable(LoadGraphicFile(state.buffer.as_ptr()));
                }
                b"MUSIC" => {
                    utf8StringCopy(
                        (state.buffer.as_mut_ptr()).cast(),
                        state.buffer.len(),
                        (src).cast(),
                    );
                    if !state.music_ref.is_null() {
                        stop_music();
                        DestroyMusic(state.music_ref);
                    }
                    state.music_ref = (LoadMusicFile(state.buffer.as_ptr())).cast();
                    PlayMusic(state.music_ref, 0, 1);
                }
                b"WAIT" | b"SYNC" | b"DSYNC" => {
                    let mut msecs = 0;
                    unbatch(state, true);
                    if libc::sscanf(src, c"%d".as_ptr(), ptr::addr_of_mut!(msecs)) == 1 {
                        let ticks = (msecs * ONE_SECOND as c_int / 1000) as TimeCount;
                        match CStr::from_ptr(opcode.as_ptr()).to_bytes() {
                            b"WAIT" => {
                                state.time_out = GetTimeCounter().wrapping_add(ticks);
                                state.time_out_on_skip = 1;
                            }
                            b"SYNC" => {
                                state.last_sync_time = state.start_time.wrapping_add(ticks);
                                state.time_out = state.last_sync_time;
                                state.time_out_on_skip = 0;
                            }
                            _ => {
                                state.time_out = state.last_sync_time.wrapping_add(ticks);
                                state.time_out_on_skip = 0;
                            }
                        }
                        return PresentationStep::Input(true);
                    }
                }
                b"RESYNC" => {
                    state.start_time = GetTimeCounter();
                    state.last_sync_time = state.start_time;
                }
                b"TC" => {
                    parse_color(src, &mut state.text_color);
                }
                b"TBC" => {
                    parse_color(src, &mut state.text_back_color);
                }
                b"TFC" => {
                    parse_color(src, &mut state.text_fade_color);
                }
                b"TVA" => {
                    state.text_vpos = libc::toupper(*src as u8 as c_int);
                }
                b"TE" => {
                    state.text_effect = libc::toupper(*src as u8 as c_int);
                }
                b"TEXT" => {
                    let (mut x, mut y) = (0, 0);
                    if libc::sscanf(
                        src,
                        c"%d %d %255[^\n]".as_ptr(),
                        ptr::addr_of_mut!(x),
                        ptr::addr_of_mut!(y),
                        state.buffer.as_mut_ptr(),
                    ) == 3
                    {
                        let mut text = TEXT {
                            align: TEXT_ALIGN_ALIGN_CENTER,
                            pStr: state.buffer.as_ptr(),
                            CharCount: COUNT::MAX,
                            baseline: POINT {
                                x: x as COORD,
                                y: y as COORD,
                            },
                        };
                        draw_text(
                            &mut text,
                            state.text_color,
                            state.text_back_color,
                            state.text_effect,
                        );
                    }
                }
                b"TFI" => {
                    utf8StringCopy(
                        (state.buffer.as_mut_ptr()).cast(),
                        state.buffer.len(),
                        (src).cast(),
                    );
                    state.lines_count = parse_text_lines(
                        state.text_lines.as_mut_ptr(),
                        MAX_TEXT_LINES,
                        state.buffer.as_mut_ptr(),
                    ) as COUNT;
                    unbatch(state, true);
                    let mut leading: SIZE = 0;
                    GetContextFontLeading(&mut leading);
                    state.tfade_r = text::position_lines(
                        &mut state.text_lines[..state.lines_count as usize],
                        state.clip_r,
                        leading,
                        state.text_vpos,
                        ScreenWidth,
                    );
                    for text in &mut state.text_lines[..state.lines_count as usize] {
                        draw_text(
                            text,
                            state.text_fade_color,
                            state.text_fade_color,
                            state.text_effect,
                        );
                    }
                    SetTransitionSource(&state.tfade_r);
                    BatchGraphics();
                    for text in &mut state.text_lines[..state.lines_count as usize] {
                        draw_text(
                            text,
                            state.text_color,
                            state.text_back_color,
                            state.text_effect,
                        );
                    }
                    ScreenTransition(3, &state.tfade_r);
                    UnbatchGraphics();
                }
                b"TFO" => {
                    unbatch(state, true);
                    SetTransitionSource(&state.tfade_r);
                    BatchGraphics();
                    for text in &mut state.text_lines[..state.lines_count as usize] {
                        draw_text(
                            text,
                            state.text_fade_color,
                            state.text_fade_color,
                            state.text_effect,
                        );
                    }
                    ScreenTransition(3, &state.tfade_r);
                    UnbatchGraphics();
                }
                b"SAVEBG" => TFB_DrawScreen_Copy(
                    &state.clip_r,
                    SCREEN_TFB_SCREEN_MAIN,
                    SCREEN_TFB_SCREEN_EXTRA,
                ),
                b"RESTBG" => TFB_DrawScreen_Copy(
                    &state.clip_r,
                    SCREEN_TFB_SCREEN_EXTRA,
                    SCREEN_TFB_SCREEN_MAIN,
                ),
                b"DRAW" => {
                    let mut image_name = [0 as c_char; 16];
                    let (mut index, mut x, mut y, mut scale, mut angle) = (0, 0, 0, 0, 0);
                    let sis = if libc::sscanf(src, c"%15s".as_ptr(), image_name.as_mut_ptr()) == 1 {
                        for ch in &mut image_name {
                            *ch = libc::toupper(*ch as u8 as c_int) as c_char;
                        }
                        CStr::from_ptr(image_name.as_ptr()).to_bytes() == b"SIS"
                    } else {
                        false
                    };
                    let (draw_kind, scale_mode, cargs) = if sis {
                        (
                            1,
                            SCALE_TFB_SCALE_NEAREST,
                            libc::sscanf(
                                src,
                                c"%*s %d %d %d %d".as_ptr(),
                                ptr::addr_of_mut!(x),
                                ptr::addr_of_mut!(y),
                                ptr::addr_of_mut!(scale),
                                ptr::addr_of_mut!(angle),
                            ) + 1,
                        )
                    } else {
                        (
                            0,
                            SCALE_TFB_SCALE_BILINEAR,
                            libc::sscanf(
                                src,
                                c"%d %d %d %d %d".as_ptr(),
                                ptr::addr_of_mut!(index),
                                ptr::addr_of_mut!(x),
                                ptr::addr_of_mut!(y),
                                ptr::addr_of_mut!(scale),
                                ptr::addr_of_mut!(angle),
                            ),
                        )
                    };
                    if cargs < 1 {
                        crate::log_abi::log_message(
                            LOG_WARNING,
                            &format!(
                                "Bad DRAW command '{}'",
                                std::ffi::CStr::from_ptr(src).to_string_lossy()
                            ),
                        );
                        continue;
                    }
                    if cargs < 5 {
                        angle = 0;
                    }
                    if cargs < 4 {
                        scale = GSCALE_IDENTITY as c_int;
                    }
                    if cargs < 3 {
                        x = 0;
                        y = 0;
                    }
                    let mut stamp = STAMP {
                        origin: POINT {
                            x: x as COORD,
                            y: y as COORD,
                        },
                        frame: ptr::null_mut(),
                    };
                    if draw_kind == 0 {
                        stamp.frame = SetAbsFrameIndex(state.frame, index as COUNT);
                    } else {
                        if state.sis_frame.is_null() {
                            generate_sis(state);
                        }
                        stamp.frame = SetAbsFrameIndex(state.sis_frame, 0);
                    }
                    if angle != 0 {
                        if angle != state.last_angle || draw_kind != state.last_draw_kind {
                            DestroyDrawable(ReleaseDrawable(state.rotated_frame));
                            state.rotated_frame = CaptureDrawable(RotateFrame(stamp.frame, -angle));
                            state.last_angle = angle;
                            state.last_draw_kind = draw_kind;
                        }
                        stamp.frame = state.rotated_frame;
                    }
                    let old_mode = SetGraphicScaleMode(scale_mode as c_int);
                    let old_scale = SetGraphicScale(scale);
                    DrawStamp(&mut stamp);
                    SetGraphicScale(old_scale);
                    SetGraphicScaleMode(old_mode);
                }
                b"BATCH" => {
                    if state.batched == 0 {
                        state.batched = 1;
                        BatchGraphics();
                    }
                }
                b"UNBATCH" => unbatch(state, false),
                b"FTC" | b"FTB" | b"FTW" => {
                    unbatch(state, true);
                    let kind = match CStr::from_ptr(opcode.as_ptr()).to_bytes() {
                        b"FTC" => ScreenFadeType_FadeAllToColor,
                        b"FTB" => ScreenFadeType_FadeAllToBlack,
                        _ => ScreenFadeType_FadeAllToWhite,
                    };
                    return PresentationStep::Input(fade_screen(state, src, kind) != 0);
                }
                b"CLS" => {
                    unbatch(state, true);
                    ClearDrawable();
                }
                b"CALL" => {
                    unbatch(state, true);
                    utf8StringCopy(
                        (state.buffer.as_mut_ptr()).cast(),
                        state.buffer.len(),
                        (src).cast(),
                    );
                    return PresentationStep::Call;
                }
                b"LINE" => {
                    let (mut x1, mut y1, mut x2, mut y2) = (0, 0, 0, 0);
                    if libc::sscanf(
                        src,
                        c"%d %d %d %d".as_ptr(),
                        ptr::addr_of_mut!(x1),
                        ptr::addr_of_mut!(y1),
                        ptr::addr_of_mut!(x2),
                        ptr::addr_of_mut!(y2),
                    ) == 4
                    {
                        let mut line = LINE {
                            first: POINT {
                                x: x1 as COORD,
                                y: y1 as COORD,
                            },
                            second: POINT {
                                x: x2 as COORD,
                                y: y2 as COORD,
                            },
                        };
                        SetContextForeGroundColor(state.text_color);
                        DrawLine(&mut line);
                    } else {
                        crate::log_abi::log_message(
                            LOG_WARNING,
                            &format!(
                                "Bad LINE command '{}'",
                                std::ffi::CStr::from_ptr(src).to_string_lossy()
                            ),
                        );
                    }
                }
                b"MOVIE" => {
                    let (mut fps, mut from, mut to) = (0, 0, 0);
                    if libc::sscanf(
                        src,
                        c"%d %d %d".as_ptr(),
                        ptr::addr_of_mut!(fps),
                        ptr::addr_of_mut!(from),
                        ptr::addr_of_mut!(to),
                    ) == 3
                        && fps > 0
                        && from >= 0
                        && to >= 0
                        && to >= from
                    {
                        unbatch(state, true);
                        state.movie_frame = from;
                        state.movie_end_frame = to;
                        state.interframe_delay = ONE_SECOND as c_int / fps;
                        state.time_out = GetTimeCounter();
                        state.time_out_on_skip = 1;
                        return PresentationStep::Input(true);
                    } else {
                        crate::log_abi::log_message(
                            LOG_WARNING,
                            &format!(
                                "Bad MOVIE command '{}'",
                                std::ffi::CStr::from_ptr(src).to_string_lossy()
                            ),
                        );
                    }
                }
                b"NOOP" => (),
                _ => (),
            }
        }
        if state.ending {
            io.observer
                .checkpoint(&CheckpointView::PresentationCompleted);
        }
        PresentationStep::Input(false)
    }

    unsafe fn presentation_frame(
        cx: &mut Cx<'_>,
        io: &mut Io<'_>,
        state: &mut PresentationState,
    ) -> bool {
        let activity = cx.world.activity();
        if let Some(keep) = cx.with_legacy(|| admit_presentation(state, io, activity)) {
            return keep;
        }
        loop {
            match cx.with_legacy(|| presentation_commands(state, io)) {
                PresentationStep::Input(keep) => return keep,
                PresentationStep::Call => {
                    show_file(cx, io, state.buffer.as_ptr());
                }
            }
        }
    }

    unsafe fn show_slides(
        cx: &mut Cx<'_>,
        io: &mut Io<'_>,
        table: STRING,
        ending: bool,
    ) -> BOOLEAN {
        let mut state: PresentationState = std::mem::zeroed();
        state.slide_show = table;
        if state.slide_show.is_null() {
            io.observer
                .checkpoint(&CheckpointView::PresentationCancelled);
            return 0;
        }
        state.ending = ending;
        if ending {
            io.observer.checkpoint(&CheckpointView::PresentationStarted);
        }
        state.slide_show = SetAbsStringTableIndex(state.slide_show, 0);
        state.oper_index = 0;
        let context = cx.kernel.screen_context;
        let (old_context, old_font, mut old_rect) = cx.with_legacy(|| {
            let old_context = SetContext(context);
            let mut old_rect: RECT = std::mem::zeroed();
            GetContextClipRect(&mut old_rect);
            let old_font = SetContextFont(ptr::null_mut());
            SetContextBackGroundColor(BLACK_COLOR);
            SetMenuSounds(0, 0);
            state.last_draw_kind = -1;
            state.text_vpos = b'B' as c_int;
            state.movie_frame = -1;
            state.start_time = GetTimeCounter();
            state.last_sync_time = state.start_time;
            (old_context, old_font, old_rect)
        });
        cx.run_input(io, true, |cx, io| presentation_frame(cx, io, &mut state));
        cx.with_legacy(|| {
            SleepThreadUntil(FadeMusic(0, ONE_SECOND as SIZE));
            stop_music();
            FadeMusic(NORMAL_VOLUME as BYTE, 0);
            DestroyMusic(state.music_ref);
            DestroyDrawable(ReleaseDrawable(state.rotated_frame));
            DestroyDrawable(ReleaseDrawable(state.frame));
            for font in state.fonts {
                DestroyFont(font);
            }
            SetContextFont(old_font);
            SetContextClipRect(&mut old_rect);
            SetContext(old_context);
        });
        1
    }

    unsafe fn video_input(video: *mut LegacyVideoRef, activity: u16) -> BOOLEAN {
        if PlayingLegacyVideo(video) == 0 {
            return 0;
        }
        if menu(KEY_MENU_SELECT)
            || menu(KEY_MENU_CANCEL)
            || menu(KEY_MENU_SPECIAL)
            || activity & CHECK_ABORT as u16 != 0
        {
            return 0;
        }
        if menu(KEY_MENU_LEFT) || menu(KEY_MENU_RIGHT) {
            let mut position = VidGetPosition() as SDWORD;
            if menu(KEY_MENU_LEFT) {
                position = position.wrapping_sub(2000);
            } else if menu(KEY_MENU_RIGHT) {
                position = position.wrapping_add(1000);
            }
            if position < 0 {
                position = 0;
            }
            VidSeek(position as DWORD);
        } else {
            if VidProcessFrame() == 0 {
                return 0;
            }
            SleepThread(ONE_SECOND / 40);
        }
        1
    }

    unsafe fn fade_clear_screen() {
        SleepThreadUntil(FadeScreen(
            ScreenFadeType_FadeAllToBlack,
            (ONE_SECOND / 2) as SIZE,
        ));
        SetContext(Kernel::current().read(|owner| owner.screen_context));
        SetContextBackGroundColor(BLACK_COLOR);
        ClearDrawable();
        FadeScreen(ScreenFadeType_FadeAllToColor, 0);
    }

    unsafe fn show_video(cx: &mut Cx<'_>, io: &mut Io<'_>, video: *mut LegacyVideo) -> BOOLEAN {
        cx.with_legacy(|| fade_clear_screen());
        let reference = cx.with_legacy(|| PlayLegacyVideo(video));
        if reference.is_null() {
            return 0;
        }
        cx.with_legacy(|| SetMenuSounds(0, 0));
        cx.run_input(io, true, |cx, _| {
            let activity = cx.world.activity();
            cx.with_legacy(|| video_input(reference, activity) != 0)
        });
        cx.with_legacy(|| {
            StopLegacyVideo(reference);
            fade_clear_screen();
        });
        1
    }

    /// # Safety
    /// `presentation` must identify a registered resource. Graphics, audio and input
    /// must be initialized, and this call must run on the game thread.
    pub unsafe fn show_presentation(
        cx: &mut Cx<'_>,
        io: &mut Io<'_>,
        presentation: RESOURCE,
    ) -> BOOLEAN {
        let kind = cx.with_legacy(|| res_GetResourceType(presentation));
        if kind.is_null() {
            return 0;
        }
        match CStr::from_ptr(kind).to_bytes() {
            b"STRTAB" => {
                let table =
                    cx.with_legacy(|| CaptureStringTable(LoadStringTableInstance(presentation)));
                let ending = CStr::from_ptr(presentation).to_bytes()
                    == &FINALPRES_STRTAB[..FINALPRES_STRTAB.len() - 1];
                let result = show_slides(cx, io, table, ending);
                cx.with_legacy(|| DestroyStringTable(ReleaseStringTable(table)));
                result
            }
            b"3DOVID" => {
                let video = cx.with_legacy(|| LoadLegacyVideoInstance(presentation));
                let result = show_video(cx, io, video);
                cx.with_legacy(|| DestroyLegacyVideo(video));
                result
            }
            _ => {
                crate::log_abi::log_message(
                    LOG_WARNING,
                    &format!(
                        "Tried to present '{}', of non-presentable type '{}'",
                        std::ffi::CStr::from_ptr(presentation).to_string_lossy(),
                        std::ffi::CStr::from_ptr(kind).to_string_lossy()
                    ),
                );
                0
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CStr;

    #[test]
    fn script_colors_use_rgb_bytes_and_zero_alpha() {
        unsafe {
            let mut color = BLACK_COLOR;
            assert_eq!(parse_color(c" 0x1234ab trailing".as_ptr(), &mut color), 1);
            assert_eq!((color.r, color.g, color.b, color.a), (0x12, 0x34, 0xab, 0));
            assert_eq!(parse_color(c"invalid".as_ptr(), &mut color), 0);
            assert_eq!((color.r, color.g, color.b, color.a), (0x12, 0x34, 0xab, 0));
            assert_eq!(parse_color(c"ff00ff!".as_ptr(), &mut color), 1);
            assert_eq!((color.r, color.g, color.b, color.a), (255, 0, 255, 0));
        }
    }

    #[test]
    fn rgb15_expands_all_five_bits_and_is_opaque() {
        let color = rgb15(1, 10, 31);
        assert_eq!((color.r, color.g, color.b, color.a), (8, 82, 255, 255));
    }

    #[test]
    fn line_parser_preserves_empty_rows_and_ignores_trailing_newline() {
        unsafe {
            let mut buffer = *b"first\n\nlast\n\0";
            let mut lines: [TEXT; 5] = std::mem::zeroed();
            let count =
                parse_text_lines(lines.as_mut_ptr(), lines.len(), buffer.as_mut_ptr().cast());
            assert_eq!(count, 3);
            let result: Vec<_> = lines[..count]
                .iter()
                .map(|line| CStr::from_ptr(line.pStr).to_bytes())
                .collect();
            assert_eq!(result, vec![b"first".as_slice(), b"", b"last"]);
            assert!(lines[..count]
                .iter()
                .all(|line| line.CharCount == COUNT::MAX));
        }
    }

    #[test]
    fn line_parser_honors_capacity_and_empty_input() {
        unsafe {
            let mut lines: [TEXT; 2] = std::mem::zeroed();
            let mut buffer = *b"a\nb\nc\0";
            assert_eq!(
                parse_text_lines(lines.as_mut_ptr(), 2, buffer.as_mut_ptr().cast()),
                2
            );
            assert_eq!(CStr::from_ptr(lines[1].pStr), c"b");
            let mut empty = [0 as c_char];
            assert_eq!(
                parse_text_lines(lines.as_mut_ptr(), 2, empty.as_mut_ptr()),
                0
            );
        }
    }
}
