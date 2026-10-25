//! All mineral surface frames through the production visual and stamp path.
use std::{ffi::CString, ptr};
use uqm_rust::{
    graphics::{
        cmap_abi::{SetColorMap, UninitColorMaps},
        context_abi::*,
        drawable_abi::*,
        filegfx::LoadGraphicFile,
        frame_abi::ClearDrawable,
        native_abi::*,
        pixmap_abi::*,
    },
    io::uio_bridge as uio,
    mainloop::dir_prep::contentDir,
    planet_side::{
        assembly::{SurfaceAssembly, SurfaceVisualPort},
        collision_adapter::SurfaceMasks,
        entities::{SurfaceEntity, SurfaceEntityKind, SurfaceWorld},
        generation::{GeneratedEntity, ScanNodeId, ScanType},
        graphics_adapter::{draw_surface_entities, SurfaceFrameRegistry},
        mask_adapter::extract_frame_mask,
        model::SurfacePoint,
        visual_adapter::CffiSurfaceVisuals,
    },
    planets::c_api::{FORMAT_CONV_SURF, SDL_SCREEN},
};

#[test]
fn every_category_and_gross_size_draws_native_content_pixels_and_hotspot() {
    let base = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../sc2/content/base");
    let animation = std::fs::read_to_string(base.join("ui/miscdata.ani")).unwrap();
    let rows: Vec<Vec<&str>> = animation
        .lines()
        .map(|row| row.split_whitespace().collect())
        .collect();
    let format =
        sdl2::surface::Surface::new(1, 1, sdl2::pixels::PixelFormatEnum::RGBA8888).unwrap();
    let screen =
        sdl2::surface::Surface::new(1, 1, sdl2::pixels::PixelFormatEnum::RGBX8888).unwrap();
    unsafe {
        FORMAT_CONV_SURF = format.raw();
        SDL_SCREEN = screen.raw();
        uio::uio_init();
        let repo = uio::uio_openRepository(0);
        let host = CString::new(base.to_str().unwrap()).unwrap();
        assert!(!uio::uio_mountDir(
            repo,
            c"/".as_ptr(),
            1,
            ptr::null_mut(),
            ptr::null(),
            host.as_ptr(),
            ptr::null_mut(),
            6,
            ptr::null_mut()
        )
        .is_null());
        contentDir = uio::uio_openDir(repo, c"/".as_ptr(), 0).cast();
        let drawable = LoadGraphicFile(c"ui/miscdata.ani".as_ptr());
        assert!(!drawable.is_null());
        let frames = CaptureDrawable(drawable);
        let mut visuals =
            CffiSurfaceVisuals::new(frames.cast(), ptr::null_mut(), [ptr::null_mut(); 3]).unwrap();
        #[cfg(feature = "debug-process")]
        let context = CreateContextAux(c"mineral surface parity".as_ptr());
        #[cfg(not(feature = "debug-process"))]
        let context = CreateContextAux();
        let previous_context = SetContext(context);
        let destination = CreateDrawable(0, 320, 200, 1);
        let destination_frame = CaptureDrawable(destination);
        SetContextFGFrame(destination_frame);
        let mut clip = RECT {
            corner: POINT { x: 8, y: 16 },
            extent: EXTENT {
                width: 242,
                height: 162,
            },
        };
        SetContextClipRect(&mut clip);
        let background = Color {
            r: 1,
            g: 2,
            b: 3,
            a: 255,
        };
        SetContextBackGroundColor(background);
        let empty_mask = extract_frame_mask(frames.cast()).unwrap();
        for category in 0..8 {
            for gross in 0..3 {
                // GeneratePlanetSide: 2*NUM_SCANDOT_TRANSITIONS +
                // ElementCategory(type)*5 + LOBYTE(density)+1, then IncFrameIndex.
                let original_frame = 8 + category * 5 + gross + 2;
                let row = &rows[original_frame];
                let png = std::fs::read(base.join("ui").join(row[0])).unwrap();
                let mut decoder = png::Decoder::new(std::io::Cursor::new(&png));
                decoder.set_transformations(png::Transformations::IDENTITY);
                let mut reader = decoder.read_info().unwrap();
                let palette = reader.info().palette.as_ref().unwrap().to_vec();
                let mut indices = vec![0; reader.output_buffer_size().unwrap()];
                let info = reader.next_frame(&mut indices).unwrap();
                assert_eq!(info.bit_depth, png::BitDepth::Eight);
                assert_eq!(info.color_type, png::ColorType::Indexed);
                let (width, height) = (info.width, info.height);
                let transparent: u8 = row[1].parse().unwrap();
                let mut colormap = vec![0, 0];
                colormap.extend_from_slice(&palette);
                colormap.resize(2 + 256 * 3, 0);
                assert_ne!(SetColorMap(colormap.as_ptr()), 0);
                let hotspot_x: i32 = row[3].parse().unwrap();
                let hotspot_y: i32 = row[4].parse().unwrap();
                // MAG_SHIFT=2 scales node coordinates, not the sprite extent.
                // Cover a normal camera, horizontal seam, and a translated camera.
                for camera in [
                    SurfacePoint { x: 400, y: 200 },
                    SurfacePoint { x: 0, y: 200 },
                    SurfacePoint { x: 967, y: 250 },
                ] {
                    let entity = SurfaceEntity {
                        kind: SurfaceEntityKind::MineralNode {
                            category,
                            size: gross as u16,
                            quantity: 19,
                        },
                        position: SurfacePoint {
                            x: (camera.x - (6 << 2)).rem_euclid(242 << 2),
                            y: camera.y + (3 << 2),
                        },
                        finite_life: None,
                    };
                    let mut surface = SurfaceAssembly {
                        world: SurfaceWorld::new(),
                        generated: Vec::new(),
                        frames: SurfaceFrameRegistry::default(),
                        masks: SurfaceMasks::new(vec![empty_mask.clone(); 16]).unwrap(),
                    };
                    let id = surface.world.insert(entity);
                    let mapping = GeneratedEntity {
                        entity: id,
                        scan: ScanType::Mineral,
                        node: ScanNodeId::new(0).unwrap(),
                    };
                    let visual = visuals
                        .visual_for(mapping, surface.world.get(id).unwrap())
                        .unwrap();
                    assert_eq!(usize::from(visual.frame.index), original_frame);
                    let selected = SetAbsFrameIndex(visual.frame.base.cast(), visual.frame.index);
                    assert_eq!(
                        (
                            (*selected).Bounds.width as u32,
                            (*selected).Bounds.height as u32
                        ),
                        (width, height)
                    );
                    assert_eq!(
                        (
                            i32::from((*selected).HotSpot.x),
                            i32::from((*selected).HotSpot.y)
                        ),
                        (hotspot_x, hotspot_y)
                    );
                    surface.frames.insert(id, visual.frame);
                    ClearDrawable();
                    draw_surface_entities(&surface, camera);
                    let mut pixels = vec![
                        Color {
                            r: 0,
                            g: 0,
                            b: 0,
                            a: 0
                        };
                        320 * 200
                    ];
                    assert_ne!(
                        ReadFramePixelColors(destination_frame, pixels.as_mut_ptr(), 320, 200),
                        0
                    );
                    let left = 8 + 242 / 2 - (6 << 2) - hotspot_x;
                    let top = 16 + 162 / 2 + (3 << 2) - hotspot_y;
                    let mut bounds = (320, 200, -1, -1);
                    for y in 16..178 {
                        for x in 8..250 {
                            let actual = pixels[(y * 320 + x) as usize];
                            let sx = x - left;
                            let sy = y - top;
                            let expected =
                                if sx >= 0 && sy >= 0 && sx < width as i32 && sy < height as i32 {
                                    let index = indices[(sy as u32 * width + sx as u32) as usize];
                                    if index != transparent {
                                        let color = usize::from(index) * 3;
                                        [palette[color], palette[color + 1], palette[color + 2]]
                                    } else {
                                        [1, 2, 3]
                                    }
                                } else {
                                    [1, 2, 3]
                                };
                            assert_eq!(
                                [actual.r, actual.g, actual.b],
                                expected,
                                "category={category} gross={gross} at ({x},{y}) camera={camera:?}"
                            );
                            if expected != [1, 2, 3] {
                                bounds = (
                                    bounds.0.min(x),
                                    bounds.1.min(y),
                                    bounds.2.max(x),
                                    bounds.3.max(y),
                                );
                            }
                        }
                    }
                    assert_eq!(
                        bounds,
                        (left, top, left + width as i32 - 1, top + height as i32 - 1),
                        "category={category} gross={gross}"
                    );
                }
            }
        }
        drop(visuals);
        SetContextFGFrame(ptr::null_mut());
        DestroyDrawable(destination);
        DestroyDrawable(drawable);
        SetContext(previous_context);
        DestroyContext(context);
        uio::uio_closeDir(contentDir.cast());
        contentDir = ptr::null_mut();
        uio::uio_closeRepository(repo);
        uio::uio_unInit();
        UninitColorMaps();
        FORMAT_CONV_SURF = ptr::null_mut();
        SDL_SCREEN = ptr::null_mut();
    }
}
