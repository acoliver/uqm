use super::*;
use crate::engine::graphics::{
    context_abi::LoadedContext,
    drawable_abi::LoadedDrawable,
    native_abi::{Color, POINT},
};

#[test]
#[serial_test::serial]
fn retained_pixels_preserve_row_padding_and_survive_owner_release() {
    let mut game = crate::engine::Game::new();
    let display = crate::campaign_display_test::CampaignDisplay::install(&mut game).unwrap();
    {
        let owner = LoadedDrawable::create(2 | 8, 3, 2, 1).unwrap();
        let frame = owner.lease().context_frame(0);
        let color = Color {
            r: 23,
            g: 45,
            b: 67,
            a: 255,
        };
        frame.write_colors(&[color; 6], 3, 2);
        frame.replace_hotspot(POINT { x: -2, y: 7 });
        drop(owner);
        let sentinel = Color {
            r: 1,
            g: 2,
            b: 3,
            a: 4,
        };
        let mut pixels = [sentinel; 10];
        frame.read_colors(&mut pixels, 5, 2);
        for row in pixels.chunks_exact(5) {
            assert!(row[..3]
                .iter()
                .all(|p| (p.r, p.g, p.b, p.a) == (23, 45, 67, 255)));
            assert!(row[3..]
                .iter()
                .all(|p| (p.r, p.g, p.b, p.a) == (1, 2, 3, 4)));
        }
        assert_eq!(frame.rect().corner.x, 2);
        assert_eq!(frame.rect().corner.y, -7);
        assert_eq!(frame.pixel(POINT { x: 2, y: 1 }).g, 45);
        let context = LoadedContext::create(c"planet pixel regression");
        context.lease().set_frame(frame);
    }
    display.finish(game);
}

#[test]
fn real_rotation_and_zoom_compile_in_the_library_test_target() {
    let _: fn(&mut SphereState, i32, bool) = live::init_sphere_rotation;
    let _: fn(&crate::graphics::context_abi::ContextLease, &std::cell::RefCell<SphereState>) =
        live::zoom_in_planet_sphere;
    let _: fn(&crate::graphics::context_abi::ContextLease, &SphereState) =
        live::draw_default_planet_sphere;
}

#[test]
#[serial_test::serial]
fn generated_shield_and_rotating_frames_use_the_supplied_owners() {
    use crate::engine::graphics::native_abi::{DrawMode, EXTENT, RECT};
    let mut game = crate::engine::Game::new();
    let display = crate::campaign_display_test::CampaignDisplay::install(&mut game).unwrap();
    {
        let definition = LoadedDrawable::create(2, 242, 75, 1).unwrap();
        let source = Color {
            r: 61,
            g: 109,
            b: 173,
            a: 255,
        };
        definition
            .lease()
            .context_frame(0)
            .write_colors(&vec![source; 242 * 75], 242, 75);
        let mut sphere = SphereState::default();
        let planet = crate::engine::world::planet::PlanetDesc {
            data_index: 128,
            rand_seed: 12345,
            ..Default::default()
        };
        let info = crate::engine::world::planet::PlanetInfo {
            atmo_density: u16::MAX,
            axial_tilt: 17,
            ..Default::default()
        };
        let mut rng = crate::math::RandomContext::with_seed(37);
        crate::planets::plangen::live::generate_planet_surface(
            &mut sphere,
            &game.kernel.planet_animation,
            true,
            crate::planets::plangen::live::SurfaceInputs {
                rng: &mut rng,
                planet: &planet,
                info: &info,
                location: crate::graphics::drawable::Point { x: 10, y: -7 },
                width: 242,
            },
            Some(definition),
        );
        assert_eq!(sphere.rotation.point, 241);
        assert_eq!(sphere.elevation().len(), 242 * 75);
        let transparent = sphere.sphere_frame().pixel(POINT { x: 0, y: 0 });
        assert_eq!(transparent.a, 0);
        let center = sphere.sphere_frame().pixel(POINT { x: 37, y: 37 });
        assert_eq!(center.a, 255);
        assert!(center.r > center.g);
        let shield = sphere
            .images()
            .shield
            .as_ref()
            .unwrap()
            .lease()
            .context_frame(0);
        assert_eq!((shield.rect().corner.x, shield.rect().corner.y), (-45, -45));
        assert_eq!(shield.pixel(POINT { x: 0, y: 0 }).a, 0);
        assert!(sphere.images().throb.is_some());
        let owner = LoadedContext::create(c"sphere drawing regression");
        let context = owner.lease();
        let target = LoadedDrawable::create(2, 242, 147, 1).unwrap();
        context.set_frame(target.lease().context_frame(0));
        context.set_clip(RECT {
            corner: POINT { x: 0, y: 0 },
            extent: EXTENT {
                width: 242,
                height: 147,
            },
        });
        live::draw_default_planet_sphere(&context, &sphere);
        let sphere = std::cell::RefCell::new(sphere);
        live::rotate_planet_sphere(&context, &sphere, &game.kernel.planet_animation, false);
        assert_eq!(sphere.borrow().rotation.point, 240);
        let offscreen = LoadedContext::create(c"sphere tint regression");
        context.with_selected(|| {
            live::draw_planet(
                &offscreen.lease(),
                &mut sphere.borrow_mut(),
                37,
                crate::comm::locdata::CColor {
                    r: 17,
                    g: 71,
                    b: 131,
                    a: 255,
                },
            )
        });
        context.with_draw_mode(
            DrawMode {
                kind: 1,
                factor: 83,
            },
            || {
                context.with_scale(1, 137, || {
                    let mode = unsafe { crate::graphics::context_abi::GetContextDrawMode() };
                    assert_eq!((mode.kind, mode.factor), (1, 83));
                    assert_eq!(
                        unsafe { crate::graphics::gfx_common_abi::GetGraphicScale() },
                        137
                    );
                });
            },
        );
        context.flush();
        assert_eq!(context.clip_rect().extent.height, 147);
    }
    display.finish(game);
}
