//! The SDL2 window: present the composed screen canvases through a software
//! renderer, with the optional 2x and xBRZ filters and scanlines.
use super::super::canvas::Canvas;
use super::super::presentation_diagnostics::{screen_result, PresentationSample};
use super::super::presented_frame::PresentedFrame;
use super::super::software_scalers::scale_screen;
use super::super::state::{GFXFLAGS_FULLSCREEN, GFXFLAGS_SCANLINES, SCREEN_HEIGHT, SCREEN_WIDTH};
use crate::bridge_log::rust_bridge_log_msg;
use crate::host::{Display, Layer, PresentFrame};
use sdl2::pixels::PixelFormatEnum;
use sdl2::render::BlendMode;

#[cfg(all(feature = "debug-process", target_os = "macos"))]
use super::macos;

const XBRZ3: i32 = 1 << 8;
const XBRZ4: i32 = 1 << 9;

/// The software scale factor `flags` select: xBRZ 3x or 4x, 2x for the
/// integer filters, none for bilinear or no scaling.
fn scale_factor(flags: i32) -> Option<u32> {
    let soft_only = (1 << 4) | (1 << 5) | (1 << 6) | (1 << 7) | XBRZ3 | XBRZ4;
    if flags & soft_only == 0 {
        None
    } else if flags & XBRZ3 != 0 {
        Some(3)
    } else if flags & XBRZ4 != 0 {
        Some(4)
    } else {
        Some(2)
    }
}

/// The SDL2 window, renderer and presentation diagnostics.
pub struct SdlDisplay {
    // Field order is drop order: renderer and window before SDL itself.
    canvas: sdl2::render::Canvas<sdl2::video::Window>,
    video: sdl2::VideoSubsystem,
    sdl: sdl2::Sdl,
    flags: i32,
    /// The configured window size (C: `ScreenWidthActual`/`ScreenHeightActual`).
    size: (u32, u32),
    scratch: Vec<u8>,
    software_scaler_logged: bool,
    xbrz_logged: bool,
    presentation_count: u64,
    samples: Vec<PresentationSample>,
    #[cfg(feature = "debug-process")]
    native_position: Option<(i32, i32)>,
    #[cfg(feature = "debug-process")]
    native_display_bounds: Option<sdl2::rect::Rect>,
}

impl SdlDisplay {
    /// Open the window (C: `TFB_Pure_InitGraphics`).
    pub fn open(
        flags: i32,
        native_position: Option<(i32, i32)>,
        width: i32,
        height: i32,
    ) -> Result<Self, String> {
        rust_bridge_log_msg(&format!(
            "RUST_GFX_INIT: flags=0x{flags:x} ({flags}) width={width} height={height}"
        ));
        log_scaler_flags(flags);
        let sdl = sdl2::init().map_err(|e| format!("SDL2 init failed: {e}"))?;
        let video = sdl.video().map_err(|e| format!("video subsystem: {e}"))?;
        rust_bridge_log_msg(&format!(
            "RUST_GFX_INIT: SDL2 video driver: {}",
            video.current_video_driver()
        ));
        let mut window = video.window(
            "The Ur-Quan Masters v0.8.0 (Rust)",
            width as u32,
            height as u32,
        );
        window.position_centered();
        if let Some((x, y)) = native_position {
            // A decorated 960px client exceeds a 1080px desktop's work area
            // once the menu bar and Dock are included, and Cocoa then moves
            // it on activation. Borderless windows also skip the title-frame
            // show animation.
            window.borderless().position(x, y);
        }
        let window = window
            .build()
            .map_err(|e| format!("window creation failed: {e}"))?;
        // The software renderer avoids GPU format surprises.
        let mut canvas = window
            .into_canvas()
            .software()
            .present_vsync()
            .build()
            .map_err(|e| format!("renderer creation failed: {e}"))?;
        // Nearest-neighbour scaling keeps pixels crisp.
        sdl2::hint::set("SDL_HINT_RENDER_SCALE_QUALITY", "0");
        canvas
            .set_logical_size(SCREEN_WIDTH as u32, SCREEN_HEIGHT as u32)
            .map_err(|e| format!("set_logical_size failed: {e}"))?;
        rust_bridge_log_msg(&format!(
            "RUST_GFX_INIT: Logical size {SCREEN_WIDTH}x{SCREEN_HEIGHT} (window {width}x{height})"
        ));
        rust_bridge_log_msg(&format!("RUST_GFX_INIT: Renderer: {}", canvas.info().name));
        #[cfg(not(feature = "debug-process"))]
        let _ = native_position;
        let mut display = Self {
            canvas,
            video,
            sdl,
            flags,
            size: (width as u32, height as u32),
            scratch: Vec::new(),
            software_scaler_logged: false,
            xbrz_logged: false,
            presentation_count: 0,
            samples: Vec::new(),
            #[cfg(feature = "debug-process")]
            native_position,
            #[cfg(feature = "debug-process")]
            native_display_bounds: None,
        };
        if flags & GFXFLAGS_FULLSCREEN != 0 {
            display.reconfigure(flags, width, height)?;
        }
        rust_bridge_log_msg("RUST_GFX_INIT: Success");
        Ok(display)
    }

