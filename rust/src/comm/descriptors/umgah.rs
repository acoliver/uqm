#[cfg(not(test))]
use crate::comm::live::umgah as live;
use crate::comm::locdata::*;
use crate::comm::races::script::{animation, as_comm_data};
use crate::comm::{types::CommData, Comm};
use crate::engine::battle::Cx;
pub(crate) fn descriptor() -> CLocData {
    let mut d = CLocData {
        alien_frame_res: c"comm.umgah.graphics".as_ptr(),
        alien_font_res: c"comm.umgah.font".as_ptr(),
        alien_colormap_res: c"comm.umgah.colortable".as_ptr(),
        alien_song_res: c"comm.umgah.music".as_ptr(),
        conversation_phrases_res: c"comm.umgah.dialogue".as_ptr(),
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
        num_animations: 16,
        ..CLocData::default()
    };
    d.alien_ambient_array[0] = animation(5, 3, 4, (42, 0), (840, 2520));
    d.alien_ambient_array[0].block_mask = 32;
    d.alien_ambient_array[1] = animation(8, 3, 4, (42, 0), (840, 2520));
    d.alien_ambient_array[1].block_mask = 64;
    d.alien_ambient_array[2] = animation(11, 2, 4, (42, 0), (840, 2520));
    d.alien_ambient_array[2].block_mask = 128;
    d.alien_ambient_array[3] = animation(13, 2, 4, (42, 0), (840, 2520));
    d.alien_ambient_array[3].block_mask = 256;
    d.alien_ambient_array[4] = animation(15, 2, 4, (42, 0), (840, 2520));
    d.alien_ambient_array[4].block_mask = 512;
    d.alien_ambient_array[5] = animation(17, 3, 2, (98, 0), (2520, 2520));
    d.alien_ambient_array[5].block_mask = 1;
    d.alien_ambient_array[6] = animation(20, 3, 2, (98, 0), (2520, 2520));
    d.alien_ambient_array[6].block_mask = 2;
    d.alien_ambient_array[7] = animation(23, 2, 2, (98, 0), (2520, 2520));
    d.alien_ambient_array[7].block_mask = 4;
    d.alien_ambient_array[8] = animation(25, 2, 2, (98, 0), (2520, 2520));
    d.alien_ambient_array[8].block_mask = 8;
    d.alien_ambient_array[9] = animation(27, 2, 2, (98, 0), (2520, 2520));
    d.alien_ambient_array[9].block_mask = 16;
    d.alien_ambient_array[10] = animation(29, 3, 4, (84, 0), (840, 2520));
    d.alien_ambient_array[11] = animation(32, 3, 4, (84, 0), (840, 2520));
    d.alien_ambient_array[12] = animation(35, 5, 4, (84, 0), (840, 2520));
    d.alien_ambient_array[13] = animation(40, 6, 4, (84, 0), (84, 0));
    d.alien_ambient_array[14] = animation(46, 2, 2, (168, 0), (840, 2520));
    d.alien_ambient_array[14].block_mask = 32768;
    d.alien_ambient_array[15] = animation(48, 2, 2, (168, 0), (840, 2520));
    d.alien_ambient_array[15].block_mask = 16384;
    d.alien_transition_desc = animation(0, 0, 0, (0, 0), (0, 0));
    d.alien_talk_desc = animation(1, 4, 0, (56, 0), (70, 0));
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
        as_comm_data(&live::init_umgah_comm(_comm, _cx))
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
