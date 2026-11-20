//! Graphics: owned canvases, images, drawables, contexts, fonts, colour maps
//! and the draw command queue, drawn on the game thread and presented on the
//! display.
pub mod arena;
pub mod batch;
pub mod bbox;
pub mod canvas;
pub mod colormap;
pub mod context;
pub mod dcq;
mod draw;
pub mod drawable;
pub mod events;
mod flash_math;
pub mod font;
pub mod geometry;
pub mod highlight;
pub mod image;
pub mod intersection;
mod load;
pub mod menu;
pub mod palette_cycle;
pub mod present;
pub mod presentation_diagnostics;
pub mod presented_frame;
pub mod screen;
pub mod sdl;
pub mod software_scalers;
pub mod state;
pub mod surface;
