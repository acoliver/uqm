#[cfg(not(test))]
use crate::comm::live::blackur as live;
use crate::comm::locdata::*;
use crate::comm::races::blackur::*;
use crate::comm::races::script::{animation, as_comm_data};
use crate::comm::{types::CommData, Comm};
use crate::engine::battle::Cx;
pub(crate) fn descriptor(screen_width: i32) -> CLocData {
    let (baseline, width) = text_layout(screen_width);
    let mut d = CLocData {
        alien_frame_res: c"comm.kohrah.graphics".as_ptr(),
        alien_font_res: c"comm.kohrah.font".as_ptr(),
        alien_colormap_res: c"comm.kohrah.colortable".as_ptr(),
        alien_song_res: c"comm.kohrah.music".as_ptr(),
        conversation_phrases_res: c"comm.kohrah.dialogue".as_ptr(),
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
        alien_text_baseline: baseline,
        alien_text_width: width,
        alien_text_align: 1,
        alien_text_valign: 0,
        num_animations: 8,
        ..CLocData::default()
    };
    d.alien_ambient_array[0] = animation(7, 6, 2, (56, 0), (840, 2520));
    d.alien_ambient_array[1] = animation(13, 7, 2, (56, 0), (840, 2520));
    for (i, s) in [20, 23, 26].into_iter().enumerate() {
        d.alien_ambient_array[i + 2] = animation(s, 3, 4, (56, 0), (840, 2520));
    }
    d.alien_ambient_array[5] = animation(29, 4, 1, (84, 0), (84, 0));
    d.alien_ambient_array[6] = animation(33, 5, 0x0a, (84, 0), (840, 2520));
    d.alien_ambient_array[7] = animation(38, 4, 2, (84, 0), (840, 2520));
    d.alien_transition_desc = animation(1, 2, 0, (140, 0), (0, 0));
    d.alien_talk_desc = animation(2, 5, 0, (56, 0), (70, 0));
    #[cfg(not(test))]
    {
        d.init_encounter_func = Some(live::intro);
        d.post_encounter_func = Some(live::post);
        d.uninit_encounter_func = Some(live::uninit);
    }
    d
}

pub(crate) fn init(_comm: &mut Comm, _cx: &mut Cx<'_>) -> CommData {
    #[cfg(not(test))]
    {
        // SAFETY: initialization holds the exclusive game-thread session and Cx loans.
        unsafe { as_comm_data(&live::init_blackurq_comm(_comm, _cx)) }
    }
    #[cfg(test)]
    {
        as_comm_data(&descriptor(320))
    }
}

pub(crate) fn intro(_comm: &mut Comm, _cx: &mut Cx<'_>) {
    #[cfg(not(test))]
    live::intro(_comm, _cx);
}

pub(crate) fn post_encounter(_comm: &mut Comm, _cx: &mut Cx<'_>) {
    #[cfg(not(test))]
    live::post(_comm, _cx);
}
