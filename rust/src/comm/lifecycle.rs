//! Mode-side dialogue lifecycle routing.

use crate::comm::races::descriptor::CommDescriptor;
use crate::comm::{
    live,
    locdata::{session_view, SessionData},
    races, Comm,
};
use crate::engine::{battle::Cx, StoryKey};

struct Lifecycle {
    definition: fn(&Cx<'_>) -> CommDescriptor,
    initialize: fn(&mut Comm, &mut Cx<'_>, &mut CommDescriptor),
    intro: fn(&mut Comm, &mut Cx<'_>),
    post: fn(&mut Comm, &mut Cx<'_>),
    uninit: fn(&mut Comm, &mut Cx<'_>) -> u16,
}

const LIFECYCLES: [Lifecycle; DialogueKind::COUNT] = [
    Lifecycle {
        definition: |cx| races::blackur::descriptor::descriptor(cx.screen_dimensions().0),
        initialize: live::blackur::initialize,
        intro: live::blackur::intro,
        post: live::blackur::post,
        uninit: live::blackur::uninit,
    },
    Lifecycle {
        definition: |_| races::chmmr::descriptor::descriptor(),
        initialize: live::chmmr::initialize,
        intro: live::chmmr::intro,
        post: live::chmmr::post,
        uninit: live::chmmr::uninit,
    },
    Lifecycle {
        definition: |cx| races::yehat::descriptor::descriptor(cx.screen_dimensions().0),
        initialize: live::yehat::initialize,
        intro: live::yehat::intro,
        post: live::yehat::post,
        uninit: live::yehat::uninit,
    },
    Lifecycle {
        definition: |_| races::umgah::descriptor::descriptor(),
        initialize: live::umgah::initialize,
        intro: live::umgah::intro,
        post: live::umgah::post,
        uninit: live::umgah::uninit,
    },
    Lifecycle {
        definition: |_| races::mycon::descriptor::descriptor(),
        initialize: live::mycon::initialize,
        intro: live::mycon::intro,
        post: live::mycon::post,
        uninit: live::mycon::uninit,
    },
    Lifecycle {
        definition: |_| races::talkpet::descriptor::descriptor(),
        initialize: live::talkpet::initialize,
        intro: live::talkpet::intro,
        post: live::talkpet::post,
        uninit: live::talkpet::uninit,
    },
    Lifecycle {
        definition: |_| races::utwig::descriptor::descriptor(),
        initialize: live::utwig::initialize,
        intro: live::utwig::intro,
        post: live::utwig::post,
        uninit: live::utwig::uninit,
    },
    Lifecycle {
        definition: |_| races::druuge::descriptor::descriptor(),
        initialize: live::druuge::initialize,
        intro: live::druuge::intro,
        post: live::druuge::post,
        uninit: live::druuge::uninit,
    },
    Lifecycle {
        definition: |_| races::syreen::descriptor::descriptor(),
        initialize: live::syreen::initialize,
        intro: live::syreen::intro,
        post: live::syreen::post,
        uninit: live::syreen::uninit,
    },
    Lifecycle {
        definition: |_| races::orz::descriptor::descriptor(),
        initialize: live::orz::initialize,
        intro: live::orz::intro,
        post: live::orz::post,
        uninit: live::orz::uninit,
    },
    Lifecycle {
        definition: |cx| races::spahome::descriptor::descriptor(cx.screen_dimensions().0),
        initialize: live::spahome::initialize,
        intro: live::spahome::intro,
        post: live::spahome::post,
        uninit: live::spahome::uninit,
    },
    Lifecycle {
        definition: |_| races::ilwrath::descriptor::descriptor(),
        initialize: live::ilwrath::initialize,
        intro: live::ilwrath::intro,
        post: live::ilwrath::post,
        uninit: live::ilwrath::uninit,
    },
    Lifecycle {
        definition: |_| races::starbas::descriptor::descriptor(),
        initialize: live::starbas::initialize,
        intro: live::starbas::intro,
        post: live::starbas::post,
        uninit: live::starbas::uninit,
    },
    Lifecycle {
        definition: |cx| races::slyhome::descriptor::descriptor(cx.screen_dimensions().0),
        initialize: live::slyhome::initialize,
        intro: live::slyhome::intro,
        post: live::slyhome::post,
        uninit: live::slyhome::uninit,
    },
    Lifecycle {
        definition: |_| races::arilou::descriptor::descriptor(),
        initialize: live::arilou::initialize,
        intro: live::arilou::intro,
        post: live::arilou::post,
        uninit: live::arilou::uninit,
    },
    Lifecycle {
        definition: |_| races::supox::descriptor::descriptor(),
        initialize: live::supox::initialize,
        intro: live::supox::intro,
        post: live::supox::post,
        uninit: live::supox::uninit,
    },
    Lifecycle {
        definition: |cx| races::shofixt::descriptor::descriptor(cx.screen_dimensions().0),
        initialize: live::shofixt::initialize,
        intro: live::shofixt::intro,
        post: live::shofixt::post,
        uninit: live::shofixt::uninit,
    },
    Lifecycle {
        definition: |_| races::pkunk::descriptor::descriptor(),
        initialize: live::pkunk::initialize,
        intro: live::pkunk::intro,
        post: live::pkunk::post,
        uninit: live::pkunk::uninit,
    },
    Lifecycle {
        definition: |cx| races::rebel::descriptor::descriptor(cx.screen_dimensions().0),
        initialize: live::rebel::initialize,
        intro: live::rebel::intro,
        post: live::rebel::post,
        uninit: live::rebel::uninit,
    },
    Lifecycle {
        definition: |cx| races::slyland::descriptor::descriptor(cx.screen_dimensions().0),
        initialize: live::slyland::initialize,
        intro: live::slyland::intro,
        post: live::slyland::post,
        uninit: live::slyland::uninit,
    },
    Lifecycle {
        definition: |cx| races::vux::descriptor::descriptor(cx.screen_dimensions().0),
        initialize: live::vux::initialize,
        intro: live::vux::intro,
        post: live::vux::post,
        uninit: live::vux::uninit,
    },
    Lifecycle {
        definition: |_| races::melnorm::descriptor::descriptor(),
        initialize: live::melnorm::initialize,
        intro: live::melnorm::intro,
        post: live::melnorm::post,
        uninit: live::melnorm::uninit,
    },
    Lifecycle {
        definition: |_| races::zoqfot::descriptor::descriptor(),
        initialize: live::zoqfot::initialize,
        intro: live::zoqfot::intro,
        post: live::zoqfot::post,
        uninit: live::zoqfot::uninit,
    },
    Lifecycle {
        definition: |cx| {
            let mut desc = races::comandr::descriptor::descriptor();
            races::comandr::configure(
                &mut desc,
                cx.world.story_state(StoryKey::RADIOACTIVES_PROVIDED) != 0,
            );
            desc
        },
        initialize: live::comandr::initialize,
        intro: live::comandr::intro,
        post: live::comandr::post,
        uninit: live::comandr::uninit,
    },
    Lifecycle {
        definition: |cx| races::urquan::descriptor::descriptor(cx.screen_dimensions().0),
        initialize: live::urquan::initialize,
        intro: live::urquan::intro,
        post: live::urquan::post,
        uninit: live::urquan::uninit,
    },
    Lifecycle {
        definition: |_| races::thradd::descriptor::descriptor(),
        initialize: live::thradd::initialize,
        intro: live::thradd::intro,
        post: live::thradd::post,
        uninit: live::thradd::uninit,
    },
    Lifecycle {
        definition: |_| races::spathi::descriptor::descriptor(),
        initialize: live::spathi::initialize,
        intro: live::spathi::intro,
        post: live::spathi::post,
        uninit: live::spathi::uninit,
    },
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(usize)]
pub(super) enum DialogueKind {
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
pub(super) fn for_conversation(conversation: u32) -> Option<DialogueKind> {
    CONVERSATIONS.get(conversation as usize).copied().flatten()
}

pub(crate) fn initialize_conversation(comm: &mut Comm, cx: &mut Cx<'_>, conversation: u32) {
    if let Some(kind) = for_conversation(conversation) {
        initialize(comm, kind, cx);
    }
}

fn initialize(comm: &mut Comm, kind: DialogueKind, cx: &mut Cx<'_>) {
    let data = if kind == DialogueKind::Comandr {
        // Commander historically configures its first pass without executing the script.
        SessionData::new((LIFECYCLES[kind as usize].definition)(cx))
    } else {
        session(comm, cx, kind)
    };
    comm.set_comm_data(session_view(&data));
}

pub(super) fn session(comm: &mut Comm, cx: &mut Cx<'_>, kind: DialogueKind) -> SessionData {
    let entry = &LIFECYCLES[kind as usize];
    let mut definition = (entry.definition)(cx);
    (entry.initialize)(comm, cx, &mut definition);
    SessionData {
        definition,
        init_encounter_func: Some(entry.intro),
        post_encounter_func: Some(entry.post),
        uninit_encounter_func: Some(entry.uninit),
        ..SessionData::default()
    }
}

#[cfg(test)]
mod tests;
