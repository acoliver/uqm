//! Menu cursor stepping, including the grouped settings entries.

pub const PM_SCAN: u8 = 0;
pub const PM_STARMAP: u8 = 1;
pub const PM_DEVICES: u8 = 2;
pub const PM_CARGO: u8 = 3;
pub const PM_ROSTER: u8 = 4;
pub const PM_GAME_MENU: u8 = 5;
pub const PM_NAVIGATE: u8 = 6;
pub const PM_MIN_SCAN: u8 = 7;
pub const PM_ENE_SCAN: u8 = 8;
pub const PM_BIO_SCAN: u8 = 9;
pub const PM_EXIT_SCAN: u8 = 10;
pub const PM_AUTO_SCAN: u8 = 11;
pub const PM_LAUNCH_LANDER: u8 = 12;
pub const PM_SAVE_GAME: u8 = 13;
pub const PM_LOAD_GAME: u8 = 14;
pub const PM_QUIT_GAME: u8 = 15;
pub const PM_CHANGE_SETTINGS: u8 = 16;
pub const PM_EXIT_GAME_MENU: u8 = 17;
pub const PM_CONVERSE: u8 = 18;
pub const PM_ATTACK: u8 = 19;
pub const PM_ENCOUNTER_GAME_MENU: u8 = 20;
pub const PM_FUEL: u8 = 21;
pub const PM_MODULE: u8 = 22;
pub const PM_OUTFIT_GAME_MENU: u8 = 23;
pub const PM_EXIT_OUTFIT: u8 = 24;
pub const PM_CREW: u8 = 25;
pub const PM_SHIPYARD_GAME_MENU: u8 = 26;
pub const PM_EXIT_SHIPYARD: u8 = 27;
pub const PM_SOUND_ON: u8 = 28;
pub const PM_SOUND_OFF: u8 = 29;
pub const PM_MUSIC_ON: u8 = 30;
pub const PM_MUSIC_OFF: u8 = 31;
pub const PM_CYBORG_OFF: u8 = 32;
pub const PM_CYBORG_NORMAL: u8 = 33;
pub const PM_CYBORG_DOUBLE: u8 = 34;
pub const PM_CYBORG_SUPER: u8 = 35;
pub const PM_CHANGE_CAPTAIN: u8 = 36;
pub const PM_CHANGE_SHIP: u8 = 37;
pub const PM_EXIT_SETTINGS: u8 = 38;
pub const PM_ALT_SCAN: u8 = 39;
pub const PM_ALT_STARMAP: u8 = 40;
pub const PM_ALT_MANIFEST: u8 = 41;
pub const PM_ALT_GAME_MENU: u8 = 42;
pub const PM_ALT_NAVIGATE: u8 = 43;
pub const PM_ALT_CARGO: u8 = 44;
pub const PM_ALT_DEVICES: u8 = 45;
pub const PM_ALT_ROSTER: u8 = 46;
pub const PM_ALT_EXIT_MANIFEST: u8 = 47;
pub const PM_ALT_MSCAN: u8 = 48;
pub const PM_ALT_ESCAN: u8 = 49;
pub const PM_ALT_BSCAN: u8 = 50;
pub const PM_ALT_ASCAN: u8 = 51;
pub const PM_ALT_DISPATCH: u8 = 52;
pub const PM_ALT_EXIT_SCAN: u8 = 53;
pub const ALT_MANIFEST: u8 = 0x80;
pub const ALT_EXIT_MANIFEST: u8 = 0x81;

pub fn end_state(base: u8) -> u8 {
    match base {
        PM_SCAN | PM_STARMAP => PM_NAVIGATE,
        PM_MIN_SCAN => PM_LAUNCH_LANDER,
        PM_SAVE_GAME => PM_EXIT_GAME_MENU,
        PM_CONVERSE => PM_ENCOUNTER_GAME_MENU,
        PM_FUEL => PM_EXIT_OUTFIT,
        PM_CREW => PM_EXIT_SHIPYARD,
        PM_SOUND_ON => PM_EXIT_SETTINGS,
        PM_ALT_SCAN | PM_ALT_STARMAP => PM_ALT_NAVIGATE,
        PM_ALT_CARGO => PM_ALT_EXIT_MANIFEST,
        PM_ALT_MSCAN => PM_ALT_EXIT_SCAN,
        _ => base,
    }
}

