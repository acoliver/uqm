//! What the ruined race homeworlds of `genpku.c`, `gensup.c` and `gendru.c`
//! have in common: flying into orbit starts the race's conversation while its
//! sphere of influence can be tracked and otherwise loads the ruins scene, and
//! the ruins hold one artifact that any city ruin gives up.
//!
//! The callers check the body; everything here is about planet 0.

use crate::planet_side::report_adapter::DiscoveryReportPort;
use crate::planets::generate::default;
use crate::planets::generate::special::Story;
use crate::planets::generate::yehat::{CHECK_ABORT, CHECK_LOAD, START_INTERPLANETARY};

/// `GLOBAL_FLAGS_AND_DATA` value that marks the conversation as a home
/// contact: `1 << 7`.
pub const HOME_CONTACT_FLAGS: u8 = 1 << 7;

/// What entering orbit at a race home does to the rest of the game, one method
/// per host call, in the order the C makes them. The race's ship and
/// conversation are the host's business.
pub trait HomeContactSite {
    /// `StartSphereTracking (race)`.
    fn start_sphere_tracking(&mut self) -> bool;
    /// `NotifyOthers (race, IPNL_ALL_CLEAR)`.
    fn notify_others_all_clear(&mut self);
    /// `PutGroupInfo (GROUPS_RANDOM, GROUP_SAVE_IP)`.
    fn save_ip_groups(&mut self);
    /// `ReinitQueue (&GLOBAL (ip_group_q))`.
    fn reinit_ip_group_q(&mut self);
    /// `CountLinks (&GLOBAL (npc_built_ship_q))`.
    fn built_ship_count(&self) -> u16;
    /// `CloneShipFragment (race, &GLOBAL (npc_built_ship_q), INFINITE_FLEET)`.
    fn clone_fleet_into_built_queue(&mut self);
    /// `GLOBAL (CurrentActivity)`.
    fn current_activity(&self) -> u16;
    fn set_current_activity(&mut self, activity: u16);
    /// `SET_GAME_STATE (GLOBAL_FLAGS_AND_DATA, value)`.
    fn set_global_flags_and_data(&mut self, value: u8);
    /// `InitCommunication (race conversation)`.
    fn talk(&mut self);
    /// `ReinitQueue (&GLOBAL (npc_built_ship_q))`.
    fn reinit_built_ship_q(&mut self);
    /// `GetGroupInfo (GROUPS_RANDOM, GROUP_LOAD_IP)`.
    fn load_ip_groups(&mut self);
    /// `LoadStdLanderFont`, the ruins mask and the ruins report strings, with
    /// the report skipped to its second entry when `artifact_taken`.
    fn load_ruins_scene(&mut self, artifact_taken: bool);
}

/// The part of `generateOrbital` before the default orbital: true when the
/// conversation took the visit, so the default orbital is skipped.
pub fn enter_orbit(site: &mut impl HomeContactSite, artifact_taken: bool) -> bool {
    if site.start_sphere_tracking() {
        meet(site);
        true
    } else {
        site.load_ruins_scene(artifact_taken);
        false
    }
}

/// The contact sequence: the interplanetary groups are put away, the race's
/// flagship stands alone in the built-ship queue for the conversation, and
/// unless the conversation aborted or loaded a game the world is put back.
fn meet(site: &mut impl HomeContactSite) {
    site.notify_others_all_clear();
    site.save_ip_groups();
    site.reinit_ip_group_q();
    assert_eq!(
        site.built_ship_count(),
        0,
        "the built-ship queue must be empty before the flagship is cloned into it"
    );

    site.clone_fleet_into_built_queue();

    let activity = site.current_activity();
    site.set_current_activity(activity | START_INTERPLANETARY);
    site.set_global_flags_and_data(HOME_CONTACT_FLAGS);
    site.talk();

    let activity = site.current_activity();
    if activity & (CHECK_ABORT | CHECK_LOAD) == 0 {
        site.set_current_activity(activity & !START_INTERPLANETARY);
        site.reinit_built_ship_q();
        site.load_ip_groups();
    }
}

/// `SetLanderTakeoff ()`.
pub trait LanderTakeoff {
    fn lander_takeoff(&mut self);
}

/// The pickup of the ruins: the standard cycled report, and the first time
/// the artifact is the player's the lander takes off and `artifact_flag` and
/// the `also_set` flags are set. The ruin node is never removed.
pub fn pickup_artifact(
    story: &mut impl Story,
    report: &mut impl DiscoveryReportPort,
    lander: &mut impl LanderTakeoff,
    artifact_flag: &'static str,
    also_set: &[&'static str],
) -> bool {
    default::lander_report_cycle(report);
    if story.flag(artifact_flag) == 0 {
        lander.lander_takeoff();
        story.set_flag(artifact_flag, 1);
        for flag in also_set {
            story.set_flag(flag, 1);
        }
    }
    false
}

#[cfg(test)]
pub(crate) mod tests;
