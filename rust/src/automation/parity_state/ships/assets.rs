//! Descriptor asset keys and borrowed loaded handles, never pointer values.
use super::super::{
    resources::{Frame, Resource, StringEntry},
    value, Fields,
};
use crate::automation::parity::Journal;
use uqm_engine::battle::record::{DataRecord, ShipInfoRecord};
use uqm_engine::graphics::drawable::FrameId;
use uqm_engine::resource::ResourceKey;

pub(super) struct Assets<'a> {
    keys: Vec<(&'static str, Resource<'a>)>,
    frames: Vec<(&'static str, Frame)>,
    strings: StringEntry,
    sounds: StringEntry,
    music: usize,
    sets: [(&'static str, &'a uqm_engine::battle::frames::FrameArray); 3],
}

impl<'a> Assets<'a> {
    pub(super) fn borrow(info: &'a ShipInfoRecord, data: &'a DataRecord) -> Self {
        let keys = resource_keys(info, data)
            .into_iter()
            .map(|(name, key)| (name, Resource(key)))
            .collect();
        let frames = frame_handles(info, data)
            .into_iter()
            .map(|(name, pointer)| (name, Frame(pointer)))
            .collect();
        let (strings, sounds) = (info.race_strings, data.ship_sounds);
        Self {
            keys,
            frames,
            strings: StringEntry(strings),
            sounds: StringEntry(sounds),
            music: data.victory_ditty as usize,
            sets: [
                ("ship", &data.ship),
                ("weapon", &data.weapon),
                ("special", &data.special),
            ],
        }
    }

    pub(super) fn resource_group(&self) -> [usize; 3] {
        [self.strings.address(), self.sounds.address(), self.music]
    }

    pub(super) fn register(&self, journal: &mut Journal, owner: &str) {
        journal.register_resource(self.strings.address(), "strings");
        journal.register_resource(self.sounds.address(), "sounds");
        journal.register_resource(self.music, "music");
        for (role, views) in self
            .sets
            .into_iter()
            .filter(|(_, views)| !views.is_absent())
        {
            journal.register_frame_set(
                views.as_ptr() as usize,
                views
                    .views()
                    .iter()
                    .map(|frame| frame.raw().bits())
                    .collect(),
                format!("frame-set:{owner}:{role}"),
            );
        }
    }

    pub(super) fn write(&self, fields: &mut Fields, prefix: &str, journal: &Journal) {
        for (name, key) in &self.keys {
            key.write(fields, prefix, name);
        }
        for (name, frame) in &self.frames {
            frame.write(fields, &format!("{prefix}.{name}"), journal);
        }
        self.strings
            .write(fields, &format!("{prefix}.race.strings"), journal);
        self.sounds
            .write(fields, &format!("{prefix}.data.sounds"), journal);
        value(fields, prefix, "data.victory.loaded", self.music != 0);
        value(
            fields,
            prefix,
            "data.victory.resource_owner",
            journal.resource(self.music),
        );
    }
}

fn resource_keys(info: &ShipInfoRecord, data: &DataRecord) -> [(&'static str, ResourceKey); 15] {
    let captain = &data.captain_control;
    [
        ("race.strings.resource", info.race_strings_rsc),
        ("race.icons.resource", info.icons_rsc),
        ("race.melee_icon.resource", info.melee_icon_rsc),
        ("data.captain.resource", captain.captain_rsc),
        ("data.victory.resource", data.victory_ditty_rsc),
        ("data.sounds.resource", data.ship_sounds_rsc),
        ("data.ship.0.resource", data.ship_rsc[0]),
        ("data.ship.1.resource", data.ship_rsc[1]),
        ("data.ship.2.resource", data.ship_rsc[2]),
        ("data.weapon.0.resource", data.weapon_rsc[0]),
        ("data.weapon.1.resource", data.weapon_rsc[1]),
        ("data.weapon.2.resource", data.weapon_rsc[2]),
        ("data.special.0.resource", data.special_rsc[0]),
        ("data.special.1.resource", data.special_rsc[1]),
        ("data.special.2.resource", data.special_rsc[2]),
    ]
}

fn frame_handles(info: &ShipInfoRecord, data: &DataRecord) -> [(&'static str, FrameId); 16] {
    let captain = &data.captain_control;
    [
        ("race.icons", info.icons),
        ("race.melee_icon", info.melee_icon),
        ("data.captain.background", captain.background),
        ("data.captain.turn", captain.turn),
        ("data.captain.thrust", captain.thrust),
        ("data.captain.weapon", captain.weapon),
        ("data.captain.special", captain.special.raw()),
        ("data.ship.0", data.ship.view(0).raw()),
        ("data.ship.1", data.ship.view(1).raw()),
        ("data.ship.2", data.ship.view(2).raw()),
        ("data.weapon.0", data.weapon.view(0).raw()),
        ("data.weapon.1", data.weapon.view(1).raw()),
        ("data.weapon.2", data.weapon.view(2).raw()),
        ("data.special.0", data.special.view(0).raw()),
        ("data.special.1", data.special.view(1).raw()),
        ("data.special.2", data.special.view(2).raw()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn descriptor_registration_distinguishes_absent_arrays_from_present_empty_views() {
        use uqm_engine::battle::{frames::FrameArray, record::RaceDescriptor};
        let directory = crate::automation::parity_state::test_directory();
        let mut journal = Journal::create(directory.path())
            .unwrap_or_else(|error| panic!("create frame-source journal: {error}"));
        let mut descriptor = RaceDescriptor::EMPTY;
        descriptor.ship_data.ship =
            FrameArray::fixture([uqm_engine::graphics::drawable::FrameId::NULL; 3], ());
        let assets = Assets::borrow(&descriptor.ship_info, &descriptor.ship_data);
        assets.register(&mut journal, "test");
        assert_eq!(journal.frame_set(0), None);
        assert_eq!(
            journal.frame_set(descriptor.ship_data.ship.as_ptr() as usize),
            Some([0; 3].as_slice())
        );
    }
}
