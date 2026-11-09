#[cfg(not(test))]
use crate::comm::live::vux as live;
use crate::comm::locdata::*;
use crate::comm::races::script::{animation, as_comm_data};
use crate::comm::races::vux::*;
use crate::comm::{types::CommData, Comm};
use crate::engine::battle::Cx;
pub(crate) fn descriptor(screen_width: i32) -> CLocData {
    let (baseline, width) = text_layout(screen_width);
    let mut desc = CLocData {
        alien_frame_res: c"comm.vux.graphics".as_ptr(),
        alien_font_res: c"comm.vux.font".as_ptr(),
        alien_colormap_res: c"comm.vux.colortable".as_ptr(),
        alien_song_res: c"comm.vux.music".as_ptr(),
        conversation_phrases_res: c"comm.vux.dialogue".as_ptr(),
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
        num_animations: 17,
        ..CLocData::default()
    };
    let specs = [
        (12, 3, 1 << 0, (28, 28), (28, 28), 0),
        (15, 5, 1 << 0, (28, 28), (28, 28), 0),
        (20, 14, 1 << 1, (28, 0), (28, 0), 0),
        (34, 7, 1 << 1, (28, 28), (28, 28), 0),
        (41, 6, 1 << 0, (28, 28), (28, 28), 0),
        (47, 11, 1 << 1, (28, 28), (28, 28), 0),
        (58, 3, 1 << 0, (28, 28), (28, 28), 0),
        (61, 4, 1 << 0, (28, 28), (28, 28), 0),
        (65, 4, 1 << 0, (28, 28), (28, 28), 0),
        (69, 2, 1 << 0, (28, 28), (28, 28), 0),
        (71, 3, 1 << 2, (42, 0), (840, 2520), 0),
        (74, 6, 1 << 2, (56, 56), (56, 56), 0),
        (80, 5, 1 << 2, (56, 56), (56, 56), 1 << 14),
        (85, 5, 1 << 2, (56, 56), (56, 56), 0),
        (90, 5, 1 << 2, (56, 56), (56, 56), 1 << 12),
        (95, 4, 1 << 2, (56, 56), (4200, 4200), 0),
        (99, 4, 1 << 2, (56, 56), (4200, 4200), 0),
    ];
    for (index, (start, frames, flags, frame_rate, restart_rate, block_mask)) in
        specs.into_iter().enumerate()
    {
        desc.alien_ambient_array[index] = animation(start, frames, flags, frame_rate, restart_rate);
        desc.alien_ambient_array[index].block_mask = block_mask;
    }
    desc.alien_talk_desc = animation(1, 11, 0, (56, 0), (70, 0));
    #[cfg(not(test))]
    {
        desc.init_encounter_func = Some(live::intro);
        desc.post_encounter_func = Some(live::post);
        desc.uninit_encounter_func = Some(live::uninit);
    }
    desc
}

pub(crate) fn init(_comm: &mut Comm, _cx: &mut Cx<'_>) -> CommData {
    #[cfg(not(test))]
    {
        // SAFETY: initialization holds the exclusive game-thread session and Cx loans.
        unsafe { as_comm_data(&live::init_vux_comm(_comm, _cx)) }
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
