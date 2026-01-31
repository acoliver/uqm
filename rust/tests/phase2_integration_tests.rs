//! Phase 2 integration smoke tests for graphics subsystems.

use std::sync::{Arc, RwLock};

use uqm_rust::graphics::{
    Canvas, ColorMapManager, DcqColor, DcqConfig, DcqDrawMode, DrawCommand, DrawCommandQueue,
    FadeType, RenderContext, Screen, ScreenType, FADE_FULL_INTENSITY, FADE_NO_INTENSITY,
};

fn setup_queue_with_screens() -> (
    DrawCommandQueue,
    Arc<RwLock<Canvas>>,
    Arc<RwLock<Canvas>>,
    Arc<RwLock<Canvas>>,
) {
    let render_context = Arc::new(RwLock::new(RenderContext::new()));
    let main = Arc::new(RwLock::new(Canvas::new_rgba(8, 8)));
    let extra = Arc::new(RwLock::new(Canvas::new_rgba(8, 8)));
    let transition = Arc::new(RwLock::new(Canvas::new_rgba(8, 8)));

    {
        let mut ctx = render_context.write().unwrap();
        ctx.set_screen(ScreenType::Main, Arc::clone(&main));
        ctx.set_screen(ScreenType::Extra, Arc::clone(&extra));
        ctx.set_screen(ScreenType::Transition, Arc::clone(&transition));
    }

    let queue = DrawCommandQueue::with_config(DcqConfig::debug(), render_context);
    (queue, main, extra, transition)
}

fn read_pixel(canvas: &Canvas, x: i32, y: i32) -> [u8; 4] {
    let bytes_per_pixel = canvas.format().bytes_per_pixel as usize;
    assert_eq!(bytes_per_pixel, 4);
    let width = canvas.width() as usize;
    let offset = (y as usize * width + x as usize) * bytes_per_pixel;
    let pixels = canvas.pixels();
    [
        pixels[offset],
        pixels[offset + 1],
        pixels[offset + 2],
        pixels[offset + 3],
    ]
}

#[test]
fn smoke_multi_screen_rendering() {
    let (queue, main, extra, transition) = setup_queue_with_screens();

    queue
        .push(DrawCommand::Line {
            x1: 1,
            y1: 1,
            x2: 1,
            y2: 1,
            color: DcqColor::new(255, 0, 0, 255),
            draw_mode: DcqDrawMode::Normal,
            dest: Screen::Main,
        })
        .unwrap();

    queue
        .push(DrawCommand::Line {
            x1: 2,
            y1: 2,
            x2: 2,
            y2: 2,
            color: DcqColor::new(0, 255, 0, 255),
            draw_mode: DcqDrawMode::Normal,
            dest: Screen::Extra,
        })
        .unwrap();

    queue
        .push(DrawCommand::Line {
            x1: 3,
            y1: 3,
            x2: 3,
            y2: 3,
            color: DcqColor::new(0, 0, 255, 255),
            draw_mode: DcqDrawMode::Normal,
            dest: Screen::Transition,
        })
        .unwrap();

    queue.process_commands().unwrap();

    let main_pixel = read_pixel(&main.read().unwrap(), 1, 1);
    let extra_pixel = read_pixel(&extra.read().unwrap(), 2, 2);
    let transition_pixel = read_pixel(&transition.read().unwrap(), 3, 3);

    assert_eq!(main_pixel, [255, 0, 0, 255]);
    assert_eq!(extra_pixel, [0, 255, 0, 255]);
    assert_eq!(transition_pixel, [0, 0, 255, 255]);
}

#[test]
fn smoke_dcq_batching_defers_processing() {
    let (queue, main, _, _) = setup_queue_with_screens();

    let _guard = queue.batch();
    queue
        .push(DrawCommand::Line {
            x1: 4,
            y1: 4,
            x2: 4,
            y2: 4,
            color: DcqColor::new(200, 100, 50, 255),
            draw_mode: DcqDrawMode::Normal,
            dest: Screen::Main,
        })
        .unwrap();

    queue.process_commands().unwrap();
    let pixel_before = read_pixel(&main.read().unwrap(), 4, 4);
    assert_eq!(pixel_before, [0, 0, 0, 0]);

    drop(_guard);
    queue.process_commands().unwrap();
    let pixel_after = read_pixel(&main.read().unwrap(), 4, 4);
    assert_eq!(pixel_after, [200, 100, 50, 255]);
}

#[test]
fn smoke_color_fade_finish() {
    let mgr = ColorMapManager::new();
    mgr.fade_screen(FadeType::FadeToBlack, 0);
    assert_eq!(mgr.get_fade_amount(), FADE_NO_INTENSITY);

    mgr.fade_screen(FadeType::FadeToWhite, 100);
    mgr.finish_fade();
    assert_eq!(mgr.get_fade_amount(), FADE_FULL_INTENSITY);
}
