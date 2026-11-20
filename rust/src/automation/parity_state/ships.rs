//! Read-only ship and descriptor borrows, with no copied ABI records or padding.
use super::{
    custom::RaceData,
    resources::{Frame, StringEntry},
    value, Fields,
};
use crate::automation::parity::Journal;
use crate::engine::battle::record::{
    CharacteristicRecord, FleetRecord, IntelRecord, ShipInfoRecord,
};
mod assets;

pub(super) struct Ship<'a> {
    pub(super) observation: crate::battle::context_access::observe::ShipObservation<'a>,
}
impl<'a> Ship<'a> {
    pub(super) fn queue(
        queue: &'a crate::engine::battle::link_queue::RacePool,
        elements: &crate::engine::battle::ElementTable,
    ) -> Vec<Self> {
        crate::battle::context_access::observe::ships(queue, elements)
            .into_iter()
            .map(|observation| Self { observation })
            .collect()
    }
    pub(super) fn address(&self) -> usize {
        self.observation.address
    }
    pub(super) fn side(&self) -> i16 {
        self.observation.state.player
    }
    pub(super) fn species(&self) -> u32 {
        self.observation.pick.species
    }
    #[cfg(test)]
    pub(super) fn facing(&self) -> u16 {
        self.observation.state.facing
    }
    pub(super) fn write(&self, fields: &mut Fields, prefix: &str, journal: &Journal) {
        let s = &self.observation;
        for (name, number) in [
            ("species", i64::from(s.pick.species)),
            ("captain", i64::from(s.pick.captain)),
            ("crew", i64::from(s.pick.crew)),
            ("max_crew", i64::from(s.pick.max_crew)),
            ("cost", i64::from(s.pick.cost)),
            ("index", i64::from(s.pick.index)),
            ("weapon_counter", i64::from(s.state.weapon_counter)),
            ("special_counter", i64::from(s.state.special_counter)),
            ("energy_counter", i64::from(s.state.energy_counter)),
            ("input", i64::from(s.state.input)),
            ("status", i64::from(s.state.status)),
            ("old_status", i64::from(s.state.old_status)),
            ("facing", i64::from(s.state.facing)),
            ("side", i64::from(s.state.player)),
            ("control", i64::from(s.state.control)),
        ] {
            value(fields, prefix, name, number);
        }
        value(fields, prefix, "identity", journal.parent(self.address()));
        value(fields, prefix, "pred", journal.parent(s.pred));
        value(fields, prefix, "succ", journal.parent(s.succ));
        value(
            fields,
            prefix,
            "element",
            if s.element == self.address() {
                journal.parent(self.address())
            } else {
                journal.identity(s.element)
            },
        );
    }

    pub(super) fn resources(&self) -> ShipResources<'_> {
        let state = &self.observation;
        let race = state.race.as_ref().map(|race| {
            let assets = assets::Assets::borrow(race.info, race.data);
            Race {
                species: state.pick.species,
                address: race.address,
                info: race.info,
                fleet: race.fleet,
                characteristics: race.characteristics,
                intel: race.intel,
                assets,
                custom: &race.custom,
                callbacks: race.callbacks,
                code_loaded: race.code_loaded,
            }
        });
        let icons = state.icons;
        let strings = state.strings;
        ShipResources {
            icons: Frame(icons),
            strings: StringEntry(strings),
            race,
        }
    }
}

pub(super) struct ShipResources<'a> {
    icons: Frame,
    strings: StringEntry,
    race: Option<Race<'a>>,
}
impl ShipResources<'_> {
    pub(super) fn register(&self, journal: &mut Journal, owner: &str) {
        journal.register_resource(self.strings.address(), "strings");
        if let Some(race) = &self.race {
            journal.register_resource(race.address, "descriptor");
            journal.register_resource_group(race.address, race.assets.resource_group());
            race.assets.register(
                journal,
                &format!("{owner}:{}", journal.resource(race.address)),
            );
        }
    }
    pub(super) fn write(
        &self,
        fields: &mut Fields,
        prefix: &str,
        journal: &Journal,
        hooks: &[(usize, &'static str)],
    ) {
        self.icons
            .write(fields, &format!("{prefix}.icons"), journal);
        self.strings
            .write(fields, &format!("{prefix}.strings"), journal);
        value(fields, prefix, "race.present", self.race.is_some());
        if let Some(race) = &self.race {
            race.write(fields, prefix, journal, hooks);
        }
    }
}

struct Race<'a> {
    species: u32,
    address: usize,
    info: &'a ShipInfoRecord,
    fleet: &'a FleetRecord,
    characteristics: &'a CharacteristicRecord,
    intel: &'a IntelRecord,
    assets: assets::Assets<'a>,
    custom: &'a RaceData<'a>,
    callbacks: [usize; 5],
    code_loaded: bool,
}