    /// Upload a screen canvas, whole or one area, onto the frame.
    fn screen_layer(
        &mut self,
        screen: i32,
        canvas: &Canvas,
        alpha: u8,
        rect: Option<sdl2::rect::Rect>,
    ) {
        let words = canvas.words();
        let (bytes, pitch, scale) = match scale_factor(self.flags) {
            Some(factor) => {
                let (width, height) = (canvas.width() as usize, canvas.height() as usize);
                let scaled = width * factor as usize;
                self.scratch
                    .resize(scaled * height * factor as usize * 4, 0);
                if self.flags & (XBRZ3 | XBRZ4) != 0 {
                    if !self.xbrz_logged {
                        rust_bridge_log_msg(&format!("RUST_GFX: xBRZ scaler active ({factor}x)"));
                        self.xbrz_logged = true;
                    }
                    let rgba: Vec<u8> = words
                        .iter()
                        .flat_map(|&word| {
                            [
                                (word >> 24) as u8,
                                (word >> 16) as u8,
                                (word >> 8) as u8,
                                0xff,
                            ]
                        })
                        .collect();
                    let scaled_rgba = xbrz::scale_rgba(&rgba, width, height, factor as usize);
                    for (out, pixel) in self
                        .scratch
                        .as_chunks_mut::<4>()
                        .0
                        .iter_mut()
                        .zip(scaled_rgba.as_chunks::<4>().0)
                    {
                        *out = [0xff, pixel[2], pixel[1], pixel[0]];
                    }
                } else {
                    if !self.software_scaler_logged {
                        rust_bridge_log_msg(&format!(
                            "RUST_GFX: integer 2x scaler active (flags {:#x})",
                            self.flags
                        ));
                        self.software_scaler_logged = true;
                    }
                    scale_screen(canvas, self.flags, &mut self.scratch);
                }
                (std::mem::take(&mut self.scratch), scaled * 4, factor)
            }
            None => {
                native_bytes(words, &mut self.scratch);
                (std::mem::take(&mut self.scratch), canvas.pitch(), 1)
            }
        };
        self.upload(screen, alpha, &bytes, pitch, scale, rect);
        self.scratch = bytes;
    }

    fn upload(
        &mut self,
        screen: i32,
        alpha: u8,
        pixels: &[u8],
        pitch: usize,
        scale: u32,
        rect: Option<sdl2::rect::Rect>,
    ) {
        let samples = &mut self.samples;
        let creator = self.canvas.texture_creator();
        let mut texture = match creator.create_texture_streaming(
            PixelFormatEnum::RGBX8888,
            SCREEN_WIDTH as u32 * scale,
            SCREEN_HEIGHT as u32 * scale,
        ) {
            Ok(texture) => texture,
            Err(error) => {
                screen_result(samples, screen, alpha, "create", &Err(error));
                return;
            }
        };
        let result = texture.update(None, pixels, pitch);
        screen_result(samples, screen, alpha, "upload", &result);
        if result.is_err() {
            return;
        }
        if alpha == 255 {
            texture.set_blend_mode(BlendMode::None);
        } else {
            texture.set_blend_mode(BlendMode::Blend);
            texture.set_alpha_mod(alpha);
        }
        let source = rect.map(|r| {
            sdl2::rect::Rect::new(
                r.x() * scale as i32,
                r.y() * scale as i32,
                r.width() * scale,
                r.height() * scale,
            )
        });
        let result = self.canvas.copy(&texture, source, rect);
        screen_result(samples, screen, alpha, "copy", &result);
    }

