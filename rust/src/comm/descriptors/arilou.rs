#[cfg(not(test))]
use crate::comm::live::arilou as live;
use crate::comm::locdata::*;
use crate::comm::races::script::{animation, as_comm_data};
use crate::comm::{types::CommData, Comm};
use crate::engine::battle::Cx;
pub(crate) fn descriptor() -> CLocData {
    let mut d = CLocData {
        alien_frame_res: c"comm.arilou.graphics".as_ptr(),
        alien_font_res: c"comm.arilou.font".as_ptr(),
        alien_colormap_res: c"comm.arilou.colortable".as_ptr(),
        alien_song_res: c"comm.arilou.music".as_ptr(),
        conversation_phrases_res: c"comm.arilou.dialogue".as_ptr(),
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
        num_animations: 20,
        ..CLocData::default()
    };
    d.alien_ambient_array[0] = {
        let mut a = animation(4, 9, 2, (28, 0), (840, 2520));
        a.block_mask = 65538;
        a
    };
    d.alien_ambient_array[1] = {
        let mut a = animation(13, 9, 2, (28, 0), (840, 2520));
        a.block_mask = 65537;
        a
    };
    d.alien_ambient_array[2] = {
        let mut a = animation(22, 9, 2, (28, 0), (840, 2520));
        a.block_mask = 65536;
        a
    };
    d.alien_ambient_array[3] = {
        let mut a = animation(31, 9, 2, (28, 0), (840, 2520));
        a.block_mask = 16;
        a
    };
    d.alien_ambient_array[4] = {
        let mut a = animation(40, 10, 2, (28, 0), (840, 2520));
        a.block_mask = 8;
        a
    };
    d.alien_ambient_array[5] = {
        let mut a = animation(50, 9, 2, (28, 0), (840, 2520));
        a.block_mask = 128;
        a
    };
    d.alien_ambient_array[6] = animation(59, 8, 2, (28, 0), (840, 2520));
    d.alien_ambient_array[7] = {
        let mut a = animation(67, 9, 2, (28, 0), (840, 2520));
        a.block_mask = 32;
        a
    };
    d.alien_ambient_array[8] = {
        let mut a = animation(76, 9, 2, (28, 0), (840, 2520));
        a.block_mask = 512;
        a
    };
    d.alien_ambient_array[9] = {
        let mut a = animation(85, 9, 2, (28, 0), (840, 2520));
        a.block_mask = 256;
        a
    };
    d.alien_ambient_array[10] = animation(94, 9, 2, (28, 0), (840, 2520));
    d.alien_ambient_array[11] = {
        let mut a = animation(103, 9, 2, (28, 0), (840, 2520));
        a.block_mask = 8192;
        a
    };
    d.alien_ambient_array[12] = animation(112, 9, 2, (28, 0), (840, 2520));
    d.alien_ambient_array[13] = {
        let mut a = animation(121, 8, 2, (28, 0), (840, 2520));
        a.block_mask = 2048;
        a
    };
    d.alien_ambient_array[14] = {
        let mut a = animation(129, 9, 2, (28, 0), (840, 2520));
        a.block_mask = 32768;
        a
    };
    d.alien_ambient_array[15] = {
        let mut a = animation(138, 8, 2, (28, 0), (840, 2520));
        a.block_mask = 16384;
        a
    };
    d.alien_ambient_array[16] = {
        let mut a = animation(146, 9, 2, (28, 0), (840, 2520));
        a.block_mask = 7;
        a
    };
    d.alien_ambient_array[17] = {
        let mut a = animation(155, 2, 10, (56, 0), (840, 2520));
        a.block_mask = 524288;
        a
    };
    d.alien_ambient_array[18] = {
        let mut a = animation(157, 2, 10, (56, 0), (840, 2520));
        a.block_mask = 524288;
        a
    };
    d.alien_ambient_array[19] = {
        let mut a = animation(159, 4, 2, (70, 0), (840, 2520));
        a.block_mask = 393216;
        a
    };
    d.alien_transition_desc = animation(0, 0, 0, (0, 0), (0, 0));
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
        as_comm_data(&live::init_arilou_comm(_comm, _cx))
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
