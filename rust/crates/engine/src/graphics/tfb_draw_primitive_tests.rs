//! Content checks for the canvas conversion helpers, the `Canvas` and
//! `TFImage` primitive traits, image flags and the error types.

use super::*;
use crate::graphics::init_global_state;

fn rect(x: i32, y: i32, width: i32, height: i32) -> Rect {
    Rect::from_parts(Point::new(x, y), Extent::new(width, height))
}

fn rgba_at(canvas: &Canvas, x: i32, y: i32) -> [u8; 4] {
    let at = (y * canvas.width() + x) as usize * 4;
    canvas.pixels()[at..at + 4].try_into().unwrap()
}

const CLEAR: [u8; 4] = [0, 0, 0, 0];

// ---------------------------------------------------------------------------
// convert_canvas_format
// ---------------------------------------------------------------------------

#[test]
fn rgb_converts_to_rgba_with_an_opaque_alpha_channel() {
    let mut rgb = Canvas::new_rgb(2, 1);
    rgb.with_pixels_mut(|pixels| pixels.copy_from_slice(&[10, 20, 30, 40, 50, 60]))
        .unwrap();

    let rgba = convert_canvas_format(&rgb, CanvasFormat::rgba(), None).unwrap();

    assert_eq!(rgba.format(), CanvasFormat::rgba());
    assert_eq!(rgba.pixels(), vec![10, 20, 30, 255, 40, 50, 60, 255]);
}

#[test]
fn rgba_converts_to_rgb_by_dropping_alpha() {
    let mut rgba = Canvas::new_rgba(2, 1);
    rgba.with_pixels_mut(|pixels| pixels.copy_from_slice(&[1, 2, 3, 128, 4, 5, 6, 0]))
        .unwrap();

    let rgb = convert_canvas_format(&rgba, CanvasFormat::rgb(), None).unwrap();

    assert_eq!(rgb.format(), CanvasFormat::rgb());
    assert_eq!(rgb.pixels(), vec![1, 2, 3, 4, 5, 6]);
}

#[test]
fn converting_to_the_same_format_keeps_every_pixel() {
    let mut rgba = Canvas::new_rgba(2, 2);
    fill_rect(&mut rgba, 0, 0, 1, 1, Color::new(9, 8, 7, 200)).unwrap();

    let same = convert_canvas_format(&rgba, CanvasFormat::rgba(), None).unwrap();

    assert_eq!(same.pixels(), rgba.pixels());
}

#[test]
fn paletted_converts_to_rgba_through_its_palette() {
    let mut palette = [Color::new(0, 0, 0, 255); 256];
    palette[1] = Color::new(200, 100, 50, 255);
    palette[2] = Color::new(1, 2, 3, 255);
    let mut paletted = Canvas::new_paletted(3, 1, palette);
    paletted
        .with_pixels_mut(|pixels| pixels.copy_from_slice(&[2, 1, 2]))
        .unwrap();

    let rgba = convert_canvas_format(&paletted, CanvasFormat::rgba(), None).unwrap();

    assert_eq!(rgba_at(&rgba, 0, 0), [1, 2, 3, 255]);
    assert_eq!(rgba_at(&rgba, 1, 0), [200, 100, 50, 255]);
    assert_eq!(rgba_at(&rgba, 2, 0), [1, 2, 3, 255]);
}

#[test]
fn rgba_converts_to_paletted_by_nearest_palette_entry() {
    let mut palette = [Color::new(0, 0, 0, 255); 256];
    palette[7] = Color::new(250, 0, 0, 255);
    palette[9] = Color::new(0, 0, 250, 255);
    let mut rgba = Canvas::new_rgba(2, 1);
    rgba.with_pixels_mut(|pixels| pixels.copy_from_slice(&[240, 10, 5, 255, 5, 5, 240, 255]))
        .unwrap();

    let paletted = convert_canvas_format(&rgba, CanvasFormat::paletted(), Some(palette)).unwrap();

    assert_eq!(paletted.pixels(), vec![7, 9]);
}

// ---------------------------------------------------------------------------
// Flags and error types
// ---------------------------------------------------------------------------

