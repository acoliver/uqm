use crate::comm::live::yehat as live;
use crate::comm::locdata::*;
use crate::comm::races::script::{animation, as_comm_data};
use crate::comm::races::yehat::*;
use crate::comm::{types::CommData, Comm};
use crate::engine::battle::Cx;
pub(crate) fn descriptor(screen_width: i32) -> CLocData {
    let (baseline, width) = text_layout(screen_width);
    let mut desc = CLocData {
        alien_frame_res: c"comm.yehat.graphics".as_ptr(),
        alien_font_res: c"comm.yehat.font".as_ptr(),
        alien_colormap_res: c"comm.yehat.colortable".as_ptr(),
        alien_song_res: c"comm.yehat.music".as_ptr(),
        conversation_phrases_res: c"comm.yehat.dialogue".as_ptr(),
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
        alien_text_valign: 1,
        num_animations: 15,
        ..CLocData::default()
    };
    let specs = [
        (
            4,
            3,
            (1 << 2) | (1 << 3),
            (84, 84),
            (210, 420),
            (1 << 6) | (1 << 7),
        ),
        (
            7,
            3,
            (1 << 2) | (1 << 3),
            (84, 84),
            (210, 420),
            (1 << 6) | (1 << 7),
        ),
        (10, 3, 1 << 2, (42, 0), (840, 2520), (1 << 4) | (1 << 14)),
        (13, 3, 1 << 2, (42, 0), (840, 2520), 1 << 5),
        (16, 5, 1 << 2, (56, 56), (5040, 2520), (1 << 2) | (1 << 14)),
        (21, 5, 1 << 2, (56, 56), (5040, 2520), 1 << 3),
        (
            26,
            2,
            (1 << 2) | (1 << 3),
            (56, 56),
            (5040, 2520),
            (1 << 0) | (1 << 1),
        ),
        (
            28,
            2,
            (1 << 2) | (1 << 3),
            (56, 56),
            (5040, 2520),
            (1 << 0) | (1 << 1),
        ),
        (30, 3, 1 << 2, (28, 28), (28, 28), 0),
        (33, 3, 1 << 2, (28, 28), (28, 28), 0),
        (36, 3, 1 << 2, (28, 28), (28, 28), 0),
        (39, 3, 1 << 2, (28, 28), (28, 28), 0),
        (42, 3, 1 << 2, (28, 28), (28, 28), 0),
        (45, 3, 1 << 2, (28, 28), (28, 28), 0),
        (
            48,
            4,
            (1 << 2) | (1 << 3),
            (28, 0),
            (840, 2520),
            (1 << 2) | (1 << 4),
        ),
    ];
    for (index, (start, frames, flags, frame_rate, restart_rate, block_mask)) in
        specs.into_iter().enumerate()
    {
        desc.alien_ambient_array[index] = animation(start, frames, flags, frame_rate, restart_rate);
        desc.alien_ambient_array[index].block_mask = block_mask;
    }
    desc.alien_talk_desc = animation(1, 3, 0, (56, 0), (70, 0));
    desc.init_encounter_func = Some(live::intro);
    desc.post_encounter_func = Some(live::post);
    desc.uninit_encounter_func = Some(live::uninit);
    desc
}

pub(crate) fn init(_comm: &mut Comm, _cx: &mut Cx<'_>) -> CommData {
    #[cfg(not(test))]
    {
        // SAFETY: initialization holds the exclusive game-thread session and Cx loans.
        unsafe { as_comm_data(&live::init_yehat_comm(_comm, _cx)) }
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

pub(crate) fn uninit(_comm: &mut Comm, _cx: &mut Cx<'_>) -> u32 {
    #[cfg(not(test))]
    return live::uninit(_comm, _cx) as u32;
    #[cfg(test)]
    0
}
