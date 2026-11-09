#[cfg(not(test))]
use crate::comm::live::urquan as live;
use crate::comm::locdata::*;
use crate::comm::races::script::{animation, as_comm_data};
use crate::comm::races::urquan::*;
use crate::comm::{types::CommData, Comm};
use crate::engine::battle::Cx;
pub(crate) fn descriptor(screen_width: i32) -> CLocData {
    let (baseline, width) = text_layout(screen_width);
    let mut d = CLocData {
        alien_frame_res: c"comm.urquan.graphics".as_ptr(),
        alien_font_res: c"comm.urquan.font".as_ptr(),
        alien_colormap_res: c"comm.urquan.colortable".as_ptr(),
        alien_song_res: c"comm.urquan.music".as_ptr(),
        conversation_phrases_res: c"comm.urquan.dialogue".as_ptr(),
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
        num_animations: 7,
        ..CLocData::default()
    };
    d.alien_ambient_array[0] = animation(7, 6, 1 << 1, (84, 0), (84, 0));
    for (index, start) in [13, 16, 19].into_iter().enumerate() {
        d.alien_ambient_array[index + 1] = animation(start, 3, 1 << 2, (42, 0), (840, 2520));
    }
    for (index, (start, frames)) in [(22, 7), (29, 7), (36, 8)].into_iter().enumerate() {
        d.alien_ambient_array[index + 4] = animation(start, frames, 1 << 1, (56, 0), (840, 2520));
    }
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
        unsafe { as_comm_data(&live::init_urquan_comm(_comm, _cx)) }
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
