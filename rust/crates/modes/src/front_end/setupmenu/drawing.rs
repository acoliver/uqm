//! Retained setup rendering, independent of ambient widget focus and resources.
use super::{
    strings::SetupStrings,
    widgets::{MenuState, Widget},
    NameEntry,
};
use crate::graphics::{
    context_abi::ContextLease,
    drawable_abi::LoadedDrawable,
    native_abi::{Color, EXTENT, POINT, RECT, TEXT_ALIGN_ALIGN_CENTER, TEXT_ALIGN_ALIGN_LEFT},
    native_font::FontLease,
};
use std::ffi::{CStr, CString};

pub struct SetupResources {
    pub(super) context: ContextLease,
    pub(super) font: FontLease,
    pub(super) background: LoadedDrawable,
    pub(super) strings: SetupStrings,
}

pub(super) const fn color(r: u8, g: u8, b: u8) -> Color {
    Color {
        r: (r << 3) | (r >> 2),
        g: (g << 3) | (g >> 2),
        b: (b << 3) | (b >> 2),
        a: 255,
    }
}
const ACTIVE: Color = color(31, 31, 0);
const INACTIVE: Color = color(24, 24, 31);
const SELECTED: Color = color(31, 31, 31);
const COLUMNS: [i16; 24] = [
    3, 2, 3, 3, 2, 2, 2, 2, 2, 2, 2, 2, 3, 2, 2, 3, 3, 2, 3, 3, 3, 2, 2, 2,
];

pub(super) struct View<'a> {
    pub menu: &'a MenuState,
    pub names: &'a [CString; 6],
    pub controls: &'a [[CString; 2]; 7],
    pub entry: &'a NameEntry,
    pub gamma: f32,
}

impl SetupResources {
    pub fn load(context: ContextLease, font: FontLease) -> Self {
        let table = uqm_engine::strings::owned_table::LoadedTextTable::load(c"text.setupmenu")
            .unwrap_or_else(|| panic!("setup menu strings not loaded"));
        let strings = SetupStrings::parse(&table);
        let background = LoadedDrawable::load(c"graphics.setupmenu")
            .unwrap_or_else(|| panic!("setup background not loaded"));
        Self {
            context,
            font,
            background,
            strings,
        }
    }

    pub(super) fn draw(&self, view: &View<'_>) {
        self.context.draw_batch(|| {
            self.context.with_text_state(|| {
                self.context.with_font_effect(None, || {
                    let screen = self.context.target().rect().extent;
                    let frame = self
                        .background
                        .lease()
                        .context_frame([0, 1, 1, 2, 3, 1, 2, 1][view.menu.current]);
                    self.context.draw_stamp(&frame, POINT { x: 0, y: 0 });
                    self.text(&self.strings.title, screen.width / 2, 10, SELECTED, true);
                    self.text(
                        &self.strings.subtitles[view.menu.current],
                        screen.width / 2,
                        18,
                        SELECTED,
                        true,
                    );
                    let children = &view.menu.children[view.menu.current];
                    let height: i16 = children
                        .iter()
                        .map(|child| self.height(*child, view.menu) + 8)
                        .sum::<i16>()
                        - 8;
                    let mut y = (screen.height - height) / 2;
                    for (index, child) in children.iter().enumerate() {
                        self.draw_widget(
                            *child,
                            y,
                            index == view.menu.focus[view.menu.current],
                            view,
                            screen,
                        );
                        y += self.height(*child, view.menu) + 8;
                    }
                })
            })
        });
    }

    fn height(&self, widget: Widget, menu: &MenuState) -> i16 {
        match widget {
            Widget::Choice(choice) => ((menu.counts[choice] as i16 + 2) / 3) * 8,
            Widget::Label(label) => self.strings.labels[label].len() as i16 * 8,
            _ => 8,
        }
    }

    fn text(&self, text: &CStr, x: i16, y: i16, color: Color, centered: bool) {
        self.context.draw_text(
            &self.font,
            text,
            POINT { x, y },
            if centered {
                TEXT_ALIGN_ALIGN_CENTER
            } else {
                TEXT_ALIGN_ALIGN_LEFT
            },
            color,
        );
    }

    fn tips(&self, tips: &[CString; 3], screen: EXTENT) {
        let y = screen.height - 34;
        for (index, tip) in tips.iter().enumerate() {
            self.text(tip, screen.width / 2, y + 8 * index as i16, SELECTED, true);
        }
    }

    fn draw_widget(&self, widget: Widget, y: i16, focused: bool, view: &View<'_>, screen: EXTENT) {
        match widget {
            Widget::Choice(choice) => self.draw_choice(choice, y, focused, view, screen),
            Widget::Slider(slider) => self.draw_slider(slider, y, focused, view, screen),
            Widget::Button(button) => {
                let text = &self.strings.buttons[button];
                if focused {
                    self.tips(&text.tips, screen);
                }
                self.text(
                    &text.name,
                    160,
                    y,
                    if focused { ACTIVE } else { INACTIVE },
                    true,
                );
            }
            Widget::Label(label) => {
                for (index, line) in self.strings.labels[label].iter().enumerate() {
                    self.text(line, 160, y + index as i16 * 8, SELECTED, true);
                }
            }
            Widget::Text => self.draw_entry(view.entry, y, focused, screen),
            Widget::Control(control) => {
                self.text(
                    &self.strings.control_categories[control],
                    0,
                    y,
                    if focused { ACTIVE } else { SELECTED },
                    false,
                );
                for (index, name) in view.controls[control].iter().enumerate() {
                    self.text(
                        if name.is_empty() { c"---" } else { name },
                        screen.width / 2 + index as i16 * (screen.width / 3),
                        y,
                        if focused && view.menu.binding_column == index {
                            ACTIVE
                        } else {
                            SELECTED
                        },
                        true,
                    );
                }
            }
        }
    }

