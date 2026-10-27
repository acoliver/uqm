use uqm_rust::callback::ffi::{
    Alarm_addAbsoluteMs, Alarm_init, Alarm_processOne, Alarm_uninit, Async_process,
};
use uqm_rust::time::{counter_abi::GetTimeCounter, source};

#[test]
fn replay_alarms_follow_game_time_while_host_time_passes() {
    source::install(true);
    Alarm_init();
    let (tx, rx) = std::sync::mpsc::channel();
    for deadline in [30, 10, 20] {
        Alarm_addAbsoluteMs(
            deadline,
            {
                let tx = tx.clone();
                move |_| tx.send(deadline).unwrap()
            },
            (),
        );
    }
    std::thread::sleep(std::time::Duration::from_millis(40));
    assert_eq!(GetTimeCounter(), 0);
    assert!(!Alarm_processOne());
    source::advance_to(17); // 20 ms, truncated from 840-Hz game ticks.
    assert!(Alarm_processOne());
    assert_eq!(rx.try_recv(), Ok(10));
    assert!(Alarm_processOne());
    assert_eq!(rx.try_recv(), Ok(20));
    assert!(!Alarm_processOne());
    let before = GetTimeCounter();
    Async_process();
    assert_eq!(GetTimeCounter(), before);
    std::thread::sleep(std::time::Duration::from_millis(10));
    assert!(!Alarm_processOne());
    source::advance_to(26);
    assert!(Alarm_processOne());
    assert_eq!(rx.try_recv(), Ok(30));
    assert!(!Alarm_processOne());
    let before = GetTimeCounter();
    let steps = source::steps();
    // DoInput yields here even when its callback (communication responses)
    // has no frame-pacing sleep of its own.
    for _ in 0..10_001 {
        uqm_rust::mainloop::input_abi::yield_input_loop(|| panic!("native sleep during replay"));
        assert!(GetTimeCounter() > before);
    }
    assert_eq!(GetTimeCounter(), before + 10_001);
    assert_eq!(source::steps(), steps + 10_001);
    Alarm_uninit();
}
