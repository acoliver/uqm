//! Existing resource descriptor factories used by the conversation adapters.

pub(crate) mod arilou;
pub(crate) mod blackur;
pub(crate) mod chmmr;
pub(crate) mod comandr;
pub(crate) mod druuge;
pub(crate) mod ilwrath;
pub(crate) mod melnorm;
pub(crate) mod mycon;
pub(crate) mod orz;
pub(crate) mod pkunk;
pub(crate) mod rebel;
pub(crate) mod shofixt;
pub(crate) mod slyhome;
pub(crate) mod slyland;
pub(crate) mod spahome;
pub(crate) mod spathi;
pub(crate) mod starbas;
pub(crate) mod supox;
pub(crate) mod syreen;
pub(crate) mod talkpet;
pub(crate) mod thradd;
pub(crate) mod umgah;
pub(crate) mod urquan;
pub(crate) mod utwig;
pub(crate) mod vux;
pub(crate) mod yehat;
pub(crate) mod zoqfot;

use crate::comm::{
    races::{DialogueHost, DialogueKind},
    Comm,
};
use crate::engine::battle::Cx;

struct Lifecycle {
    init: fn(&mut Comm, &mut Cx<'_>) -> crate::comm::types::CommData,
    intro: fn(&mut Comm, &mut Cx<'_>),
    post: fn(&mut Comm, &mut Cx<'_>),
    uninit: Option<fn(&mut Comm, &mut Cx<'_>) -> u32>,
}

impl Lifecycle {
    const fn new(
        init: fn(&mut Comm, &mut Cx<'_>) -> crate::comm::types::CommData,
        intro: fn(&mut Comm, &mut Cx<'_>),
        post: fn(&mut Comm, &mut Cx<'_>),
        uninit: Option<fn(&mut Comm, &mut Cx<'_>) -> u32>,
    ) -> Self {
        Self {
            init,
            intro,
            post,
            uninit,
        }
    }
}

const LIFECYCLES: [Lifecycle; DialogueKind::COUNT] = [
    Lifecycle::new(blackur::init, blackur::intro, blackur::post_encounter, None),
    Lifecycle::new(chmmr::init, chmmr::intro, chmmr::post_encounter, None),
    Lifecycle::new(
        yehat::init,
        yehat::intro,
        yehat::post_encounter,
        Some(yehat::uninit),
    ),
    Lifecycle::new(umgah::init, umgah::intro, umgah::post_encounter, None),
    Lifecycle::new(mycon::init, mycon::intro, mycon::post_encounter, None),
    Lifecycle::new(talkpet::init, talkpet::intro, talkpet::post_encounter, None),
    Lifecycle::new(utwig::init, utwig::intro, utwig::post_encounter, None),
    Lifecycle::new(druuge::init, druuge::intro, druuge::post_encounter, None),
    Lifecycle::new(syreen::init, syreen::intro, syreen::post_encounter, None),
    Lifecycle::new(orz::init, orz::intro, orz::post_encounter, None),
    Lifecycle::new(spahome::init, spahome::intro, spahome::post_encounter, None),
    Lifecycle::new(ilwrath::init, ilwrath::intro, ilwrath::post_encounter, None),
    Lifecycle::new(starbas::init, starbas::intro, starbas::post_encounter, None),
    Lifecycle::new(slyhome::init, slyhome::intro, slyhome::post_encounter, None),
    Lifecycle::new(arilou::init, arilou::intro, arilou::post_encounter, None),
    Lifecycle::new(supox::init, supox::intro, supox::post_encounter, None),
    Lifecycle::new(shofixt::init, shofixt::intro, shofixt::post_encounter, None),
    Lifecycle::new(pkunk::init, pkunk::intro, pkunk::post_encounter, None),
    Lifecycle::new(rebel::init, rebel::intro, rebel::post_encounter, None),
    Lifecycle::new(slyland::init, slyland::intro, slyland::post_encounter, None),
    Lifecycle::new(
        vux::init,
        vux::intro,
        vux::post_encounter,
        Some(vux::uninit),
    ),
    Lifecycle::new(melnorm::init, melnorm::intro, melnorm::post_encounter, None),
    Lifecycle::new(zoqfot::init, zoqfot::intro, zoqfot::post_encounter, None),
    Lifecycle::new(comandr::init, comandr::intro, comandr::post_encounter, None),
    Lifecycle::new(
        urquan::init,
        urquan::intro,
        urquan::post_encounter,
        Some(urquan::uninit),
    ),
    Lifecycle::new(thradd::init, thradd::intro, thradd::post_encounter, None),
    Lifecycle::new(spathi::init, spathi::intro, spathi::post_encounter, None),
];

impl DialogueHost for Comm {
    fn initialize(&mut self, kind: DialogueKind, cx: &mut Cx<'_>) {
        let data = (LIFECYCLES[kind as usize].init)(self, cx);
        self.set_comm_data(data);
    }
    fn intro(&mut self, kind: DialogueKind, cx: &mut Cx<'_>) {
        (LIFECYCLES[kind as usize].intro)(self, cx);
    }
    fn post_encounter(&mut self, kind: DialogueKind, cx: &mut Cx<'_>) {
        (LIFECYCLES[kind as usize].post)(self, cx);
    }
    fn uninit(&mut self, kind: DialogueKind, cx: &mut Cx<'_>) -> u32 {
        (LIFECYCLES[kind as usize].uninit).map_or(0, |callback| callback(self, cx))
    }
}

#[cfg(test)]
#[test]
#[serial_test::serial]
fn host_lifecycle_runs_each_registered_race() {
    crate::comm::with_test_context(|cx| {
        let mut first = Comm::new();
        let entry = &LIFECYCLES[14];
        let copied = Lifecycle::new(entry.init, entry.intro, entry.post, entry.uninit);
        assert!(std::ptr::fn_addr_eq(copied.init, entry.init));
        for race in (0..=25).filter_map(crate::comm::races::get_race_dialogue) {
            race.init(&mut first, cx);
            assert!(!first.comm_data().unwrap().alien_frame_res.is_null());
            race.intro(&mut first, cx);
            race.post_encounter(&mut first, cx);
            assert_eq!(race.uninit(&mut first, cx), 0);
        }
    });
}
