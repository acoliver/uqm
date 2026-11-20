#![forbid(unsafe_code)]
use uqm_rust::{engine::host as source, time::counter_abi::get_time_counter};

/// Run the earliest alarm that is due on the replay clock.
fn process_one() -> bool {
    source::dispatch().process_one_alarm(source::now_ms())
}

#[test]
fn replay_alarms_follow_game_time_while_host_time_passes() {
    uqm_rust::platform::install(true);
    let (tx, rx) = std::sync::mpsc::channel();
    for deadline in [30, 10, 20] {
        source::dispatch().add_alarm_absolute_ms(
            deadline,
            {
                let tx = tx.clone();
                move |_| tx.send(deadline).unwrap()
            },
            (),
        );
    }
    std::thread::sleep(std::time::Duration::from_millis(40));
    assert_eq!(get_time_counter(), 0);
    assert!(!process_one());
    source::advance_to(17); // 20 ms, truncated from 840-Hz game ticks.
    assert!(process_one());
    assert_eq!(rx.try_recv(), Ok(10));
    assert!(process_one());
    assert_eq!(rx.try_recv(), Ok(20));
    assert!(!process_one());
    let before = get_time_counter();
    source::process_async();
    assert_eq!(get_time_counter(), before);
    std::thread::sleep(std::time::Duration::from_millis(10));
    assert!(!process_one());
    source::advance_to(26);
    assert!(process_one());
    assert_eq!(rx.try_recv(), Ok(30));
    assert!(!process_one());
    let before = get_time_counter();
    let steps = source::steps();
    // DoInput yields here even when its callback (communication responses)
    // has no frame-pacing sleep of its own.
    for _ in 0..10_001 {
        uqm_rust::threading::pacing::yield_input_loop(|| panic!("native sleep during replay"));
        assert!(get_time_counter() > before);
    }
    assert_eq!(get_time_counter(), before + 10_001);
    assert_eq!(source::steps(), steps + 10_001);
    source::dispatch().clear();
}
