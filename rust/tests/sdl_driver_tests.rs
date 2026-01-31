//! Integration tests for SDL2 and OpenGL drivers.
//!
//! These tests verify that the driver implementations work correctly.
//! Tests use the C-compatible types and APIs.

#[cfg(test)]
mod driver_tests {
    use uqm_rust::graphics::sdl::common::{DriverConfig, GraphicsDriver, RedrawMode};
    use uqm_rust::graphics::sdl::opengl::OpenGlDriver;
    use uqm_rust::graphics::sdl::sdl2::SdlDriver;

    #[test]
    fn test_sdl_driver_new() {
        let driver = SdlDriver::new();
        assert!(!driver.is_initialized());
        assert_eq!(driver.get_gamma(), 1.0);
        assert_eq!(driver.supports_hardware_scaling(), true);
    }

    #[test]
    fn test_sdl_driver_default() {
        let driver = SdlDriver::default();
        assert!(!driver.is_initialized());
        assert_eq!(driver.get_gamma(), 1.0);
    }

    #[test]
    fn test_sdl_driver_uninit_when_not_initialized() {
        let mut driver = SdlDriver::new();
        let result = driver.uninit();
        assert!(result.is_err());
        match result {
            Err(e) => {
                assert_eq!(format!("{}", e), "Graphics driver not initialized");
            }
            _ => panic!("Expected error"),
        }
    }

    #[test]
    fn test_sdl_driver_swap_buffers_when_not_initialized() {
        let mut driver = SdlDriver::new();
        let result = driver.swap_buffers(RedrawMode::None);
        assert!(result.is_err());
        match result {
            Err(e) => {
                assert_eq!(format!("{}", e), "Graphics driver not initialized");
            }
            _ => panic!("Expected error"),
        }
    }

    #[test]
    fn test_sdl_driver_set_gamma_when_not_initialized() {
        let mut driver = SdlDriver::new();
        let result = driver.set_gamma(1.5);
        assert!(result.is_err());
        match result {
            Err(e) => {
                assert_eq!(format!("{}", e), "Graphics driver not initialized");
            }
            _ => panic!("Expected error"),
        }
    }

    #[test]
    fn test_sdl_driver_toggle_fullscreen_when_not_initialized() {
        let mut driver = SdlDriver::new();
        let result = driver.toggle_fullscreen();
        assert!(result.is_err());
        match result {
            Err(e) => {
                assert_eq!(format!("{}", e), "Graphics driver not initialized");
            }
            _ => panic!("Expected error"),
        }
    }

    #[test]
    fn test_sdl_driver_get_dimensions_when_not_initialized() {
        let driver = SdlDriver::new();
        let dims = driver.get_dimensions();
        // Returns default dimensions even when not initialized
        assert_eq!(dims.width, 320);
        assert_eq!(dims.height, 240);
    }

    #[test]
    fn test_opengl_driver_new() {
        let driver = OpenGlDriver::new();
        assert!(!driver.is_initialized());
        assert_eq!(driver.get_gamma(), 1.0);
        assert_eq!(driver.supports_hardware_scaling(), true);
    }

    #[test]
    fn test_opengl_driver_default() {
        let driver = OpenGlDriver::default();
        assert!(!driver.is_initialized());
        assert_eq!(driver.get_gamma(), 1.0);
    }

    #[test]
    fn test_opengl_driver_set_keep_aspect_ratio() {
        let mut driver = OpenGlDriver::new();
        driver.set_keep_aspect_ratio(false);
        assert!(!driver.keep_aspect_ratio);
        driver.set_keep_aspect_ratio(true);
        assert!(driver.keep_aspect_ratio);
    }

    #[test]
    fn test_opengl_driver_uninit_when_not_initialized() {
        let mut driver = OpenGlDriver::new();
        let result = driver.uninit();
        assert!(result.is_err());
        match result {
            Err(e) => {
                assert_eq!(format!("{}", e), "Graphics driver not initialized");
            }
            _ => panic!("Expected error"),
        }
    }

    #[test]
    fn test_opengl_driver_swap_buffers_when_not_initialized() {
        let mut driver = OpenGlDriver::new();
        let result = driver.swap_buffers(RedrawMode::None);
        assert!(result.is_err());
        match result {
            Err(e) => {
                assert_eq!(format!("{}", e), "Graphics driver not initialized");
            }
            _ => panic!("Expected error"),
        }
    }

    #[test]
    fn test_opengl_driver_set_gamma_when_not_initialized() {
        let mut driver = OpenGlDriver::new();
        let result = driver.set_gamma(1.5);
        assert!(result.is_err());
        match result {
            Err(e) => {
                assert_eq!(format!("{}", e), "Graphics driver not initialized");
            }
            _ => panic!("Expected error"),
        }
    }

    #[test]
    fn test_opengl_driver_toggle_fullscreen_when_not_initialized() {
        let mut driver = OpenGlDriver::new();
        let result = driver.toggle_fullscreen();
        assert!(result.is_err());
        match result {
            Err(e) => {
                assert_eq!(format!("{}", e), "Graphics driver not initialized");
            }
            _ => panic!("Expected error"),
        }
    }

    #[test]
    fn test_driver_config_windowed() {
        let config = DriverConfig::windowed(640, 480);
        assert_eq!(config.width, 640);
        assert_eq!(config.height, 480);
        assert!(!config.fullscreen);
        assert!(!config.is_fullscreen());
    }

    #[test]
    fn test_driver_config_fullscreen() {
        let config = DriverConfig::fullscreen(800, 600);
        assert_eq!(config.width, 800);
        assert_eq!(config.height, 600);
        assert!(config.fullscreen);
        assert!(config.is_fullscreen());
    }

    #[test]
    fn test_redraw_mode_values() {
        assert_eq!(RedrawMode::None as i32, 0);
        assert_eq!(RedrawMode::Fading as i32, 1);
        assert_eq!(RedrawMode::Expose as i32, 2);
        assert_eq!(RedrawMode::Full as i32, 3);
    }

    #[test]
    fn test_redraw_mode_should_redraw() {
        assert!(!RedrawMode::None.should_redraw());
        assert!(RedrawMode::Fading.should_redraw());
        assert!(RedrawMode::Expose.should_redraw());
        assert!(RedrawMode::Full.should_redraw());
    }
}
