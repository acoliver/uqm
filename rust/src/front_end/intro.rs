//! Presentation scripts and legacy-video input, ported from uqm/intro.c.
#![allow(non_snake_case)]

use crate::navigation::abi::*;
use std::ffi::{c_char, c_int};
use std::ptr;

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

pub(super) unsafe fn parse_text_lines(
    lines: *mut TEXT,
    max: usize,
    mut buffer: *mut c_char,
) -> usize {
    let end = buffer.add(libc::strlen(buffer));
    let mut count = 0;
    while count < max && buffer < end {
        let mut term = libc::strchr(buffer, b'\n' as c_int);
        if term.is_null() {
            term = buffer.add(libc::strlen(buffer));
        }
        *term = 0;
        (*lines.add(count)).pStr = buffer;
        (*lines.add(count)).CharCount = COUNT::MAX;
        buffer = term.add(1);
        count += 1;
    }
    count
}

#[cfg(not(test))]
mod runtime {
    use super::*;
    use crate::navigation::globals::GlobData;
    use crate::video::control::{VidGetPosition, VidProcessFrame, VidSeek};
    use crate::video::legacy::{
        DestroyLegacyVideo, LegacyVideo, LegacyVideoRef, LoadLegacyVideoInstance, PlayLegacyVideo,
        PlayingLegacyVideo, StopLegacyVideo,
    };
    use std::ffi::{c_void, CStr};

    const LOG_WARNING: c_int = 3;
    const MAX_FONTS: usize = 5;
    const MAX_TEXT_LINES: usize = 15;

    #[repr(C)]
    struct PresentationState {
        InputFunc: Option<unsafe extern "C" fn(*mut c_void) -> BOOLEAN>,
        StartTime: TimeCount,
        LastSyncTime: TimeCount,
        TimeOut: TimeCount,
        TimeOutOnSkip: c_int,
        SlideShow: STRING,
        Fonts: [FONT; MAX_FONTS],
        Frame: FRAME,
        MusicRef: MUSIC_REF,
        Batched: BOOLEAN,
        SisFrame: FRAME,
        RotatedFrame: FRAME,
        LastDrawKind: c_int,
        Ending: bool,
        LastAngle: c_int,
        OperIndex: COUNT,
        TextFadeColor: Color,
        TextColor: Color,
        TextBackColor: Color,
        TextVPos: c_int,
        TextEffect: c_int,
        clip_r: RECT,
        tfade_r: RECT,
        TextLines: [TEXT; MAX_TEXT_LINES],
        LinesCount: COUNT,
        Buffer: [c_char; 512],
        MovieFrame: c_int,
        MovieEndFrame: c_int,
        InterframeDelay: c_int,
    }

    #[repr(C)]
    struct VideoState {
        InputFunc: Option<unsafe extern "C" fn(*mut c_void) -> BOOLEAN>,
        CurVideo: *mut LegacyVideoRef,
    }

    unsafe fn menu(key: u32) -> bool {
        PulsedInputState.menu[key as usize] != 0
    }

