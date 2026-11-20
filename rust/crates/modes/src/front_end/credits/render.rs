use super::*;
use crate::comm::locdata::CRect;
use crate::engine::input::InputFrameHook;
use crate::engine::input::InputSession;
use crate::engine::observe::CheckpointView;
use crate::front_end::intro::args::Arguments;
use crate::front_end::intro::text::owned_lines;
use crate::front_end::intro::BLACK_COLOR;
use crate::graphics::{
    context::ContextLease, context::LoadedContext, drawable::ContextFrame,
    drawable::LoadedDrawable, font::LoadedFont, geometry::Color, geometry::Extent, geometry::Point,
    geometry::Rect, geometry::TextAlign,
};
use crate::strings::owned_table::LoadedTextTable;
use std::{
    cell::{Cell, RefCell},
    ffi::CString,
    rc::Rc,
};
use uqm_engine::time::counter_abi::GetTimeCounter;

const TRANS_COLOR: Color = crate::ui::rgb15(0, 0, 31);
const TEXT_COLOR: Color = crate::ui::rgb15(31, 31, 31);

pub(super) struct Resources {
    pub table: LoadedTextTable,
    pub background: Option<LoadedDrawable>,
    pub fonts: [Option<LoadedFont>; 3],
}
impl Resources {
    pub fn load() -> Option<Self> {
        Some(Self {
            table: LoadedTextTable::load(c"credits.credits")?,
            background: LoadedDrawable::load(c"credits.background"),
            fonts: [
                c"credits.font.pt13",
                c"credits.font.pt17",
                c"credits.font.pt45",
            ]
            .map(LoadedFont::load),
        })
    }
    pub fn background_frame(&self) -> ContextFrame {
        self.background
            .as_ref()
            .map_or_else(ContextFrame::default, |image| {
                image.lease().context_frame(0)
            })
    }
}

struct CreditFrame {
    image: LoadedDrawable,
    index: i32,
}
impl CreditFrame {
    fn frame(&self) -> ContextFrame {
        self.image.lease().context_frame(0)
    }
    fn height(&self) -> i32 {
        i32::from(self.frame().rect().extent.height)
    }
}

pub(super) struct Session {
    draw: LoadedContext,
    local: LoadedContext,
    frame: LoadedDrawable,
    frames: [Option<CreditFrame>; MAX_CREDIT_FRAMES],
    pub resources: Rc<Resources>,
    pub controls: CreditsControl,
    extent: Extent,
    first: usize,
    last: usize,
    total_height: i32,
    offset: i32,
    next_time: u32,
}

fn transparent_frame(
    context: &ContextLease,
    width: i32,
    height: i32,
    color: Color,
) -> LoadedDrawable {
    let image = LoadedDrawable::create(2, width as i16, height as i16, 1)
        .unwrap_or_else(|| panic!("credits pixmap allocation failed"));
    let frame = image.lease().context_frame(0);
    frame.transparent_color(color);
    context.with_frame(frame, || {
        context.set_background(color);
        context.clear();
    });
    image
}

#[derive(Clone, Copy)]
struct ColumnFormat {
    align: TextAlign,
    x: i16,
}
struct CreditText {
    text: CString,
    column: usize,
    row: i16,
}
struct Record {
    size: i32,
    alignment: Vec<u8>,
    lines: Vec<CreditText>,
    rows: usize,
}

fn record(line: &[u8]) -> Option<Record> {
    let mut args = Arguments::new(line);
    let size = args.integer()?;
    let alignment = args.word(31)?.to_vec();
    if size <= 0 {
        return None;
    }
    args.whitespace();
    let rows = owned_lines(&args.remaining()[..args.remaining().len().min(2047)], 50);
    if rows.is_empty() {
        return None;
    }
    let mut lines = Vec::new();
    for (row, text) in rows.iter().enumerate() {
        let bytes = text.to_bytes();
        let end = bytes
            .iter()
            .position(|&byte| byte == b'\t')
            .unwrap_or(bytes.len());
        lines.push(CreditText {
            text: CString::new(&bytes[..end])
                .unwrap_or_else(|error| panic!("credits column contains NUL: {error}")),
            column: 0,
            row: (row + 1) as i16,
        });
    }
    for (row, text) in rows.iter().enumerate() {
        for (column, bytes) in text
            .to_bytes()
            .splitn(5, |&byte| byte == b'\t')
            .enumerate()
            .skip(1)
        {
            if lines.len() < 50 {
                lines.push(CreditText {
                    text: CString::new(bytes)
                        .unwrap_or_else(|error| panic!("credits column contains NUL: {error}")),
                    column,
                    row: (row + 1) as i16,
                });
            }
        }
    }
    Some(Record {
        size,
        alignment,
        lines,
        rows: rows.len(),
    })
}

