//! Netplay connection dialog, ported from uqm/cnctdlg.c.

use crate::engine::{battle::Cx, input::InputSession, observe::Io, Kernel};
use crate::graphics::{
    context_abi::ContextLease,
    native_abi::{Color, EXTENT, POINT, RECT, TEXT_ALIGN_ALIGN_CENTER, TEXT_ALIGN_ALIGN_LEFT},
    native_font::{FontLease, LoadedFont},
};
use crate::input::frames::{
    KEY_MENU_CANCEL, KEY_MENU_DELETE, KEY_MENU_DOWN, KEY_MENU_LEFT, KEY_MENU_RIGHT,
    KEY_MENU_SELECT, KEY_MENU_UP,
};
use crate::supermelee::netplay::NetplaySession;
use crate::ui::getchar::{edit_text, TextEntryCallbacks, TextEntryView};
use std::ffi::{CStr, CString};
use uqm_engine::time::counter_abi::GetTimeCounter;

const MCD_WIDTH: i16 = 260;
const MCD_HEIGHT: i16 = 110;
const MENU_FRAME_RATE: u32 = 840 / 20;
const TEXT_WIDTH: usize = 50;
const NETMELEE_STRING_BASE: usize = 560;

#[derive(Clone)]
struct TextValue {
    bytes: [u8; TEXT_WIDTH],
    cursor: usize,
    editing: bool,
    block_cursor: bool,
}
impl TextValue {
    fn new(text: &str) -> Self {
        let text = text.split('\0').next().unwrap_or_default();
        let mut length = text.len().min(TEXT_WIDTH - 1);
        while !text.is_char_boundary(length) {
            length -= 1;
        }
        let mut bytes = [0; TEXT_WIDTH];
        bytes[..length].copy_from_slice(&text.as_bytes()[..length]);
        Self {
            bytes,
            cursor: 0,
            editing: false,
            block_cursor: false,
        }
    }

    fn text(&self) -> &CStr {
        CStr::from_bytes_until_nul(&self.bytes)
            .unwrap_or_else(|error| panic!("connection text lost its terminator: {error}"))
    }

