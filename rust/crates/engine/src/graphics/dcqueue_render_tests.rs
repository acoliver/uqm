//! Tests that run queued commands against real screen canvases and check
//! the pixels that end up on them.

use super::*;
use crate::graphics::tfb_draw::{Canvas, HotSpot, TFImage};
use std::sync::atomic::AtomicUsize;

const SIDE: i32 = 12;
const RED: Color = Color::new(255, 0, 0, 255);
const GREEN: Color = Color::new(0, 255, 0, 255);
const BLUE: Color = Color::new(0, 0, 255, 255);
const CLEAR: [u8; 4] = [0, 0, 0, 0];

struct Rig {
    queue: DrawCommandQueue,
    context: Arc<RwLock<RenderContext>>,
    main: Arc<RwLock<Canvas>>,
    extra: Arc<RwLock<Canvas>>,
}

fn rig() -> Rig {
    let context = Arc::new(RwLock::new(RenderContext::new()));
    let main = Arc::new(RwLock::new(Canvas::new_rgba(SIDE, SIDE)));
    let extra = Arc::new(RwLock::new(Canvas::new_rgba(SIDE, SIDE)));
    {
        let mut ctx = context.write().unwrap();
        ctx.set_screen(ScreenType::Main, Arc::clone(&main));
        ctx.set_screen(ScreenType::Extra, Arc::clone(&extra));
    }
    let queue = DrawCommandQueue::with_config(DcqConfig::standard(), Arc::clone(&context));
    Rig {
        queue,
        context,
        main,
        extra,
    }
}

impl Rig {
    fn run(&self, commands: Vec<DrawCommand>) {
        for command in commands {
            self.queue.push(command).unwrap();
        }
        self.queue.process_commands().unwrap();
    }

    fn image(&self, canvas: Canvas) -> ImageRef {
        let id = self
            .context
            .write()
            .unwrap()
            .register_image(Arc::new(TFImage::new(canvas)));
        ImageRef::new(id)
    }
}

fn pixel(canvas: &Arc<RwLock<Canvas>>, x: i32, y: i32) -> [u8; 4] {
    let canvas = canvas.read().unwrap();
    let offset = ((y * canvas.width() + x) * 4) as usize;
    canvas.pixels()[offset..offset + 4].try_into().unwrap()
}

fn rgba(color: Color) -> [u8; 4] {
    [color.r, color.g, color.b, color.a]
}

fn solid_canvas(side: i32, color: Color) -> Canvas {
    let mut canvas = Canvas::new_rgba(side, side);
    crate::graphics::tfb_draw::fill_rect(&mut canvas, 0, 0, side - 1, side - 1, color).unwrap();
    canvas
}

fn rect(x: i32, y: i32, w: i32, h: i32) -> Rect {
    Rect::new(Point::new(x, y), Extent::new(w, h))
}

fn line(x1: i32, y1: i32, x2: i32, y2: i32, color: Color, dest: Screen) -> DrawCommand {
    DrawCommand::Line {
        x1,
        y1,
        x2,
        y2,
        color,
        draw_mode: DrawMode::Normal,
        dest,
    }
}

#[test]
fn a_line_command_draws_its_pixels_on_the_requested_screen_only() {
    let rig = rig();

    rig.run(vec![line(1, 2, 5, 2, RED, Screen::Main)]);

    for x in 1..=5 {
        assert_eq!(pixel(&rig.main, x, 2), rgba(RED), "x={x}");
    }
    assert_eq!(pixel(&rig.main, 0, 2), CLEAR);
    assert_eq!(pixel(&rig.main, 6, 2), CLEAR);
    assert_eq!(
        pixel(&rig.extra, 3, 2),
        CLEAR,
        "the other screen is untouched"
    );
    assert!(rig.queue.is_empty());
}

#[test]
fn an_outline_rectangle_leaves_its_interior_alone() {
    let rig = rig();

    rig.run(vec![DrawCommand::Rect {
        rect: rect(2, 2, 5, 4),
        color: GREEN,
        draw_mode: DrawMode::Normal,
        dest: Screen::Main,
    }]);

    assert_eq!(pixel(&rig.main, 2, 2), rgba(GREEN));
    assert_eq!(pixel(&rig.main, 6, 2), rgba(GREEN));
    assert_eq!(pixel(&rig.main, 2, 5), rgba(GREEN));
    assert_eq!(pixel(&rig.main, 6, 5), rgba(GREEN));
    assert_eq!(pixel(&rig.main, 4, 3), CLEAR, "interior stays empty");
    assert_eq!(pixel(&rig.main, 7, 3), CLEAR, "nothing outside the extent");
}

