use crate::engine::battle::Cx;
use crate::engine::input::{frames::InputState, InputSession};
use crate::engine::observe::Io;
use crate::navigation::abi::*;
use crate::ui::settings::play_menu_sound;

use libc::{strcmp, strlen};
use std::{ffi::c_char, mem, ptr};

pub struct TextEntryView<'a> {
    pub text: &'a std::ffi::CStr,
    pub cursor: usize,
    pub joystick: bool,
}

impl TextEntryView<'_> {
    unsafe fn capture(text: &TEXTENTRY_STATE) -> TextEntryView<'_> {
        TextEntryView {
            text: std::ffi::CStr::from_ptr(text.BaseStr),
            cursor: usize::try_from(text.CursorPos)
                .unwrap_or_else(|error| panic!("negative text cursor: {error}")),
            joystick: text.JoystickMode != 0,
        }
    }
}

pub trait TextEntryCallbacks {
    fn changed(&mut self, entry: TextEntryView<'_>, cx: &mut Cx<'_>, input: &InputSession) -> bool;
    fn frame(
        &mut self,
        entry: TextEntryView<'_>,
        cx: &mut Cx<'_>,
        io: &mut Io<'_>,
        input: &InputSession,
    ) -> bool;
}

/// Lend one terminated buffer and its callbacks through a synchronous editor session.
pub fn edit_text(
    cx: &mut Cx<'_>,
    io: &mut Io<'_>,
    input: &InputSession,
    buffer: &mut [u8],
    cursor: usize,
    callbacks: &mut dyn TextEntryCallbacks,
) -> bool {
    if cx.world.activity() & CHECK_ABORT as u16 != 0 {
        return false;
    }
    std::ffi::CStr::from_bytes_until_nul(buffer)
        .unwrap_or_else(|error| panic!("text editor buffer is not terminated: {error}"));
    // SAFETY: every pointer starts absent, and the borrowed buffer is stable until
    // this synchronous loop and all its callbacks finish. The core owns its cache.
    unsafe {
        let mut entry: TEXTENTRY_STATE = mem::zeroed();
        entry.BaseStr = buffer.as_mut_ptr().cast();
        entry.MaxSize = i32::try_from(buffer.len())
            .unwrap_or_else(|error| panic!("text editor buffer too large: {error}"));
        entry.CursorPos = i32::try_from(cursor)
            .unwrap_or_else(|error| panic!("text editor cursor too large: {error}"));
        let text = &raw mut entry;
        begin_text_entry(text, || {
            cx.run_input(io, true, |cx, io| {
                text_entry_frame_impl(text, cx, io, input, Some(&mut *callbacks)) != 0
            });
        }) != 0
    }
}

const FAILURE: u16 = MENU_SOUND_EFFECT_MENU_SOUND_FAILURE as u16;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct JoyChar {
    len: u8,
    enc: [c_char; 7],
}

unsafe fn char_from_utf8(src: *const c_char) -> JoyChar {
    let mut result = JoyChar {
        len: 0,
        enc: [0; 7],
    };
    if src.is_null() {
        return result;
    }
    let len = skipUTF8Chars(src.cast(), 1).offset_from(src.cast()) as usize;
    result.len = len as u8;
    ptr::copy_nonoverlapping(src, result.enc.as_mut_ptr(), len);
    result
}

unsafe fn find_char(ch: &JoyChar, set: *const JoyChar, count: i32) -> i32 {
    for i in 0..count {
        if strcmp((*set.add(i as usize)).enc.as_ptr(), ch.enc.as_ptr()) == 0 {
            return i;
        }
    }
    -1
}

unsafe fn switch_case(ch: &mut JoyChar, from: *const JoyChar, to: *const JoyChar, count: i32) {
    let i = find_char(ch, from, count);
    if i >= 0 {
        *ch = *to.add(i as usize);
    }
}

/// Run initialization, the input loop and cleanup for a complete entry session.
/// Prepared entries are stepped with [`text_entry_frame`].
///
/// # Safety
/// text and its strings, callbacks, and input subsystem must remain valid for the duration of the call.
// This legacy entry needs the application's published live input loop. The owned
// editor and its frame implementation below compile unchanged in unit tests.
#[cfg(not(test))]
pub unsafe fn do_text_entry(text: *mut TEXTENTRY_STATE, input: &InputSession) -> BOOLEAN {
    if crate::engine::World::current().read(|world| world.activity()) & CHECK_ABORT as u16 != 0 {
        return 0;
    }
    begin_text_entry(text, || {
        DoInput(1, &mut |cx: &mut Cx<'_>, io: &mut Io<'_>| {
            text_entry_frame(text, cx, io, input) != 0
        });
    })
}