    fn draw_choice(&self, choice: usize, y: i16, focused: bool, view: &View<'_>, screen: EXTENT) {
        let text = &self.strings.choices[choice];
        self.text(
            &text.category,
            0,
            y,
            if focused { ACTIVE } else { SELECTED },
            false,
        );
        let home = 3 * (screen.width / ((COLUMNS[choice] + 1) * 2));
        for index in 0..view.menu.counts[choice] {
            let option = &text.options[index];
            let color = if focused && view.menu.highlight[choice] == index {
                self.tips(&option.tips, screen);
                ACTIVE
            } else if view.menu.choices[choice] == index {
                SELECTED
            } else {
                INACTIVE
            };
            let name = if matches!(choice, 18..=20) {
                &view.names[index]
            } else {
                &option.name
            };
            self.text(
                name,
                home + (index % 3) as i16 * (screen.width / (COLUMNS[choice] + 1)),
                y + (index / 3) as i16 * 8,
                color,
                true,
            );
        }
    }

    fn draw_slider(&self, slider: usize, y: i16, focused: bool, view: &View<'_>, screen: EXTENT) {
        let text = &self.strings.sliders[slider];
        let color = if focused {
            self.tips(&text.tips, screen);
            ACTIVE
        } else {
            SELECTED
        };
        self.text(&text.name, 0, y, color, false);
        let tick = screen.width / 8;
        self.context.fill_rectangle(
            color,
            RECT {
                corner: POINT {
                    x: 3 * tick,
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
                    x: 3 * tick + (3 * i32::from(tick) * view.menu.sliders[slider] / 100) as i16
                        - 1,
                    y: y - 7,
                },
                extent: EXTENT {
                    width: 3,
                    height: 8,
                },
            },
        );
        let value = if slider == 3 {
            format!("{:.4}", view.gamma)
        } else {
            view.menu.sliders[slider].to_string()
        };
        let value = CString::new(value).unwrap_or_else(|_| unreachable!("formatted number"));
        self.text(&value, 7 * tick, y, color, true);
    }

    fn draw_entry(&self, entry: &NameEntry, y: i16, focused: bool, screen: EXTENT) {
        self.text(
            &self.strings.text_category,
            0,
            y,
            if focused { ACTIVE } else { SELECTED },
            false,
        );
        if !entry.editing {
            self.text(
                &entry.text,
                160,
                y,
                if focused { ACTIVE } else { INACTIVE },
                true,
            );
            return;
        }
        let measured = self.font.measure_bounded::<50>(
            &entry.text,
            POINT { x: 90, y },
            TEXT_ALIGN_ALIGN_LEFT,
            u16::MAX,
        );
        let mut rect = RECT {
            corner: POINT {
                x: 89,
                y: y - measured.leading + 2,
            },
            extent: EXTENT {
                width: screen.width - 99,
                height: measured.leading + 2,
            },
        };
        self.context.fill_rectangle(ACTIVE, rect);
        rect.corner.x += measured
            .deltas
            .iter()
            .take(entry.cursor)
            .map(|delta| i16::from(*delta))
            .sum::<i16>();
        let characters = entry.text.to_string_lossy().chars().count();
        if entry.cursor < characters {
            rect.corner.x -= 1;
        }
        rect.extent.width = if !entry.block || entry.cursor == characters {
            1
        } else {
            i16::from(measured.deltas[entry.cursor])
                + if entry.cursor + 1 == characters { 2 } else { 1 }
        };
        rect.corner.x += 1;
        rect.corner.y += 1;
        rect.extent.height -= 2;
        self.context.fill_rectangle(color(0, 0, 0), rect);
        self.text(&entry.text, 90, y, INACTIVE, false);
    }

    pub(super) fn binding_prompt(&self) {
        let lines = &self.strings.labels[3];
        let width = lines
            .iter()
            .map(|line| line.to_string_lossy().chars().count())
            .max()
            .unwrap_or(0) as i16
            * 6
            + 16;
        let height = lines.len() as i16 * 8 + 16;
        let screen = self.context.target().rect().extent;
        let rect = RECT {
            corner: POINT {
                x: (screen.width - width) / 2,
                y: (screen.height - height) / 2,
            },
            extent: EXTENT { width, height },
        };
        self.context.draw_batch(|| {
            self.context.with_text_state(|| {
                self.context.with_font_effect(None, || {
                    self.context.draw_shadowed_box(
                        rect,
                        color(20, 20, 20),
                        color(8, 8, 8),
                        color(16, 16, 16),
                    );
                    for (index, line) in lines.iter().enumerate() {
                        self.text(
                            line,
                            rect.corner.x + width / 2,
                            rect.corner.y + 16 + index as i16 * 8,
                            color(0, 0, 0),
                            true,
                        );
                    }
                })
            })
        });
    }
}
