// Race-specific ship behavior implementations
// @plan PLAN-20260314-SHIPS.P11
//
// Phase P11: 8 simple ships implemented
// Phase P12: 8 mode-switching ships implemented
// Phase P13: 12 complex & non-melee ships implemented

pub mod androsynth;
pub mod arilou;
pub mod black_urquan;
pub mod chenjesu;
pub mod chmmr;
pub mod druuge;
pub mod human;
pub mod ilwrath;
pub mod melnorme;
pub mod mmrnmhrm;
pub mod mycon;
pub mod orz;
pub mod pkunk;
pub mod probe;
pub mod samatra;
pub mod shofixti;
pub mod sis_ship;
pub mod slylandro;
pub mod spathi;
pub mod supox;
pub mod syreen;
pub mod thraddash;
pub mod umgah;
pub mod urquan;
pub mod utwig;
pub mod vux;
pub mod yehat;
pub mod zoqfotpik;

// Re-export ship types for convenience
pub use androsynth::AndrosynthShip;
pub use arilou::ArilouShip;
pub use black_urquan::BlackUrquanShip;
pub use chenjesu::ChenjesuShip;
pub use chmmr::ChmmrShip;
pub use druuge::DruugeShip;
pub use human::HumanShip;
pub use ilwrath::IlwrathShip;
pub use melnorme::MelnormeShip;
pub use mmrnmhrm::MmrnmhrmShip;
pub use mycon::MyconShip;
pub use orz::OrzShip;
pub use pkunk::PkunkShip;
pub use probe::ProbeShip;
pub use samatra::SamatraShip;
pub use shofixti::ShofixtiShip;
pub use sis_ship::SisShip;
pub use slylandro::SlylandroShip;
pub use spathi::SpathiShip;
pub use supox::SupoxShip;
pub use syreen::SyreenShip;
pub use thraddash::ThraddashShip;
pub use umgah::UmgahShip;
pub use urquan::UrquanShip;
pub use utwig::UtwigShip;
pub use vux::VuxShip;
pub use yehat::YehatShip;
pub use zoqfotpik::ZoqfotpikShip;
