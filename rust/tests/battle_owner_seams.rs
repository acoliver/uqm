#![forbid(unsafe_code)]

use uqm_rust::engine::{
    battle::{Battle, Cx},
    Game,
};
use uqm_rust::navigation::{hyper_units::Geometry, hyperspace::move_sis};

#[test]
#[serial_test::serial]
fn movement_clips_displacement_from_an_unpublished_context() {
    let mut game = Game::new();
    let mut other = Game::new();
    other.world.data.flagship.log_x = 77;
    let mut battle = Battle::new();
    let mut cx = Cx::new(
        &mut battle,
        &mut game.world,
        &mut game.kernel,
        uqm_rust::math::random,
    );
    let (mut dx, mut dy) = (1, 1);
    move_sis(
        &mut cx,
        &mut dx,
        &mut dy,
        Geometry::new((320, 240)),
        |_, _| panic!("zero fuel must not debit"),
    );
    assert_eq!((dx, dy), (0, 0));
    assert_eq!(
        (cx.world.data.flagship.log_x, cx.world.data.flagship.log_y),
        (0, 0)
    );
    cx.world.data.flagship.log_x = 1;
    cx.world.data.flagship.log_y = 1;
    (dx, dy) = (2, 2);
    move_sis(
        &mut cx,
        &mut dx,
        &mut dy,
        Geometry::new((320, 240)),
        |_, _| panic!("zero fuel must not debit"),
    );
    assert_eq!((dx, dy), (1, 1));
    assert_eq!(
        (cx.world.data.flagship.log_x, cx.world.data.flagship.log_y),
        (0, 0)
    );
    assert_eq!(other.world.data.flagship.log_x, 77);
}

#[test]
#[serial_test::serial]
fn unthrottled_runtime_processes_callbacks_without_sampling_or_flushing_input() {
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    };
    use uqm_rust::callback::ffi::{Callback_add, Callback_init, Callback_uninit};
    use uqm_rust::engine::input::{frames::KEY_MENU_SELECT, InputControls, InputRuntime};
    let mut game = Game::new();
    let controls = InputControls::default();
    controls.set_menu_key(KEY_MENU_SELECT as usize, true);
    controls.advance(Default::default(), controls.immediate(), 0);
    controls.set_menu_key(KEY_MENU_SELECT as usize, false);
    let current = controls.current();
    let pulsed = controls.pulsed();
    let runtime = std::rc::Rc::new(uqm_rust::mainloop::input_abi::NativeInputRuntime::new(
        None,
        controls.clone(),
        game.kernel.configuration.clone(),
    ));
    game.kernel.install_input_runtime(runtime.clone());
    let mut battle = Battle::new();
    let mut cx = Cx::new(
        &mut battle,
        &mut game.world,
        &mut game.kernel,
        uqm_rust::math::random,
    );
    let called = Arc::new(AtomicBool::new(false));
    Callback_init();
    Callback_add(
        |flag: Arc<AtomicBool>| flag.store(true, Ordering::SeqCst),
        called.clone(),
    )
    .unwrap();
    runtime.yield_poll(&mut cx);
    Callback_uninit();
    assert!(called.load(Ordering::SeqCst));
    assert_eq!(controls.current(), current);
    assert_eq!(controls.pulsed(), pulsed);
    assert_eq!(controls.immediate().menu[KEY_MENU_SELECT as usize], 0);
}
