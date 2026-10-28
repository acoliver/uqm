//! Empty native descriptors have explicit scalar and optional-handle initialization.
use super::abi::*;

impl race_desc {
    pub const EMPTY: Self = Self {
        ship_info: SHIP_INFO {
            ship_flags: 0,
            ship_cost: 0,
            crew_level: 0,
            max_crew: 0,
            energy_level: 0,
            max_energy: 0,
            race_strings_rsc: std::ptr::null(),
            icons_rsc: std::ptr::null(),
            melee_icon_rsc: std::ptr::null(),
            race_strings: std::ptr::null_mut(),
            icons: std::ptr::null_mut(),
            melee_icon: std::ptr::null_mut(),
        },
        fleet: FLEET_STUFF {
            strength: 0,
            known_loc: POINT { x: 0, y: 0 },
        },
        __bindgen_padding_0: [0; 5],
        characteristics: CHARACTERISTIC_STUFF {
            max_thrust: 0,
            thrust_increment: 0,
            energy_regeneration: 0,
            weapon_energy_cost: 0,
            special_energy_cost: 0,
            energy_wait: 0,
            turn_wait: 0,
            thrust_wait: 0,
            weapon_wait: 0,
            special_wait: 0,
            ship_mass: 0,
        },
        ship_data: DATA_STUFF {
            ship_rsc: [std::ptr::null(); 3],
            weapon_rsc: [std::ptr::null(); 3],
            special_rsc: [std::ptr::null(); 3],
            captain_control: CAPTAIN_STUFF {
                captain_rsc: std::ptr::null(),
                background: std::ptr::null_mut(),
                turn: std::ptr::null_mut(),
                thrust: std::ptr::null_mut(),
                weapon: std::ptr::null_mut(),
                special: std::ptr::null_mut(),
            },
            victory_ditty_rsc: std::ptr::null(),
            ship_sounds_rsc: std::ptr::null(),
            ship: [std::ptr::null_mut(); 3],
            weapon: [std::ptr::null_mut(); 3],
            special: [std::ptr::null_mut(); 3],
            victory_ditty: std::ptr::null_mut(),
            ship_sounds: std::ptr::null_mut(),
        },
        cyborg_control: INTEL_STUFF {
            ManeuverabilityIndex: 0,
            WeaponRange: 0,
            intelligence_func: None,
        },
        uninit_func: None,
        preprocess_func: None,
        postprocess_func: None,
        init_weapon_func: None,
        data: std::ptr::null_mut(),
        CodeRef: std::ptr::null_mut(),
        race: None,
        race_state: crate::engine::battle::RaceState::None,
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_descriptor_has_no_resources_or_callbacks() {
        let descriptor = race_desc::EMPTY;
        assert!(descriptor.data.is_null() && descriptor.CodeRef.is_null());
        assert!(descriptor.uninit_func.is_none() && descriptor.preprocess_func.is_none());
        assert!(descriptor.postprocess_func.is_none() && descriptor.init_weapon_func.is_none());
        assert!(descriptor.cyborg_control.intelligence_func.is_none());
        assert_eq!(descriptor.ship_info.crew_level, 0);
        assert_eq!(descriptor.characteristics.max_thrust, 0);
        for resource in descriptor
            .ship_data
            .ship_rsc
            .into_iter()
            .chain(descriptor.ship_data.weapon_rsc)
            .chain(descriptor.ship_data.special_rsc)
        {
            assert!(resource.is_null());
        }
        for frame in descriptor
            .ship_data
            .ship
            .into_iter()
            .chain(descriptor.ship_data.weapon)
            .chain(descriptor.ship_data.special)
        {
            assert!(frame.is_null());
        }
    }
}
