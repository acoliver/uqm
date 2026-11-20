//! Game content: race behaviour and descriptors, system generators, dialogue
//! scripts and data tables.
#![forbid(unsafe_code)]

pub mod comm {
    pub mod races;
}
pub mod planets {
    pub mod generate;
    pub mod tables;
}
pub mod ships;
