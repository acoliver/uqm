#[cfg(not(test))]
use crate::comm::live::shofixt as live;
use crate::comm::locdata::*;
use crate::comm::races::script::{animation, as_comm_data};
use crate::comm::races::shofixt::*;
use crate::comm::{types::CommData, Comm};
use crate::engine::battle::Cx;
pub(crate) fn descriptor(screen_width: i32) -> CLocData {
    let (baseline, width) = text_layout(screen_width);
    let mut d = CLocData {
        alien_frame_res: c"comm.shofixti.graphics".as_ptr(),
        alien_font_res: c"comm.shofixti.font".as_ptr(),
        alien_colormap_res: c"comm.shofixti.colortable".as_ptr(),
        alien_song_res: c"comm.shofixti.music".as_ptr(),
        conversation_phrases_res: c"comm.shofixti.dialogue".as_ptr(),
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
        num_animations: 11,
        ..CLocData::default()
    };
    d.alien_ambient_array[0] = animation(5, 15, 2, (28, 0), (28, 0));
    d.alien_ambient_array[1] = animation(20, 3, 1, (84, 0), (420, 1260));
    d.alien_ambient_array[2] = animation(23, 3, 1, (84, 0), (420, 1260));
    d.alien_ambient_array[3] = animation(26, 3, 4, (28, 0), (840, 2520));
    d.alien_ambient_array[4] = animation(29, 4, 4, (56, 0), (840, 2520));
    d.alien_ambient_array[5] = animation(33, 6, 2, (42, 28), (42, 28));
    d.alien_ambient_array[6] = animation(39, 7, 2, (42, 28), (42, 28));
    d.alien_ambient_array[6].block_mask = 1 << 7;
    d.alien_ambient_array[7] = animation(46, 6, 2, (42, 28), (42, 28));
    d.alien_ambient_array[7].block_mask = 1 << 6;
    d.alien_ambient_array[8] = animation(52, 4, 2, (42, 28), (42, 28));
    d.alien_ambient_array[9] = animation(56, 7, 2, (42, 28), (42, 28));
    d.alien_ambient_array[9].block_mask = 1 << 10;
    d.alien_ambient_array[10] = animation(63, 6, 2, (42, 28), (42, 28));
    d.alien_ambient_array[10].block_mask = 1 << 9;
    d.alien_talk_desc = animation(1, 4, 0, (42, 0), (56, 0));
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
        unsafe { as_comm_data(&live::init_shofixti_comm(_comm, _cx)) }
    }
    #[cfg(test)]
    {
        as_comm_data(&descriptor(320))
    }
}

pub(crate) fn intro(_comm: &mut Comm, _cx: &mut Cx<'_>) {
    #[cfg(not(test))]
    live::intro(_comm, _cx)
}

pub(crate) fn post_encounter(_comm: &mut Comm, _cx: &mut Cx<'_>) {
    #[cfg(not(test))]
    live::post(_comm, _cx)
}
