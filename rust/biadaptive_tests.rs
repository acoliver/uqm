// Tests to add to the scaling.rs test module

    // ==============================================================================
    // Biadaptive Tests
    // ==============================================================================

    #[test]
    fn test_biadaptive_scaler_creation() {
        let scaler = BiadaptiveScaler::new();
        assert!(scaler.supports(ScaleMode::Biadaptive));
        assert!(!scaler.supports(ScaleMode::Nearest));
        assert!(!scaler.supports(ScaleMode::Bilinear));
    }

    #[test]
    fn test_biadaptive_scaling_dimensions() {
        let src = create_test_pixmap(8, 10, 10);
        let scaler = BiadaptiveScaler::new();
        let params = ScaleParams::new(512, ScaleMode::Biadaptive); // 2x scale

        let result = scaler.scale(&src, params);
        assert!(result.is_ok());

        let dst = result.unwrap();
        assert_eq!(dst.width(), 20); // 10 * 2
        assert_eq!(dst.height(), 20); // 10 * 2
        assert_eq!(dst.format(), PixmapFormat::Rgba32);
    }

    #[test]
    fn test_biadaptive_scaling_3x() {
        let src = create_test_pixmap(9, 10, 10);
        let scaler = BiadaptiveScaler::new();
        let params = ScaleParams::new(768, ScaleMode::Biadaptive); // 3x scale

        let result = scaler.scale(&src, params);
        assert!(result.is_ok());

        let dst = result.unwrap();
        assert_eq!(dst.width(), 30); // 10 * 3
        assert_eq!(dst.height(), 30); // 10 * 3
    }

    #[test]
    fn test_biadaptive_scaling_15x() {
        // Test arbitrary non-integer scale factor (1.5x)
        let src = create_test_pixmap(10, 10, 10);
        let scaler = BiadaptiveScaler::new();
        let params = ScaleParams::new(384, ScaleMode::Biadaptive); // 1.5x scale

        let result = scaler.scale(&src, params);
        assert!(result.is_ok());

        let dst = result.unwrap();
        assert_eq!(dst.width(), 15); // 10 * 1.5 = 15
        assert_eq!(dst.height(), 15); // 10 * 1.5 = 15
    }

    #[test]
    fn test_biadaptive_scaling_downscale() {
        let src = create_test_pixmap(11, 100, 100);
        let scaler = BiadaptiveScaler::new();
        let params = ScaleParams::new(128, ScaleMode::Biadaptive); // 0.5x scale

        let result = scaler.scale(&src, params);
        assert!(result.is_ok());

        let dst = result.unwrap();
        assert_eq!(dst.width(), 50); // 100 * 0.5 = 50
        assert_eq!(dst.height(), 50); // 100 * 0.5 = 50
    }

    #[test]
    fn test_biadaptive_unsupported_mode() {
        let src = create_test_pixmap(12, 10, 10);
        let scaler = BiadaptiveScaler::new();
        let params = ScaleParams::new(512, ScaleMode::Nearest);

        let result = scaler.scale(&src, params);
        assert!(result.is_err());

        let error = result.unwrap_err().to_string();
        assert!(error.contains("Unsupported mode"));
    }

    #[test]
    fn test_biadaptive_rgb_to_luminance() {
        // Test luminance calculation with ITU-R BT.709 coefficients
        let y = BiadaptiveScaler::rgb_to_luminance(255, 255, 255);
        assert!((y - 255.0).abs() < 0.01, "White should have max luminance");

        let y = BiadaptiveScaler::rgb_to_luminance(0, 0, 0);
        assert!((y - 0.0).abs() < 0.01, "Black should have min luminance");

        // Green has highest luminance component
        let y = BiadaptiveScaler::rgb_to_luminance(0, 255, 0);
        assert!(y > 128.0, "Pure green should have high luminance");

        // Blue has lowest luminance component
        let y = BiadaptiveScaler::rgb_to_luminance(0, 0, 255);
        assert!(y < 50.0, "Pure blue should have low luminance");
    }

    #[test]
    fn test_biadaptive_edge_detection() {
        // Create a test image with a sharp edge
        let id = NonZeroU32::new(100).unwrap();
        let mut src = Pixmap::new(id, 4, 4, PixmapFormat::Rgba32).unwrap();
        let data = src.data_mut();

        // Left half black, right half white
        for y in 0..4 {
            for x in 0..4 {
                let idx = (y * 4 + x) * 4;
                if x < 2 {
                    data[idx] = 0;     // R
                    data[idx + 1] = 0; // G
                    data[idx + 2] = 0; // B
                    data[idx + 3] = 255; // A
                } else {
                    data[idx] = 255;   // R
                    data[idx + 1] = 255; // G
                    data[idx + 2] = 255; // B
                    data[idx + 3] = 255; // A
                }
            }
        }

        // Check that edge detection finds higher gradients at the boundary
        let src_data = src.data();
        let grad_left = BiadaptiveScaler::compute_gradient(src_data, 0, 1, 4, 4);
        let grad_edge = BiadaptiveScaler::compute_gradient(src_data, 1, 1, 4, 4);
        let grad_right = BiadaptiveScaler::compute_gradient(src_data, 2, 1, 4, 4);

        // The edge should have higher gradient than smooth areas
        assert!(grad_edge > grad_left, "Edge should have higher gradient");
        assert!(grad_edge > grad_right, "Edge should have higher gradient");
        assert!(grad_edge > 50.0, "Edge gradient should be significant");
    }

    #[test]
    fn test_biadaptive_smooth_area() {
        // Create a uniform image (should have low gradients)
        let id = NonZeroU32::new(101).unwrap();
        let mut src = Pixmap::new(id, 4, 4, PixmapFormat::Rgba32).unwrap();
        let data = src.data_mut();

        let color = [128, 128, 128, 255];
        for i in 0..16 {
            data[i * 4] = color[0];
            data[i * 4 + 1] = color[1];
            data[i * 4 + 2] = color[2];
            data[i * 4 + 3] = color[3];
        }

        // Check gradients are low in smooth areas
        let src_data = src.data();
        let grad = BiadaptiveScaler::compute_gradient(src_data, 1, 1, 4, 4);

        assert!(grad < 1.0, "Smooth area should have negligible gradient");
    }

    #[test]
    fn test_biadaptive_gradient_diagonal_edge() {
        // Create an image with a diagonal edge
        let id = NonZeroU32::new(102).unwrap();
        let mut src = Pixmap::new(id, 3, 3, PixmapFormat::Rgba32).unwrap();
        let data = src.data_mut();

        // Top-left to bottom-right diagonal: top-left = white, rest = black
        // W B B
        // B W B
        // B B B
        let white = [255, 255, 255, 255];
        let black = [0, 0, 0, 255];

        data[0] = white[0]; data[1] = white[1]; data[2] = white[2]; data[3] = white[3];
        for i in 1..9 {
            data[i * 4] = black[0];
            data[i * 4 + 1] = black[1];
            data[i * 4 + 2] = black[2];
            data[i * 4 + 3] = black[3];
        }
        data[4 * 4] = white[0]; data[4 * 4 + 1] = white[1]; data[4 * 4 + 2] = white[2]; data[4 * 4 + 3] = white[3];

        // Check diagonal edge detection
        let src_data = src.data();
        let grad_corner = BiadaptiveScaler::compute_gradient(src_data, 0, 0, 3, 3);
        let grad_center = BiadaptiveScaler::compute_gradient(src_data, 1, 1, 3, 3);

        // Both positions should have significant gradients
        assert!(grad_corner > 50.0, "Corner should have significant gradient");
        assert!(grad_center > 50.0, "Center should have significant gradient");
    }

    #[test]
    fn test_biadaptive_blending_behavior() {
        // Create a test image with edge and smooth regions
        let id = NonZeroU32::new(103).unwrap();
        let mut src = Pixmap::new(id, 3, 3, PixmapFormat::Rgba32).unwrap();
        let data = src.data_mut();

        // Top edge, smooth bottom
        //  R R R
        //  G G G
        //  G G G
        let red = [255, 0, 0, 255];
        let green = [0, 255, 0, 255];

        for x in 0..3 {
            for y in 0..3 {
                let idx = (y * 3 + x) * 4;
                if y == 0 {
                    data[idx] = red[0];
                    data[idx + 1] = red[1];
                    data[idx + 2] = red[2];
                    data[idx + 3] = red[3];
                } else {
                    data[idx] = green[0];
                    data[idx + 1] = green[1];
                    data[idx + 2] = green[2];
                    data[idx + 3] = green[3];
                }
            }
        }

        let src_data = src.data();
        let grad_edge = BiadaptiveScaler::compute_gradient(src_data, 1, 0, 3, 3);
        let grad_smooth = BiadaptiveScaler::compute_gradient(src_data, 1, 1, 3, 3);

        // Edge should have higher gradient than smooth area
        assert!(grad_edge > grad_smooth, "Edge gradient > smooth gradient");
        assert!(grad_smooth < 10.0, "Smooth area should have low gradient");
    }

    #[test]
    fn test_biadaptive_with_scaler_manager() {
        let manager = ScalerManager::new();
        let src = create_test_pixmap(13, 10, 10);
        let params = ScaleParams::new(512, ScaleMode::Biadaptive);

        let result = manager.scale(&src, params);
        assert!(result.is_ok());

        let dst = result.unwrap();
        assert_eq!(dst.width(), 20);
        assert_eq!(dst.height(), 20);

        // Second call should hit the cache
        let result = manager.scale(&src, params);
        assert!(result.is_ok());

        let (hits, misses, size) = manager.cache_stats();
        assert_eq!(hits, 1);
        assert_eq!(misses, 1);
        assert_eq!(size, 1);
    }

    #[test]
    fn test_biadaptive_default_trait() {
        let scaler = BiadaptiveScaler::default();
        assert!(scaler.supports(ScaleMode::Biadaptive));
    }

    #[test]
    fn test_scale_mode_biadaptive_value() {
        assert_eq!(ScaleMode::Biadaptive as u8, 5);
    }

    #[test]
    fn test_scale_mode_biadaptive_properties() {
        // Biadaptive is a software scaler
        assert!(ScaleMode::Biadaptive.is_software());
        assert!(!ScaleMode::Biadaptive.is_hardware());
    }

    #[test]
    fn test_biadaptive_bilinear_sample() {
        // Create a simple 2x2 test image
        let data: Vec<u8> = vec![
            0, 0, 0, 255,    // Top-left: black
            255, 255, 255, 255, // Top-right: white
            128, 128, 128, 255, // Bottom-left: gray
            64, 64, 64, 255,     // Bottom-right: dark gray
        ];

        // Sample at exact corner should give that pixel
        let p00 = BiadaptiveScaler::bilinear_sample(&data, 0.0, 0.0, 2, 2);
        assert_eq!(p00, [0, 0, 0, 255]);

        let p01 = BiadaptiveScaler::bilinear_sample(&data, 0.0, 1.0, 2, 2);
        assert_eq!(p01, [128, 128, 128, 255]);

        let p10 = BiadaptiveScaler::bilinear_sample(&data, 1.0, 0.0, 2, 2);
        assert_eq!(p10, [255, 255, 255, 255]);

        // Sample at center should be blend of all four
        let pc = BiadaptiveScaler::bilinear_sample(&data, 0.5, 0.5, 2, 2);
        assert!(pc[0] > 64 && pc[0] < 192); // Intermediate brightness
    }

    #[test]
    fn test_biadaptive_nearest_sample() {
        // Create a 3x3 test image
        let mut data = vec![0u8; 9 * 4];
        for y in 0..3 {
            for x in 0..3 {
                let idx = (y * 3 + x) * 4;
                data[idx] = (y * 3 + x) as u8; // Unique color value
                data[idx + 1] = ((y * 3 + x) >> 8) as u8;
                data[idx + 2] = ((y * 3 + x) >> 16) as u8;
                data[idx + 3] = 255;
            }
        }

        // Nearest sample should round to nearest pixel
        let p0 = BiadaptiveScaler::nearest_sample(&data, 0.3, 0.3, 3, 3); // Should map to (0, 0)
        assert_eq!(p0[0], 0);

        let p1 = BiadaptiveScaler::nearest_sample(&data, 1.6, 1.4, 3, 3); // Should map to (2, 1)
        assert_eq!(p1[0], 5); // y=1, x=2 => index = 1*3 + 2 = 5
    }

    #[test]
    fn test_biadaptive_invalid_scale_factor() {
        let src = create_test_pixmap(14, 10, 10);
        let scaler = BiadaptiveScaler::new();
        let params = ScaleParams::new(0, ScaleMode::Biadaptive);

        let result = scaler.scale(&src, params);
        assert!(result.is_err());

        let error = result.unwrap_err().to_string();
        assert!(error.contains("Invalid scale factor"));
    }

    #[test]
    fn test_biadaptive_uniform_image_preserved() {
        // Test that a uniform image scales reasonably
        let id = NonZeroU32::new(104).unwrap();
        let mut src = Pixmap::new(id, 4, 4, PixmapFormat::Rgba32).unwrap();
        let data = src.data_mut();

        let color = [100, 150, 200, 255];
        for i in 0..16 {
            data[i * 4] = color[0];
            data[i * 4 + 1] = color[1];
            data[i * 4 + 2] = color[2];
            data[i * 4 + 3] = color[3];
        }

        let scaler = BiadaptiveScaler::new();
        let params = ScaleParams::new(512, ScaleMode::Biadaptive);

        let result = scaler.scale(&src, params);
        assert!(result.is_ok());

        let dst = result.unwrap();
        assert_eq!(dst.width(), 8);
        assert_eq!(dst.height(), 8);

        // All pixels should be close to the original color
        // (may have small variations due to edge detection, but should be minimal)
        let dst_data = dst.data();
        for i in 0..dst_data.len() / 4 {
            let r = dst_data[i * 4];
            let g = dst_data[i * 4 + 1];
            let b = dst_data[i * 4 + 2];
            assert!((r as i32 - color[0] as i32).abs() <= 5, "Red channel variation too large");
            assert!((g as i32 - color[1] as i32).abs() <= 5, "Green channel variation too large");
            assert!((b as i32 - color[2] as i32).abs() <= 5, "Blue channel variation too large");
        }
    }

    #[test]
    fn test_biadaptive_format_mismatch() {
        // Biadaptive only supports RGBA format
        let id = NonZeroU32::new(105).unwrap();
        let src = Pixmap::new(id, 2, 2, PixmapFormat::Rgb24).unwrap();

        let scaler = BiadaptiveScaler::new();
        let params = ScaleParams::new(512, ScaleMode::Biadaptive);

        let result = scaler.scale(&src, params);
        assert!(result.is_err());

        let error = result.unwrap_err().to_string();
        assert!(error.contains("Format mismatch"));
    }
