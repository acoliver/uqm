//! Real UIO -> PNG -> font-page loading with the remaining engine linked.
use std::ffi::CString;
use std::io::Write;
use std::ptr;
use uqm_rust::graphics::{gfxload, native_abi as abi};
use uqm_rust::io::uio_bridge as uio;
use uqm_rust::mainloop::dir_prep::{contentDir, repository};
use uqm_rust::memory::rust_hfree;

fn png(width: u32, height: u32, color: png::ColorType, pixels: &[u8]) -> Vec<u8> {
    let mut output = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut output, width, height);
        encoder.set_color(color);
        encoder.set_depth(png::BitDepth::Eight);
        encoder
            .write_header()
            .unwrap()
            .write_image_data(pixels)
            .unwrap();
    }
    output
}

unsafe fn check_font(path: &std::ffi::CStr, zipped: bool, alpha: &[u8]) {
    abi::_cur_resfile_name = path.as_ptr();
    let fp = if zipped {
        uio::uio_fopen(contentDir.cast(), path.as_ptr(), c"rb".as_ptr())
    } else {
        usize::MAX as *mut uio::uio_Stream
    };
    assert!(!fp.is_null());
    let font = gfxload::_GetFontData(fp.cast(), 0).cast::<abi::font_desc>();
    if zipped {
        assert_eq!(uio::uio_fclose(fp), 0);
    }
    abi::_cur_resfile_name = ptr::null();
    assert!(!font.is_null());
    assert_eq!(((*font).Leading, (*font).LeadingWidth), (10, 4));
    let page = (*font).fontPages;
    assert!(!page.is_null());
    assert_eq!(
        ((*page).pageStart, (*page).firstChar, (*page).numChars),
        (0, 65, 2)
    );
    assert!((*page).next.is_null());
    let a = &*(*page).charDesc;
    assert_eq!(
        (
            a.extent.width,
            a.extent.height,
            a.disp.width,
            a.disp.height,
            a.HotSpot.x,
            a.HotSpot.y,
            a.pitch
        ),
        (3, 8, 4, 9, 0, 7, 3)
    );
    assert_eq!(std::slice::from_raw_parts(a.data, 24), alpha);
    let b = &*(*page).charDesc.add(1);
    assert_eq!(
        (
            b.extent.width,
            b.extent.height,
            b.disp.width,
            b.disp.height,
            b.HotSpot.y,
            b.pitch
        ),
        (3, 2, 4, 3, 2, 3)
    );
    assert_eq!(
        std::slice::from_raw_parts(b.data, 6),
        [0, 255, 18, 72, 145, 36]
    );
    // Production font release queues data deletion; this isolated loader test has
    // no drawing queue, so consume the owned page allocations directly.
    for i in 0..(*page).numChars {
        rust_hfree((*(*page).charDesc.add(i)).data.cast());
    }
    rust_hfree((*page).charDesc.cast());
    rust_hfree(page.cast());
    rust_hfree(font.cast());
}

unsafe fn check_animation(path: &std::ffi::CStr) {
    abi::_cur_resfile_name = path.as_ptr();
    let relative_path = CString::new(
        path.to_bytes()
            .strip_prefix(b"/")
            .unwrap_or(path.to_bytes()),
    )
    .unwrap();
    let fp = uio::uio_fopen(contentDir.cast(), relative_path.as_ptr(), c"rb".as_ptr());
    assert!(!fp.is_null());
    assert_eq!(uio::uio_fseek(fp, 0, libc::SEEK_END), 0);
    let length = uio::uio_ftell(fp) as u32;
    assert_eq!(uio::uio_fseek(fp, 0, libc::SEEK_SET), 0);
    let drawable = gfxload::_GetCelData(fp.cast(), length).cast::<abi::drawable_desc>();
    uio::uio_fclose(fp);
    abi::_cur_resfile_name = ptr::null();
    assert!(!drawable.is_null());
    assert_eq!(((*drawable).Flags, (*drawable).MaxIndex), (2, 1));
    for i in 0..2 {
        let frame = &*(*drawable).Frame.add(i);
        assert_eq!(
            (
                frame.Type,
                frame.Index,
                frame.Bounds.width,
                frame.Bounds.height
            ),
            (0, i as u16, 3, 2)
        );
        assert_eq!(frame.parent, drawable);
        assert_eq!(
            (frame.HotSpot.x, frame.HotSpot.y),
            if i == 0 { (-5, 11) } else { (0, 0) }
        );
        assert_eq!((*frame.image).colormap_index, if i == 0 { 17 } else { 0 });
        let surface = (*frame.image).NormalImg.cast::<sdl2_sys::SDL_Surface>();
        let mut key = 0;
        assert_eq!(
            sdl2_sys::SDL_GetColorKey(surface, &mut key),
            if i == 0 { 0 } else { -1 }
        );
        if i == 0 {
            assert_eq!(key, 1);
        }
        for y in 0..2 {
            assert_eq!(
                std::slice::from_raw_parts(
                    (*surface)
                        .pixels
                        .cast::<u8>()
                        .add(y * (*surface).pitch as usize),
                    3
                ),
                [0, 1, 2]
            );
        }
        abi::TFB_DrawImage_Delete(frame.image);
    }
    rust_hfree((*drawable).Frame.cast());
    rust_hfree(drawable.cast());
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
    let host = CString::new(temp.path().to_str().unwrap()).unwrap();
    unsafe {
        uio::uio_init();
        let repo = uio::uio_openRepository(0);
        let mount = uio::uio_mountDir(
            repo,
            c"/".as_ptr(),
            1,
            ptr::null_mut(),
            ptr::null(),
            host.as_ptr(),
            ptr::null_mut(),
            6,
            ptr::null_mut(),
        );
        assert!(!mount.is_null());
        let dir = uio::uio_openDir(repo, c"/".as_ptr(), 0);
        assert!(!dir.is_null());
        let saved_dir = contentDir;
        let saved_repo = repository;
        contentDir = dir.cast();
        repository = repo.cast();
        check_font(c"font", false, &alpha);
        check_font(c"font.zip", true, &alpha);
        abi::_cur_resfile_name = c"busy".as_ptr();
        assert!(uqm_rust::graphics::filegfx::LoadGraphicFile(c"frames.ani".as_ptr()).is_null());
        assert!(uqm_rust::graphics::filegfx::LoadFontFile(c"font".as_ptr()).is_null());
        abi::_cur_resfile_name = ptr::null();
        assert!(uqm_rust::graphics::filegfx::LoadGraphicFile(c"missing.ani".as_ptr()).is_null());
        assert!(ptr::addr_of!(abi::_cur_resfile_name).read().is_null());
        let font = uqm_rust::graphics::filegfx::LoadFontFile(c"font".as_ptr());
        assert!(!font.is_null());
        assert_eq!(((*font).Leading, (*font).LeadingWidth), (10, 4));
        let page = (*font).fontPages;
        for i in 0..(*page).numChars {
            rust_hfree((*(*page).charDesc.add(i)).data.cast());
        }
        rust_hfree((*page).charDesc.cast());
        rust_hfree(page.cast());
        rust_hfree(font.cast());
        assert!(ptr::addr_of!(abi::_cur_resfile_name).read().is_null());
        check_animation(c"frames.ani");
        check_animation(c"/animation/packed.ani");
        contentDir = saved_dir;
        repository = saved_repo;
        uio::uio_closeDir(dir);
        uio::uio_closeRepository(repo);
        uio::uio_unInit();
    }
}
