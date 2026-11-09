#[cfg(not(test))]
use crate::comm::live::rebel as live;
use crate::comm::locdata::*;
use crate::comm::races::rebel::*;
use crate::comm::races::script::{animation, as_comm_data};
use crate::comm::{types::CommData, Comm};
use crate::engine::battle::Cx;
pub(crate) fn descriptor(screen_width: i32) -> CLocData {
    let (baseline, width) = text_layout(screen_width);
    let mut d = CLocData {
        alien_frame_res: c"comm.yehat.graphics".as_ptr(),
        alien_font_res: c"comm.yehat.font".as_ptr(),
        alien_colormap_res: c"comm.yehat.colortable".as_ptr(),
        alien_song_res: c"comm.yehat.music".as_ptr(),
        alien_alt_song_res: c"comm.yehat.rebel.music".as_ptr(),
        conversation_phrases_res: c"comm.yehat.rebel.dialogue".as_ptr(),
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
        alien_song_flags: 1,
        num_animations: 15,
        ..CLocData::default()
    };
    d.alien_ambient_array[0] = animation(4, 3, 0x0c, (84, 84), (840, 2520));
    d.alien_ambient_array[0].block_mask = (1 << 6) | (1 << 7);
    d.alien_ambient_array[1] = animation(7, 3, 0x0c, (84, 84), (840, 2520));
    d.alien_ambient_array[1].block_mask = (1 << 6) | (1 << 7);
    d.alien_ambient_array[2] = animation(10, 3, 4, (42, 0), (840, 2520));
    d.alien_ambient_array[2].block_mask = (1 << 4) | (1 << 14);
    d.alien_ambient_array[3] = animation(13, 3, 4, (42, 0), (840, 2520));
    d.alien_ambient_array[3].block_mask = 1 << 5;
    d.alien_ambient_array[4] = animation(16, 5, 4, (56, 56), (8400, 2520));
    d.alien_ambient_array[4].block_mask = (1 << 2) | (1 << 14);
    d.alien_ambient_array[5] = animation(21, 5, 4, (56, 56), (8400, 2520));
    d.alien_ambient_array[5].block_mask = 1 << 3;
    d.alien_ambient_array[6] = animation(26, 2, 0x0c, (56, 56), (8400, 2520));
    d.alien_ambient_array[6].block_mask = (1 << 0) | (1 << 1);
    d.alien_ambient_array[7] = animation(28, 2, 0x0c, (56, 56), (8400, 2520));
    d.alien_ambient_array[7].block_mask = (1 << 0) | (1 << 1);
    for (i, start) in (30..=45).step_by(3).enumerate() {
        d.alien_ambient_array[i + 8] = animation(start, 3, 4, (28, 28), (28, 28));
    }
    d.alien_ambient_array[14] = animation(48, 4, 0x0c, (28, 0), (840, 2520));
    d.alien_ambient_array[14].block_mask = (1 << 2) | (1 << 4);
    d.alien_talk_desc = animation(1, 3, 0, (56, 0), (70, 0));
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
        unsafe { as_comm_data(&live::init_rebel_yehat_comm(_comm, _cx)) }
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