/// # Safety
/// `BaseStr` and `CacheStr` are distinct writable buffers of at least `MaxSize` bytes.
/// `BaseStr` is NUL-terminated; `InsPt` points to one of its UTF-8 character boundaries.
/// Callbacks and their state remain valid and must not invalidate these buffers.
pub unsafe fn text_entry_frame(
    text: *mut TEXTENTRY_STATE,
    cx: &mut Cx<'_>,
    io: &mut Io<'_>,
    input: &InputSession,
) -> BOOLEAN {
    text_entry_frame_impl(text, cx, io, input, None)
}

unsafe fn text_entry_frame_impl(
    text: *mut TEXTENTRY_STATE,
    cx: &mut Cx<'_>,
    io: &mut Io<'_>,
    input: &InputSession,
    mut callbacks: Option<&mut dyn TextEntryCallbacks>,
) -> BOOLEAN {
    if cx.world.activity() & CHECK_ABORT as u16 != 0 {
        return 0;
    }
    let pulse = input.pulsed();
    let mut string = (*text).InsPt;
    let mut len = strlen(string.cast()) as i32;
    let (cached_ins, cached_cursor) = (string, (*text).CursorPos);
    ptr::copy_nonoverlapping((*text).BaseStr, (*text).CacheStr, (*text).MaxSize as usize);
    let mut changed = insert_characters(cx, &pulse, &mut *text, &mut string, len);
    changed |= edit_cursor(&pulse, &mut *text, &mut string, &mut len);
    if !(*text).JoyAlpha.is_null()
        && [
            KEY_MENU_UP,
            KEY_MENU_DOWN,
            KEY_MENU_PAGE_UP,
            KEY_MENU_PAGE_DOWN,
        ]
        .iter()
        .any(|key| pulse.menu[*key as usize] != 0)
    {
        joystick_character(cx, input, text, &mut string, &mut len, &mut changed);
    }
    if pulse.menu[KEY_MENU_SELECT as usize] != 0 {
        (*text).Success = 1;
        return 0;
    }
    if pulse.menu[KEY_MENU_EDIT_CANCEL as usize] != 0 {
        (*text).Success = 0;
        return 0;
    }
    (*text).InsPt = string;
    if changed {
        let accepted = if let Some(callbacks) = callbacks.as_deref_mut() {
            callbacks.changed(TextEntryView::capture(&*text), cx, input)
        } else if let Some(callback) = (*text).ChangeCallback {
            callback(text, cx, input) != 0
        } else {
            true
        };
        if !accepted {
            ptr::copy_nonoverlapping((*text).CacheStr, (*text).BaseStr, (*text).MaxSize as usize);
            (*text).InsPt = cached_ins;
            (*text).CursorPos = cached_cursor;
            play_menu_sound(cx.kernel, cx.world, FAILURE);
        }
    }
    if let Some(callbacks) = callbacks {
        u32::from(callbacks.frame(TextEntryView::capture(&*text), cx, io, input))
    } else if let Some(callback) = (*text).FrameCallback {
        callback(text, cx, io, input)
    } else {
        cx.kernel.input_runtime().sleep(cx, ONE_SECOND / 30);
        1
    }
}

unsafe fn insert_characters(
    cx: &mut Cx<'_>,
    pulse: &InputState,
    text: &mut TEXTENTRY_STATE,
    position: &mut *mut UNICODE,
    len: i32,
) -> bool {
    let mut string = *position;
    let mut changed = false;
    let mut ch = GetNextCharacter();
    if ch == 0 && pulse.menu[KEY_MENU_ANY as usize] != 0 {
        ch = GetLastCharacter();
    }
    while ch != 0 {
        let mut encoded = [0 as UNICODE; 8];
        text.JoystickMode = 0;
        let size = getStringFromChar(
            (encoded.as_mut_ptr()).cast(),
            mem::size_of_val(&encoded),
            ch,
        );
        if UniChar_isPrint(ch) != 0 && size > 0 {
            let offset = string.offset_from(text.BaseStr);
            if offset + len as isize + (size as isize) < text.MaxSize as isize {
                ptr::copy(string, string.add(size as usize), len as usize + 1);
                ptr::copy_nonoverlapping(encoded.as_ptr(), string, size as usize);
                string = string.add(size as usize);
                text.CursorPos += 1;
                changed = true;
            } else {
                play_menu_sound(cx.kernel, cx.world, FAILURE);
            }
        }
        ch = GetNextCharacter();
    }
    *position = string;
    changed
}