fn fix_state(state: u8, flags: u8) -> u8 {
    match state {
        PM_SOUND_ON => {
            if flags & (1 << 4) != 0 {
                PM_SOUND_OFF
            } else {
                PM_SOUND_ON
            }
        }
        PM_MUSIC_ON => {
            if flags & (1 << 3) != 0 {
                PM_MUSIC_OFF
            } else {
                PM_MUSIC_ON
            }
        }
        PM_CYBORG_OFF => PM_CYBORG_OFF + ((flags & 0xc0) >> 6),
        _ => state,
    }
}

pub fn next_state(base: u8, current: u8, flags: u8) -> u8 {
    let adj = base.wrapping_sub(u8::from(base == PM_STARMAP));
    let absolute = i32::from(adj) + i32::from(current);
    let mut next = match absolute {
        value if value == i32::from(PM_SOUND_ON) || value == i32::from(PM_SOUND_OFF) => PM_MUSIC_ON,
        value if value == i32::from(PM_MUSIC_ON) || value == i32::from(PM_MUSIC_OFF) => {
            PM_CYBORG_OFF
        }
        value if (i32::from(PM_CYBORG_OFF)..=i32::from(PM_CYBORG_SUPER)).contains(&value) => {
            PM_CHANGE_CAPTAIN
        }
        _ => absolute.wrapping_add(1) as u8,
    };
    if next > end_state(base) {
        next = base;
    }
    fix_state(next, flags).wrapping_sub(adj)
}

pub fn previous_state(base: u8, current: u8, flags: u8) -> u8 {
    let adj = base.wrapping_sub(u8::from(base == PM_STARMAP));
    let absolute = i32::from(adj) + i32::from(current);
    let mut next = match absolute {
        value if value == i32::from(PM_SOUND_OFF) => PM_EXIT_SETTINGS as i16,
        value if value == i32::from(PM_MUSIC_ON) || value == i32::from(PM_MUSIC_OFF) => {
            PM_SOUND_ON as i16
        }
        value if (i32::from(PM_CYBORG_OFF)..=i32::from(PM_CYBORG_SUPER)).contains(&value) => {
            PM_MUSIC_ON as i16
        }
        value if value == i32::from(PM_CHANGE_CAPTAIN) => PM_CYBORG_OFF as i16,
        _ => (absolute - 1) as i16,
    };
    if next < i16::from(base) {
        next = i16::from(end_state(base));
    }
    fix_state(next as u8, flags).wrapping_sub(adj)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn outfit_and_shipyard_wrap_both_directions() {
        assert_eq!(next_state(PM_FUEL, 3, 0), 0);
        assert_eq!(previous_state(PM_FUEL, 0, 0), 3);
        assert_eq!(next_state(PM_CREW, 2, 0), 0);
        assert_eq!(previous_state(PM_CREW, 0, 0), 2);
    }
    #[test]
    fn starmap_skips_scan_and_keeps_shared_offsets() {
        assert_eq!(next_state(PM_STARMAP, 6, 0), 1);
        assert_eq!(previous_state(PM_STARMAP, 1, 0), 6);
    }
    #[test]
    fn settings_skip_toggle_variants_but_reflect_current_flags() {
        assert_eq!(next_state(PM_SOUND_ON, 0, 0), PM_MUSIC_ON - PM_SOUND_ON);
        assert_eq!(previous_state(PM_SOUND_ON, 2, 1 << 4), 1);
        assert_eq!(
            next_state(PM_SOUND_ON, 2, 2 << 6),
            PM_CYBORG_DOUBLE - PM_SOUND_ON
        );
    }
}