    fn finish_edit(&mut self, text: &CStr, accepted: bool) {
        if accepted {
            let text = text
                .to_str()
                .unwrap_or_else(|error| panic!("invalid edited UTF-8: {error}"));
            let replacement = Self::new(text);
            self.bytes = replacement.bytes;
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Event {
    Up,
    Down,
    Left,
    Right,
    Select,
    Cancel,
    Delete,
}
impl Event {
    fn from_input(input: &InputSession) -> Option<Self> {
        let pulse = input.pulsed();
        [
            (KEY_MENU_UP, Self::Up),
            (KEY_MENU_DOWN, Self::Down),
            (KEY_MENU_LEFT, Self::Left),
            (KEY_MENU_RIGHT, Self::Right),
            (KEY_MENU_SELECT, Self::Select),
            (KEY_MENU_CANCEL, Self::Cancel),
            (KEY_MENU_DELETE, Self::Delete),
        ]
        .into_iter()
        .find_map(|(key, event)| (pulse.menu[key as usize] != 0).then_some(event))
    }
}
#[derive(Debug, PartialEq, Eq)]
enum Action {
    None,
    Edit(usize),
    Commit(bool),
    Cancel,
}
struct DialogValues {
    text: [TextValue; 2],
    delay: i32,
    focus: usize,
}
impl DialogValues {
    fn new(session: &NetplaySession, side: usize) -> Self {
        let peer = &session.options.peer[side];
        Self {
            text: [TextValue::new(&peer.host), TextValue::new(&peer.port)],
            delay: session.options.input_delay as i32,
            focus: 0,
        }
    }

    fn event(&mut self, event: Event) -> Action {
        match event {
            Event::Up => self.focus = (self.focus + 5) % 6,
            Event::Down => self.focus = (self.focus + 1) % 6,
            Event::Cancel => {
                self.focus = 5;
                return Action::Cancel;
            }
            Event::Left if self.focus == 3 => self.delay = self.delay.wrapping_sub(1).max(0),
            Event::Right if self.focus == 3 => self.delay = self.delay.wrapping_add(1).min(9),
            Event::Select => {
                return match self.focus {
                    0 => Action::Commit(true),
                    1 => Action::Edit(0),
                    2 => Action::Commit(false),
                    4 => Action::Edit(1),
                    5 => Action::Cancel,
                    _ => Action::None,
                }
            }
            _ => {}
        }
        Action::None
    }
}

fn apply_connection_options(
    session: &mut NetplaySession,
    side: usize,
    values: &DialogValues,
    server: bool,
) {
    let options = session.options_mut();
    let peer = &mut options.peer[side];
    if !server {
        peer.host = values.text[0].text().to_string_lossy().into_owned();
    }
    peer.port = values.text[1].text().to_string_lossy().into_owned();
    peer.server = server;
    options.input_delay = values.delay as usize;
}

const fn rgb15(r: u8, g: u8, b: u8) -> Color {
    Color {
        r: (r << 3) | (r >> 2),
        g: (g << 3) | (g >> 2),
        b: (b << 3) | (b >> 2),
        a: 255,
    }
}
const MENU_TEXT_COLOR: Color = rgb15(6, 6, 6);
const MENU_HIGHLIGHT_COLOR: Color = rgb15(31, 31, 0);
const MENU_CURSOR_COLOR: Color = rgb15(31, 31, 31);

struct DialogResources {
    context: ContextLease,
    player_font: FontLease,
    button_font: FontLease,
    labels: [CString; 6],
    width: i16,
    height: i16,
}
impl DialogResources {
    fn new(kernel: &Kernel, player_font: &LoadedFont) -> Self {
        Self {
            context: ContextLease::require(kernel.screen_context),
            player_font: player_font.lease(),
            button_font: kernel
                .battle_font()
                .unwrap_or_else(|| panic!("connection button font not loaded")),
            labels: std::array::from_fn(|index| {
                kernel
                    .game_text(NETMELEE_STRING_BASE + 19 + index)
                    .to_owned()
            }),
            width: kernel.screen_width,
            height: kernel.screen_height,
        }
    }

    fn draw(&self, values: &DialogValues) {
        let rect = RECT {
            corner: POINT {
                x: (self.width - MCD_WIDTH) >> 1,
                y: (self.height - MCD_HEIGHT) >> 1,
            },
            extent: EXTENT {
                width: MCD_WIDTH,
                height: MCD_HEIGHT,
            },
        };
        self.context
            .draw_shadowed_box(rect, rgb15(20, 20, 20), rgb15(8, 8, 8), rgb15(16, 16, 16));
        let x = rect.corner.x + 10;
        let y = rect.corner.y + 18;
        // Child order is listen, host, connect, delay, port, cancel.
        for index in 0..6 {
            let row_y = y + index as i16 * 16;
            let focused = values.focus == index;
            match index {
                1 => self.draw_text_entry(&values.text[0], &self.labels[3], x, row_y, focused),
                3 => self.draw_slider(values, x, row_y),
                4 => self.draw_text_entry(&values.text[1], &self.labels[4], x, row_y, focused),
                _ => self.draw_button(
                    match index {
                        0 => 1,
                        2 => 0,
                        _ => 2,
                    },
                    row_y,
                    focused,
                ),
            }
        }
    }

    fn draw_button(&self, label: usize, y: i16, focused: bool) {
        self.context.with_text_state(|| {
            self.context.with_font_effect(None, || {
                self.context.draw_text(
                    &self.button_font,
                    &self.labels[label],
                    POINT { x: 160, y },
                    TEXT_ALIGN_ALIGN_CENTER,
                    Self::color(focused),
                );
            })
        });
    }

    fn color(focused: bool) -> Color {
        if focused {
            MENU_HIGHLIGHT_COLOR
        } else {
            MENU_TEXT_COLOR
        }
    }

    fn draw_slider(&self, values: &DialogValues, x: i16, y: i16) {
        let color = Self::color(values.focus == 3);
        let tick = MCD_WIDTH / 8;
        self.context.with_text_state(|| {
            self.context.with_font_effect(None, || {
                self.context.draw_text(
                    &self.player_font,
                    &self.labels[5],
                    POINT { x, y },
                    TEXT_ALIGN_ALIGN_LEFT,
                    color,
                );
                self.context.fill_rectangle(
                    color,
                    RECT {
                        corner: POINT {
                            x: x + 3 * tick,
                            y: y - 4,
                        },
                        extent: EXTENT {
                            width: 3 * tick,
                            height: 2,
                        },
                    },
                );
                self.context.fill_rectangle(
                    color,
                    RECT {
                        corner: POINT {
                            x: (i32::from(x + 3 * tick) + i32::from(3 * tick) * values.delay / 9
                                - 1) as i16,
                            y: y - 7,
                        },
                        extent: EXTENT {
                            width: 3,
                            height: 8,
                        },
                    },
                );
                let value = CString::new(values.delay.to_string())
                    .unwrap_or_else(|error| panic!("delay text: {error}"));
                self.context.draw_text(
                    &self.player_font,
                    &value,
                    POINT { x: x + 7 * tick, y },
                    TEXT_ALIGN_ALIGN_CENTER,
                    color,
                );
            })
        });
    }

    fn draw_text_entry(&self, value: &TextValue, label: &CStr, x: i16, y: i16, focused: bool) {
        let color = Self::color(focused);
        self.context.with_text_state(|| {
            self.context.with_font_effect(None, || {
                self.context.draw_batch(|| {
                    self.context.draw_text(
                        &self.player_font,
                        label,
                        POINT { x, y },
                        TEXT_ALIGN_ALIGN_LEFT,
                        color,
                    );
                    if value.editing {
                        self.draw_edit(value, x, y);
                    } else {
                        self.context.draw_text(
                            &self.player_font,
                            value.text(),
                            POINT { x: 160, y },
                            TEXT_ALIGN_ALIGN_CENTER,
                            color,
                        );
                    }
                })
            })
        });
    }

    fn draw_edit(&self, value: &TextValue, x: i16, y: i16) {
        let baseline = POINT { x: x + 90, y };
        let count = value
            .text()
            .to_str()
            .unwrap_or_else(|error| panic!("connection UTF-8: {error}"))
            .chars()
            .count();
        let measured = self.player_font.measure_bounded::<TEXT_WIDTH>(
            value.text(),
            baseline,
            TEXT_ALIGN_ALIGN_LEFT,
            count as u16,
        );
        let mut rect = RECT {
            corner: POINT {
                x: baseline.x - 1,
                y: baseline.y - measured.leading + 2,
            },
            extent: EXTENT {
                width: MCD_WIDTH - baseline.x + 1 - 10,
                height: measured.leading + 2,
            },
        };
        self.context.fill_rectangle(MENU_HIGHLIGHT_COLOR, rect);
        rect.corner.x += measured.deltas[..value.cursor]
            .iter()
            .map(|delta| i16::from(*delta))
            .sum::<i16>();
        if value.cursor < count {
            rect.corner.x -= 1;
        }
        rect.extent.width = if !value.block_cursor || value.cursor == count {
            1
        } else {
            i16::from(measured.deltas[value.cursor]) + if value.cursor + 1 == count { 2 } else { 1 }
        };
        rect.corner.x += 1;
        rect.corner.y += 1;
        rect.extent.height -= 2;
        self.context.fill_rectangle(MENU_CURSOR_COLOR, rect);
        self.context.draw_text_bounded(
            &self.player_font,
            value.text(),
            baseline,
            TEXT_ALIGN_ALIGN_LEFT,
            MENU_TEXT_COLOR,
            count as u16,
        );
    }
}

struct ConnectDialogState<'a> {
    session: &'a mut NetplaySession,
    side: usize,
    values: DialogValues,
    graphics: DialogResources,
    next_time: u32,
    initialized: bool,
    confirmed: bool,
    done: bool,
}
impl<'a> ConnectDialogState<'a> {
    fn new(session: &'a mut NetplaySession, side: usize, graphics: DialogResources) -> Self {
        assert!(side < 2, "connection side must index a peer");
        Self {
            values: DialogValues::new(session, side),
            session,
            side,
            graphics,
            next_time: 0,
            initialized: false,
            confirmed: true,
            done: false,
        }
    }

    fn edit(&mut self, index: usize, cx: &mut Cx<'_>, io: &mut Io<'_>, input: &InputSession) {
        let saved = self.values.text[index].clone();
        let mut buffer = saved.bytes;
        self.values.text[index].cursor = saved
            .text()
            .to_str()
            .unwrap_or_else(|error| panic!("connection UTF-8: {error}"))
            .chars()
            .count();
        self.values.text[index].editing = true;
        self.values.text[index].block_cursor = false;
        self.graphics.draw(&self.values);
        let cursor = self.values.text[index].cursor;
        let mut callbacks = EditCallbacks {
            values: &mut self.values,
            graphics: &self.graphics,
            index,
            next_time: GetTimeCounter().wrapping_add(MENU_FRAME_RATE),
        };
        let accepted = edit_text(
            cx,
            io,
            input,
            TEXT_WIDTH - 1,
            &mut buffer,
            cursor,
            &mut callbacks,
        );
        self.values.text[index] = saved;
        let text = CStr::from_bytes_until_nul(&buffer)
            .unwrap_or_else(|error| panic!("edited connection terminator: {error}"));
        self.values.text[index].finish_edit(text, accepted);
        self.graphics.draw(&self.values);
    }
}

struct EditCallbacks<'a> {
    values: &'a mut DialogValues,
    graphics: &'a DialogResources,
    index: usize,
    next_time: u32,
}
impl TextEntryCallbacks for EditCallbacks<'_> {
    fn changed(
        &mut self,
        entry: TextEntryView<'_>,
        _cx: &mut Cx<'_>,
        _input: &InputSession,
    ) -> bool {
        let value = &mut self.values.text[self.index];
        value.finish_edit(entry.text, true);
        value.cursor = entry.cursor;
        value.block_cursor = entry.joystick;
        true
    }

