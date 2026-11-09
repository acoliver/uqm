#[cfg(not(test))]
use crate::comm::live::orz as live;
use crate::comm::locdata::*;
use crate::comm::races::script::{animation, as_comm_data};
use crate::comm::{types::CommData, Comm};
use crate::engine::battle::Cx;
pub(crate) fn descriptor() -> CLocData {
    let mut d = CLocData {
        alien_frame_res: c"comm.orz.graphics".as_ptr(),
        alien_font_res: c"comm.orz.font".as_ptr(),
        alien_colormap_res: c"comm.orz.colortable".as_ptr(),
        alien_song_res: c"comm.orz.music".as_ptr(),
        conversation_phrases_res: c"comm.orz.dialogue".as_ptr(),
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
        alien_text_baseline: CPoint { x: 121, y: 0 },
        alien_text_width: 224,
        alien_text_align: 1,
        alien_text_valign: 0,
        num_animations: 12,
        ..CLocData::default()
    };
    d.alien_ambient_array[0] = animation(4, 6, 2, (84, 0), (840, 2520));
    d.alien_ambient_array[1] = animation(10, 5, 2, (84, 0), (840, 2520));
    d.alien_ambient_array[2] = animation(15, 2, 1, (84, 84), (840, 2520));
    d.alien_ambient_array[3] = animation(17, 3, 4, (84, 84), (840, 2520));
    d.alien_ambient_array[4] = {
        let mut a = animation(20, 2, 4, (84, 0), (84, 2520));
        a.block_mask = 128;
        a
    };
    d.alien_ambient_array[5] = {
        let mut a = animation(22, 8, 2, (84, 0), (84, 2520));
        a.block_mask = 64;
        a
    };
    d.alien_ambient_array[6] = {
        let mut a = animation(30, 3, 2, (84, 0), (84, 2520));
        a.block_mask = 32;
        a
    };
    d.alien_ambient_array[7] = {
        let mut a = animation(33, 3, 4, (84, 0), (84, 2520));
        a.block_mask = 16;
        a
    };
    d.alien_ambient_array[8] = animation(36, 25, 2, (14, 56), (840, 2520));
    d.alien_ambient_array[9] = animation(61, 15, 2, (14, 56), (840, 2520));
    d.alien_ambient_array[10] = {
        let mut a = animation(76, 17, 2, (14, 56), (840, 2520));
        a.block_mask = 4096;
        a
    };
    d.alien_ambient_array[11] = animation(93, 25, 2, (14, 56), (840, 2520));
    d.alien_ambient_array[12] = {
        let mut a = animation(118, 11, 2, (14, 56), (840, 2520));
        a.block_mask = 1024;
        a
    };
    d.alien_transition_desc = animation(0, 0, 0, (0, 0), (0, 0));
    d.alien_talk_desc = animation(1, 3, 0, (56, 56), (70, 315));
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
        as_comm_data(&live::init_orz_comm(_comm, _cx))
    }
    #[cfg(test)]
    {
        as_comm_data(&descriptor())
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
