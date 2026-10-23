use super::*;

fn words(bytes: &[u8]) -> Vec<u32> {
    bytes
        .as_chunks::<4>()
        .0
        .iter()
        .copied()
        .map(u32::from_le_bytes)
        .collect()
}

#[test]
fn c_scaler_surface_fixtures() {
    let fixture = words(include_bytes!(
        "../../../tests/fixtures/software-scalers.bin"
    ));
    let mut records = fixture.as_slice();
    let mut checked = [0; 6];
    while !records.is_empty() {
        let (header, rest) = records.split_at(14);
        let [kind, w, h, sp, dp, format] = header[..6].try_into().unwrap();
        let (input, rest) = rest.split_at((sp * h) as usize);
        let (expected, rest) = rest.split_at((dp * h * 2) as usize);
        records = rest;
        let scaler: ScaleFunc = match kind {
            0 => scale_nearest,
            1 => scale_bilinear_filter,
            2 => scale_tri_scan_filter,
            3 => scale_bi_adapt_adv_filter,
            4 => scale_bi_adapt_filter,
            5 => scale_hq_filter,
            _ => panic!("unknown fixture scaler"),
        };
        let mut input = input.to_vec();
        let mut output = vec![0xdeadbeef; expected.len()];
        let shifts = [[24, 16, 8], [16, 8, 0], [8, 16, 24], [0, 8, 16]][format as usize];
        // SDL's surface and pixel format are C aggregates. No SDL resources
        // are owned by these fixture-only descriptors.
        let mut fmt: SDL_PixelFormat = unsafe { std::mem::zeroed() };
        fmt.BytesPerPixel = 4;
        fmt.BitsPerPixel = 32;
        fmt.Rshift = shifts[0];
        fmt.Gshift = shifts[1];
        fmt.Bshift = shifts[2];
        let mut src: SDL_Surface = unsafe { std::mem::zeroed() };
        src.w = w as i32;
        src.h = h as i32;
        src.pitch = (sp * 4) as i32;
        src.format = &mut fmt;
        src.pixels = input.as_mut_ptr().cast();
        let mut dst: SDL_Surface = unsafe { std::mem::zeroed() };
        dst.w = (w * 2) as i32;
        dst.h = (h * 2) as i32;
        dst.pitch = (dp * 4) as i32;
        dst.format = &mut fmt;
        dst.pixels = output.as_mut_ptr().cast();
        let mut rect = SDL_Rect {
            x: header[6] as i32,
            y: header[7] as i32,
            w: header[8] as i32,
            h: header[9] as i32,
        };
        unsafe {
            scaler(&mut src, &mut dst, &mut rect);
        }
        assert_eq!(
            [rect.x, rect.y, rect.w, rect.h],
            header[10..14]
                .iter()
                .map(|v| *v as i32)
                .collect::<Vec<_>>()
                .as_slice()
        );
        assert_eq!(
            output, expected,
            "filter {kind}, {w}x{h}, format {format}, case {}",
            checked[kind as usize]
        );
        if header[6..10] == [0, 0, w, h] {
            let flags = [0, 1 << 3, 1 << 6, 1 << 5, 1 << 4, 1 << 7][kind as usize];
            let mut rgbx = vec![0; (w * h * 16) as usize];
            unsafe {
                scale_surface_to_rgbx(&mut src, flags, &mut rgbx);
            }
            let actual: Vec<_> = rgbx
                .as_chunks::<4>()
                .0
                .iter()
                .map(|p| u32::from_ne_bytes(*p))
                .collect();
            let expected: Vec<_> = expected
                .chunks(dp as usize)
                .flat_map(|row| row[..(w * 2) as usize].iter().copied())
                .collect();
            assert_eq!(
                actual, expected,
                "renderer filter {kind}, {w}x{h}, format {format}"
            );
        }
        checked[kind as usize] += 1;
    }
    assert_eq!(checked, [64; 6]);
}

#[test]
fn rectangle_expansion_matches_limit_rules() {
    let limits = SDL_Rect {
        x: 2,
        y: 3,
        w: 10,
        h: 12,
    };
    let mut rect = SDL_Rect {
        x: 3,
        y: 6,
        w: 2,
        h: 3,
    };
    expand_rect(&mut rect, 2, &limits);
    assert_eq!([rect.x, rect.y, rect.w, rect.h], [2, 4, 5, 7]);
}