    unsafe fn unbatch(state: &mut PresentationState, yield_task: bool) {
        if state.Batched != 0 {
            UnbatchGraphics();
            state.Batched = 0;
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
        let old_context = SetContext(OffScreenContext);
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
        let sis_state = ptr::addr_of!(GlobData.SIS_state);
        for (pieces, x, y, slots) in [
            (
                ptr::addr_of!((*sis_state).DriveSlots).cast::<BYTE>(),
                DRIVE_TOP_X,
                DRIVE_TOP_Y,
                NUM_DRIVE_SLOTS,
            ),
            (
                ptr::addr_of!((*sis_state).JetSlots).cast::<BYTE>(),
                JET_TOP_X,
                JET_TOP_Y,
                NUM_JET_SLOTS,
            ),
            (
                ptr::addr_of!((*sis_state).ModuleSlots).cast::<BYTE>(),
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
        state.SisFrame = sis;
    }

    unsafe fn draw_movie_frame(state: &PresentationState) {
        let mut stamp = STAMP {
            origin: POINT { x: 0, y: 0 },
            frame: SetAbsFrameIndex(state.Frame, state.MovieFrame as COUNT),
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
            state.TimeOut = FadeScreen(kind, (msecs * ONE_SECOND as c_int / 1000) as SIZE)
                .wrapping_add(ONE_SECOND / 10);
            state.TimeOutOnSkip = 0;
        }
        1
    }

    unsafe fn show_file(name: *const c_char) -> BOOLEAN {
        let table = CaptureStringTable(LoadStringTableFile(contentDir, name));
        let result = show_slides(table, false);
        DestroyStringTable(ReleaseStringTable(table));
        result
    }

    unsafe extern "C" fn do_presentation(raw: *mut c_void) -> BOOLEAN {
        let state = &mut *raw.cast::<PresentationState>();
        if menu(KEY_MENU_CANCEL) || GlobData.Game_state.CurrentActivity & CHECK_ABORT as u16 != 0 {
            crate::automation::ending::presentation_cancelled();
            return 0;
        }
        if state.TimeOut != 0 {
            let mut delay = ONE_SECOND / 84;
            if GetTimeCounter() >= state.TimeOut {
                if state.MovieFrame >= 0 {
                    draw_movie_frame(state);
                    state.MovieFrame += 1;
                    if state.MovieFrame > state.MovieEndFrame {
                        state.MovieFrame = -1;
                    }
                    delay = state.InterframeDelay as TimePeriod;
                } else {
                    state.TimeOut = 0;
                    return 1;
                }
            }
            if state.TimeOutOnSkip != 0
                && (menu(KEY_MENU_SELECT) || menu(KEY_MENU_SPECIAL) || menu(KEY_MENU_RIGHT))
            {
                state.TimeOut = 0;
                state.MovieFrame = -1;
                return 1;
            }
            SleepThread(delay);
            return 1;
        }
        while state.OperIndex < GetStringTableCount(state.SlideShow) {
            let mut opcode = [0 as c_char; 16];
            let mut src = GetStringAddress(state.SlideShow);
            state.OperIndex = state.OperIndex.wrapping_add(1);
            state.SlideShow = SetRelStringTableIndex(state.SlideShow, 1);
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
                    state.Buffer[0] = 0;
                    if libc::sscanf(
                        src,
                        c"%d %255[^\n]".as_ptr(),
                        ptr::addr_of_mut!(index),
                        state.Buffer.as_mut_ptr(),
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
                    let font = &mut state.Fonts[index as usize];
                    if state.Buffer[0] != 0 {
                        if !font.is_null() {
                            DestroyFont(*font);
                        }
                        *font = LoadFontFile(state.Buffer.as_ptr());
                    }
                    SetContextFont(*font);
                }
                b"ANI" => {
                    utf8StringCopy(state.Buffer.as_mut_ptr(), state.Buffer.len(), src);
                    if !state.Frame.is_null() {
                        DestroyDrawable(ReleaseDrawable(state.Frame));
                    }
                    state.Frame = CaptureDrawable(LoadGraphicFile(state.Buffer.as_ptr()));
                }
                b"MUSIC" => {
                    utf8StringCopy(state.Buffer.as_mut_ptr(), state.Buffer.len(), src);
                    if !state.MusicRef.is_null() {
                        StopMusic();
                        DestroyMusic(state.MusicRef);
                    }
                    state.MusicRef = LoadMusicFile(state.Buffer.as_ptr());
                    PlayMusic(state.MusicRef, 0, 1);
                }
                b"WAIT" | b"SYNC" | b"DSYNC" => {
                    let mut msecs = 0;
                    unbatch(state, true);
                    if libc::sscanf(src, c"%d".as_ptr(), ptr::addr_of_mut!(msecs)) == 1 {
                        let ticks = (msecs * ONE_SECOND as c_int / 1000) as TimeCount;
                        match CStr::from_ptr(opcode.as_ptr()).to_bytes() {
                            b"WAIT" => {
                                state.TimeOut = GetTimeCounter().wrapping_add(ticks);
                                state.TimeOutOnSkip = 1;
                            }
                            b"SYNC" => {
                                state.LastSyncTime = state.StartTime.wrapping_add(ticks);
                                state.TimeOut = state.LastSyncTime;
                                state.TimeOutOnSkip = 0;
                            }
                            _ => {
                                state.TimeOut = state.LastSyncTime.wrapping_add(ticks);
                                state.TimeOutOnSkip = 0;
                            }
                        }
                        return 1;
                    }
                }
                b"RESYNC" => {
                    state.StartTime = GetTimeCounter();
                    state.LastSyncTime = state.StartTime;
                }
                b"TC" => {
                    parse_color(src, &mut state.TextColor);
                }
                b"TBC" => {
                    parse_color(src, &mut state.TextBackColor);
                }
                b"TFC" => {
                    parse_color(src, &mut state.TextFadeColor);
                }
                b"TVA" => {
                    state.TextVPos = libc::toupper(*src as u8 as c_int);
                }
                b"TE" => {
                    state.TextEffect = libc::toupper(*src as u8 as c_int);
                }
                b"TEXT" => {
                    let (mut x, mut y) = (0, 0);
                    if libc::sscanf(
                        src,
                        c"%d %d %255[^\n]".as_ptr(),
                        ptr::addr_of_mut!(x),
                        ptr::addr_of_mut!(y),
                        state.Buffer.as_mut_ptr(),
                    ) == 3
                    {
                        let mut text = TEXT {
                            align: TEXT_ALIGN_ALIGN_CENTER,
                            pStr: state.Buffer.as_ptr(),
                            CharCount: COUNT::MAX,
                            baseline: POINT {
                                x: x as COORD,
                                y: y as COORD,
                            },
                        };
                        draw_text(
                            &mut text,
                            state.TextColor,
                            state.TextBackColor,
                            state.TextEffect,
                        );
                    }
                }
                b"TFI" => {
                    utf8StringCopy(state.Buffer.as_mut_ptr(), state.Buffer.len(), src);
                    state.LinesCount = parse_text_lines(
                        state.TextLines.as_mut_ptr(),
                        MAX_TEXT_LINES,
                        state.Buffer.as_mut_ptr(),
                    ) as COUNT;
                    unbatch(state, true);
                    let mut leading: SIZE = 0;
                    GetContextFontLeading(&mut leading);
                    let height = state.LinesCount as c_int * leading as c_int;
                    let mut y = match state.TextVPos {
                        84 => leading,
                        77 => ((state.clip_r.extent.height as c_int - height) / 2) as COORD,
                        _ => (state.clip_r.extent.height as c_int - height) as COORD,
                    };
                    state.tfade_r = state.clip_r;
                    state.tfade_r.corner.y =
                        (state.tfade_r.corner.y as c_int + y as c_int - leading as c_int) as COORD;
                    state.tfade_r.extent.height =
                        ((state.LinesCount as c_int + 1) * leading as c_int) as COORD;
                    for text in &mut state.TextLines[..state.LinesCount as usize] {
                        text.align = TEXT_ALIGN_ALIGN_CENTER;
                        text.baseline.x = (ScreenWidth / 2) as COORD;
                        text.baseline.y = y;
                        y = (y as c_int + leading as c_int) as COORD;
                    }
                    for text in &mut state.TextLines[..state.LinesCount as usize] {
                        draw_text(
                            text,
                            state.TextFadeColor,
                            state.TextFadeColor,
                            state.TextEffect,
                        );
                    }
                    SetTransitionSource(&state.tfade_r);
                    BatchGraphics();
                    for text in &mut state.TextLines[..state.LinesCount as usize] {
                        draw_text(text, state.TextColor, state.TextBackColor, state.TextEffect);
                    }
                    ScreenTransition(3, &state.tfade_r);
                    UnbatchGraphics();
                }
                b"TFO" => {
                    unbatch(state, true);
                    SetTransitionSource(&state.tfade_r);
                    BatchGraphics();
                    for text in &mut state.TextLines[..state.LinesCount as usize] {
                        draw_text(
                            text,
                            state.TextFadeColor,
                            state.TextFadeColor,
                            state.TextEffect,
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
                        stamp.frame = SetAbsFrameIndex(state.Frame, index as COUNT);
                    } else {
                        if state.SisFrame.is_null() {
                            generate_sis(state);
                        }
                        stamp.frame = SetAbsFrameIndex(state.SisFrame, 0);
                    }
                    if angle != 0 {
                        if angle != state.LastAngle || draw_kind != state.LastDrawKind {
                            DestroyDrawable(ReleaseDrawable(state.RotatedFrame));
                            state.RotatedFrame = CaptureDrawable(RotateFrame(stamp.frame, -angle));
                            state.LastAngle = angle;
                            state.LastDrawKind = draw_kind;
                        }
                        stamp.frame = state.RotatedFrame;
                    }
                    let old_mode = SetGraphicScaleMode(scale_mode as c_int);
                    let old_scale = SetGraphicScale(scale);
                    DrawStamp(&mut stamp);
                    SetGraphicScale(old_scale);
                    SetGraphicScaleMode(old_mode);
                }
                b"BATCH" => {
                    if state.Batched == 0 {
                        state.Batched = 1;
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
                    return fade_screen(state, src, kind);
                }
                b"CLS" => {
                    unbatch(state, true);
                    ClearDrawable();
                }
                b"CALL" => {
                    unbatch(state, true);
                    utf8StringCopy(state.Buffer.as_mut_ptr(), state.Buffer.len(), src);
                    show_file(state.Buffer.as_ptr());
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
                        SetContextForeGroundColor(state.TextColor);
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
                        state.MovieFrame = from;
                        state.MovieEndFrame = to;
                        state.InterframeDelay = ONE_SECOND as c_int / fps;
                        state.TimeOut = GetTimeCounter();
                        state.TimeOutOnSkip = 1;
                        return 1;
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
        if state.Ending {
            crate::automation::ending::presentation_completed();
        }
        0
    }

    unsafe fn show_slides(table: STRING, ending: bool) -> BOOLEAN {
        let mut state: PresentationState = std::mem::zeroed();
        state.SlideShow = table;
        if state.SlideShow.is_null() {
            crate::automation::ending::presentation_cancelled();
            return 0;
        }
        state.Ending = ending;
        if ending {
            crate::automation::ending::presentation_started();
        }
        state.SlideShow = SetAbsStringTableIndex(state.SlideShow, 0);
        state.OperIndex = 0;
        let old_context = SetContext(ScreenContext);
        let mut old_rect: RECT = std::mem::zeroed();
        GetContextClipRect(&mut old_rect);
        let old_font = SetContextFont(ptr::null_mut());
        SetContextBackGroundColor(BLACK_COLOR);
        SetMenuSounds(0, 0);
        state.InputFunc = Some(do_presentation);
        state.LastDrawKind = -1;
        state.TextVPos = b'B' as c_int;
        state.MovieFrame = -1;
        state.StartTime = GetTimeCounter();
        state.LastSyncTime = state.StartTime;
        DoInput(ptr::addr_of_mut!(state).cast(), 1);
        SleepThreadUntil(FadeMusic(0, ONE_SECOND as SIZE));
        StopMusic();
        FadeMusic(NORMAL_VOLUME as BYTE, 0);
        DestroyMusic(state.MusicRef);
        DestroyDrawable(ReleaseDrawable(state.RotatedFrame));
        DestroyDrawable(ReleaseDrawable(state.Frame));
        for font in state.Fonts {
            DestroyFont(font);
        }
        SetContextFont(old_font);
        SetContextClipRect(&mut old_rect);
        SetContext(old_context);
        1
    }

    unsafe extern "C" fn video_input(raw: *mut c_void) -> BOOLEAN {
        let state = &mut *raw.cast::<VideoState>();
        if PlayingLegacyVideo(state.CurVideo) == 0 {
            return 0;
        }
        if menu(KEY_MENU_SELECT)
            || menu(KEY_MENU_CANCEL)
            || menu(KEY_MENU_SPECIAL)
            || GlobData.Game_state.CurrentActivity & CHECK_ABORT as u16 != 0
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
        SetContext(ScreenContext);
        SetContextBackGroundColor(BLACK_COLOR);
        ClearDrawable();
        FadeScreen(ScreenFadeType_FadeAllToColor, 0);
    }

    unsafe fn show_video(video: *mut LegacyVideo) -> BOOLEAN {
        fade_clear_screen();
        let reference = PlayLegacyVideo(video);
        if reference.is_null() {
            return 0;
        }
        let mut state = VideoState {
            InputFunc: Some(video_input),
            CurVideo: reference,
        };
        SetMenuSounds(0, 0);
        DoInput(ptr::addr_of_mut!(state).cast(), 1);
        StopLegacyVideo(reference);
        fade_clear_screen();
        1
    }

    /// # Safety
    /// `presentation` must identify a registered resource. Graphics, audio and input
    /// must be initialized, and this call must run on the game thread.
    #[no_mangle]
    pub unsafe extern "C" fn ShowPresentation(presentation: RESOURCE) -> BOOLEAN {
        let kind = res_GetResourceType(presentation);
        if kind.is_null() {
            return 0;
        }
        match CStr::from_ptr(kind).to_bytes() {
            b"STRTAB" => {
                let table = CaptureStringTable(LoadStringTableInstance(presentation));
                let ending = CStr::from_ptr(presentation).to_bytes()
                    == &FINALPRES_STRTAB[..FINALPRES_STRTAB.len() - 1];
                let result = show_slides(table, ending);
                DestroyStringTable(ReleaseStringTable(table));
                result
            }
            b"3DOVID" => {
                let video = LoadLegacyVideoInstance(presentation);
                let result = show_video(video);
                DestroyLegacyVideo(video);
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

#[cfg(not(test))]
pub use runtime::ShowPresentation;

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
