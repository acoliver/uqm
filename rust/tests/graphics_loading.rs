//! Real UIO -> PNG -> font and animation loading with the remaining engine linked.
#![forbid(unsafe_code)]
use std::io::Write;
use uqm_engine::graphics::drawable::{FrameKind, LoadedDrawable};
use uqm_engine::graphics::font::LoadedFont;
use uqm_engine::graphics::geometry::{Extent, HotSpot};
use uqm_engine::test_support::native_probe;

fn png(width: u32, height: u32, color: png::ColorType, pixels: &[u8]) -> Vec<u8> {
    let mut output = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut output, width, height);
        encoder.set_color(color);
        encoder.set_depth(png::BitDepth::Eight);
        encoder
            .write_header()
            .unwrap_or_else(|error| panic!("write PNG header: {error}"))
            .write_image_data(pixels)
            .unwrap_or_else(|error| panic!("write PNG data: {error}"));
    }
    output
}

fn check_font(files: &uqm_engine::io::Dir, path: &std::ffi::CStr, alpha: &[u8]) {
    let font =
        LoadedFont::load_file(files, path).unwrap_or_else(|error| panic!("font loads: {error}"));
    native_probe::with_font(font.font(), |font| {
        assert_eq!((font.leading, font.leading_width), (10, 4));
        assert_eq!(font.page_layout(), [(0, 65, 2)]);
        let a = font.glyph(65).unwrap_or_else(|| panic!("glyph A"));
        assert_eq!(
            (
                a.extent.width,
                a.extent.height,
                a.disp.width,
                a.disp.height,
                a.hot_spot.x,
                a.hot_spot.y,
                a.pitch
            ),
            (3, 8, 4, 9, 0, 7, 3)
        );
        assert_eq!(&a.data[..24], alpha);
        let b = font.glyph(66).unwrap_or_else(|| panic!("glyph B"));
        assert_eq!(
            (
                b.extent.width,
                b.extent.height,
                b.disp.width,
                b.disp.height,
                b.hot_spot.y,
                b.pitch
            ),
            (3, 2, 4, 3, 2, 3)
        );
        assert_eq!(&b.data[..6], [0, 255, 18, 72, 145, 36]);
    });
}

fn check_animation(files: &uqm_engine::io::Dir, path: &std::ffi::CStr) {
    let drawable = LoadedDrawable::load_file(files, path)
        .unwrap_or_else(|error| panic!("animation loads: {error}"));
    let first = drawable.first_frame();
    assert_eq!(first.count(), 2);
    for i in 0..2u16 {
        let frame = first.at(i);
        let probe = native_probe::probe_frame(frame);
        assert_eq!(
            (probe.kind, probe.index, probe.bounds),
            (Some(FrameKind::Rom), i, Extent::new(3, 2))
        );
        assert_eq!(
            probe.hot_spot,
            if i == 0 {
                HotSpot::new(-5, 11)
            } else {
                HotSpot::new(0, 0)
            }
        );
        assert_eq!(probe.colormap_index, if i == 0 { 17 } else { 0 });
        assert_eq!(probe.key, (i == 0).then_some(1));
        for y in 0..2 {
            assert_eq!(native_probe::frame_index_row(frame, y, 3), [0, 1, 2]);
        }
    }
}

#[test]
fn native_loaders_preserve_directory_zip_glyphs_and_animation_metadata() {
    let tmp_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("tmp");
    std::fs::create_dir_all(&tmp_root).unwrap();
    let temp = tempfile::tempdir_in(tmp_root).unwrap();
    let font_dir = temp.path().join("font");
    std::fs::create_dir(&font_dir).unwrap();
    let alpha: Vec<u8> = (0..8)
        .flat_map(|y| (0..3).map(move |x| (x * 71 + y * 17) as u8))
        .collect();
    let rgba: Vec<u8> = alpha.iter().flat_map(|&a| [31, 67, 99, a]).collect();
    let a = png(3, 8, png::ColorType::Rgba, &rgba);
    let b = png(
        3,
        2,
        png::ColorType::Rgb,
        &[
            0, 0, 0, 255, 255, 255, 10, 20, 30, 255, 0, 0, 0, 255, 0, 0, 0, 255,
        ],
    );
    std::fs::write(font_dir.join("0041.png"), &a).unwrap();
    std::fs::write(font_dir.join("0042.png"), &b).unwrap();
    std::fs::write(font_dir.join("10000.png"), &a).unwrap();
    let mut zip = zip::ZipWriter::new(std::fs::File::create(temp.path().join("font.zip")).unwrap());
    for (name, bytes) in [("0041.png", &a), ("0042.png", &b)] {
        zip.start_file(name, zip::write::SimpleFileOptions::default())
            .unwrap();
        zip.write_all(bytes).unwrap();
    }
    zip.finish().unwrap();
    let mut indexed = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut indexed, 3, 2);
        encoder.set_color(png::ColorType::Indexed);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.set_palette(&[0, 0, 0, 10, 20, 30, 200, 100, 50][..]);
        encoder.set_trns(&[255, 0, 255][..]);
        encoder
            .write_header()
            .unwrap()
            .write_image_data(&[0, 1, 2, 0, 1, 2])
            .unwrap();
    }
    let ani = b"sprite.png -2 17 -5 11
sprite.png -1 0 0 0
";
    std::fs::write(temp.path().join("sprite.png"), &indexed).unwrap();
    std::fs::write(temp.path().join("frames.ani"), ani).unwrap();
    std::fs::create_dir(temp.path().join("animation")).unwrap();
    let mut archive = zip::ZipWriter::new(
        std::fs::File::create(temp.path().join("animation/packed.ani")).unwrap(),
    );
    for (name, bytes) in [("packed.ani", &ani[..]), ("sprite.png", &indexed[..])] {
        archive
            .start_file(name, zip::write::SimpleFileOptions::default())
            .unwrap();
        archive.write_all(bytes).unwrap();
    }
    archive.finish().unwrap();
    {
        let mount = uqm_engine::test_support::native_session::ContentMount::new(temp.path());
        let _graphics = native_probe::DrawQueue::init();
        check_font(mount.content_directory(), c"font", &alpha);
        check_font(mount.content_directory(), c"font.zip", &alpha);
        assert!(LoadedDrawable::load_file(mount.content_directory(), c"missing.ani").is_err());
        check_animation(mount.content_directory(), c"frames.ani");
        check_animation(mount.content_directory(), c"/animation/packed.ani");
        drop(mount);
    }
}
