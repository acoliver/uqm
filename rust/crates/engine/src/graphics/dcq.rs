//! The draw command queue, ported from dcqueue.c (GPL-2.0-or-later).
//!
//! Drawing onto the screens is queued and replayed in order at flush time.
//! Commands pushed while batching stay invisible to the flush until the
//! outermost batch ends; a queue that reaches its capacity is flushed.
use super::colormap::ColorMap;
use super::font::Glyph;
use super::geometry::{Color, DrawMode, HotSpot, Rect};
use super::image::ImageId;
use super::state::VideoMode;
use std::collections::VecDeque;
use std::sync::Arc;

/// `DCQ_MAX`: queue capacity.
pub const DCQ_MAX: usize = 16384;

/// The three screen canvases (C: `SCREEN`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Screen {
    Main = 0,
    Extra = 1,
    Transition = 2,
}

impl Screen {
    pub const ALL: [Self; 3] = [Self::Main, Self::Extra, Self::Transition];

    pub fn index(self) -> usize {
        self as usize
    }
}

/// A function run on the game thread at its place in the queue.
pub type Callback = Box<dyn FnOnce()>;

/// One queued screen operation (C: `TFB_DrawCommand`).
pub enum DrawCommand {
    Line {
        points: [i32; 4],
        color: Color,
        mode: DrawMode,
        dest: Screen,
    },
    Rect {
        rect: Rect,
        color: Color,
        mode: DrawMode,
        dest: Screen,
    },
    Image {
        image: ImageId,
        x: i32,
        y: i32,
        colormap: Option<Arc<ColorMap>>,
        mode: DrawMode,
        scale: i32,
        scale_mode: i32,
        dest: Screen,
    },
    FilledImage {
        image: ImageId,
        x: i32,
        y: i32,
        color: Color,
        mode: DrawMode,
        scale: i32,
        scale_mode: i32,
        dest: Screen,
    },
    FontChar {
        glyph: Glyph,
        backing: ImageId,
        x: i32,
        y: i32,
        mode: DrawMode,
        dest: Screen,
    },
    Copy {
        rect: Rect,
        source: Screen,
        dest: Screen,
    },
    CopyToImage {
        image: ImageId,
        rect: Rect,
        source: Screen,
    },
    ScissorEnable(Rect),
    ScissorDisable,
    SetMipmap {
        image: ImageId,
        mipmap: ImageId,
        hot: HotSpot,
    },
    DeleteImage(ImageId),
    /// C: `DELETEDATA`: font glyph pixels released after the draws queued
    /// before them.
    DeleteData(Arc<[u8]>),
    ReinitVideo(VideoMode),
    Callback(Callback),
}

impl DrawCommand {
    /// Commands drawn onto a screen, which the scissor state applies to.
    pub fn draws(&self) -> bool {
        matches!(
            self,
            Self::Line { .. }
                | Self::Rect { .. }
                | Self::Image { .. }
                | Self::FilledImage { .. }
                | Self::FontChar { .. }
                | Self::Copy { .. }
                | Self::CopyToImage { .. }
        )
    }
}

impl std::fmt::Debug for DrawCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Self::Line { .. } => "Line",
            Self::Rect { .. } => "Rect",
            Self::Image { .. } => "Image",
            Self::FilledImage { .. } => "FilledImage",
            Self::FontChar { .. } => "FontChar",
            Self::Copy { .. } => "Copy",
            Self::CopyToImage { .. } => "CopyToImage",
            Self::ScissorEnable(_) => "ScissorEnable",
            Self::ScissorDisable => "ScissorDisable",
            Self::SetMipmap { .. } => "SetMipmap",
            Self::DeleteImage(_) => "DeleteImage",
            Self::DeleteData(_) => "DeleteData",
            Self::ReinitVideo(_) => "ReinitVideo",
            Self::Callback(_) => "Callback",
        };
        f.write_str(name)
    }
}

/// The queue: every pushed command, of which the first `visible` may be
/// replayed. Batching hides later pushes until it ends.
#[derive(Debug, Default)]
pub(crate) struct Queue {
    commands: VecDeque<DrawCommand>,
    visible: usize,
    batching: i32,
    /// The scissor rectangle the queue last enabled (`width == 0`: none).
    pub scissor: Rect,
}

impl Queue {
    pub const fn new() -> Self {
        Self {
            commands: VecDeque::new(),
            visible: 0,
            batching: 0,
            scissor: Rect::new(0, 0, 0, 0),
        }
    }

    fn synchronize(&mut self) {
        if self.batching == 0 {
            self.visible = self.commands.len();
        }
    }

    /// Whether a push must first flush (`lock_dcq(1)`).
    pub fn full(&self) -> bool {
        self.commands.len() >= DCQ_MAX - 1
    }

    pub fn push(&mut self, command: DrawCommand) {
        self.commands.push_back(command);
        self.synchronize();
    }

    pub fn pop(&mut self) -> Option<DrawCommand> {
        if self.visible == 0 {
            return None;
        }
        self.visible -= 1;
        self.commands.pop_front()
    }

    pub fn visible(&self) -> usize {
        self.visible
    }

    /// Every queued command, including those still held by a batch.
    #[cfg(all(feature = "test-support", not(test)))]
    pub fn len(&self) -> usize {
        self.commands.len()
    }

    pub fn batch(&mut self) {
        self.batching += 1;
    }

    pub fn unbatch(&mut self) {
        if self.batching != 0 {
            self.batching -= 1;
        }
        self.synchronize();
    }

    pub fn batch_depth(&self) -> i32 {
        self.batching
    }

    pub fn reset_batch(&mut self) {
        self.batching = 0;
        self.synchronize();
    }

    /// Drop every queued command, keeping what still has to be released.
    pub fn drain(&mut self) -> impl Iterator<Item = DrawCommand> + '_ {
        self.visible = 0;
        self.commands.drain(..)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line() -> DrawCommand {
        DrawCommand::Line {
            points: [0; 4],
            color: Color::default(),
            mode: DrawMode::REPLACE,
            dest: Screen::Main,
        }
    }

    #[test]
    fn batching_hides_commands_until_the_outermost_batch_ends() {
        let mut queue = Queue::new();
        queue.push(line());
        queue.batch();
        queue.batch();
        queue.push(line());
        assert_eq!(queue.visible(), 1);
        queue.unbatch();
        assert_eq!(queue.visible(), 1);
        queue.unbatch();
        assert_eq!(queue.visible(), 2);
        queue.unbatch();
        assert_eq!(queue.batch_depth(), 0);
        assert!(queue.pop().is_some());
        assert!(queue.pop().is_some());
        assert!(queue.pop().is_none());
    }

    #[test]
    fn a_batch_reset_publishes_hidden_commands() {
        let mut queue = Queue::new();
        queue.batch();
        queue.push(line());
        assert_eq!(queue.visible(), 0);
        queue.reset_batch();
        assert_eq!((queue.visible(), queue.batch_depth()), (1, 0));
    }
}