#[test]
fn image_flags_report_flips_colormap_and_the_first_matching_rotation() {
    let none = ImageFlags::new(ImageFlags::NONE);
    assert!(!none.flip_horizontal() && !none.flip_vertical() && !none.use_colormap());
    assert_eq!(none.rotation(), Rotation::None);

    let flipped = ImageFlags::new(ImageFlags::FLIP_HORIZONTAL | ImageFlags::USE_COLORMAP);
    assert!(flipped.flip_horizontal());
    assert!(!flipped.flip_vertical());
    assert!(flipped.use_colormap());
    assert_eq!(
        flipped.bits(),
        ImageFlags::FLIP_HORIZONTAL | ImageFlags::USE_COLORMAP
    );

    assert!(ImageFlags::new(ImageFlags::FLIP_VERTICAL).flip_vertical());
    assert_eq!(
        ImageFlags::new(ImageFlags::ROTATE_90).rotation(),
        Rotation::Degrees90
    );
    assert_eq!(
        ImageFlags::new(ImageFlags::ROTATE_180).rotation(),
        Rotation::Degrees180
    );
    assert_eq!(
        ImageFlags::new(ImageFlags::ROTATE_270).rotation(),
        Rotation::Degrees270
    );
    assert_eq!(
        ImageFlags::new(ImageFlags::ROTATE_270 | ImageFlags::ROTATE_90).rotation(),
        Rotation::Degrees90,
        "the smallest requested rotation wins"
    );
}

#[test]
fn canvas_errors_describe_themselves() {
    assert_eq!(
        CanvasError::AlreadyLocked.to_string(),
        "Canvas is already locked"
    );
    assert_eq!(CanvasError::NotLocked.to_string(), "Canvas is not locked");
    assert_eq!(
        CanvasError::InvalidRect.to_string(),
        "Invalid rectangle specification"
    );
    assert_eq!(
        CanvasError::InvalidPoint.to_string(),
        "Invalid point specification"
    );
    assert_eq!(
        CanvasError::InvalidOperation("no".into()).to_string(),
        "Invalid operation: no"
    );
    assert_eq!(CanvasError::FormatMismatch.to_string(), "Format mismatch");
}

#[test]
fn image_errors_describe_themselves_and_wrap_canvas_errors() {
    assert_eq!(
        TFImageError::NoPrimaryCanvas.to_string(),
        "TFImage has no primary canvas"
    );
    assert_eq!(
        TFImageError::InvalidMipmap.to_string(),
        "Mipmap invalid: colormap restriction violated"
    );
    assert_eq!(
        TFImageError::InvalidPaletteConversion.to_string(),
        "Palette conversion requires paletted canvas"
    );
    let wrapped = TFImageError::from(CanvasError::InvalidRect);
    assert_eq!(wrapped, TFImageError::Canvas(CanvasError::InvalidRect));
    assert_eq!(
        wrapped.to_string(),
        "Canvas error: Invalid rectangle specification"
    );
}

#[test]
fn image_debug_output_reports_extent_hot_spot_and_dirty_state() {
    let image = TFImage::new_rgba(3, 2);
    image.set_normal_hot_spot(HotSpot::new(1, 1));
    image.mark_dirty();

    let text = format!("{image:?}");

    assert!(text.contains("TFImage"), "{text}");
    assert!(text.contains("dirty: true"), "{text}");
    assert!(text.contains("has_scaled: false"), "{text}");
    assert!(text.contains("has_filled: false"), "{text}");
    assert!(text.contains("x: 1"), "{text}");
}

// ---------------------------------------------------------------------------
// CanvasPrimitive
// ---------------------------------------------------------------------------

#[test]
fn canvas_line_sets_exactly_the_diagonal_pixels() {
    let canvas = Canvas::new_rgba(5, 5);
    let color = Color::new(1, 2, 3, 255);

    CanvasPrimitive::draw_line(&canvas, 0, 0, 4, 4, color, DrawMode::Normal).unwrap();

    for y in 0..5 {
        for x in 0..5 {
            let expected = if x == y { [1, 2, 3, 255] } else { CLEAR };
            assert_eq!(rgba_at(&canvas, x, y), expected, "({x}, {y})");
        }
    }
}

#[test]
fn canvas_line_is_clipped_to_the_canvas() {
    let canvas = Canvas::new_rgba(4, 4);
    let color = Color::new(9, 9, 9, 255);

    CanvasPrimitive::draw_line(&canvas, -3, 2, 6, 2, color, DrawMode::Normal).unwrap();

    for x in 0..4 {
        assert_eq!(rgba_at(&canvas, x, 2), [9, 9, 9, 255], "x={x}");
    }
    for (x, y) in [(0, 1), (3, 3), (0, 0)] {
        assert_eq!(rgba_at(&canvas, x, y), CLEAR, "({x}, {y})");
    }
}

