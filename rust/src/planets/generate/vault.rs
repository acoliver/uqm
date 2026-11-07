//! `genvault.c`: the Precursor ship vault, planet 0 moon 0.
//!
//! Overridden: `generateOrbital`, `generateEnergy`, `pickupEnergy`. Story
//! flags: `SHIP_VAULT_UNLOCKED`, `SYREEN_SHUTTLE_ON_SHIP`, `SYREEN_HOME_VISITS`,
//! `KNOW_SYREEN_VAULT`.

use crate::engine::world::planet::{NodeInfo, SystemInfo};
use crate::engine::StoryKey;
use crate::math::RandomContext;
use crate::planets::generate::default;
use crate::planets::generate::special::{Body, Story};

const VAULT: Body = Body::Moon { planet: 0, moon: 0 };

/// The vault's scene and lander.
pub trait VaultSite {
    /// `LoadStdLanderFont`, the vault mask and the vault report strings, with
    /// the report started at `first_report`.
    fn load_vault_scene(
        &mut self,
        _system: &mut crate::engine::world::generation::SolarSys<'_>,
        first_report: u16,
    );
    /// `set_lander_takeoff ()`.
    fn lander_takeoff(&mut self);
}

/// `GenerateVault_generateOrbital`: the report to start at depends on how far
/// the Syreen shuttle plot has gone.
pub fn generate_orbital(
    system: &mut SolarSys<'_>,
    body: Body,
    site: &mut (impl VaultSite + Story + super::host::PlanetSite + ?Sized),
) -> bool {
    if body == VAULT {
        let first_report = if site.flag(StoryKey::SHIP_VAULT_UNLOCKED) != 0 {
            2
        } else if site.flag(StoryKey::SYREEN_SHUTTLE_ON_SHIP) != 0 {
            1
        } else {
            0
        };
        site.load_vault_scene(system, first_report);
    }
    super::special::default_orbital(system, body, site)
}

/// `GenerateVault_generateEnergy`: the vault is one artifact node.
pub fn generate_energy(
    rng: &mut RandomContext,
    sys_info: &SystemInfo,
    body: Body,
    which_node: u16,
    info: &mut NodeInfo,
) -> u16 {
    if body == VAULT {
        default::generate_artifact(rng, sys_info, which_node, info)
    } else {
        0
    }
}

/// `GenerateVault_pickupEnergy`: the vault is never picked up.
pub fn pickup_energy(
    system: &mut SolarSys<'_>,
    body: Body,
    which_node: u16,
    site: &mut (impl VaultSite + Story + super::host::ReportSite + ?Sized),
) -> bool {
    if body != VAULT {
        return false;
    }
    assert_eq!(which_node, 0, "the vault has a single energy node");
    if site.flag(StoryKey::SHIP_VAULT_UNLOCKED) != 0 {
        site.report(
            system,
            crate::engine::world::generation::ReportDisposition::Cycle,
        );
    } else if site.flag(StoryKey::SYREEN_SHUTTLE_ON_SHIP) != 0 {
        site.report(
            system,
            crate::engine::world::generation::ReportDisposition::Cycle,
        );
        site.lander_takeoff();
        site.set_flag(StoryKey::SHIP_VAULT_UNLOCKED, 1);
        site.set_flag(StoryKey::SYREEN_SHUTTLE_ON_SHIP, 0);
        site.set_flag(StoryKey::SYREEN_HOME_VISITS, 0);
    } else {
        site.report(
            system,
            crate::engine::world::generation::ReportDisposition::Once,
        );
        if site.flag(StoryKey::KNOW_SYREEN_VAULT) == 0 {
            site.set_flag(StoryKey::KNOW_SYREEN_VAULT, 1);
        }
    }
    false
}

use super::generator::SystemGenerator;
use super::host::SiteHost;
use crate::engine::world::generation::SolarSys;

pub struct Generator;

impl SystemGenerator for Generator {
    fn generate_energy(
        &self,
        system: &mut SolarSys<'_>,
        body: Body,
        node: u16,
        info: &mut NodeInfo,
        _host: &dyn SiteHost,
    ) -> u16 {
        system.scan(body, |rng, sys, _frame| {
            generate_energy(rng, sys, body, node, info)
        })
    }

    fn generate_orbital(
        &self,
        system: &mut SolarSys<'_>,
        body: Body,
        host: &mut dyn SiteHost,
    ) -> bool {
        generate_orbital(system, body, host)
    }
    fn pickup_energy(
        &self,
        system: &mut SolarSys<'_>,
        body: Body,
        node: u16,
        host: &mut dyn SiteHost,
    ) -> bool {
        pickup_energy(system, body, node, host)
    }
}
