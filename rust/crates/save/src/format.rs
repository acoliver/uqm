//! Tags and sizes of the UMS version 1 format, from `sc2/src/uqm/save.h` and
//! the byte counts `save.c` writes. All integers in the file are
//! little-endian.

pub const SAVEFILE_TAG: u32 = 0x0153_4d55; // "UMS\x01"
pub const SUMMARY_TAG: u32 = 0x6d6d_7553; // "Summ"
pub const GLOBAL_STATE_TAG: u32 = 0x7453_6c47; // "GlSt"
pub const GAME_STATE_TAG: u32 = 0x7453_6d47; // "GmSt"
pub const EVENTS_TAG: u32 = 0x7374_7645; // "Evts"
pub const ENCOUNTERS_TAG: u32 = 0x7463_6e45; // "Enct"
pub const RACE_Q_TAG: u32 = 0x5163_6152; // "RacQ"
pub const IP_GRP_Q_TAG: u32 = 0x5170_4749; // "IGpQ"
pub const NPC_SHIP_Q_TAG: u32 = 0x5163_704e; // "NpcQ"
pub const SHIP_Q_TAG: u32 = 0x5170_6853; // "ShpQ"
pub const STAR_TAG: u32 = 0x7261_7453; // "Star"
pub const SCAN_TAG: u32 = 0x6e61_6353; // "Scan"
pub const BATTLE_GROUP_TAG: u32 = 0x7047_7442; // "BtGp"
pub const GROUP_LIST_TAG: u32 = 0x7370_7247; // "Grps"

/// Summ body without the save name (save.c `SaveSummary`).
pub const SUMMARY_FIXED_BYTES: u32 = 160;
/// GlSt body; the loader rejects any other size (load.c `LoadGameState`).
pub const GLOBAL_STATE_BYTES: u32 = 75;
pub const RACE_ENTRY_BYTES: u32 = 30;
pub const IP_GROUP_ENTRY_BYTES: u32 = 13;
pub const SHIP_FRAGMENT_ENTRY_BYTES: u32 = 11;
pub const EVENT_ENTRY_BYTES: u32 = 5;
pub const ENCOUNTER_ENTRY_BYTES: u32 = 65;
pub const STAR_BYTES: u32 = 8;
/// "Grps" body: the last-encounter byte, then entries of this size.
pub const GROUP_LIST_ENTRY_BYTES: u32 = 14;
/// "BtGp" body before the fleets: encounter id, current flag, star index,
/// day, month, year, fleet count.
pub const BATTLE_GROUP_HEADER_BYTES: u32 = 12;
/// A fleet inside "BtGp": icon byte and ship count.
pub const BATTLE_FLEET_HEADER_BYTES: u32 = 2;
pub const BATTLE_SHIP_BYTES: u32 = 10;

/// Queue chunks are counted with a 16-bit `COUNT` by `SaveGame`/`LoadGame`.
pub const MAX_QUEUE_ENTRIES: u64 = u16::MAX as u64;
/// The group count, and each fleet's ship count, are single bytes.
pub const MAX_BYTE_COUNT: u64 = u8::MAX as u64;

/// Printable form of a tag for diagnostics and tests.
pub fn tag_text(tag: u32) -> String {
    tag.to_le_bytes()
        .iter()
        .map(|&b| {
            if (0x20..0x7f).contains(&b) {
                b as char
            } else {
                '.'
            }
        })
        .collect()
}

pub const CANNED_NODES_TAG: u32 = u32::from_le_bytes(*b"Cans");
