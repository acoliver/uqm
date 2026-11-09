#[cfg(not(test))]
use crate::comm::live::comandr as live;
use crate::comm::locdata::*;
use crate::comm::races::comandr::*;
use crate::comm::races::script::{animation, as_comm_data};
use crate::comm::{types::CommData, Comm};
use crate::engine::battle::Cx;
pub(crate) fn descriptor() -> CLocData {
    let mut desc = CLocData {
        alien_frame_res: c"comm.commander.graphics".as_ptr(),
        alien_font_res: c"comm.commander.font".as_ptr(),
        alien_colormap_res: c"comm.commander.colortable".as_ptr(),
        alien_song_res: c"comm.commander.music".as_ptr(),
        conversation_phrases_res: c"comm.commander.dialogue".as_ptr(),
        alien_text_fcolor: CColor {
            r: 255,
            g: 255,
            b: 255,
            a: 255,
        },
        alien_text_bcolor: CColor {
            r: 0,
            g: 0,
            b: 0,
            a: 255,
        },
        alien_text_baseline: CPoint { x: 164, y: 20 },
        alien_text_width: 143,
        alien_text_align: 1,
        alien_text_valign: 1,
        num_animations: 3,
        alien_talk_desc: animation(
            4,
            6,
            0,
            (ONE_SECOND / 10, ONE_SECOND / 15),
            (ONE_SECOND * 7 / 60, ONE_SECOND / 12),
        ),
        ..CLocData::default()
    };
    desc.alien_ambient_array[0] =
        animation(1, 3, 1 << 2, (ONE_SECOND / 15, 0), (0, ONE_SECOND * 8));
    desc.alien_ambient_array[1] =
        animation(10, 30, 1 << 1, (ONE_SECOND / 40, 0), (ONE_SECOND * 2, 0));
    desc.alien_ambient_array[2] = animation(
        1,
        3,
        1 | (1 << 4),
        (0, ONE_SECOND / 30),
        (0, ONE_SECOND / 15),
    );
    #[cfg(not(test))]
    {
        desc.init_encounter_func = Some(live::intro);
        desc.post_encounter_func = Some(live::post);
        desc.uninit_encounter_func = Some(live::uninit);
    }
    desc
}

pub(crate) fn init(_comm: &mut Comm, cx: &mut Cx<'_>) -> CommData {
    let mut desc = descriptor();
    configure(
        &mut desc,
        cx.world.story_state(StoryKey::RADIOACTIVES_PROVIDED) != 0,
    );
    as_comm_data(&desc)
}

pub(crate) fn intro(_comm: &mut Comm, _cx: &mut Cx<'_>) {
    #[cfg(not(test))]
    live::intro(_comm, _cx);
}

pub(crate) fn post_encounter(_comm: &mut Comm, _cx: &mut Cx<'_>) {}
