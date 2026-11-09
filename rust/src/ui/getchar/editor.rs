//! Bounded UTF-8 editing with synchronous borrowed callbacks.
use crate::engine::{
    battle::Cx,
    input::{frames::*, InputSession},
    observe::Io,
};
use std::ffi::CStr;
use uqm_engine::input_port::sdl_input::{
    EnterCharacterMode, ExitCharacterMode, GetLastCharacter, GetNextCharacter,
};

pub struct TextEntryView<'a> {
    pub text: &'a CStr,
    pub cursor: usize,
    pub joystick: bool,
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

#[derive(Default)]
pub struct Alphabet {
    characters: Vec<char>,
    upper: Vec<char>,
    lower: Vec<char>,
}

impl Alphabet {
    fn new(characters: &CStr, upper: &CStr, lower: &CStr) -> Self {
        let decode = |text: &CStr| {
            text.to_str()
                .unwrap_or_else(|error| panic!("joystick alphabet is not UTF-8: {error}"))
                .chars()
                .collect::<Vec<_>>()
        };
        let mut result = Self {
            characters: decode(characters),
            upper: decode(upper),
            lower: decode(lower),
        };
        let count = result.upper.len().min(result.lower.len());
        if result.upper.len() != result.lower.len() {
            crate::log_abi::log_message(3, &format!("Warning: Joystick upper-lower registers size mismatch; using the smallest subset ({count})"));
        }
        result.upper.truncate(count);
        result.lower.truncate(count);
        result
    }

    fn load() -> Self {
        crate::strings::owned_table::LoadedTextTable::load(c"text.joyalpha")
            .map_or_else(Self::default, |table| {
                Self::new(table.label(0).0, table.label(1).0, table.label(2).0)
            })
    }

    fn switch(ch: char, from: &[char], to: &[char]) -> char {
        from.iter()
            .position(|value| *value == ch)
            .map_or(ch, |i| to[i])
    }
}

pub struct Editor<'a> {
    buffer: &'a mut [u8],
    length: usize,
    position: usize,
    cursor: usize,
    joystick: bool,
    upper: bool,
    cache: Vec<u8>,
    cached_cursor: usize,
    cached_position: usize,
    success: bool,
}

impl<'a> Editor<'a> {
    pub fn new(buffer: &'a mut [u8], capacity: usize, cursor: usize) -> Self {
        assert!(
            (1..=buffer.len()).contains(&capacity),
            "text editor capacity exceeds its storage"
        );
        let buffer = &mut buffer[..capacity];
        let text = CStr::from_bytes_until_nul(buffer)
            .unwrap_or_else(|error| panic!("text editor buffer is not terminated: {error}"));
        let string = text
            .to_str()
            .unwrap_or_else(|error| panic!("text editor text is not UTF-8: {error}"));
        let length = text.to_bytes().len();
        let cursor = cursor.min(string.chars().count());
        let position = string
            .char_indices()
            .nth(cursor)
            .map_or(length, |(offset, _)| offset);
        Self {
            buffer,
            length,
            position,
            cursor,
            joystick: false,
            upper: true,
            cache: vec![0; capacity],
            cached_cursor: cursor,
            cached_position: position,
            success: false,
        }
    }

