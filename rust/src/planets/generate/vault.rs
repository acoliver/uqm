//! `genvault.c`: the Precursor ship vault, planet 0 moon 0.
//!
//! Overridden: `generateOrbital`, `generateEnergy`, `pickupEnergy`. Story
//! flags: `SHIP_VAULT_UNLOCKED`, `SYREEN_SHUTTLE_ON_SHIP`, `SYREEN_HOME_VISITS`,
//! `KNOW_SYREEN_VAULT`.

use crate::c_bindings::global_state::{NODE_INFO, SYSTEM_INFO};
use crate::math::RandomContext;
use crate::planet_side::report_adapter::DiscoveryReportPort;
use crate::planets::generate::default;
use crate::planets::generate::special::{Body, Orbit, Story};

const VAULT: Body = Body::Moon { planet: 0, moon: 0 };

/// The vault's scene and lander.
pub trait VaultSite {
    /// `LoadStdLanderFont`, the vault mask and the vault report strings, with
    /// the report started at `first_report`.
    fn load_vault_scene(&mut self, first_report: u16);
    /// `set_lander_takeoff ()`.
    fn lander_takeoff(&mut self);
}

/// `GenerateVault_generateOrbital`: the report to start at depends on how far
/// the Syreen shuttle plot has gone.
pub fn generate_orbital(
    orbit: &mut Orbit<'_>,
    body: Body,
    story: &impl Story,
    site: &mut impl VaultSite,
    load_planet: impl FnOnce(),
) -> bool {
    if body == VAULT {
        let first_report = if story.flag("SHIP_VAULT_UNLOCKED") != 0 {
            2
        } else if story.flag("SYREEN_SHUTTLE_ON_SHIP") != 0 {
            1
        } else {
            0
        };
        site.load_vault_scene(first_report);
    }
    orbit.default_orbital(load_planet)
}

/// `GenerateVault_generateEnergy`: the vault is one artifact node.
pub fn generate_energy(
    rng: &mut RandomContext,
    sys_info: &SYSTEM_INFO,
    body: Body,
    which_node: u16,
    info: &mut NODE_INFO,
) -> u16 {
    if body == VAULT {
        default::generate_artifact(rng, sys_info, which_node, info)
    } else {
        0
    }
}

/// `GenerateVault_pickupEnergy`: the vault is never picked up.
pub fn pickup_energy(
    body: Body,
    which_node: u16,
    story: &mut impl Story,
    site: &mut impl VaultSite,
    report: &mut impl DiscoveryReportPort,
) -> bool {
    if body != VAULT {
        return false;
    }
    assert_eq!(which_node, 0, "the vault has a single energy node");
    if story.flag("SHIP_VAULT_UNLOCKED") != 0 {
        default::lander_report_cycle(report);
    } else if story.flag("SYREEN_SHUTTLE_ON_SHIP") != 0 {
        default::lander_report_cycle(report);
        site.lander_takeoff();
        story.set_flag("SHIP_VAULT_UNLOCKED", 1);
        story.set_flag("SYREEN_SHUTTLE_ON_SHIP", 0);
        story.set_flag("SYREEN_HOME_VISITS", 0);
    } else {
        default::lander_report(report);
        if story.flag("KNOW_SYREEN_VAULT") == 0 {
            story.set_flag("KNOW_SYREEN_VAULT", 1);
        }
    }
    false
}
