//! Low-level drawing surface used by the Starbase and Outfit screens.
//!
//! [`Gfx`] mirrors the drawing calls the C screens make (`SetContext`,
//! `DrawStamp`, `font_DrawText`, ...). The drawing procedures in
//! [`super::draw`] are Rust bodies written against this trait. In the game
//! it is implemented over the existing graphics exports; in tests it is
//! implemented by a recorder that keeps the exact command sequence.

/// Opaque `FRAME` handle. `0` is the null frame.
pub type FrameId = usize;

/// An RGBA colour (`Color`, `gfxlib.h:26`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Rgba {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

const fn cc5_to_8(c: u8) -> u8 {
    (c << 3) | (c >> 2)
}

impl Rgba {
    /// `MAKE_RGB15 (r, g, b)`; the palette index of `BUILD_COLOR` is ignored
    /// by the C macro too.
    #[must_use]
    pub const fn rgb15(r: u8, g: u8, b: u8) -> Self {
        Self {
            r: cc5_to_8(r),
            g: cc5_to_8(g),
            b: cc5_to_8(b),
            a: 0xff,
        }
    }

    /// `BLACK_COLOR`.
    pub const BLACK: Self = Self::rgb15(0, 0, 0);
}

/// `POINT`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Point {
    pub x: i16,
    pub y: i16,
}

/// `RECT`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Rect {
    pub corner: Point,
    pub width: i16,
    pub height: i16,
}

/// `STAMP`: a frame drawn at an origin.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stamp {
    pub origin: Point,
    pub frame: FrameId,
}

/// The drawing contexts the screens use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContextId {
    Screen,
    Status,
    Space,
}

/// The fonts the screens use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontId {
    StarCon,
    Tiny,
    Micro,
}

/// `TEXT_ALIGN`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    Left,
    Center,
    Right,
}

/// One `font_DrawText` call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextDraw {
    pub baseline: Point,
    pub align: Align,
    pub text: String,
}

/// The target of `SetFlashRect`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlashTarget {
    /// `SetFlashRect (NULL)`: stop flashing.
    Off,
    /// `SetFlashRect (SFR_MENU_3DO)`: flash the 3DO menu frame.
    Menu3do,
    /// `SetFlashRect (SFR_MENU_ANY)`: flash the menu frame for either menu
    /// style.
    MenuAny,
    /// Flash this rectangle, in the coordinates of the current context.
    Rect(Rect),
}

/// Saved previous context, returned by [`Gfx::set_context`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContextHandle(pub usize);

/// Saved previous font, returned by [`Gfx::set_font`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FontHandle(pub usize);

/// Saved previous font effect frame, returned by
/// [`Gfx::set_font_effect`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FontEffectHandle(pub usize);

/// Drawing calls. All methods take `&self`: the game state they touch is
/// process-global in C, and the recorder uses interior mutability.
pub trait Gfx {
    /// `(ScreenWidth, ScreenHeight)`; `SCREEN_WIDTH` and `SCREEN_HEIGHT`.
    fn screen_size(&self) -> (i32, i32);

    fn set_context(&self, ctx: ContextId) -> ContextHandle;
    fn restore_context(&self, previous: ContextHandle);
    /// `GetContextClipRect` of the current context.
    fn context_clip_rect(&self) -> Rect;

    /// `SetContextForeGroundColor`; returns the old colour.
    fn set_fg_color(&self, color: Rgba) -> Rgba;
    /// `SetContextBackGroundColor`; returns the old colour.
    fn set_bg_color(&self, color: Rgba) -> Rgba;
    fn set_font(&self, font: FontId) -> FontHandle;
    fn restore_font(&self, previous: FontHandle);
    /// `SetContextFontEffect (SetAbsFrameIndex (FontGradFrame, index))`, or
    /// `SetContextFontEffect (NULL)` for `None`.
    fn set_font_effect(&self, grad_index: Option<u16>) -> FontEffectHandle;
    fn restore_font_effect(&self, previous: FontEffectHandle);
    /// `GetContextFontLeading`.
    fn font_leading(&self) -> i16;

    /// `SetContextClipRect` of the current context.
    fn set_context_clip_rect(&self, rect: Rect);

    fn fill_rect(&self, rect: Rect);
    fn draw_rect(&self, rect: Rect);
    fn draw_point(&self, point: Point);
    fn draw_stamp(&self, stamp: Stamp);
    fn draw_filled_stamp(&self, stamp: Stamp);
    fn draw_text(&self, text: &TextDraw);

    fn abs_frame(&self, frame: FrameId, index: u16) -> FrameId;
    fn rel_frame(&self, frame: FrameId, offset: i16) -> FrameId;
    fn inc_frame(&self, frame: FrameId) -> FrameId;
    fn dec_frame(&self, frame: FrameId) -> FrameId;
    fn frame_count(&self, frame: FrameId) -> u16;
    fn frame_index(&self, frame: FrameId) -> u16;
    /// The global `PlayFrame` holding the menu art.
    fn play_frame(&self) -> FrameId;

    fn batch(&self);
    fn unbatch(&self);
    fn clear_drawable(&self);
    fn set_transition_source_none(&self);
    fn screen_transition(&self, kind: i32);

    fn set_flash(&self, target: FlashTarget);
    fn pre_update_flash(&self);
    fn post_update_flash(&self);

    /// `GAME_STRING (index)`.
    fn game_string(&self, index: i32) -> String;
}
