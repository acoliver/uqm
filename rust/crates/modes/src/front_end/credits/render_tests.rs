use super::*;
use uqm_engine::graphics::{geometry::Extent, geometry::Point, geometry::Rect};
use uqm_engine::Game;

#[test]
fn records_keep_row_first_column_order_and_bounded_formats() {
    let parsed = record(b"17 L/2,C,R/77 A\tB\tC\nD\tE").unwrap();
    assert_eq!((parsed.size, parsed.rows), (17, 2));
    let lines: Vec<_> = parsed
        .lines
        .iter()
        .map(|line| (line.text.to_bytes(), line.column, line.row))
        .collect();
    assert_eq!(
        lines,
        vec![
            (b"A".as_slice(), 0, 1),
            (b"D", 0, 2),
            (b"B", 1, 1),
            (b"C", 2, 1),
            (b"E", 1, 2)
        ]
    );
    let formats = formats(&parsed.alignment, 320, 8);
    assert_eq!(
        (formats[0].align, formats[0].x),
        (uqm_engine::graphics::geometry::TextAlign::Left, 2)
    );
    assert_eq!(
        (formats[1].align, formats[1].x),
        (uqm_engine::graphics::geometry::TextAlign::Center, 160)
    );
    assert_eq!(
        (formats[2].align, formats[2].x),
        (uqm_engine::graphics::geometry::TextAlign::Right, 77)
    );
    assert_eq!(
        (formats[3].align, formats[3].x),
        (uqm_engine::graphics::geometry::TextAlign::Left, 5)
    );
    for bytes in [b"".as_slice(), b"0 L A", b"13", b"13 L", b"bad L A"] {
        assert!(record(bytes).is_none());
    }
    let record = record(b"13 L,C,R,L,R A\tB\tC\tD\tE\tF").unwrap();
    assert_eq!(record.lines.last().unwrap().text.to_bytes(), b"E\tF");
}

#[test]
#[serial_test::serial]
fn renderer_draws_pixels_and_releases_hook_state_on_unwind() {
    if !uqm_engine::test_support::native_process::own_graphics_process("front_end::credits::render::tests::renderer_draws_pixels_and_releases_hook_state_on_unwind") { return; }
    let mut game = Game::new();
    let mut loaded = None;
    let fixture =
        uqm_engine::test_support::campaign_display::CampaignDisplay::install_with_resources(
            &mut game,
            || {
                loaded = Resources::load();
            },
        )
        .unwrap();
    let loaded = loaded.unwrap();
    assert_eq!(loaded.table.len(), 1);
    assert!(loaded.background.is_some());
    assert!(loaded.fonts.iter().all(Option::is_some));
    let context = ContextLease::require(game.kernel.status_context);
    context.set_frame(ContextFrame::retain(game.kernel.screen));
    context.set_clip(Rect {
        corner: Point { x: 0, y: 0 },
        extent: Extent {
            width: 320,
            height: 240,
        },
    });
    let background = LoadedDrawable::create(2, 320, 240, 1).unwrap();
    context.with_frame(background.lease().context_frame(0), || {
        context.set_background(crate::ui::rgb15(31, 0, 0));
        context.clear();
    });
    let resources = Rc::new(Resources {
        table: LoadedTextTable::new(vec![
            c"".to_owned(),
            c"13 L/10 A".to_owned(),
            c"17 C A".to_owned(),
            c"45 R A".to_owned(),
            c"46 L A".to_owned(),
        ]),
        background: Some(background),
        fonts: loaded.fonts,
    });
    let window = Rc::new(Cell::new(CRect {
        corner: Point { x: 7, y: 5 },
        width: 306,
        height: 80,
    }));
    let session = Rc::new(RefCell::new(Session::new(
        &context,
        resources.clone(),
        true,
        window.get(),
    )));
    let independent = Session::new(&context, resources.clone(), false, window.get());
    {
        let mut scroll = session.borrow_mut();
        scroll.process_frame(&window, &mut |_| {});
        assert_eq!((scroll.first, scroll.last, scroll.offset), (0, 2, 1));
        scroll.controls.rate = -36;
        scroll.next_time = 0;
        scroll.process_frame(&window, &mut |_| {});
        assert_eq!(
            (
                scroll.first,
                scroll.last,
                scroll.offset,
                scroll.total_height
            ),
            (0, 1, 0, 240)
        );
        scroll.controls.rate = 9;
    }
    let mut index = 0;
    let text = session.borrow().render_text_frame(&mut index, 1).unwrap();
    assert_eq!(index, 2);
    let frame = text.frame();
    context.flush();
    let extent = frame.rect().extent;
    assert!((0..extent.height).any(|y| (0..extent.width).any(|x| {
        let pixel = frame.pixel(Point { x, y });
        (pixel.r, pixel.g, pixel.b) == (255, 255, 255)
    })));
    for (start, direction, expected) in [
        (2, 1, true),
        (3, 1, true),
        (4, 1, false),
        (-2, -1, false),
        (5, -1, false),
    ] {
        let mut index = start;
        assert_eq!(
            session
                .borrow()
                .render_text_frame(&mut index, direction)
                .is_some(),
            expected
        );
    }
    session.borrow_mut().controls.outtakes = true;
    session.borrow().render_screen(window.get());
    context.flush();
    let composed = session.borrow().frame.lease().context_frame(0);
    assert_eq!(composed.pixel(Point { x: 0, y: 0 }).r, 255);
    assert_eq!(composed.pixel(Point { x: 8, y: 6 }).b, 255);
    assert_eq!(independent.controls.rate, 9);
    let input = InputSession::default();
    let calls = Rc::new(Cell::new(0));
    let previous = calls.clone();
    input.replace_callback(Some(InputFrameHook::new(move |_, _| {
        previous.set(previous.get() + 1)
    })));
    let retained = Rc::downgrade(&session);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _hook = HookRestore::install(&input, session.clone(), window.clone());
        input
            .callback()
            .unwrap()
            .call(&mut |_| {}, input.controls());
        panic!("credits fixture unwind");
    }));
    assert!(result.is_err());
    input
        .callback()
        .unwrap()
        .call(&mut |_| {}, input.controls());
    assert_eq!(calls.get(), 1);
    drop(input.replace_callback(None));
    drop(frame);
    drop(text);
    drop(composed);
    drop(session);
    assert!(retained.upgrade().is_none());
    drop(independent);
    drop(resources);
    drop(context);
    fixture.finish(game);
}
