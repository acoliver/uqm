//! Empty native descriptors have explicit scalar and optional-handle initialization.
use super::record::*;
use crate::graphics::geometry::Point;

impl RaceDescriptor {
    pub const EMPTY: Self = Self {
        assets: None,
        loaded_victory: None,
        loaded_sounds: None,
        resources: crate::battle::template::RaceResources::EMPTY,
        ship_info: ShipInfoRecord {
            ship_flags: 0,
            ship_cost: 0,
            crew_level: 0,
            max_crew: 0,
            energy_level: 0,
            max_energy: 0,
            race_strings_rsc: std::ptr::null(),
            icons_rsc: std::ptr::null(),
            melee_icon_rsc: std::ptr::null(),
            race_strings: None,
            icons: crate::graphics::drawable::FrameId::NULL,
            melee_icon: crate::graphics::drawable::FrameId::NULL,
        },
        fleet: FleetRecord {
            strength: 0,
            known_loc: Point { x: 0, y: 0 },
        },
        __bindgen_padding_0: [0; 5],
        characteristics: CharacteristicRecord {
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
        ship_data: DataRecord {
            ship_rsc: [std::ptr::null(); 3],
            weapon_rsc: [std::ptr::null(); 3],
            special_rsc: [std::ptr::null(); 3],
            captain_control: CaptainRecord {
                captain_rsc: std::ptr::null(),
                background: crate::graphics::drawable::FrameId::NULL,
                turn: crate::graphics::drawable::FrameId::NULL,
                thrust: crate::graphics::drawable::FrameId::NULL,
                weapon: crate::graphics::drawable::FrameId::NULL,
                special: crate::battle::frames::FrameSelection::ABSENT,
            },
            victory_ditty_rsc: std::ptr::null(),
            ship_sounds_rsc: std::ptr::null(),
            ship: crate::battle::frames::FrameArray::EMPTY,
            weapon: crate::battle::frames::FrameArray::EMPTY,
            special: crate::battle::frames::FrameArray::EMPTY,
            victory_ditty: std::ptr::null_mut(),
            ship_sounds: None,
        },
        cyborg_control: IntelRecord {
            maneuverability_index: 0,
            weapon_range: 0,
        },
        race: None,
        race_state: crate::battle::RaceState::None,
        race_callbacks: crate::battle::Callbacks {
            pre: crate::battle::Selection::Default,
            post: crate::battle::Selection::Default,
            weapon: crate::battle::Selection::Default,
        },
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_descriptor_has_no_resources_or_callbacks() {
        let descriptor = RaceDescriptor::EMPTY;
        assert!(descriptor.race.is_none());
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
        for frames in [
            descriptor.ship_data.ship,
            descriptor.ship_data.weapon,
            descriptor.ship_data.special,
        ] {
            assert!(frames.is_absent());
        }
    }
}
