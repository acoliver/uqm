//! Window reconfiguration without replacing the live screen surfaces.
use super::*;
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
    let Some(config) = crate::automation::native_window::active_native_window_config() else {
        return Ok(());
    };
    let state = get_gfx_state().ok_or("native graphics not initialized")?;
    let display = state.canvas.window().display_index()?;
    let bounds = state.video.display_bounds(display)?;
    let display_changed = display_geometry_changed(&mut state.native_display_bounds, bounds);
    let position = state.canvas.window().position();
    if display_changed || position != (config.client_bounds.x, config.client_bounds.y) {
        // Cocoa may deliver its work-area adjustment after the display change.
        // Keep the actual proof window at the owned position before publishing.
        rust_bridge_log_msg(&format!(
            "NATIVE_PLACEMENT: display={bounds:?} changed={display_changed} client={position:?} requested=({}, {})",
            config.client_bounds.x, config.client_bounds.y
        ));
        state.canvas.window_mut().set_position(
            sdl2::video::WindowPos::Positioned(config.client_bounds.x),
            sdl2::video::WindowPos::Positioned(config.client_bounds.y),
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
