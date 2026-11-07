//! Effects available to system generators, supplied by the solar-system mode.

use super::special::Story;
use crate::engine::world::generation::{ReportDisposition, SolarSys};

pub trait PlanetSite {
    fn rainbow_ordinal(&self) -> u8;
    fn load_planet(&mut self, system: &mut SolarSys<'_>);
    fn load_earth_planet(&mut self, system: &mut SolarSys<'_>);
}

pub trait ReportSite {
    fn report(&mut self, system: &mut SolarSys<'_>, disposition: ReportDisposition) -> bool;
}

pub trait SiteHost:
    Story
    + super::default::Npcs
    + super::default::PlanetNames
    + super::special::FleetGroups
    + super::home_contact::HomeContactSite
    + super::home_contact::LanderTakeoff
    + super::burv::BurvScenes
    + super::orz::OrzSite
    + super::thrad::ThraddashSite
    + super::zoq::ZoqFotPikSite
    + super::pet::TalkingPetSite
    + super::pet::UrquanEncounterSite
    + super::utw::UtwigSite
    + super::myc::MyconSite
    + super::vux::VuxSite
    + super::sam::GuardSite
    + super::sam::SamatraSite
    + super::spa::SpathiSite
    + super::vault::VaultSite
    + super::wreck::WreckSite
    + super::syreen::StarbaseSite
    + super::slylandro::HomeSite
    + super::yehat::YehatSite
    + super::andro::AndrosynthSite
    + super::chmmr::ChmmrSite
    + super::shof::ShofixtiSite
    + super::sol::Pickup
    + super::sol::orbital::Orbital
    + super::sol::orbital::Queues
    + PlanetSite
    + ReportSite
{
}