#[test]
fn a_filled_rectangle_covers_exactly_its_extent() {
    let rig = rig();

    rig.run(vec![DrawCommand::Rect {
        rect: rect(2, 3, 4, 3),
        color: BLUE,
        draw_mode: DrawMode::Blended,
        dest: Screen::Extra,
    }]);

    for y in 0..SIDE {
        for x in 0..SIDE {
            let inside = (2..6).contains(&x) && (3..6).contains(&y);
            let expected = if inside { rgba(BLUE) } else { CLEAR };
            assert_eq!(pixel(&rig.extra, x, y), expected, "({x},{y})");
        }
    }
    assert_eq!(pixel(&rig.main, 3, 3), CLEAR);
}

#[test]
fn an_empty_filled_rectangle_draws_nothing() {
    let rig = rig();

    rig.run(vec![
        DrawCommand::Rect {
            rect: rect(4, 4, 0, 3),
            color: RED,
            draw_mode: DrawMode::Blended,
            dest: Screen::Main,
        },
        DrawCommand::Rect {
            rect: rect(4, 4, 3, 0),
            color: RED,
            draw_mode: DrawMode::Blended,
            dest: Screen::Main,
        },
    ]);

    assert!(rig.main.read().unwrap().pixels().iter().all(|&b| b == 0));
}

#[test]
fn an_image_is_drawn_with_its_hot_spot_at_the_requested_position() {
    let rig = rig();
    let image = rig.image(solid_canvas(3, RED));
    rig.context
        .read()
        .unwrap()
        .get_image(image.id())
        .unwrap()
        .set_normal_hot_spot(HotSpot::new(1, 1));

    rig.run(vec![DrawCommand::Image {
        image,
        x: 5,
        y: 5,
        dest: Screen::Main,
        colormap: None,
        draw_mode: DrawMode::Normal,
        scale: 0,
        scale_mode: ScaleMode::Nearest,
    }]);

    assert_eq!(
        pixel(&rig.main, 4, 4),
        rgba(RED),
        "top-left is one before the hot spot"
    );
    assert_eq!(pixel(&rig.main, 6, 6), rgba(RED));
    assert_eq!(pixel(&rig.main, 3, 4), CLEAR);
    assert_eq!(pixel(&rig.main, 7, 6), CLEAR);
}

#[test]
fn a_filled_image_paints_its_silhouette_in_the_fill_colour() {
    let rig = rig();
    let image = rig.image(solid_canvas(3, RED));

    rig.run(vec![DrawCommand::FilledImage {
        image,
        x: 2,
        y: 2,
        color: GREEN,
        dest: Screen::Main,
        draw_mode: DrawMode::Normal,
        scale: 0,
        scale_mode: ScaleMode::Nearest,
    }]);

    assert_eq!(pixel(&rig.main, 2, 2), rgba(GREEN));
    assert_eq!(pixel(&rig.main, 4, 4), rgba(GREEN));
    assert_eq!(pixel(&rig.main, 5, 5), CLEAR);
}

#[test]
fn commands_naming_a_missing_screen_or_image_are_skipped_without_disturbing_the_rest() {
    let rig = rig();
    let missing = ImageRef::new(9999);

    rig.run(vec![
        line(0, 0, 3, 0, RED, Screen::Transition),
        DrawCommand::Image {
            image: missing,
            x: 0,
            y: 0,
            dest: Screen::Main,
            colormap: None,
            draw_mode: DrawMode::Normal,
            scale: 0,
            scale_mode: ScaleMode::Nearest,
        },
        DrawCommand::FilledImage {
            image: missing,
            x: 0,
            y: 0,
            color: RED,
            dest: Screen::Main,
            draw_mode: DrawMode::Normal,
            scale: 0,
            scale_mode: ScaleMode::Nearest,
        },
        DrawCommand::CopyToImage {
            image: missing,
            rect: rect(0, 0, 2, 2),
            src: Screen::Main,
        },
        line(0, 1, 3, 1, BLUE, Screen::Main),
    ]);

    assert_eq!(pixel(&rig.main, 0, 0), CLEAR);
    assert_eq!(
        pixel(&rig.main, 2, 1),
        rgba(BLUE),
        "later commands still run"
    );
}

