//! Mode-side dialogue lifecycle routing.

use crate::comm::{descriptors::*, Comm};
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(usize)]
pub enum DialogueKind {
    Blackur,
    Chmmr,
    Yehat,
    Umgah,
    Mycon,
    Talkpet,
    Utwig,
    Druuge,
    Syreen,
    Orz,
    Spahome,
    Ilwrath,
    Starbas,
    Slyhome,
    Arilou,
    Supox,
    Shofixt,
    Pkunk,
    Rebel,
    Slyland,
    Vux,
    Melnorm,
    Zoqfot,
    Comandr,
    Urquan,
    Thradd,
    Spathi,
}

impl DialogueKind {
    pub(crate) const COUNT: usize = 27;
}

const CONVERSATIONS: [Option<DialogueKind>; 26] = {
    use DialogueKind::*;
    [
        Some(Arilou),
        Some(Chmmr),
        Some(Comandr),
        Some(Orz),
        Some(Pkunk),
        Some(Shofixt),
        Some(Spathi),
        Some(Supox),
        Some(Thradd),
        Some(Utwig),
        Some(Vux),
        Some(Yehat),
        Some(Melnorm),
        Some(Druuge),
        Some(Ilwrath),
        Some(Mycon),
        Some(Slyland),
        Some(Umgah),
        Some(Urquan),
        Some(Zoqfot),
        Some(Syreen),
        Some(Blackur),
        Some(Talkpet),
        Some(Slyhome),
        None,
        Some(Rebel),
    ]
};

/// Resolve the first initialization pass, before the conversation variants.
#[must_use]
pub fn for_conversation(conversation: u32) -> Option<DialogueKind> {
    CONVERSATIONS.get(conversation as usize).copied().flatten()
}

pub(crate) fn initialize_conversation(comm: &mut Comm, cx: &mut Cx<'_>, conversation: u32) {
    if let Some(kind) = for_conversation(conversation) {
        initialize(comm, kind, cx);
    }
}

pub fn initialize(comm: &mut Comm, kind: DialogueKind, cx: &mut Cx<'_>) {
    let data = (LIFECYCLES[kind as usize].init)(comm, cx);
    comm.set_comm_data(data);
}

pub fn intro(comm: &mut Comm, kind: DialogueKind, cx: &mut Cx<'_>) {
    (LIFECYCLES[kind as usize].intro)(comm, cx);
}

pub fn post_encounter(comm: &mut Comm, kind: DialogueKind, cx: &mut Cx<'_>) {
    (LIFECYCLES[kind as usize].post)(comm, cx);
}

pub fn uninit(comm: &mut Comm, kind: DialogueKind, cx: &mut Cx<'_>) -> u32 {
    (LIFECYCLES[kind as usize].uninit).map_or(0, |callback| callback(comm, cx))
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
        for race in (0..=25).filter_map(for_conversation) {
            initialize(&mut first, race, cx);
            assert!(!first.comm_data().unwrap().alien_frame_res.is_null());
            intro(&mut first, race, cx);
            post_encounter(&mut first, race, cx);
            assert_eq!(uninit(&mut first, race, cx), 0);
        }
    });
}

#[cfg(test)]
mod routing_tests {
    use crate::comm::lifecycle::{for_conversation, DialogueKind};
    use crate::engine::species::conv;

    #[test]
    fn conversation_table_preserves_every_first_pass_and_unknown_id() {
        use DialogueKind::*;
        let expected = [
            Some(Arilou),
            Some(Chmmr),
            Some(Comandr),
            Some(Orz),
            Some(Pkunk),
            Some(Shofixt),
            Some(Spathi),
            Some(Supox),
            Some(Thradd),
            Some(Utwig),
            Some(Vux),
            Some(Yehat),
            Some(Melnorm),
            Some(Druuge),
            Some(Ilwrath),
            Some(Mycon),
            Some(Slyland),
            Some(Umgah),
            Some(Urquan),
            Some(Zoqfot),
            Some(Syreen),
            Some(Blackur),
            Some(Talkpet),
            Some(Slyhome),
            None,
            Some(Rebel),
        ];
        for (conversation, kind) in expected.into_iter().enumerate() {
            assert_eq!(for_conversation(conversation as u32), kind);
        }
        for unknown in [conv::INVALID, conv::INVALID + 1, u32::MAX] {
            assert_eq!(for_conversation(unknown), None);
        }
    }
}

#[cfg(test)]
#[test]
fn initialization_skips_unknown_ids_without_replacing_the_session() {
    crate::comm::with_test_context(|cx| {
        let mut comm = Comm::new();
        initialize_conversation(&mut comm, cx, u32::MAX);
        assert!(comm.comm_data().is_none());
        initialize_conversation(&mut comm, cx, crate::engine::species::conv::ARILOU);
        let frame = comm.comm_data().unwrap().alien_frame_res;
        assert!(!frame.is_null());
        initialize_conversation(&mut comm, cx, crate::engine::species::conv::URQUAN_DRONE);
        assert_eq!(comm.comm_data().unwrap().alien_frame_res, frame);
    });
}
