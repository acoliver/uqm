//! All mineral surface frames through the production visual and stamp path.
#![forbid(unsafe_code)]
use uqm_engine::graphics::context::{ContextLease, LoadedContext};
use uqm_engine::graphics::drawable::{ContextFrame, LoadedDrawable};
use uqm_engine::graphics::{
    geometry::Color, geometry::Extent, geometry::Point, geometry::Rect, menu,
};
use uqm_engine::test_support::native_probe::{set_color_map, DrawQueue};
use uqm_modes::planet_side::assembly::SurfaceAssembly;
use uqm_modes::planet_side::assembly::SurfaceVisualPort;
use uqm_modes::planet_side::collision_adapter::SurfaceMasks;
use uqm_modes::planet_side::entities::SurfaceEntity;
use uqm_modes::planet_side::entities::SurfaceEntityKind;
use uqm_modes::planet_side::entities::SurfaceWorld;
use uqm_modes::planet_side::generation::GeneratedEntity;
use uqm_modes::planet_side::generation::ScanNodeId;
use uqm_modes::planet_side::generation::ScanType;
use uqm_modes::planet_side::graphics_adapter::draw_surface_entities;
use uqm_modes::planet_side::graphics_adapter::SurfaceFrameRegistry;
use uqm_modes::planet_side::mask_adapter::extract_frame_mask;
use uqm_modes::planet_side::model::SurfacePoint;
use uqm_modes::planet_side::visual_adapter::CffiSurfaceVisuals;

#[test]
#[serial_test::serial]
fn every_category_and_gross_size_draws_native_content_pixels_and_hotspot() {
    let base = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../sc2/content/base");
    let animation = std::fs::read_to_string(base.join("ui/miscdata.ani")).unwrap();
    let rows: Vec<Vec<&str>> = animation
        .lines()
        .map(|row| row.split_whitespace().collect())
        .collect();
    let _graphics = DrawQueue::init();
    {
        let content = uqm_engine::test_support::native_session::ContentMount::new(&base);
        let drawable = LoadedDrawable::load_file(c"ui/miscdata.ani").unwrap();
        let frames = drawable.first_frame();
        let mut visuals = CffiSurfaceVisuals::new(
            ContextFrame::retain(frames),
            None,
            std::array::from_fn(|_| None),
        )
        .unwrap();
        let context_owner = LoadedContext::create(c"mineral surface parity");
        let context = Some(context_owner.context());
        let previous_context = menu::select(context);
        let destination = LoadedDrawable::create(0, 320, 200, 1).unwrap();
        let destination_frame = destination.first_frame();
        menu::set_frame(destination_frame);
        let clip = Rect {
            corner: Point { x: 8, y: 16 },
            extent: Extent {
                width: 242,
                height: 162,
            },
        };
        menu::set_clip(clip);
        let background = Color {
            r: 1,
            g: 2,
            b: 3,
            a: 255,
        };
        menu::background(background);
        let empty_mask = extract_frame_mask(&ContextFrame::retain(frames)).unwrap();
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
                set_color_map(&colormap);
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
                    let selected = visual.frame.base.wrapped_frame(visual.frame.index);
                    assert_eq!(
                        (
                            selected.rect().extent.width as u32,
                            selected.rect().extent.height as u32
                        ),
                        (width, height)
                    );
                    assert_eq!(
                        (
                            -i32::from(selected.rect().corner.x),
                            -i32::from(selected.rect().corner.y)
                        ),
                        (hotspot_x, hotspot_y)
                    );
                    surface.frames.insert(id, visual.frame);
                    menu::clear();
                    draw_surface_entities(&ContextLease::require(context), &surface, camera);
                    let mut pixels = vec![
                        Color {
                            r: 0,
                            g: 0,
                            b: 0,
                            a: 0
                        };
                        320 * 200
                    ];
                    ContextFrame::retain(destination_frame).read_colors(&mut pixels, 320, 200);
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
        menu::set_frame(Default::default());
        drop((destination, drawable));
        menu::select(previous_context);
        drop(context_owner);
        drop(content);
    }
}