#[test]
fn canvas_rect_draws_an_outline_with_a_hollow_middle() {
    let canvas = Canvas::new_rgba(6, 6);
    let color = Color::new(255, 0, 0, 255);

    CanvasPrimitive::draw_rect(&canvas, rect(1, 1, 4, 4), color, DrawMode::Normal).unwrap();

    for (x, y) in [
        (1, 1),
        (4, 1),
        (1, 4),
        (4, 4),
        (2, 1),
        (1, 3),
        (4, 2),
        (3, 4),
    ] {
        assert_eq!(rgba_at(&canvas, x, y), [255, 0, 0, 255], "({x}, {y})");
    }
    for (x, y) in [(2, 2), (3, 3), (0, 0), (5, 5)] {
        assert_eq!(rgba_at(&canvas, x, y), CLEAR, "({x}, {y})");
    }
}

fn two_tone_sprite() -> TFImage {
    let mut sprite = Canvas::new_rgba(2, 2);
    fill_rect(&mut sprite, 0, 0, 0, 1, Color::new(255, 0, 0, 255)).unwrap();
    fill_rect(&mut sprite, 1, 0, 1, 1, Color::new(0, 255, 0, 255)).unwrap();
    TFImage::new(sprite)
}

#[test]
fn canvas_draws_an_image_at_its_hot_spot_offset() {
    let canvas = Canvas::new_rgba(6, 6);
    let image = two_tone_sprite();
    image.set_normal_hot_spot(HotSpot::new(1, 1));

    CanvasPrimitive::draw_image(
        &canvas,
        &image,
        3,
        3,
        0,
        ScaleMode::Nearest,
        DrawMode::Normal,
        0,
    )
    .unwrap();

    assert_eq!(rgba_at(&canvas, 2, 2), [255, 0, 0, 255]);
    assert_eq!(rgba_at(&canvas, 3, 2), [0, 255, 0, 255]);
    assert_eq!(rgba_at(&canvas, 2, 3), [255, 0, 0, 255]);
    assert_eq!(rgba_at(&canvas, 3, 3), [0, 255, 0, 255]);
    assert_eq!(rgba_at(&canvas, 4, 4), CLEAR);
    assert_eq!(rgba_at(&canvas, 1, 1), CLEAR);
}

#[test]
fn canvas_draws_a_filled_silhouette_of_an_image() {
    let canvas = Canvas::new_rgba(4, 4);
    let image = two_tone_sprite();
    let fill = Color::new(0, 0, 255, 255);

    CanvasPrimitive::draw_filled_image(
        &canvas,
        &image,
        1,
        1,
        fill,
        0,
        ScaleMode::Nearest,
        DrawMode::Normal,
        0,
    )
    .unwrap();

    for (x, y) in [(1, 1), (2, 1), (1, 2), (2, 2)] {
        assert_eq!(rgba_at(&canvas, x, y), [0, 0, 255, 255], "({x}, {y})");
    }
    assert_eq!(rgba_at(&canvas, 0, 0), CLEAR);
    assert_eq!(rgba_at(&canvas, 3, 3), CLEAR);
}

#[test]
fn canvas_fontchar_from_an_unregistered_page_is_refused() {
    init_global_state();
    let canvas = Canvas::new_rgba(4, 4);
    let fontchar = FontCharRef::new(0xDEAD_BEEF, 65);

    let result = CanvasPrimitive::draw_fontchar(&canvas, fontchar, None, 0, 0, DrawMode::Normal);

    match result {
        Err(CanvasError::InvalidOperation(message)) => {
            assert!(message.contains("3735928559"), "{message}");
            assert!(message.contains("not found"), "{message}");
        }
        other => panic!("expected InvalidOperation, got {other:?}"),
    }
}

#[test]
fn canvas_fontchar_paints_its_backing_image_before_looking_up_the_page() {
    init_global_state();
    let canvas = Canvas::new_rgba(4, 4);
    let backing = two_tone_sprite();

    let result = CanvasPrimitive::draw_fontchar(
        &canvas,
        FontCharRef::new(0xDEAD_BEEF, 65),
        Some(&backing),
        1,
        1,
        DrawMode::Normal,
    );

    assert!(result.is_err());
    assert_eq!(rgba_at(&canvas, 1, 1), [255, 0, 0, 255]);
    assert_eq!(rgba_at(&canvas, 2, 1), [0, 255, 0, 255]);
}