#[test]
fn packed_blends_keep_original_rounding() {
    assert_eq!(blend_11(0x01030507, 0x03050709), 0x01030507);
    assert_eq!(bilinear([0xffffffff; 4]), [0xfefefefe; 4]);
}

#[test]
fn all_hq_patterns_and_conditional_edges_match_c() {
    let fixture = include_bytes!("../../../tests/fixtures/hq2x-rules.bin");
    assert_eq!(&fixture[..8], b"HQ2X\0\0\0\0");
    let expected = words(&fixture[8..]);
    let pixels = std::array::from_fn(|i| {
        0x10203040u32
            .wrapping_mul(i as u32)
            .wrapping_add(0x07050301)
    });
    for pattern in 0..=255u8 {
        for edge_bits in 0..16 {
            let edges = std::array::from_fn(|i| edge_bits & (1 << i) != 0);
            let offset = (pattern as usize * 16 + edge_bits) * 4;
            assert_eq!(
                hq_rules::rules(pattern, edges, &pixels),
                &expected[offset..offset + 4],
                "pattern {pattern}, edges {edge_bits}"
            );
        }
    }
}

#[test]
fn selection_preserves_flag_priority_and_default_for_every_setting() {
    let expected: [ScaleFunc; 6] = [
        scale_bilinear_filter,
        scale_bi_adapt_filter,
        scale_bi_adapt_adv_filter,
        scale_tri_scan_filter,
        scale_hq_filter,
        scale_nearest,
    ];
    // Include scanlines, fullscreen, unrelated bits, and the xBRZ flags,
    // which historically select nearest in this 2x software table.
    for flags in 0..(1 << 12) {
        let index = (3..8)
            .find(|bit| flags & (1 << bit) != 0)
            .map_or(5, |bit| bit - 3);
        assert!(
            std::ptr::fn_addr_eq(
                scale_prep_platform(flags, std::ptr::null()),
                expected[index]
            ),
            "flags {flags:#x}"
        );
    }
    Scale_Init();
    Scale_Init();
}

#[test]
fn hq_difference_thresholds_are_strict_and_channel_local() {
    let base = 0x804040;
    for (shift, threshold) in [(16, 0x40), (8, 0x12), (0, 0x0c)] {
        for sign in [-1, 1] {
            let within = (base as i32 + sign * (threshold << shift)) as u32;
            let outside = (base as i32 + sign * ((threshold + 1) << shift)) as u32;
            assert!(!diff_yuv(base, within));
            assert!(diff_yuv(base, outside));
            assert!(diff_yuv(outside, base));
        }
    }
    assert!(!diff_yuv(base, base));
}

#[test]
fn every_hq_neighborhood_matches_c_in_each_channel_layout() {
    let fixture = include_bytes!("../../../tests/fixtures/hq2x-neighborhoods.bin");
    assert_eq!(&fixture[..8], b"HQIN\0\0\0\0");
    let expected = words(&fixture[8..]);
    for (layout, shifts) in [[24, 16, 8], [16, 8, 0], [8, 16, 24], [0, 8, 16]]
        .into_iter()
        .enumerate()
    {
        let mut fmt: SDL_PixelFormat = unsafe { std::mem::zeroed() };
        fmt.Rshift = shifts[0];
        fmt.Gshift = shifts[1];
        fmt.Bshift = shifts[2];
        for pattern in 0..256 {
            let mut pixels = [0x173583ff; 9];
            for (bit, index) in [0, 1, 2, 3, 5, 6, 7, 8].into_iter().enumerate() {
                if pattern & (1 << bit) != 0 {
                    pixels[index] = 0xf5fcf1ff;
                }
            }
            let image = Image {
                pixels: &pixels,
                stride: 3,
                width: 3,
                height: 3,
            };
            let offset = (layout * 256 + pattern) * 4;
            assert_eq!(
                hq(&image, &fmt, 1, 1),
                &expected[offset..offset + 4],
                "neighborhood {pattern}, layout {layout}"
            );
        }
    }
}
