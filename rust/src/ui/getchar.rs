#![cfg(any(not(test), feature = "linked_c_archive"))]
#![allow(non_snake_case)]

use crate::memory::{rust_hfree as HFree, rust_hmalloc as HMalloc};
use crate::navigation::abi::*;
use crate::navigation::globals::GlobData;
use libc::{strcmp, strlen};
use std::{ffi::c_char, mem, ptr};

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

/// # Safety
/// pTES and its strings, callbacks, and input subsystem must remain valid for the duration of the call.
#[no_mangle]
pub unsafe extern "C" fn DoTextEntry(pTES: *mut TEXTENTRY_STATE) -> BOOLEAN {
    if GlobData.Game_state.CurrentActivity & CHECK_ABORT as u16 != 0 {
        return 0;
    }
    if (*pTES).Initialized == 0 {
        let mut lower_len = 0;
        (*pTES).InputFunc = Some(DoTextEntry);
        (*pTES).Success = 0;
        (*pTES).Initialized = 1;
        (*pTES).JoystickMode = 0;
        (*pTES).UpperRegister = 1;
        let chars = utf8StringCount((*pTES).BaseStr);
        if (*pTES).CursorPos as usize > chars {
            (*pTES).CursorPos = chars as i32;
        }
        (*pTES).InsPt = skipUTF8Chars((*pTES).BaseStr, (*pTES).CursorPos as usize);
        (*pTES).JoyAlphaString = CaptureStringTable(LoadStringTableInstance(
            JOYSTICK_ALPHA_STRTAB.as_ptr().cast(),
        ));
        let table = (*pTES).JoyAlphaString;
        (*pTES).JoyAlpha = load_alpha(
            SetAbsStringTableIndex(table, 0),
            &mut (*pTES).JoyAlphaLength,
        )
        .cast();
        (*pTES).JoyUpper =
            load_alpha(SetAbsStringTableIndex(table, 1), &mut (*pTES).JoyRegLength).cast();
        (*pTES).JoyLower = load_alpha(SetAbsStringTableIndex(table, 2), &mut lower_len).cast();
        if lower_len != (*pTES).JoyRegLength {
            if lower_len < (*pTES).JoyRegLength {
                (*pTES).JoyRegLength = lower_len;
            }
            crate::log_abi::log_message(log_Level_log_Warning as i32, &format!("Warning: Joystick upper-lower registers size mismatch; using the smallest subset ({})", (*pTES).JoyRegLength));
        }
        (*pTES).CacheStr =
            HMalloc(((*pTES).MaxSize as usize).wrapping_mul(mem::size_of::<UNICODE>())).cast();
        EnterCharacterMode();
        DoInput(pTES.cast(), 1);
        ExitCharacterMode();
        if !(*pTES).CacheStr.is_null() {
            HFree((*pTES).CacheStr.cast());
        }
        if !(*pTES).JoyLower.is_null() {
            HFree((*pTES).JoyLower.cast());
        }
        if !(*pTES).JoyUpper.is_null() {
            HFree((*pTES).JoyUpper.cast());
        }
        if !(*pTES).JoyAlpha.is_null() {
            HFree((*pTES).JoyAlpha.cast());
        }
        DestroyStringTable(ReleaseStringTable((*pTES).JoyAlphaString));
        return (*pTES).Success;
    }
    let mut string = (*pTES).InsPt;
    let mut len = strlen(string.cast()) as i32;
    let cached_ins = string;
    let cached_cursor = (*pTES).CursorPos;
    ptr::copy_nonoverlapping((*pTES).BaseStr, (*pTES).CacheStr, (*pTES).MaxSize as usize);
    let mut changed = false;
    let mut ch = GetNextCharacter();
    if ch == 0 && PulsedInputState.menu[KEY_MENU_ANY as usize] != 0 {
        ch = GetLastCharacter();
    }
    while ch != 0 {
        let mut encoded = [0 as UNICODE; 8];
        (*pTES).JoystickMode = 0;
        let size = getStringFromChar(encoded.as_mut_ptr(), mem::size_of_val(&encoded), ch);
        if UniChar_isPrint(ch) != 0 && size > 0 {
            let offset = string.offset_from((*pTES).BaseStr);
            if offset + len as isize + (size as isize) < (*pTES).MaxSize as isize {
                ptr::copy(string, string.add(size as usize), len as usize + 1);
                ptr::copy_nonoverlapping(encoded.as_ptr(), string, size as usize);
                string = string.add(size as usize);
                (*pTES).CursorPos += 1;
                changed = true;
            } else {
                PlayMenuSound(MENU_SOUND_EFFECT_MENU_SOUND_FAILURE);
            }
        }
        ch = GetNextCharacter();
    }
    if PulsedInputState.menu[KEY_MENU_DELETE as usize] != 0 {
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
    } else if PulsedInputState.menu[KEY_MENU_BACKSPACE as usize] != 0 {
        if string > (*pTES).BaseStr {
            let previous = skipUTF8Chars((*pTES).BaseStr, ((*pTES).CursorPos - 1) as usize);
            ptr::copy(string, previous, len as usize + 1);
            string = previous;
            (*pTES).CursorPos -= 1;
            changed = true;
        }
    } else if PulsedInputState.menu[KEY_MENU_LEFT as usize] != 0 {
        if string > (*pTES).BaseStr {
            let previous = skipUTF8Chars((*pTES).BaseStr, ((*pTES).CursorPos - 1) as usize);
            string = previous;
            // The C code subtracts prev after assigning pStr to prev, so len is unchanged.
            (*pTES).CursorPos -= 1;
            changed = true;
        }
    } else if PulsedInputState.menu[KEY_MENU_RIGHT as usize] != 0 {
        if len > 0 {
            let current = char_from_utf8(string.cast());
            string = string.add(current.len as usize);
            len -= current.len as i32;
            (*pTES).CursorPos += 1;
            changed = true;
        }
    } else if PulsedInputState.menu[KEY_MENU_HOME as usize] != 0 {
        if string > (*pTES).BaseStr {
            string = (*pTES).BaseStr;
            len = strlen(string.cast()) as i32;
            (*pTES).CursorPos = 0;
            changed = true;
        }
    } else if PulsedInputState.menu[KEY_MENU_END as usize] != 0 && len > 0 {
        (*pTES).CursorPos += utf8StringCount(string) as i32;
        string = string.add(len as usize);
        len = 0;
        changed = true;
    }
    if !(*pTES).JoyAlpha.is_null()
        && [
            KEY_MENU_UP,
            KEY_MENU_DOWN,
            KEY_MENU_PAGE_UP,
            KEY_MENU_PAGE_DOWN,
        ]
        .iter()
        .any(|key| PulsedInputState.menu[*key as usize] != 0)
    {
        joystick_character(pTES, &mut string, &mut len, &mut changed);
    }
    if PulsedInputState.menu[KEY_MENU_SELECT as usize] != 0 {
        (*pTES).Success = 1;
        return 0;
    }
    if PulsedInputState.menu[KEY_MENU_EDIT_CANCEL as usize] != 0 {
        (*pTES).Success = 0;
        return 0;
    }
    (*pTES).InsPt = string;
    if let Some(callback) = (*pTES).ChangeCallback.filter(|_| changed) {
        if callback(pTES) == 0 {
            ptr::copy_nonoverlapping((*pTES).CacheStr, (*pTES).BaseStr, (*pTES).MaxSize as usize);
            (*pTES).InsPt = cached_ins;
            (*pTES).CursorPos = cached_cursor;
            PlayMenuSound(MENU_SOUND_EFFECT_MENU_SOUND_FAILURE);
        }
    }
    if let Some(callback) = (*pTES).FrameCallback {
        callback(pTES)
    } else {
        SleepThread(ONE_SECOND / 30);
        1
    }
}

