//!
//! Phase 2: Graphics subsystem core functionality.
//!
pub mod cmap;
pub mod context;
pub mod dcqueue;
pub mod drawable;
pub mod ffi;
pub mod font;
pub mod frame;
pub mod gfx_common;
pub mod pixmap;
pub mod render_context;
pub mod scaling;
pub mod tfb_draw;

pub mod canvas_ffi;
pub mod sdl;

// Re-export public items from dcqueue module
pub use dcqueue::{
    scoped_batch, BatchGuard, Color as DcqColor, ColorMapRef, DcqConfig, DcqError, DcqStats,
    DrawCommand, DrawCommandQueue, DrawMode as DcqDrawMode, Extent as DcqExtent, FontCharRef,
    ImageRef, Screen,
};

// Re-export from colormap module
pub use cmap::{
    Color as CmapColor, ColorMapManager, ColorMapRef as CmapColorMapRef, FadeType, NativePalette,
    FADE_FULL_INTENSITY, FADE_NORMAL_INTENSITY, FADE_NO_INTENSITY, MAX_COLORMAPS,
    NUMBER_OF_PLUTVALS, PLUTVAL_BLUE, PLUTVAL_BYTE_SIZE, PLUTVAL_GREEN, PLUTVAL_RED,
};

// Re-export from drawable module
pub use drawable::{
    Coord, DrawableFlags, DrawableRegistry, DrawableType, Extent, HotSpot, Point as DrawablePoint,
    Rect as DrawableRect,
};

// Re-export from font module
pub use font::{
    draw_text, load_font, measure_text, Extent as FontExtent, Font, FontError, FontMetrics,
    FontPage, Point as FontPoint, TFChar, UniChar,
};

// Re-export from frame module
pub use frame::{Color as FrameColor, FrameDrawMode, FrameHandle, FrameRegistry};

// Re-export from gfx_common module
pub use gfx_common::{
    global_state, init_global_state, FrameRateState, GfxDriver, GfxFlags, GraphicsError,
    GraphicsState, RedrawMode, ScaleConfig, ScaleMode as GfxScaleMode, ScreenDimensions,
};

// Re-export from pixmap module
pub use pixmap::{Pixmap, PixmapError, PixmapFormat, PixmapLayout, PixmapRegistry};

// Re-export from scaling module
pub use scaling::{
    BilinearScaler, NearestScaler, ScaleCache, ScaleError, ScaleMode, ScaleParams, Scaler,
    ScalerManager, TrilinearScaler,
};

// Re-export from render_context module
pub use render_context::{RenderContext, ScreenHandle, ScreenType};

// Re-export from tfb_draw module
pub use tfb_draw::{
    draw_line, Canvas, CanvasError, CanvasFormat, CanvasId, CanvasPrimitive, ImagePrimitive,
    ScissorRect, TFImage, TFImageError,
};
