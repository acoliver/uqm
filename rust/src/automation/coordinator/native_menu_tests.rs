use super::*;

fn records(root: &std::path::Path) -> Vec<TraceRecord> {
    std::fs::read_to_string(root.join("trace.jsonl"))
        .unwrap()
        .lines()
        .map(|line| TraceRecord::from_jsonl(line).unwrap())
        .collect()
}

#[test]
fn committed_uniform_menu_fixture_cannot_release_the_native_checkpoint() {
    let root = tempfile::tempdir().unwrap();
    let source = br#"{"version":1,"name":"black-menu-regression","steps":[{"action":"wait_for_main_menu_ready"},{"action":"finish"}],"budgets":{"max_input_ticks":300,"max_presentations":100,"max_wallclock_seconds":60}}"#;
    let script = crate::automation::validate_script(
        crate::automation::parse_script(source, "test.json").unwrap(),
        "test.json",
    )
    .unwrap();
    let plan = crate::automation::native_checkpoint::NativeCheckpointPlan::derive(source, &script)
        .unwrap();
    let observer = AutomationObserver::init_with_native_checkpoints(
        script,
        root.path().into(),
        Some(65536),
        crate::engine::World::new().activity_control(),
        Default::default(),
        Default::default(),
    );
    observer.inner.lock().present_seen = 9;
    assert!(!observer.process_main_menu_ready());
    let image = image::load_from_memory(include_bytes!(
        "../../../tests/fixtures/native-menu-opaque-black.png"
    ))
    .unwrap()
    .to_rgba8();
    assert!(image.pixels().all(|pixel| pixel.0 == [0, 0, 0, 255]));
    let mut frame = PresentedFrame {
        count: 10,
        generation: 0,
        width: image.width(),
        height: image.height(),
        rgba: image.into_raw(),
    };
    frame.validate().unwrap();
    {
        let mut inner = observer.inner.lock();
        inner.present_seen = 10;
        assert!(!observer.write_presentation_trace(&mut inner, &frame));
        assert!(!observer.commit_native_checkpoints(&mut inner, &frame));
        assert!(
            observer.native_checkpoint_pending(&inner),
            "a black committed frame must not release the next action"
        );
        assert_eq!(inner.checkpoint_step, 0);
    }
    observer.flush_native_trace().unwrap();
    assert!(plan.committed(&records(root.path())).unwrap().is_empty());
    for color in [[255, 255, 255, 255], [50, 50, 50, 255]] {
        for pixel in frame.rgba.as_chunks_mut::<4>().0 {
            pixel.copy_from_slice(&color);
        }
        let mut inner = observer.inner.lock();
        assert!(!observer.commit_native_checkpoints(&mut inner, &frame));
        assert!(observer.native_checkpoint_pending(&inner));
    }
    frame.count = 11;
    frame.rgba[0] = 0;
    {
        let mut inner = observer.inner.lock();
        inner.present_seen = 11;
        assert!(!observer.write_presentation_trace(&mut inner, &frame));
        assert!(!observer.commit_native_checkpoints(&mut inner, &frame));
        assert!(!observer.native_checkpoint_pending(&inner));
    }
    observer.flush_native_trace().unwrap();
    assert_eq!(plan.committed(&records(root.path())).unwrap().len(), 1);
}