fn formats(alignment: &[u8], width: i16, margin: i16) -> [ColumnFormat; 5] {
    let mut result = [ColumnFormat {
        align: crate::graphics::geometry::TextAlign::Left,
        x: width / 64,
    }; 5];
    for (format, column) in result.iter_mut().zip(
        alignment
            .split(|&byte| byte == b',')
            .filter(|column| !column.is_empty()),
    ) {
        *format = ColumnFormat {
            align: crate::graphics::geometry::TextAlign::Left,
            x: margin,
        };
        let x = if column.get(1) == Some(&b'/') {
            Arguments::new(&column[2..]).integer().map(|x| x as i16)
        } else {
            None
        };
        match column[0] {
            b'L' => format.x = x.unwrap_or(margin),
            b'C' => {
                format.align = crate::graphics::geometry::TextAlign::Center;
                format.x = x.unwrap_or(width / 2);
            }
            b'R' => {
                format.align = crate::graphics::geometry::TextAlign::Right;
                format.x = x.unwrap_or(width - margin);
            }
            _ => (),
        }
    }
    result
}

impl Session {
    fn active_frame(&self, index: usize) -> &CreditFrame {
        self.frames[index]
            .as_ref()
            .unwrap_or_else(|| panic!("empty credits ring entry {index}"))
    }

    pub fn new(
        context: &ContextLease,
        resources: Rc<Resources>,
        victory: bool,
        window: CRect,
    ) -> Self {
        let local = LoadedContext::create(c"Credits.LocalContext");
        let draw = LoadedContext::create(c"Credits.DrawContext");
        let extent = context.clip_rect().extent;
        let local_context = local.lease();
        let frame = transparent_frame(
            &local_context,
            i32::from(extent.width),
            i32::from(extent.height),
            TRANS_COLOR,
        );
        local_context.set_frame(frame.lease().context_frame(0));
        let blank = transparent_frame(&local_context, 1, i32::from(extent.height), TRANS_COLOR);
        let mut frames = std::array::from_fn(|_| None);
        frames[0] = Some(CreditFrame {
            image: blank,
            index: -1,
        });
        draw.lease().set_frame(context.target());
        let mut controls = CreditsControl::new(victory);
        controls.outtakes = false;
        let session = Self {
            draw,
            local,
            frame,
            frames,
            resources,
            controls,
            extent,
            first: 0,
            last: 1,
            total_height: i32::from(extent.height),
            offset: 0,
            next_time: 0,
        };
        session.render_screen(window);
        session
    }

    fn render_text_frame(&self, index: &mut i32, direction: i32) -> Option<CreditFrame> {
        let count = self.resources.table.len() as i32;
        if !(0..count).contains(index) {
            let next = *index + direction;
            if !(0..count).contains(&next) {
                return None;
            }
            *index = next;
        }
        let mut line = None;
        while (0..count).contains(index) {
            line = self.resources.table.text(*index as usize);
            *index += direction;
            if line.is_some_and(|line| !line.is_empty()) {
                break;
            }
        }
        let record = record(line?.to_bytes())?;
        let font_index = [13, 17, 45].iter().position(|&size| record.size <= size)?;
        let font = self.resources.fonts[font_index].as_ref()?.lease();
        let context = self.local.lease();
        context.set_font(&font);
        let leading = font.leading();
        let margin = font
            .measure(
                c" ",
                Point { x: 100, y: 100 },
                crate::graphics::geometry::TextAlign::Left,
                1,
            )
            .rect
            .extent
            .width;
        let columns = formats(&record.alignment, self.extent.width, margin);
        let image = transparent_frame(
            &context,
            i32::from(self.extent.width),
            i32::from(leading) * record.rows as i32 + (i32::from(leading) >> 1),
            BLACK_COLOR,
        );
        context.with_frame(image.lease().context_frame(0), || {
            context.set_foreground(TEXT_COLOR);
            for text in record.lines {
                let column = columns[text.column];
                context.draw_text(
                    &font,
                    &text.text,
                    Point {
                        x: column.x,
                        y: (i32::from(text.row) * i32::from(leading)) as i16,
                    },
                    column.align,
                    TEXT_COLOR,
                );
            }
        });
        Some(CreditFrame {
            image,
            index: *index - direction,
        })
    }

