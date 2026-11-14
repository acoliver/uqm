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
        let color = Color { r: 23, g: 45, b: 67, a: 255 };
        frame.write_colors(&[color; 6], 3, 2);
        frame.replace_hotspot(POINT { x: -2, y: 7 });
        drop(owner);
        let sentinel = Color { r: 1, g: 2, b: 3, a: 4 };
        let mut pixels = [sentinel; 10];
        frame.read_colors(&mut pixels, 5, 2);
        for row in pixels.chunks_exact(5) {
            assert!(row[..3].iter().all(|p| (p.r,p.g,p.b,p.a) == (23,45,67,255)));
            assert!(row[3..].iter().all(|p| (p.r,p.g,p.b,p.a) == (1,2,3,4)));
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
fn real_rotation_and_generation_compile_in_the_library_test_target() {
    let _: fn(&mut SphereState, i32, bool) = live::init_sphere_rotation;
    let _: fn(&crate::graphics::context_abi::ContextLease, &SphereState) = live::draw_default_planet_sphere;
}
