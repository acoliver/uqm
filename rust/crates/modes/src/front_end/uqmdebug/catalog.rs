//! Creature data used by universe-value diagnostics.

use crate::planet_side::creatures::{CreatureCatalog, CreatureKind};

pub fn uqm_debug_creature_bio_value(kind: u8) -> i32 {
    CreatureKind::new(kind)
        .map(|kind| i32::from(CreatureCatalog::raw_lifedata(kind).1 & 0x0f))
        .unwrap_or(-1)
}