    fn render_screen(&self, window: CRect) {
        let context = self.local.lease();
        context.draw_stamp(&self.resources.background_frame(), Point { x: 0, y: 0 });
        let mut y = -self.offset;
        let mut i = self.first;
        while i != self.last {
            let frame = self.frames[i]
                .as_ref()
                .unwrap_or_else(|| panic!("empty credits ring entry {i}"));
            context.draw_stamp(&frame.frame(), Point { x: 0, y: y as i16 });
            y += frame.height();
            i = frame_index(i as i32 + 1);
        }
        if self.controls.outtakes {
            context.fill_rectangle(
                TRANS_COLOR,
                Rect {
                    corner: Point {
                        x: window.corner.x,
                        y: window.corner.y,
                    },
                    extent: Extent {
                        width: window.width,
                        height: window.height,
                    },
                },
            );
        }
    }

    fn deficit_height(&self) -> i32 {
        let mut position = -self.offset;
        let mut i = self.first;
        while i != self.last {
            position += self.frames[i]
                .as_ref()
                .unwrap_or_else(|| panic!("empty credits ring entry {i}"))
                .height();
            i = frame_index(i as i32 + 1);
        }
        i32::from(self.extent.height) - position
    }

    pub fn process_frame(
        &mut self,
        window: &Cell<CRect>,
        checkpoint: &mut dyn FnMut(CheckpointView<'_>),
    ) {
        if GetTimeCounter() < self.next_time {
            return;
        }
        let (direction, step, delay) = scroll_step(self.controls.rate);
        self.next_time = GetTimeCounter().wrapping_add(delay);
        self.draw
            .lease()
            .draw_stamp(&self.frame.lease().context_frame(0), Point { x: 0, y: 0 });
        self.draw.lease().flush();
        let deficit = self.deficit_height();
        self.offset += step;
        if self.offset < -(i32::from(self.extent.height) / 20) {
            if self.controls.rate < 0 {
                self.controls.rate -= self.controls.rate / 10 - 1;
            }
        } else if deficit > i32::from(self.extent.height) / 25 {
            if self.controls.rate > 0 {
                self.controls.rate -= self.controls.rate / 10 + 1;
            }
            self.controls.running = self.controls.rate != 0;
            if !self.controls.running && self.controls.victory {
                checkpoint(CheckpointView::CreditsCompleted);
            }
        } else if !self.controls.running {
            self.controls.running = true;
        }
        self.retire_frame(direction);
        self.fill_frames(direction);
        self.render_screen(window.get());
    }

    fn retire_frame(&mut self, direction: i32) {
        if self.first == self.last {
            return;
        }
        if direction > 0 {
            let height = self.active_frame(self.first).height();
            if self.offset >= height {
                self.total_height -= height;
                drop(self.frames[self.first].take());
                self.first = frame_index(self.first as i32 + 1);
                self.offset -= height;
            }
        } else if direction < 0 {
            let index = frame_index(self.last as i32 - 1);
            let height = self.active_frame(index).height();
            if self.total_height - self.offset - height >= i32::from(self.extent.height) {
                self.last = index;
                self.total_height -= height;
                drop(self.frames[self.last].take());
            }
        }
    }

    fn fill_frames(&mut self, direction: i32) {
        if direction > 0 {
            let mut next = if self.first == self.last {
                0
            } else {
                self.active_frame(frame_index(self.last as i32 - 1)).index + 1
            };
            while self.total_height - self.offset < i32::from(self.extent.height)
                && next < self.resources.table.len() as i32
            {
                let frame = self.render_text_frame(&mut next, direction);
                if let Some(frame) = frame {
                    self.total_height += frame.height();
                    self.frames[self.last] = Some(frame);
                    self.last = frame_index(self.last as i32 + 1);
                }
            }
        } else if direction < 0 {
            let mut next = if self.first == self.last {
                self.resources.table.len() as i32 - 1
            } else {
                self.active_frame(self.first).index - 1
            };
            while self.offset < 0 && next >= 0 {
                let index = frame_index(self.first as i32 - 1);
                if let Some(frame) = self.render_text_frame(&mut next, direction) {
                    self.total_height += frame.height();
                    self.offset += frame.height();
                    self.frames[index] = Some(frame);
                    self.first = index;
                }
            }
        }
    }
}

pub(super) struct HookRestore<'a> {
    input: &'a InputSession,
    previous: Option<InputFrameHook>,
}
impl<'a> HookRestore<'a> {
    pub fn install(
        input: &'a InputSession,
        session: Rc<RefCell<Session>>,
        window: Rc<Cell<CRect>>,
    ) -> Self {
        let previous = input.replace_callback(None);
        input.replace_callback(Some(InputFrameHook::new(move |checkpoint, _| {
            session.borrow_mut().process_frame(&window, checkpoint);
        })));
        Self { input, previous }
    }
}
impl Drop for HookRestore<'_> {
    fn drop(&mut self) {
        self.input.replace_callback(None);
        self.input.replace_callback(self.previous.take());
    }
}

#[cfg(test)]
#[path = "render_tests.rs"]
mod tests;
