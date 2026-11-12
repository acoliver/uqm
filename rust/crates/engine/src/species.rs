//! Ship and conversation identities shared by content and modes.

pub mod conv {
    pub const ARILOU: u32 = 0;
    pub const CHMMR: u32 = 1;
    pub const COMMANDER: u32 = 2;
    pub const ORZ: u32 = 3;
    pub const PKUNK: u32 = 4;
    pub const SHOFIXTI: u32 = 5;
    pub const SPATHI: u32 = 6;
    pub const SUPOX: u32 = 7;
    pub const THRADD: u32 = 8;
    pub const UTWIG: u32 = 9;
    pub const VUX: u32 = 10;
    pub const YEHAT: u32 = 11;
    pub const MELNORME: u32 = 12;
    pub const DRUUGE: u32 = 13;
    pub const ILWRATH: u32 = 14;
    pub const MYCON: u32 = 15;
    pub const SLYLANDRO: u32 = 16;
    pub const UMGAH: u32 = 17;
    pub const URQUAN: u32 = 18;
    pub const ZOQFOTPIK: u32 = 19;
    pub const SYREEN: u32 = 20;
    pub const BLACKURQ: u32 = 21;
    pub const TALKING_PET: u32 = 22;
    pub const SLYLANDRO_HOME: u32 = 23;
    pub const URQUAN_DRONE: u32 = 24;
    pub const YEHAT_REBEL: u32 = 25;
    pub const INVALID: u32 = 26;
}

// ---------------------------------------------------------------------------
// Ship type IDs (from races.h enum)
// ---------------------------------------------------------------------------

pub mod ship {
    pub const ARILOU_SHIP: u8 = 0;
    pub const CHMMR_SHIP: u8 = 1;
    pub const HUMAN_SHIP: u8 = 2;
    pub const ORZ_SHIP: u8 = 3;
    pub const PKUNK_SHIP: u8 = 4;
    pub const SHOFIXTI_SHIP: u8 = 5;
    pub const SPATHI_SHIP: u8 = 6;
    pub const SUPOX_SHIP: u8 = 7;
    pub const THRADDASH_SHIP: u8 = 8;
    pub const UTWIG_SHIP: u8 = 9;
    pub const VUX_SHIP: u8 = 10;
    pub const YEHAT_SHIP: u8 = 11;
    pub const MELNORME_SHIP: u8 = 12;
    pub const DRUUGE_SHIP: u8 = 13;
    pub const ILWRATH_SHIP: u8 = 14;
    pub const MYCON_SHIP: u8 = 15;
    pub const SLYLANDRO_SHIP: u8 = 16;
    pub const UMGAH_SHIP: u8 = 17;
    pub const URQUAN_SHIP: u8 = 18;
    pub const ZOQFOTPIK_SHIP: u8 = 19;
    pub const SYREEN_SHIP: u8 = 20;
    pub const BLACK_URQUAN_SHIP: u8 = 21;
    pub const YEHAT_REBEL_SHIP: u8 = 22;
    pub const URQUAN_DRONE_SHIP: u8 = 23;
    /// `SAMATRA_SHIP = URQUAN_DRONE_SHIP` (races.h).
    pub const SAMATRA_SHIP: u8 = URQUAN_DRONE_SHIP;
}