impl<'a> Race<'a> {
    fn write(
        &self,
        fields: &mut Fields,
        prefix: &str,
        journal: &Journal,
        hooks: &[(usize, &'static str)],
    ) {
        value(
            fields,
            prefix,
            "race.identity",
            journal.resource(self.address),
        );
        let info = self.info;
        for (name, number) in [
            ("race.flags", i64::from(info.ship_flags)),
            ("race.cost", i64::from(info.ship_cost)),
            ("race.crew", i64::from(info.crew_level)),
            ("race.max_crew", i64::from(info.max_crew)),
            ("race.energy", i64::from(info.energy_level)),
            ("race.max_energy", i64::from(info.max_energy)),
            (
                "cyborg.maneuverability",
                i64::from(self.intel.maneuverability_index),
            ),
            ("cyborg.range", i64::from(self.intel.weapon_range)),
            ("fleet.strength", i64::from(self.fleet.strength)),
            ("fleet.x", i64::from(self.fleet.known_loc.x)),
            ("fleet.y", i64::from(self.fleet.known_loc.y)),
        ] {
            value(fields, prefix, name, number);
        }
        characteristics(
            fields,
            &format!("{prefix}.characteristics"),
            self.characteristics,
        );
        for (name, address) in [
            "race.uninit",
            "race.preprocess",
            "race.postprocess",
            "race.weapon",
        ]
        .into_iter()
        .chain(std::iter::once("race.intelligence"))
        .zip(self.callbacks)
        {
            value(
                fields,
                prefix,
                name,
                crate::battle::hook_registry::name_for_species(hooks, address, self.species),
            );
        }
        value(fields, prefix, "race.code.loaded", self.code_loaded);
        self.assets.write(fields, prefix, journal);
        super::custom::write_custom(self.custom, fields, prefix, journal, hooks);
    }
}

pub(super) fn characteristics(fields: &mut Fields, prefix: &str, c: &CharacteristicRecord) {
    for (name, number) in [
        ("max_thrust", c.max_thrust),
        ("thrust_increment", c.thrust_increment),
        ("energy_regeneration", u16::from(c.energy_regeneration)),
        ("weapon_energy_cost", u16::from(c.weapon_energy_cost)),
        ("special_energy_cost", u16::from(c.special_energy_cost)),
        ("energy_wait", u16::from(c.energy_wait)),
        ("turn_wait", u16::from(c.turn_wait)),
        ("thrust_wait", u16::from(c.thrust_wait)),
        ("weapon_wait", u16::from(c.weapon_wait)),
        ("special_wait", u16::from(c.special_wait)),
        ("mass", u16::from(c.ship_mass)),
    ] {
        value(fields, prefix, name, number);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::battle::record::{RaceDescriptor, ShipRecord};
    use crate::engine::species::species_id::SPECIES_ID_EARTHLING_ID;

    #[test]
    fn checked_ship_pool_serializes_the_borrowed_ship() {
        let mut state = ShipRecord::default();
        state.species_id = SPECIES_ID_EARTHLING_ID;
        state.player_nr = 1;
        state.ship_facing = 31;
        state.crew_level = 17;
        let mut queue = crate::engine::battle::link_queue::RacePool::with_capacity(1, 1);
        let id = queue.fixture_pool().alloc(state).unwrap();
        queue.fixture_pool().put(id);
        let ships = Ship::queue(
            &queue,
            &crate::engine::battle::ElementTable::with_capacity(0),
        );
        let ship = &ships[0];
        assert_eq!(ship.side(), 1);
        assert_eq!(ship.species(), SPECIES_ID_EARTHLING_ID);
        assert_eq!(ship.facing(), 31);
        let directory = super::super::test_directory();
        let mut journal = Journal::create(directory.path())
            .unwrap_or_else(|error| panic!("create ship test journal: {error}"));
        journal.register_parent(ship.address(), "ship:test".into());
        let mut fields = Fields::new();
        ship.write(&mut fields, "ship", &journal);
        assert_eq!(fields["ship.identity"], "ship:test");
        assert_eq!(fields["ship.crew"], "17");
    }

    #[test]
    fn borrowed_descriptor_resources_register_and_serialize() {
        let mut descriptor = RaceDescriptor::EMPTY;
        descriptor.ship_info.max_energy = 23;
        descriptor.ship_info.icons_rsc = Some(c"test.icons");
        descriptor.characteristics.max_thrust = 513;
        descriptor.characteristics.thrust_increment = 257;
        let assets = assets::Assets::borrow(&descriptor.ship_info, &descriptor.ship_data);
        let resources = ShipResources {
            icons: Frame(crate::graphics::drawable::FrameId::NULL),
            strings: StringEntry(None),
            race: Some(Race {
                species: SPECIES_ID_EARTHLING_ID,
                address: std::ptr::from_ref(&descriptor) as usize,
                info: &descriptor.ship_info,
                fleet: &descriptor.fleet,
                characteristics: &descriptor.characteristics,
                intel: &descriptor.cyborg_control,
                assets,
                custom: &RaceData::None,
                callbacks: [0; 5],
                code_loaded: false,
            }),
        };
        let directory = super::super::test_directory();
        let mut journal = Journal::create(directory.path())
            .unwrap_or_else(|error| panic!("create descriptor test journal: {error}"));
        resources.register(&mut journal, "ship:test");
        let mut fields = Fields::new();
        resources.write(&mut fields, "ship", &journal, &[]);
        assert_eq!(fields["ship.race.max_energy"], "23");
        assert_eq!(fields["ship.race.icons.resource"], "test.icons");
        assert_eq!(fields["ship.characteristics.max_thrust"], "513");
        assert_eq!(fields["ship.characteristics.thrust_increment"], "257");
        assert_eq!(fields["ship.race.weapon"], "none");
        assert_eq!(fields["ship.data.victory.loaded"], "false");
    }
}