    pub fn view(&self) -> TextEntryView<'_> {
        TextEntryView {
            text: CStr::from_bytes_with_nul(&self.buffer[..=self.length])
                .unwrap_or_else(|error| panic!("editor terminator invariant: {error}")),
            cursor: self.cursor,
            joystick: self.joystick,
        }
    }

    fn text(&self) -> &str {
        self.view()
            .text
            .to_str()
            .unwrap_or_else(|error| panic!("editor UTF-8 invariant: {error}"))
    }

    fn save(&mut self) {
        self.cache.copy_from_slice(self.buffer);
        self.cached_cursor = self.cursor;
        self.cached_position = self.position;
    }

    fn restore(&mut self) {
        self.buffer.copy_from_slice(&self.cache);
        self.length = CStr::from_bytes_until_nul(self.buffer)
            .unwrap_or_else(|error| panic!("editor cache terminator: {error}"))
            .to_bytes()
            .len();
        self.cursor = self.cached_cursor;
        self.position = self.cached_position;
    }

    fn replace(&mut self, removed: usize, ch: Option<char>) -> bool {
        let mut encoded = [0; 4];
        let added = ch.map_or(&[][..], |ch| ch.encode_utf8(&mut encoded).as_bytes());
        let end = self.position + removed;
        let length = self.length - removed + added.len();
        if length >= self.buffer.len() {
            return false;
        }
        self.buffer
            .copy_within(end..=self.length, self.position + added.len());
        self.buffer[self.position..self.position + added.len()].copy_from_slice(added);
        self.length = length;
        true
    }

    fn insert(&mut self, ch: char) -> bool {
        self.joystick = false;
        if !self.replace(0, Some(ch)) {
            return false;
        }
        self.position += ch.len_utf8();
        self.cursor += 1;
        true
    }

    fn delete(&mut self) -> bool {
        let Some(ch) = self.text()[self.position..].chars().next() else {
            return false;
        };
        self.replace(ch.len_utf8(), None)
    }

    fn backspace(&mut self) -> bool {
        if self.position == 0 {
            return false;
        }
        self.move_left();
        self.delete()
    }

    fn move_left(&mut self) -> bool {
        let Some(ch) = self.text()[..self.position].chars().next_back() else {
            return false;
        };
        self.position -= ch.len_utf8();
        self.cursor -= 1;
        true
    }

    fn move_right(&mut self) -> bool {
        let Some(ch) = self.text()[self.position..].chars().next() else {
            return false;
        };
        self.position += ch.len_utf8();
        self.cursor += 1;
        true
    }

    fn joystick(&mut self, alphabet: &Alphabet, up: bool, down: bool, page: bool) -> bool {
        let Some(first) = alphabet.characters.first().copied() else {
            return false;
        };
        self.joystick = true;
        let actual = self.text()[self.position..].chars().next();
        let current = actual.unwrap_or(first);
        let upper = actual.map_or(self.upper, |ch| !alphabet.lower.contains(&ch));
        let compare = Alphabet::switch(current, &alphabet.lower, &alphabet.upper);
        let index = alphabet.characters.iter().position(|ch| *ch == compare);
        let count = alphabet.characters.len();
        let mut next = if up {
            alphabet.characters[index.filter(|i| *i > 0).map_or(count - 1, |i| i - 1)]
        } else if down {
            alphabet.characters[index.map_or(0, |i| (i + 1) % count)]
        } else {
            current
        };
        if page {
            if actual.is_some() {
                next = if upper {
                    Alphabet::switch(next, &alphabet.upper, &alphabet.lower)
                } else {
                    Alphabet::switch(next, &alphabet.lower, &alphabet.upper)
                };
            } else {
                self.upper = !self.upper;
            }
        } else if upper {
            next = Alphabet::switch(next, &alphabet.lower, &alphabet.upper);
        } else {
            next = Alphabet::switch(next, &alphabet.upper, &alphabet.lower);
        }
        next != current && self.replace(actual.map_or(0, char::len_utf8), Some(next))
    }

    fn cursor_keys(&mut self, pulse: &InputState) -> bool {
        let pressed = |key| pulse.menu[key as usize] != 0;
        if pressed(KEY_MENU_DELETE) {
            self.delete()
        } else if pressed(KEY_MENU_BACKSPACE) {
            self.backspace()
        } else if pressed(KEY_MENU_LEFT) {
            self.move_left()
        } else if pressed(KEY_MENU_RIGHT) {
            self.move_right()
        } else if pressed(KEY_MENU_HOME) && self.position != 0 {
            self.position = 0;
            self.cursor = 0;
            true
        } else if pressed(KEY_MENU_END) && self.position != self.length {
            self.cursor = self.text().chars().count();
            self.position = self.length;
            true
        } else {
            false
        }
    }

    pub fn frame(
        &mut self,
        cx: &mut Cx<'_>,
        io: &mut Io<'_>,
        input: &InputSession,
        alphabet: &Alphabet,
        callbacks: &mut dyn TextEntryCallbacks,
    ) -> bool {
        if cx.world.activity() & 0x4000 != 0 {
            return false;
        }
        let pulse = input.pulsed();
        let pressed = |key| pulse.menu[key as usize] != 0;
        self.save();
        let mut changed = false;
        let mut code = GetNextCharacter();
        if code == 0 && pressed(KEY_MENU_ANY) {
            code = GetLastCharacter();
        }
        while code != 0 {
            self.joystick = false;
            if crate::strings::unicode::uni_char_is_print(code) != 0 {
                if let Some(ch) = char::from_u32(code) {
                    if self.insert(ch) {
                        changed = true;
                    } else {
                        crate::ui::settings::play_menu_sound(cx.kernel, cx.world, 2);
                    }
                }
            }
            code = GetNextCharacter();
        }
        changed |= self.cursor_keys(&pulse);
        if [
            KEY_MENU_UP,
            KEY_MENU_DOWN,
            KEY_MENU_PAGE_UP,
            KEY_MENU_PAGE_DOWN,
        ]
        .iter()
        .any(|key| pressed(*key))
        {
            changed |= self.joystick(
                alphabet,
                pressed(KEY_MENU_UP),
                pressed(KEY_MENU_DOWN),
                pressed(KEY_MENU_PAGE_UP) || pressed(KEY_MENU_PAGE_DOWN),
            );
        }
        if pressed(KEY_MENU_SELECT) {
            self.success = true;
            return false;
        }
        if pressed(KEY_MENU_EDIT_CANCEL) {
            self.success = false;
            return false;
        }
        if changed && !callbacks.changed(self.view(), cx, input) {
            self.restore();
            crate::ui::settings::play_menu_sound(cx.kernel, cx.world, 2);
        }
        callbacks.frame(self.view(), cx, io, input)
    }

    pub fn success(&self) -> bool {
        self.success
    }
}

pub fn edit_text(
    cx: &mut Cx<'_>,
    io: &mut Io<'_>,
    input: &InputSession,
    capacity: usize,
    buffer: &mut [u8],
    cursor: usize,
    callbacks: &mut dyn TextEntryCallbacks,
) -> bool {
    if cx.world.activity() & 0x4000 != 0 {
        return false;
    }
    let mut editor = Editor::new(buffer, capacity, cursor);
    let alphabet = Alphabet::load();
    EnterCharacterMode();
    cx.run_input(io, true, |cx, io| {
        editor.frame(cx, io, input, &alphabet, callbacks)
    });
    ExitCharacterMode();
    editor.success()
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