unsafe fn edit_cursor(
    pulse: &InputState,
    text: &mut TEXTENTRY_STATE,
    position: &mut *mut UNICODE,
    remaining: &mut i32,
) -> bool {
    let mut string = *position;
    let mut len = *remaining;
    let mut changed = false;
    if pulse.menu[KEY_MENU_DELETE as usize] != 0 {
        if len != 0 {
            let current = char_from_utf8(string.cast());
            ptr::copy(
                string.add(current.len as usize),
                string,
                (len - current.len as i32 + 1) as usize,
            );
            len -= current.len as i32;
            changed = true;
        }
    } else if pulse.menu[KEY_MENU_BACKSPACE as usize] != 0 {
        if string > text.BaseStr {
            let previous = skipUTF8Chars((text.BaseStr).cast(), (text.CursorPos - 1) as usize);
            ptr::copy(string, (previous).cast(), len as usize + 1);
            string = (previous).cast();
            text.CursorPos -= 1;
            changed = true;
        }
    } else if pulse.menu[KEY_MENU_LEFT as usize] != 0 {
        if string > text.BaseStr {
            let previous = skipUTF8Chars((text.BaseStr).cast(), (text.CursorPos - 1) as usize);
            string = (previous).cast();
            // The C code subtracts prev after assigning pStr to prev, so len is unchanged.
            text.CursorPos -= 1;
            changed = true;
        }
    } else if pulse.menu[KEY_MENU_RIGHT as usize] != 0 {
        if len > 0 {
            let current = char_from_utf8(string.cast());
            string = string.add(current.len as usize);
            len -= current.len as i32;
            text.CursorPos += 1;
            changed = true;
        }
    } else if pulse.menu[KEY_MENU_HOME as usize] != 0 {
        if string > text.BaseStr {
            string = text.BaseStr;
            len = strlen(string.cast()) as i32;
            text.CursorPos = 0;
            changed = true;
        }
    } else if pulse.menu[KEY_MENU_END as usize] != 0 && len > 0 {
        text.CursorPos += utf8StringCount((string).cast()) as i32;
        string = string.add(len as usize);
        len = 0;
        changed = true;
    }
    *position = string;
    *remaining = len;
    changed
}

unsafe fn begin_text_entry(text: *mut TEXTENTRY_STATE, run: impl FnOnce()) -> BOOLEAN {
    (*text).Success = 0;
    (*text).Initialized = 1;
    (*text).JoystickMode = 0;
    (*text).UpperRegister = 1;
    let chars = utf8StringCount(((*text).BaseStr).cast());
    if (*text).CursorPos as usize > chars {
        (*text).CursorPos = chars as i32;
    }
    (*text).InsPt = (skipUTF8Chars(((*text).BaseStr).cast(), (*text).CursorPos as usize)).cast();
    (*text).JoyAlphaString = CaptureStringTable(LoadStringTableInstance(
        JOYSTICK_ALPHA_STRTAB.as_ptr().cast(),
    ));
    let table = (*text).JoyAlphaString;
    let mut alpha = load_alpha(SetAbsStringTableIndex(table, 0));
    let mut upper = load_alpha(SetAbsStringTableIndex(table, 1));
    let mut lower = load_alpha(SetAbsStringTableIndex(table, 2));
    (*text).JoyAlphaLength = i32::try_from(alpha.len())
        .unwrap_or_else(|error| panic!("joystick alphabet too large: {error}"));
    (*text).JoyRegLength = i32::try_from(upper.len().min(lower.len()))
        .unwrap_or_else(|error| panic!("joystick register too large: {error}"));
    (*text).JoyAlpha = alpha
        .first_mut()
        .map_or(ptr::null_mut(), ptr::from_mut)
        .cast();
    (*text).JoyUpper = upper
        .first_mut()
        .map_or(ptr::null_mut(), ptr::from_mut)
        .cast();
    (*text).JoyLower = lower
        .first_mut()
        .map_or(ptr::null_mut(), ptr::from_mut)
        .cast();
    if lower.len() != upper.len() {
        crate::log_abi::log_message(log_Level_log_Warning as i32, &format!("Warning: Joystick upper-lower registers size mismatch; using the smallest subset ({})", (*text).JoyRegLength));
    }
    let size = usize::try_from((*text).MaxSize)
        .unwrap_or_else(|error| panic!("negative text buffer size: {error}"));
    let mut cache = vec![0u8; size];
    (*text).CacheStr = cache.as_mut_ptr().cast();
    EnterCharacterMode();
    run();
    ExitCharacterMode();
    DestroyStringTable(ReleaseStringTable((*text).JoyAlphaString));
    (*text).Success
}