#[test]
fn a_copy_command_moves_a_rectangle_between_screens_at_the_same_position() {
    let rig = rig();
    rig.run(vec![
        DrawCommand::Rect {
            rect: rect(0, 0, SIDE, SIDE),
            color: RED,
            draw_mode: DrawMode::Blended,
            dest: Screen::Main,
        },
        DrawCommand::Rect {
            rect: rect(4, 4, 3, 3),
            color: GREEN,
            draw_mode: DrawMode::Blended,
            dest: Screen::Main,
        },
    ]);

    rig.run(vec![DrawCommand::Copy {
        rect: rect(3, 3, 5, 5),
        src: Screen::Main,
        dest: Screen::Extra,
    }]);

    assert_eq!(pixel(&rig.extra, 3, 3), rgba(RED));
    assert_eq!(pixel(&rig.extra, 5, 5), rgba(GREEN));
    assert_eq!(pixel(&rig.extra, 7, 7), rgba(RED));
    assert_eq!(
        pixel(&rig.extra, 2, 2),
        CLEAR,
        "outside the copied rectangle"
    );
    assert_eq!(pixel(&rig.extra, 8, 8), CLEAR);
}

#[test]
fn copying_a_screen_region_into_an_image_captures_its_pixels() {
    let rig = rig();
    rig.run(vec![DrawCommand::Rect {
        rect: rect(2, 2, 3, 3),
        color: BLUE,
        draw_mode: DrawMode::Blended,
        dest: Screen::Main,
    }]);
    let image = rig.image(Canvas::new_rgba(3, 3));

    rig.run(vec![DrawCommand::CopyToImage {
        image,
        rect: rect(2, 2, 3, 3),
        src: Screen::Main,
    }]);

    let stored = rig.context.read().unwrap().get_image(image.id()).unwrap();
    assert!(stored.is_dirty(), "the cached scalings must be refreshed");
    let pixels = stored.normal().unwrap().pixels();
    assert_eq!(pixels.len(), 3 * 3 * 4);
    assert!(
        pixels.chunks(4).all(|p| p == rgba(BLUE)),
        "every image pixel came from the blue square"
    );
}

#[test]
fn scissoring_confines_later_drawing_until_it_is_disabled() {
    let rig = rig();

    rig.run(vec![
        DrawCommand::ScissorEnable {
            rect: rect(2, 0, 3, SIDE),
        },
        line(0, 1, 9, 1, RED, Screen::Main),
        DrawCommand::ScissorDisable,
        line(0, 3, 9, 3, RED, Screen::Main),
    ]);

    assert_eq!(pixel(&rig.main, 1, 1), CLEAR);
    assert_eq!(pixel(&rig.main, 3, 1), rgba(RED));
    assert_eq!(pixel(&rig.main, 6, 1), CLEAR);
    assert_eq!(
        pixel(&rig.main, 0, 3),
        rgba(RED),
        "after disabling, the whole row draws"
    );
    assert_eq!(pixel(&rig.main, 9, 3), rgba(RED));
}

#[test]
fn setting_a_mipmap_attaches_the_canvas_with_its_hot_spot() {
    let rig = rig();
    let image = rig.image(solid_canvas(4, RED));
    let mipmap = rig.image(solid_canvas(2, GREEN));

    rig.run(vec![DrawCommand::SetMipmap {
        image,
        mipmap,
        hotx: 1,
        hoty: 2,
    }]);

    let stored = rig.context.read().unwrap().get_image(image.id()).unwrap();
    let attached = stored.mipmap().expect("a mipmap is attached");
    assert_eq!((attached.width(), attached.height()), (2, 2));
    assert_eq!(stored.mipmap_hot_spot(), HotSpot::new(1, 2));
}