#[test]
fn canvas_trait_copy_rect_moves_pixels() {
    let source = Canvas::new_rgba(4, 4);
    fill_rect(&mut source.clone(), 0, 0, 1, 1, Color::new(7, 7, 7, 255)).unwrap();
    let destination = Canvas::new_rgba(4, 4);

    CanvasPrimitive::copy_rect(&destination, &source, rect(0, 0, 2, 2), Point::new(2, 2)).unwrap();

    assert_eq!(rgba_at(&destination, 2, 2), [7, 7, 7, 255]);
    assert_eq!(rgba_at(&destination, 3, 3), [7, 7, 7, 255]);
    assert_eq!(rgba_at(&destination, 0, 0), CLEAR);
}

// ---------------------------------------------------------------------------
// ImagePrimitive
// ---------------------------------------------------------------------------

#[test]
fn drawing_on_an_image_writes_its_canvas_and_marks_it_dirty() {
    let image = TFImage::new_rgba(5, 5);
    assert!(!image.is_dirty());

    ImagePrimitive::draw_line(
        &image,
        0,
        0,
        4,
        0,
        Color::new(5, 6, 7, 255),
        DrawMode::Normal,
    )
    .unwrap();

    assert!(image.is_dirty());
    let canvas = image.normal().unwrap();
    for x in 0..5 {
        assert_eq!(rgba_at(&canvas, x, 0), [5, 6, 7, 255], "x={x}");
    }
    assert_eq!(rgba_at(&canvas, 0, 1), CLEAR);
}

#[test]
fn image_rect_outline_and_copy_rect_mark_the_image_dirty() {
    let image = TFImage::new_rgba(6, 6);
    ImagePrimitive::draw_rect(
        &image,
        rect(0, 0, 3, 3),
        Color::new(1, 1, 1, 255),
        DrawMode::Normal,
    )
    .unwrap();
    assert!(image.is_dirty());
    image.mark_clean();

    let other = TFImage::new_rgba(6, 6);
    ImagePrimitive::copy_rect(&other, &image, rect(0, 0, 3, 3), Point::new(3, 3)).unwrap();

    assert!(other.is_dirty());
    let canvas = other.normal().unwrap();
    assert_eq!(rgba_at(&canvas, 3, 3), [1, 1, 1, 255]);
    assert_eq!(
        rgba_at(&canvas, 4, 4),
        CLEAR,
        "the hollow middle stays empty"
    );
    assert_eq!(rgba_at(&canvas, 5, 5), [1, 1, 1, 255]);
}

#[test]
fn image_draws_another_image_and_its_silhouette() {
    let target = TFImage::new_rgba(4, 4);
    let sprite = two_tone_sprite();

    ImagePrimitive::draw_image(
        &target,
        &sprite,
        0,
        0,
        0,
        ScaleMode::Nearest,
        DrawMode::Normal,
        0,
    )
    .unwrap();
    assert!(target.is_dirty());
    target.mark_clean();
    ImagePrimitive::draw_filled_image(
        &target,
        &sprite,
        2,
        2,
        Color::new(0, 0, 255, 255),
        0,
        ScaleMode::Nearest,
        DrawMode::Normal,
        0,
    )
    .unwrap();

    assert!(target.is_dirty());
    let canvas = target.normal().unwrap();
    assert_eq!(rgba_at(&canvas, 0, 0), [255, 0, 0, 255]);
    assert_eq!(rgba_at(&canvas, 1, 0), [0, 255, 0, 255]);
    assert_eq!(rgba_at(&canvas, 2, 2), [0, 0, 255, 255]);
    assert_eq!(rgba_at(&canvas, 3, 3), [0, 0, 255, 255]);
}

#[test]
fn image_fontchar_failure_surfaces_as_a_canvas_error() {
    init_global_state();
    let image = TFImage::new_rgba(4, 4);

    let result = ImagePrimitive::draw_fontchar(
        &image,
        FontCharRef::new(0xDEAD_BEEF, 1),
        None,
        0,
        0,
        DrawMode::Normal,
    );

    assert!(matches!(
        result,
        Err(TFImageError::Canvas(CanvasError::InvalidOperation(_)))
    ));
    assert!(!image.is_dirty(), "a failed draw leaves the image clean");
}
