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
        let filter = match kind {
            0 => Filter::Nearest,
            1 => Filter::Bilinear,
            2 => Filter::Triscan,
            _ => continue,
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
            scale(&mut src, &mut dst, &mut rect, filter);
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
        checked[kind as usize] += 1;
    }
    assert_eq!(&checked[..3], &[64; 3]);
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