#[test]
fn deleting_an_image_or_data_releases_it_from_the_render_context() {
    let rig = rig();
    let image = rig.image(solid_canvas(2, RED));
    rig.context.write().unwrap().register_data_ptr(0xBEEF);
    assert_eq!(rig.context.read().unwrap().data_ptr_count(), 1);

    rig.run(vec![
        DrawCommand::DeleteImage { image },
        DrawCommand::DeleteData { data: 0xBEEF },
    ]);

    assert!(rig.context.read().unwrap().get_image(image.id()).is_none());
    assert_eq!(rig.context.read().unwrap().data_ptr_count(), 0);
}

#[test]
fn a_signal_command_raises_its_flag_only_once_the_queue_reaches_it() {
    let rig = rig();
    let signal = Arc::new(AtomicBool::new(false));
    rig.queue
        .push(DrawCommand::SendSignal {
            signal: Arc::clone(&signal),
        })
        .unwrap();
    assert!(
        !signal.load(Ordering::Acquire),
        "pushing alone does not signal"
    );

    rig.queue.process_commands().unwrap();

    assert!(signal.load(Ordering::Acquire));
}

#[test]
fn callbacks_and_drawing_run_in_the_order_they_were_queued() {
    let rig = rig();
    static SEEN_RED_AT_CALLBACK: AtomicUsize = AtomicUsize::new(0);
    static MAIN_PTR: AtomicUsize = AtomicUsize::new(0);
    fn observe(_: u64) {
        let canvas = unsafe { &*(MAIN_PTR.load(Ordering::SeqCst) as *const RwLock<Canvas>) };
        let canvas = canvas.read().unwrap();
        let red_pixels = canvas.pixels().chunks(4).filter(|p| p[0] == 255).count();
        SEEN_RED_AT_CALLBACK.store(red_pixels, Ordering::SeqCst);
    }
    MAIN_PTR.store(Arc::as_ptr(&rig.main) as usize, Ordering::SeqCst);

    rig.run(vec![
        line(0, 0, 3, 0, RED, Screen::Main),
        DrawCommand::Callback {
            callback: observe,
            arg: 0,
        },
        line(0, 1, 3, 1, RED, Screen::Main),
    ]);

    assert_eq!(
        SEEN_RED_AT_CALLBACK.load(Ordering::SeqCst),
        4,
        "the callback saw the first line but not the second"
    );
    assert_eq!(
        pixel(&rig.main, 3, 1),
        rgba(RED),
        "the second line still drew afterwards"
    );
}

#[test]
fn a_video_reinit_request_is_accepted_without_touching_the_screens() {
    let rig = rig();

    rig.run(vec![DrawCommand::ReinitVideo {
        driver: 1,
        flags: 0,
        width: 640,
        height: 480,
    }]);

    assert!(rig.main.read().unwrap().pixels().iter().all(|&b| b == 0));
    assert!(rig.queue.is_empty());
}

#[test]
fn screens_convert_between_queue_and_render_context_identifiers() {
    for (screen, kind) in [
        (Screen::Main, ScreenType::Main),
        (Screen::Extra, ScreenType::Extra),
        (Screen::Transition, ScreenType::Transition),
    ] {
        assert_eq!(ScreenType::from(screen), kind);
        assert_eq!(Screen::from(kind), screen);
    }
}

#[test]
fn an_over_full_queue_is_batch_reset_before_draining() {
    let config = DcqConfig {
        max_size: 8,
        force_slowdown_size: 4,
        force_break_size: 3,
        livelock_max: 2,
    };
    let context = Arc::new(RwLock::new(RenderContext::new()));
    let main = Arc::new(RwLock::new(Canvas::new_rgba(SIDE, SIDE)));
    context
        .write()
        .unwrap()
        .set_screen(ScreenType::Main, Arc::clone(&main));
    let queue = DrawCommandQueue::with_config(config, context);
    let _batch = queue.batch();
    for y in 0..5 {
        queue.push(line(0, y, 2, y, RED, Screen::Main)).unwrap();
    }
    assert_eq!(queue.len(), 0, "batched commands are not yet visible");
    assert_eq!(queue.full_size(), 5);

    queue.process_commands().unwrap();

    assert_eq!(queue.full_size(), 0);
    for y in 0..5 {
        assert_eq!(pixel(&main, 1, y), rgba(RED), "row {y} was drawn");
    }
}
