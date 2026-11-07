use super::*;
#[cfg(not(test))]
use crate::battle::context_access::race_access::RaceOps;
use crate::engine::{
    battle::{input::InputContext, net::ProtocolNoticeKind, Battle},
    input::{Frame, InputRuntime, InputSession},
    observe::{Observer, Unobserved},
    Game,
};
use std::{cell::RefCell, rc::Rc};

struct Runtime {
    input: InputSession,
    resets: Rc<RefCell<Vec<bool>>>,
}

impl InputRuntime for Runtime {
    fn input(&self) -> &InputSession {
        &self.input
    }

    fn observer(&self) -> Box<dyn Observer<Option<crate::engine::observe::CampaignView>>> {
        Box::new(Unobserved)
    }

    fn update(&self, _: &mut Cx<'_>) {}

    fn run(&self, cx: &mut Cx<'_>, io: &mut Io<'_>, reset: bool, frame: &mut dyn Frame) {
        self.resets.borrow_mut().push(reset);
        while frame.frame(cx, io) {}
    }

    fn battle_input(&self, _: &InputContext) -> u8 {
        0
    }
}

#[test]
fn notices_and_wait_polls_borrow_the_same_live_world_synchronously() {
    let mut game = Game::new();
    let resets = Rc::new(RefCell::new(Vec::new()));
    game.kernel.install_input_runtime(Rc::new(Runtime {
        input: InputSession::default(),
        resets: resets.clone(),
    }));
    let mut battle = Battle::new();
    let mut cx = Cx::new(
        &mut battle,
        &mut game.world,
        &mut game.kernel,
        crate::math::random,
    );
    let mut observer = Unobserved;
    let mut io = Io {
        observer: &mut observer,
    };
    let notice = ProtocolNotice {
        player: 1,
        battle_active: true,
        kind: ProtocolNoticeKind::ShipSelected,
    };
    let events = RefCell::new(Vec::new());
    let mut receive = |cx: &mut Cx<'_>, _: &mut Io<'_>, event| {
        assert_eq!(event, notice);
        events.borrow_mut().push(("notice", cx.world.activity()));
        cx.world.set_activity(cx.world.activity() + 1);
    };
    let mut polls = 0;
    {
        let mut driver = Driver::new(&mut cx, &mut io, &mut receive);
        driver.notice(notice);
        driver.wait(&mut |frame| {
            polls += 1;
            events.borrow_mut().push(("poll", polls));
            frame.notice(notice);
            polls < 2
        });
    }
    assert_eq!(cx.world.activity(), 3);
    assert_eq!(polls, 2);
    assert_eq!(*resets.borrow(), [false]);
    assert_eq!(
        *events.borrow(),
        [
            ("notice", 0),
            ("poll", 1),
            ("notice", 1),
            ("poll", 2),
            ("notice", 2)
        ]
    );
}