    fn color_layer(&mut self, [r, g, b, a]: [u8; 4]) {
        self.canvas.set_blend_mode(if a == 255 {
            BlendMode::None
        } else {
            BlendMode::Blend
        });
        self.canvas
            .set_draw_color(sdl2::pixels::Color::RGBA(r, g, b, a));
        let result = self.canvas.fill_rect(None);
        self.samples.push(PresentationSample::Color {
            rgba: [r, g, b, a],
            error: result.err(),
        });
    }

    fn scanlines(&mut self) -> Result<(), String> {
        let canvas = &mut self.canvas;
        canvas.set_draw_color(sdl2::pixels::Color::RGBA(0, 0, 0, 64));
        canvas.set_blend_mode(BlendMode::Blend);
        let (width, height) = (SCREEN_WIDTH as u32 * 2, SCREEN_HEIGHT as u32 * 2);
        canvas
            .set_logical_size(width, height)
            .map_err(|e| e.to_string())?;
        for y in (0..height).step_by(2) {
            canvas.draw_line(
                sdl2::rect::Point::new(0, y as i32),
                sdl2::rect::Point::new((width - 1) as i32, y as i32),
            )?;
        }
        canvas
            .set_logical_size(SCREEN_WIDTH as u32, SCREEN_HEIGHT as u32)
            .map_err(|e| e.to_string())
    }

    fn readback(&self, generation: u64) -> Result<PresentedFrame, String> {
        let (width, height) = self.canvas.output_size()?;
        let mut rgba = self.canvas.read_pixels(None, PixelFormatEnum::RGBA32)?;
        for pixel in rgba.as_chunks_mut::<4>().0 {
            pixel[3] = u8::MAX;
        }
        Ok(PresentedFrame {
            count: self.presentation_count,
            generation,
            width,
            height,
            rgba,
        })
    }

    #[cfg(feature = "debug-process")]
    fn sync_native_display_geometry(&mut self) -> Result<(), String> {
        let Some((x, y)) = self.native_position else {
            return Ok(());
        };
        let display = self.canvas.window().display_index()?;
        let bounds = self.video.display_bounds(display)?;
        let display_changed = self
            .native_display_bounds
            .is_some_and(|known| known != bounds);
        self.native_display_bounds = Some(bounds);
        let position = self.canvas.window().position();
        #[cfg(target_os = "macos")]
        let native = macos::position(self.canvas.window())?;
        #[cfg(not(target_os = "macos"))]
        let native = position;
        if display_changed || position != (x, y) || native != (x, y) {
            // Cocoa can rebase its bottom-left frame before SDL updates its
            // cached top-left position, even after the display-change
            // placement ran.
            rust_bridge_log_msg(&format!(
                "NATIVE_PLACEMENT: display={bounds:?} changed={display_changed} client={position:?} native={native:?} requested=({x}, {y})"
            ));
            self.samples.push(PresentationSample::Display {
                event: format!(
                    "placement:display={bounds:?}:changed={display_changed}:client={position:?}:native={native:?}:requested=({x},{y})"
                ),
            });
            self.canvas.window_mut().set_position(
                sdl2::video::WindowPos::Positioned(x),
                sdl2::video::WindowPos::Positioned(y),
            );
        }
        Ok(())
    }
}