    fn frame(
        &mut self,
        _entry: TextEntryView<'_>,
        cx: &mut Cx<'_>,
        _io: &mut Io<'_>,
        _input: &InputSession,
    ) -> bool {
        self.graphics.draw(self.values);
        cx.kernel.input_runtime().sleep_until(cx, self.next_time);
        self.next_time = GetTimeCounter().wrapping_add(MENU_FRAME_RATE);
        true
    }
}

fn do_melee_connect_dialog(
    state: &mut ConnectDialogState<'_>,
    cx: &mut Cx<'_>,
    io: &mut Io<'_>,
    input: &InputSession,
) -> bool {
    input.set_paused(false);
    if !state.initialized {
        state.initialized = true;
        input.configure_repeat(840 / 12, 840 / 2, 840 / 12, false, GetTimeCounter());
        state.next_time = GetTimeCounter();
        state.graphics.draw(&state.values);
    }
    if let Some(event) = Event::from_input(input) {
        match state.values.event(event) {
            Action::Commit(server) => {
                apply_connection_options(state.session, state.side, &state.values, server);
                state.confirmed = true;
                state.done = true;
            }
            Action::Cancel => {
                state.confirmed = false;
                state.done = true;
            }
            Action::Edit(index) => state.edit(index, cx, io, input),
            Action::None => {}
        }
        state.graphics.draw(&state.values);
    }
    cx.kernel
        .input_runtime()
        .sleep_until(cx, state.next_time.wrapping_add(MENU_FRAME_RATE));
    state.next_time = GetTimeCounter();
    cx.world.activity() & 0x4000 == 0 && !state.done
}

/// Run one dialog with resources and options retained until its input loop exits.
pub fn melee_connect_dialog(
    cx: &mut Cx<'_>,
    io: &mut Io<'_>,
    input: &InputSession,
    session: &mut NetplaySession,
    side: usize,
) -> bool {
    let player_font = LoadedFont::load(c"font.player")
        .unwrap_or_else(|| panic!("connection player font not loaded"));
    let mut state =
        ConnectDialogState::new(session, side, DialogResources::new(cx.kernel, &player_font));
    cx.run_input(io, true, |cx, io| {
        do_melee_connect_dialog(&mut state, cx, io, input)
    });
    state.confirmed
}

#[cfg(test)]
mod tests;
