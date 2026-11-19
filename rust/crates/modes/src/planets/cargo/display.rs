//! Mineral display resources retained independently of the input callback owner.

use crate::engine::{Kernel, World};
use crate::graphics::{
    context_abi::{ContextFrame, ContextLease},
    native_abi::{
        Color, EXTENT, POINT, RECT, TEXT_ALIGN, TEXT_ALIGN_ALIGN_CENTER, TEXT_ALIGN_ALIGN_RIGHT,
    },
    native_font::FontLease,
};
use std::{ffi::CString, ptr::NonNull};

const BACK: Color = rgb15(0, 0, 20);
const WORTH: Color = rgb15(10, 10, 31);
const AMOUNT: Color = rgb15(0, 20, 20);
const SELECTED: Color = rgb15(10, 31, 31);
const CARGO_STRINGS: usize = 162;

const fn rgb15(r: u8, g: u8, b: u8) -> Color {
    Color {
        r: (r << 3) | (r >> 2),
        g: (g << 3) | (g >> 2),
        b: (b << 3) | (b >> 2),
        a: 255,
    }
}

pub(crate) fn remaining_capacity(world: &World) -> u32 {
    let ship = &world.data.flagship;
    let capacity = crate::ui::sis::capacity::storage_capacity(ship);
    u32::from(capacity).wrapping_sub(u32::from(ship.total_element_mass))
}

pub struct CargoDisplay {
    context: ContextLease,
    tiny: FontLease,
    heading: FontLease,
    icons: ContextFrame,
    title: CString,
    capacity_label: CString,
}

impl CargoDisplay {
    pub fn new(kernel: &Kernel) -> Self {
        Self {
            context: ContextLease::require(kernel.status_context),
            tiny: kernel
                .tiny_font_lease()
                .unwrap_or_else(|| panic!("cargo tiny font is not loaded")),
            heading: FontLease::retain(
                NonNull::new(kernel.star_con_font)
                    .unwrap_or_else(|| panic!("cargo heading font is not loaded")),
            ),
            icons: ContextFrame::retain(NonNull::new(kernel.misc_data_frame)),
            title: kernel.game_text(CARGO_STRINGS).to_owned(),
            capacity_label: kernel.game_text(CARGO_STRINGS + 1).to_owned(),
        }
    }

    fn fill(&self, x: i16, y: i16, width: i16, height: i16, color: Color) {
        self.context.fill_rectangle(
            color,
            RECT {
                corner: POINT { x, y },
                extent: EXTENT { width, height },
            },
        );
    }

    fn text(&self, x: i16, y: i16, align: TEXT_ALIGN, value: impl ToString, color: Color) {
        let text =
            CString::new(value.to_string()).unwrap_or_else(|_| panic!("cargo text contains a NUL"));
        self.context
            .draw_text(&self.tiny, &text, POINT { x, y }, align, color);
    }

    fn amount(&self, world: &World, element: u8, selected: bool) {
        let y = if element == 8 {
            119
        } else {
            35 + i16::from(element) * 9
        };
        self.fill(19, y, 40, 7, if selected { WORTH } else { BACK });
        let amount = if element == 8 {
            world.data.flagship.total_bio_mass
        } else {
            self.text(
                32,
                y + 6,
                TEXT_ALIGN_ALIGN_RIGHT,
                world.data.game.element_worth[usize::from(element)],
                if selected { AMOUNT } else { WORTH },
            );
            world.data.flagship.element_amounts[usize::from(element)]
        };
        self.text(
            58,
            y + 6,
            TEXT_ALIGN_ALIGN_RIGHT,
            amount,
            if selected { SELECTED } else { AMOUNT },
        );
    }

    pub fn show_capacity(&self, world: &World) {
        self.context.with_selected(|| {
            self.context.set_font(&self.tiny);
            self.context.with_batch(|| {
                self.fill(40, 107, 20, 7, BACK);
                self.text(
                    59,
                    113,
                    TEXT_ALIGN_ALIGN_RIGHT,
                    remaining_capacity(world),
                    WORTH,
                );
            });
        });
    }

    fn display(&self, world: &World) {
        let rectangle = RECT {
            corner: POINT { x: 2, y: 20 },
            extent: EXTENT {
                width: 60,
                height: 109,
            },
        };
        crate::ui::util::box_rectangles(
            rectangle,
            1,
            rgb15(16, 16, 16),
            rgb15(8, 8, 8),
            Some(BACK),
            |color, rect| self.context.fill_rectangle(color, rect),
        );
        self.context.draw_text(
            &self.heading,
            &self.title,
            POINT { x: 31, y: 27 },
            TEXT_ALIGN_ALIGN_CENTER,
            SELECTED,
        );
        self.context.set_font(&self.tiny);
        let mut frame = self.icons.indexed_frame(11);
        let bounds = frame.rect();
        self.text(32, 34, TEXT_ALIGN_ALIGN_RIGHT, "$", WORTH);
        self.text(58, 34, TEXT_ALIGN_ALIGN_RIGHT, "#", WORTH);
        for element in 0..8 {
            let y = 35 + i16::from(element) * 9;
            self.fill(
                7,
                y,
                bounds.extent.width,
                bounds.extent.height,
                rgb15(0, 0, 0),
            );
            self.context.draw_stamp(
                &frame,
                POINT {
                    x: 7 + (bounds.extent.width >> 1),
                    y: y + (bounds.extent.height >> 1),
                },
            );
            frame = frame.wrapped_frame(11 + u16::from(element + 1) * 5);
            self.amount(world, element, false);
        }
        self.fill(
            7,
            119,
            bounds.extent.width,
            bounds.extent.height,
            rgb15(0, 0, 0),
        );
        self.context.draw_stamp(
            &frame.indexed_frame(68),
            POINT {
                x: 7 + (bounds.extent.width >> 1),
                y: 119 + (bounds.extent.height >> 1),
            },
        );
        self.amount(world, 8, false);
        self.fill(4, 117, 56, 1, WORTH);
        self.context.draw_text(
            &self.tiny,
            &self.capacity_label,
            POINT { x: 5, y: 113 },
            crate::graphics::native_abi::TEXT_ALIGN_ALIGN_LEFT,
            WORTH,
        );
        self.show_capacity(world);
    }

    pub fn draw(&self, world: &World, mut old_element: u8, new_element: u8) {
        self.context.with_selected(|| {
            self.context.set_font(&self.tiny);
            self.context.with_batch(|| {
                if old_element > 8 {
                    self.display(world);
                    old_element = new_element;
                }
                if old_element != new_element {
                    self.amount(world, old_element, false);
                }
                if new_element != u8::MAX {
                    self.amount(world, new_element, true);
                }
            });
        });
    }
}

pub fn draw_cargo(kernel: &Kernel, world: &World, old: u8, new: u8) {
    CargoDisplay::new(kernel).draw(world, old, new);
}

pub fn show_capacity(kernel: &Kernel, world: &World) {
    CargoDisplay::new(kernel).show_capacity(world);
}