impl Display for SdlDisplay {
    fn present(
        &mut self,
        frame: &PresentFrame<'_>,
        capture: &mut dyn FnMut(u64, &[PresentationSample]) -> Option<u64>,
    ) -> Option<PresentedFrame> {
        self.samples.push(PresentationSample::Preprocess {
            force_redraw: frame.force_redraw,
            transition_amount: frame.transition,
            fade_amount: frame.fade,
        });
        self.canvas.set_blend_mode(BlendMode::None);
        self.canvas.set_draw_color(sdl2::pixels::Color::BLACK);
        self.canvas.clear();
        for layer in &frame.layers {
            match layer {
                Layer::Screen {
                    screen,
                    canvas,
                    alpha,
                    rect,
                } => {
                    let rect = rect.map(|r| {
                        sdl2::rect::Rect::new(
                            i32::from(r.corner.x),
                            i32::from(r.corner.y),
                            r.extent.width.max(0) as u32,
                            r.extent.height.max(0) as u32,
                        )
                    });
                    self.screen_layer(*screen, canvas, *alpha, rect);
                }
                Layer::Color { rgba } => self.color_layer(*rgba),
            }
        }
        #[cfg(feature = "debug-process")]
        if let Err(error) = self.sync_native_display_geometry() {
            rust_bridge_log_msg(&format!(
                "RUST_GFX_POSTPROCESS: display placement failed: {error}"
            ));
            std::process::exit(1);
        }
        if self.flags & GFXFLAGS_SCANLINES != 0 {
            if let Err(error) = self.scanlines() {
                rust_bridge_log_msg(&format!("RUST_GFX_POSTPROCESS: scanlines failed: {error}"));
                std::process::exit(1);
            }
        }
        self.presentation_count = self.presentation_count.saturating_add(1);
        let presented = capture(self.presentation_count, &self.samples).and_then(|generation| {
            self.readback(generation)
                .map_err(|error| {
                    rust_bridge_log_msg(&format!(
                        "RUST_GFX_POSTPROCESS: presented-frame readback failed: {error}"
                    ));
                })
                .ok()
        });
        self.samples.clear();
        self.canvas.present();
        presented
    }

    fn present_video(&mut self, pixels: &[u32], width: u32, height: u32) -> Result<(), String> {
        let creator = self.canvas.texture_creator();
        let mut texture = creator
            .create_texture_streaming(PixelFormatEnum::RGBX8888, width, height)
            .map_err(|e| format!("Failed to create video texture: {e}"))?;
        let bytes: Vec<u8> = pixels
            .iter()
            .flat_map(|&p| [0, (p >> 16) as u8, (p >> 8) as u8, p as u8])
            .collect();
        texture
            .update(None, &bytes, width as usize * 4)
            .map_err(|e| format!("Failed to update video texture: {e}"))?;
        let (window_width, window_height) = self.size;
        let (old_width, old_height) = self.canvas.logical_size();
        self.canvas
            .set_logical_size(window_width, window_height)
            .map_err(|e| {
                format!("Failed to set logical size {window_width}x{window_height}: {e}")
            })?;
        let scale = (window_width as f32 / width as f32).min(window_height as f32 / height as f32);
        let (scaled_width, scaled_height) = (
            (width as f32 * scale) as u32,
            (height as f32 * scale) as u32,
        );
        let (x, y) = (
            (window_width - scaled_width) / 2,
            (window_height - scaled_height) / 2,
        );
        self.canvas.set_draw_color(sdl2::pixels::Color::BLACK);
        self.canvas.clear();
        let copied = self.canvas.copy(
            &texture,
            None,
            Some(sdl2::rect::Rect::new(
                x as i32,
                y as i32,
                scaled_width,
                scaled_height,
            )),
        );
        if copied.is_ok() {
            self.canvas.present();
        }
        let _ = self.canvas.set_logical_size(old_width, old_height);
        copied.map_err(|e| format!("Failed to copy video texture: {e}"))?;
        rust_bridge_log_msg(&format!(
            "RUST_VIDEO: Presented frame {width}x{height} scaled to {scaled_width}x{scaled_height} at ({x},{y}) window={window_width}x{window_height}"
        ));
        Ok(())
    }

    #[cfg(feature = "debug-process")]
    fn native_window(&mut self, show: bool) -> (u32, (i32, i32, u32, u32)) {
        let window = self.canvas.window_mut();
        if show {
            window.show();
            window.raise();
        }
        let (x, y) = window.position();
        let (width, height) = window.size();
        (window.id(), (x, y, width, height))
    }