unsafe fn load_alpha(string: STRING) -> Vec<JoyChar> {
    let source = GetStringAddress(string);
    if source.is_null() {
        return Vec::new();
    }
    let total = utf8StringCount((source).cast());
    let mut chars = Vec::with_capacity(total);
    let mut cursor = source;
    for _ in 0..total {
        let ch = char_from_utf8(cursor.cast());
        chars.push(ch);
        cursor = cursor.add(ch.len as usize);
    }
    chars
}

unsafe fn joystick_character(
    cx: &mut Cx<'_>,
    input: &InputSession,
    text: *mut TEXTENTRY_STATE,
    string: &mut *mut UNICODE,
    len: &mut i32,
    changed: &mut bool,
) {
    let alpha = (*text).JoyAlpha.cast::<JoyChar>();
    let upper = (*text).JoyUpper.cast::<JoyChar>();
    let lower = (*text).JoyLower.cast::<JoyChar>();
    (*text).JoystickMode = 1;
    let (current, upper_case) = if *len != 0 {
        let c = char_from_utf8((*string).cast());
        (c, find_char(&c, lower, (*text).JoyRegLength) < 0)
    } else {
        (*alpha, (*text).UpperRegister != 0)
    };
    let mut next = current;
    let mut compare = current;
    switch_case(&mut compare, lower, upper, (*text).JoyRegLength);
    let mut index = find_char(&compare, alpha, (*text).JoyAlphaLength);
    if input.pulsed().menu[KEY_MENU_UP as usize] != 0 {
        index -= 1;
        if index < 0 {
            index = (*text).JoyAlphaLength - 1;
        }
        next = *alpha.add(index as usize);
    } else if input.pulsed().menu[KEY_MENU_DOWN as usize] != 0 {
        index += 1;
        if index >= (*text).JoyAlphaLength {
            index = 0;
        }
        next = *alpha.add(index as usize);
    }
    if input.pulsed().menu[KEY_MENU_PAGE_UP as usize] != 0
        || input.pulsed().menu[KEY_MENU_PAGE_DOWN as usize] != 0
    {
        if *len != 0 {
            if upper_case {
                switch_case(&mut next, upper, lower, (*text).JoyRegLength);
            } else {
                switch_case(&mut next, lower, upper, (*text).JoyRegLength);
            }
        } else {
            (*text).UpperRegister = u32::from((*text).UpperRegister == 0);
        }
    } else if upper_case {
        switch_case(&mut next, lower, upper, (*text).JoyRegLength);
    } else {
        switch_case(&mut next, upper, lower, (*text).JoyRegLength);
    }
    if strcmp(next.enc.as_ptr(), current.enc.as_ptr()) == 0 {
        return;
    }
    let offset = (*string).offset_from((*text).BaseStr);
    if *len != 0 {
        let rest = *len - current.len as i32;
        if offset + rest as isize + (next.len as isize) < (*text).MaxSize as isize {
            if next.len != current.len {
                ptr::copy(
                    (*string).add(current.len as usize),
                    (*string).add(next.len as usize),
                    rest as usize + 1,
                );
            }
            ptr::copy_nonoverlapping(next.enc.as_ptr(), (*string).cast(), next.len as usize);
            *len = rest + next.len as i32;
            *changed = true;
        }
    } else if offset + *len as isize + (next.len as isize) < (*text).MaxSize as isize {
        ptr::copy_nonoverlapping(next.enc.as_ptr(), (*string).cast(), next.len as usize);
        *(*string).add(next.len as usize) = 0;
        *len += next.len as i32;
        *changed = true;
    } else {
        play_menu_sound(cx.kernel, cx.world, FAILURE);
    }
}
