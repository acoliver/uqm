use super::text_layout;
use crate::comm::races::descriptor::{Color, CommDescriptor};
use crate::comm::races::script::animation;

pub(crate) fn descriptor(screen_width: i32) -> CommDescriptor {
    let (baseline, width) = text_layout(screen_width);
    let mut d = CommDescriptor {
        alien_frame_res: Some(c"comm.yehat.graphics"),
        alien_font_res: Some(c"comm.yehat.font"),
        alien_colormap_res: Some(c"comm.yehat.colortable"),
        alien_song_res: Some(c"comm.yehat.music"),
        alien_alt_song_res: Some(c"comm.yehat.rebel.music"),
        conversation_phrases_res: Some(c"comm.yehat.rebel.dialogue"),
        alien_text_fcolor: Color {
            r: 255,
            g: 255,
            b: 255,
            a: 255,
        },
        alien_text_bcolor: Color {
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
        ..CommDescriptor::default()
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
    d
}