    fn record(&mut self, sample: PresentationSample) {
        self.samples.push(sample);
    }

    fn reconfigure(&mut self, flags: i32, width: i32, height: i32) -> Result<(), String> {
        let fullscreen = flags & GFXFLAGS_FULLSCREEN != 0;
        self.sdl.mouse().show_cursor(!fullscreen);
        if width <= 0 || height <= 0 {
            return Err("invalid video dimensions".into());
        }
        self.samples.push(PresentationSample::Display {
            event: format!("reconfigure:flags={flags}:width={width}:height={height}"),
        });
        let window = self.canvas.window_mut();
        window.set_fullscreen(if fullscreen {
            sdl2::video::FullscreenType::Desktop
        } else {
            sdl2::video::FullscreenType::Off
        })?;
        if !fullscreen {
            window
                .set_size(width as u32, height as u32)
                .map_err(|e| e.to_string())?;
        }
        self.flags = flags;
        self.size = (width as u32, height as u32);
        self.software_scaler_logged = false;
        self.xbrz_logged = false;
        Ok(())
    }
}

/// Write canvas words into the reused upload buffer in native byte order.
///
/// Runs on every present, so it must not allocate or go through flattening
/// iterators: unoptimised builds spent most of each replay frame there.
fn native_bytes(words: &[u32], out: &mut Vec<u8>) {
    out.resize(words.len() * 4, 0);
    for (bytes, word) in out.as_chunks_mut::<4>().0.iter_mut().zip(words) {
        *bytes = word.to_ne_bytes();
    }
}

fn log_scaler_flags(flags: i32) {
    for (bit, name) in [
        (3, "SCALE_BILINEAR"),
        (4, "SCALE_BIADAPT"),
        (5, "SCALE_BIADAPTADV"),
        (6, "SCALE_TRISCAN"),
        (7, "SCALE_HQXX"),
        (8, "SCALE_XBRZ3"),
        (9, "SCALE_XBRZ4"),
    ] {
        if flags & (1 << bit) != 0 {
            rust_bridge_log_msg(&format!("  {name} set"));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scale_factor_follows_the_soft_scaler_flags() {
        assert_eq!(scale_factor(0), None);
        assert_eq!(scale_factor(1 << 3), None);
        assert_eq!(scale_factor(1 << 4), Some(2));
        assert_eq!(scale_factor(1 << 7), Some(2));
        assert_eq!(scale_factor(XBRZ3), Some(3));
        assert_eq!(scale_factor(XBRZ4 | (1 << 5)), Some(4));
    }

    #[test]
    fn native_bytes_reuses_the_buffer_in_native_byte_order() {
        let mut out = vec![0xaa; 64];
        native_bytes(&[0x1122_3344, 0xff00_ee01], &mut out);
        let expected: Vec<u8> = [0x1122_3344u32, 0xff00_ee01]
            .iter()
            .flat_map(|word| word.to_ne_bytes())
            .collect();
        assert_eq!(out, expected);
        native_bytes(&[0x0102_0304; 3], &mut out);
        assert_eq!(out, [0x0102_0304u32.to_ne_bytes(); 3].as_flattened());
    }

    #[test]
    fn reconfiguration_keeps_the_window_and_applies_fullscreen_and_size() {
        use sdl2::video::FullscreenType;
        std::env::set_var("SDL_VIDEODRIVER", "dummy");
        let mut display = SdlDisplay::open(0, None, 640, 480).unwrap();
        let window = display.canvas.window().id();
        display.reconfigure(GFXFLAGS_FULLSCREEN, 640, 480).unwrap();
        assert_eq!(
            display.canvas.window().fullscreen_state(),
            FullscreenType::Desktop
        );
        assert_eq!(display.canvas.window().id(), window);
        display.reconfigure(0, 800, 600).unwrap();
        assert_eq!(
            display.canvas.window().fullscreen_state(),
            FullscreenType::Off
        );
        assert_eq!(display.canvas.window().size(), (800, 600));
    }
}
