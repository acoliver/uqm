//! Plain descriptor inputs; legacy resource pointer construction stays in the engine.

use crate::engine::battle::{record::*, views::*, Race, RaceDesc, RaceState};
use crate::graphics::native_abi::POINT;
use std::ffi::CStr;

#[derive(Clone, Copy, Debug, Default)]
pub struct RaceResources {
    pub strings: Option<&'static CStr>,
    pub icons: Option<&'static CStr>,
    pub melee_icon: Option<&'static CStr>,
    pub ship: [Option<&'static CStr>; 3],
    pub weapon: [Option<&'static CStr>; 3],
    pub special: [Option<&'static CStr>; 3],
    pub captain: Option<&'static CStr>,
    pub victory: Option<&'static CStr>,
    pub sounds: Option<&'static CStr>,
}

impl RaceResources {
    pub const EMPTY: Self = Self {
        strings: None,
        icons: None,
        melee_icon: None,
        ship: [None; 3],
        weapon: [None; 3],
        special: [None; 3],
        captain: None,
        victory: None,
        sounds: None,
    };
}

#[derive(Clone, Copy, Debug)]
pub struct RaceTemplate {
    pub info: Info,
    pub characteristics: CHARACTERISTIC_STUFF,
    pub intelligence: Intelligence,
    pub fleet_strength: u16,
    pub fleet_location: POINT,
    pub resources: RaceResources,
    pub state: RaceState,
}

impl Default for RaceTemplate {
    fn default() -> Self {
        Self {
            info: Info::default(),
            characteristics: RaceDesc::empty().raw.characteristics,
            intelligence: Intelligence::default(),
            fleet_strength: 0,
            fleet_location: POINT { x: 0, y: 0 },
            resources: RaceResources::default(),
            state: RaceState::None,
        }
    }
}

impl RaceTemplate {
    pub fn descriptor(self, race: &'static Race) -> RaceDesc {
        let mut raw = RaceDesc::empty().raw;
        RaceStats::apply(
            &mut raw,
            RaceStats {
                ship_info: self.info,
                characteristics: self.characteristics,
                intelligence: self.intelligence,
                state: self.state,
            },
        );
        let token = |s: Option<&'static CStr>| s.map_or(std::ptr::null(), CStr::as_ptr);
        let r = self.resources;
        raw.ship_info.race_strings_rsc = token(r.strings);
        raw.ship_info.icons_rsc = token(r.icons);
        raw.ship_info.melee_icon_rsc = token(r.melee_icon);
        raw.ship_data.ship_rsc = r.ship.map(token);
        raw.ship_data.weapon_rsc = r.weapon.map(token);
        raw.ship_data.special_rsc = r.special.map(token);
        raw.ship_data.captain_control.captain_rsc = token(r.captain);
        raw.ship_data.victory_ditty_rsc = token(r.victory);
        raw.ship_data.ship_sounds_rsc = token(r.sounds);
        raw.fleet.strength = self.fleet_strength;
        raw.fleet.known_loc = self.fleet_location;
        raw.race = Some(race);
        raw.resources = r;
        RaceDesc::from_storage(raw)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    static RACE: Race = Race {
        descriptor: |_| unreachable!(),
        preprocess: None,
        postprocess: None,
        init_weapon: None,
        intelligence: None,
        uninit: None,
    };

    #[test]
    fn typed_resource_keys_survive_changes_to_legacy_mirrors() {
        let resources = RaceResources {
            strings: Some(c"race.names"),
            icons: Some(c"race.icons"),
            ship: [Some(c"race.big"), Some(c"race.medium"), None],
            ..RaceResources::default()
        };
        let mut descriptor = RaceTemplate {
            resources,
            ..RaceTemplate::default()
        }
        .descriptor(&RACE);
        descriptor.raw.ship_info.icons_rsc = std::ptr::null();
        descriptor.raw.ship_data.ship_rsc = [std::ptr::null(); 3];
        assert_eq!(descriptor.raw.resources.strings, resources.strings);
        assert_eq!(descriptor.raw.resources.icons, resources.icons);
        assert_eq!(descriptor.raw.resources.ship, resources.ship);
        let descriptor = RaceDesc::from_legacy(descriptor.into_legacy());
        assert_eq!(descriptor.raw.resources.strings, resources.strings);
        assert_eq!(descriptor.raw.resources.icons, resources.icons);
        assert_eq!(descriptor.raw.resources.ship, resources.ship);
    }
}
