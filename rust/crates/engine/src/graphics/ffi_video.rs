//! Window reconfiguration without replacing the live screen surfaces.
use super::*;

#[cfg(all(feature = "debug-process", target_os = "macos"))]
#[path = "ffi_video_macos.rs"]
mod macos;

#[cfg(feature = "debug-process")]
fn native_position_changed(cached: (i32, i32), native: (i32, i32), requested: (i32, i32)) -> bool {
    cached != requested || native != requested
}

#[cfg(feature = "debug-process")]
fn display_geometry_changed(
    previous: &mut Option<sdl2::rect::Rect>,
    current: sdl2::rect::Rect,
) -> bool {
    let changed = previous.is_some_and(|bounds| bounds != current);
    *previous = Some(current);
    changed
}

#[cfg(feature = "debug-process")]
pub(super) fn sync_native_display_geometry() -> Result<(), String> {
    let Some(state) = get_gfx_state() else {
        return Ok(());
    };
    let Some((x, y)) = state.native_position else {
        return Ok(());
    };
    let display = state.canvas.window().display_index()?;
    let bounds = state.video.display_bounds(display)?;
    let display_changed = display_geometry_changed(&mut state.native_display_bounds, bounds);
    let position = state.canvas.window().position();
    #[cfg(target_os = "macos")]
    let native = macos::position(state.canvas.window())?;
    #[cfg(not(target_os = "macos"))]
    let native = position;
    if display_changed || native_position_changed(position, native, (x, y)) {
        // Cocoa can rebase its bottom-left frame before SDL updates its cached
        // top-left position, even after the display-change placement ran.
        rust_bridge_log_msg(&format!(
            "NATIVE_PLACEMENT: display={bounds:?} changed={display_changed} client={position:?} native={native:?} requested=({}, {})",
            x, y
        ));
        state.canvas.window_mut().set_position(
            sdl2::video::WindowPos::Positioned(x),
            sdl2::video::WindowPos::Positioned(y),
        );
    }
    Ok(())
}

pub(crate) fn reconfigure_video(flags: c_int, width: c_int, height: c_int) -> Result<(), String> {
    if width <= 0 || height <= 0 {
        return Err("invalid video dimensions".into());
    }
    let state = get_gfx_state().ok_or("graphics not initialized")?;
    let fullscreen = flags & 1 != 0;
    let window = state.canvas.window_mut();
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
    state.flags = flags;
    state.width = width as u32;
    state.height = height as u32;
    state.fullscreen = fullscreen;
    let buffer_size = scale_factor_from_flags(flags)
        .map(|scale| (SCREEN_WIDTH * scale * SCREEN_HEIGHT * scale * 4) as usize);
    for buffer in &mut state.scaled_buffers {
        *buffer = buffer_size.map(|size| vec![0; size]);
    }
    state.software_scaler_logged = false;
    state.xbrz_logged = false;
    Ok(())
}

pub(super) fn readback<T: sdl2::render::RenderTarget>(
    canvas: &sdl2::render::Canvas<T>,
) -> Result<(u32, u32, Vec<u8>), String> {
    let (width, height) = canvas.output_size()?;
    let rgba = canvas.read_pixels(None, sdl2::pixels::PixelFormatEnum::RGBA32)?;
    Ok((width, height, rgba))
}

pub(super) fn presented_frame<T: sdl2::render::RenderTarget>(
    canvas: &sdl2::render::Canvas<T>,
    count: u64,
    generation: u64,
) -> Result<PresentedFrame, String> {
    let (width, height, mut rgba) = readback(canvas)?;
    for pixel in rgba.as_chunks_mut::<4>().0 {
        pixel[3] = u8::MAX;
    }
    Ok(PresentedFrame {
        count,
        generation,
        width,
        height,
        rgba,
    })
}

pub(super) fn scanlines(
    canvas: &mut sdl2::render::Canvas<sdl2::video::Window>,
) -> Result<(), String> {
    canvas.set_draw_color(sdl2::pixels::Color::RGBA(0, 0, 0, 64));
    canvas.set_blend_mode(BlendMode::Blend);
    canvas
        .set_logical_size(SCREEN_WIDTH * 2, SCREEN_HEIGHT * 2)
        .map_err(|e| e.to_string())?;
    for y in (0..SCREEN_HEIGHT * 2).step_by(2) {
        canvas.draw_line(
            sdl2::rect::Point::new(0, y as i32),
            sdl2::rect::Point::new((SCREEN_WIDTH * 2 - 1) as i32, y as i32),
        )?;
    }
    canvas
        .set_logical_size(SCREEN_WIDTH, SCREEN_HEIGHT)
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "debug-process")]
    #[test]
    fn native_placement_detects_a_cocoa_move_with_an_unchanged_sdl_position() {
        let requested = (80, 48);
        let sdl_cached = requested;
        let cocoa_position = (80, 85);
        assert!(native_position_changed(
            sdl_cached,
            cocoa_position,
            requested
        ));
        assert!(!native_position_changed(requested, requested, requested));
        assert!(native_position_changed((80, 30), requested, requested));
    }

    #[cfg(all(feature = "debug-process", target_os = "macos"))]
    #[test]
    fn cocoa_bottom_left_origin_is_rebased_against_the_live_primary_height() {
        assert_eq!(macos::top_left_y(1080.0, 72.0, 960.0), 48);
        assert_eq!(macos::top_left_y(1117.0, 72.0, 960.0), 85);
        assert_eq!(macos::top_left_y(1117.0, 109.0, 960.0), 48);
    }

    #[cfg(feature = "debug-process")]
    #[test]
    fn native_placement_updates_only_when_the_desktop_geometry_changes() {
        let original = sdl2::rect::Rect::new(0, 0, 1920, 1080);
        let resized = sdl2::rect::Rect::new(0, 0, 1728, 1117);
        let mut previous = None;
        assert!(!display_geometry_changed(&mut previous, original));
        assert!(!display_geometry_changed(&mut previous, original));
        assert!(display_geometry_changed(&mut previous, resized));
        assert!(!display_geometry_changed(&mut previous, resized));
        assert!(display_geometry_changed(&mut previous, original));
    }

    #[test]
    fn readback_reports_renderer_dimensions_instead_of_logical_dimensions() {
        let surface =
            sdl2::surface::Surface::new(7, 5, sdl2::pixels::PixelFormatEnum::RGBA32).unwrap();
        let mut canvas = surface.into_canvas().unwrap();
        canvas.set_logical_size(3, 2).unwrap();
        let (width, height, bytes) = readback(&canvas).unwrap();
        assert_eq!((width, height), (7, 5));
        assert_eq!(bytes.len(), (width * height * 4) as usize);
        let frame = presented_frame(&canvas, 7, 19).unwrap();
        assert_eq!((frame.count, frame.generation), (7, 19));
        assert_eq!((frame.width, frame.height), (7, 5));
        assert_eq!(frame.validate(), Ok(()));
        assert!(frame
            .rgba
            .as_chunks::<4>()
            .0
            .iter()
            .all(|pixel| pixel[3] == 255));
    }

    #[test]
    fn rejects_invalid_dimensions_before_accessing_sdl_state() {
        assert_eq!(
            reconfigure_video(0, 0, 480),
            Err("invalid video dimensions".into())
        );
        assert_eq!(
            reconfigure_video(1, 640, -1),
            Err("invalid video dimensions".into())
        );
    }
}