unsafe fn load_alpha(string: STRING, count: *mut i32) -> *mut JoyChar {
    *count = 0;
    let source = GetStringAddress(string);
    if source.is_null() {
        return ptr::null_mut();
    }
    let total = utf8StringCount(source) as i32;
    let chars = HMalloc(total as usize * mem::size_of::<JoyChar>()).cast::<JoyChar>();
    if chars.is_null() {
        return ptr::null_mut();
    }
    let mut cursor = source;
    for i in 0..total {
        let ch = char_from_utf8(cursor.cast());
        *chars.add(i as usize) = ch;
        cursor = cursor.add(ch.len as usize);
    }
    *count = total;
    chars
}

unsafe fn joystick_character(
    pTES: *mut TEXTENTRY_STATE,
    string: &mut *mut UNICODE,
    len: &mut i32,
    changed: &mut bool,
) {
    let alpha = (*pTES).JoyAlpha.cast::<JoyChar>();
    let upper = (*pTES).JoyUpper.cast::<JoyChar>();
    let lower = (*pTES).JoyLower.cast::<JoyChar>();
    (*pTES).JoystickMode = 1;
    let (current, upper_case) = if *len != 0 {
        let c = char_from_utf8((*string).cast());
        (c, find_char(&c, lower, (*pTES).JoyRegLength) < 0)
    } else {
        (*alpha, (*pTES).UpperRegister != 0)
    };
    let mut next = current;
    let mut compare = current;
    switch_case(&mut compare, lower, upper, (*pTES).JoyRegLength);
    let mut index = find_char(&compare, alpha, (*pTES).JoyAlphaLength);
    if PulsedInputState.menu[KEY_MENU_UP as usize] != 0 {
        index -= 1;
        if index < 0 {
            index = (*pTES).JoyAlphaLength - 1;
        }
        next = *alpha.add(index as usize);
    } else if PulsedInputState.menu[KEY_MENU_DOWN as usize] != 0 {
        index += 1;
        if index >= (*pTES).JoyAlphaLength {
            index = 0;
        }
        next = *alpha.add(index as usize);
    }
    if PulsedInputState.menu[KEY_MENU_PAGE_UP as usize] != 0
        || PulsedInputState.menu[KEY_MENU_PAGE_DOWN as usize] != 0
    {
        if *len != 0 {
            if upper_case {
                switch_case(&mut next, upper, lower, (*pTES).JoyRegLength);
            } else {
                switch_case(&mut next, lower, upper, (*pTES).JoyRegLength);
            }
        } else {
            (*pTES).UpperRegister = u32::from((*pTES).UpperRegister == 0);
        }
    } else if upper_case {
        switch_case(&mut next, lower, upper, (*pTES).JoyRegLength);
    } else {
        switch_case(&mut next, upper, lower, (*pTES).JoyRegLength);
    }
    if strcmp(next.enc.as_ptr(), current.enc.as_ptr()) == 0 {
        return;
    }
    let offset = (*string).offset_from((*pTES).BaseStr);
    if *len != 0 {
        let rest = *len - current.len as i32;
        if offset + rest as isize + (next.len as isize) < (*pTES).MaxSize as isize {
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
    } else if offset + *len as isize + (next.len as isize) < (*pTES).MaxSize as isize {
        ptr::copy_nonoverlapping(next.enc.as_ptr(), (*string).cast(), next.len as usize);
        *(*string).add(next.len as usize) = 0;
        *len += next.len as i32;
        *changed = true;
    } else {
        PlayMenuSound(MENU_SOUND_EFFECT_MENU_SOUND_FAILURE);
    }
}
