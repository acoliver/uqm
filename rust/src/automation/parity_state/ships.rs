//! Read-only ship and descriptor borrows, with no copied ABI records or padding.
use super::{
    custom::RaceData,
    resources::{Frame, StringEntry},
    value, Fields,
};
use crate::automation::parity::Journal;
use crate::ships::native::abi::*;
mod assets;

pub(super) struct Ship<'a> {
    pub(super) state: &'a STARSHIP,
}

impl<'a> Ship<'a> {
    /// # Safety
    /// The queue authority and pool must remain live and immutable for `'a`.
    pub(super) unsafe fn queue(queue: &'a crate::comm::locdata::CQueue) -> Vec<Self> {
        use crate::savegame::runtime::queues::{walk, QueueKind};
        if queue.pq_tab.is_null() && queue.head.is_null() {
            assert!(queue.tail.is_null(), "uninitialized ship queue has a tail");
            return Vec::new();
        }
        walk(queue, QueueKind::PlayerShips)
            .unwrap_or_else(|error| panic!("live ship queue: {error}"))
            .into_iter()
            .map(|pointer| {
                Self {
                    // SAFETY: walk checked the slot and links; the caller keeps the
                    // queue pool live and immutable for the returned borrow.
                    state: unsafe { &*pointer },
                }
            })
            .collect()
    }

    pub(super) fn address(&self) -> usize {
        std::ptr::from_ref(self.state) as usize
    }
    pub(super) fn side(&self) -> i16 {
        self.state.playerNr
    }
    pub(super) fn species(&self) -> u32 {
        self.state.SpeciesID
    }
    pub(super) fn facing(&self) -> u16 {
        self.state.ShipFacing
    }

    pub(super) fn write(&self, fields: &mut Fields, prefix: &str, journal: &Journal) {
        let s = self.state;
        for (name, number) in [
            ("species", i64::from(s.SpeciesID)),
            ("captain", i64::from(s.captains_name_index)),
            ("crew", i64::from(s.crew_level)),
            ("max_crew", i64::from(s.max_crew)),
            ("cost", i64::from(s.ship_cost)),
            ("index", i64::from(s.index)),
            ("weapon_counter", i64::from(s.weapon_counter)),
            ("special_counter", i64::from(s.special_counter)),
            ("energy_counter", i64::from(s.energy_counter)),
            ("input", i64::from(s.ship_input_state)),
            ("status", i64::from(s.cur_status_flags)),
            ("old_status", i64::from(s.old_status_flags)),
            ("facing", i64::from(s.ShipFacing)),
            ("side", i64::from(s.playerNr)),
            ("control", i64::from(s.control)),
        ] {
            value(fields, prefix, name, number);
        }
        value(fields, prefix, "identity", journal.parent(self.address()));
        value(fields, prefix, "pred", journal.parent(s.pred as usize));
        value(fields, prefix, "succ", journal.parent(s.succ as usize));
        value(
            fields,
            prefix,
            "element",
            if s.hShip as usize == self.address() {
                journal.parent(self.address())
            } else {
                journal.identity(s.hShip as usize)
            },
        );
    }

    /// # Safety
    /// All resource and custom-data pointers reachable from this ship must remain
    /// live and immutable for the returned borrow, on the game thread.
    #[cfg(not(test))]
    pub(super) unsafe fn resources(&self) -> ShipResources<'a> {
        let state = self.state;
        let icon_pointer = state
            .icons
            .cast::<crate::graphics::native_abi::frame_desc>();
        // SAFETY: the ship graph retains these native handles, descriptor and
        // its concrete species allocation for this immutable resource view.
        let (icons, strings, race) = unsafe {
            let race = state.RaceDescPtr.as_ref().map(|race| {
                (
                    race,
                    assets::Assets::borrow(&race.ship_info, &race.ship_data),
                    RaceData::borrow(state.SpeciesID, race),
                )
            });
            (icon_pointer.as_ref(), state.race_strings.as_ref(), race)
        };
        let race =
            race.map(|(race, assets, custom)| Race::new(state.SpeciesID, race, assets, custom));

        ShipResources {
            icons: Frame(icons),
            strings: StringEntry(strings),
            race,
        }
    }
}

pub(super) struct ShipResources<'a> {
    icons: Frame<'a>,
    strings: StringEntry<'a>,
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
    info: &'a SHIP_INFO,
    fleet: &'a FLEET_STUFF,
    characteristics: &'a CHARACTERISTIC_STUFF,
    intel: &'a INTEL_STUFF,
    assets: assets::Assets<'a>,
    custom: RaceData<'a>,
    callbacks: [usize; 4],
    code_loaded: bool,
}

impl<'a> Race<'a> {
    #[cfg(not(test))]
    fn new(
        species: u32,
        race: &'a RACE_DESC,
        assets: assets::Assets<'a>,
        custom: RaceData<'a>,
    ) -> Self {
        Self {
            species,
            address: std::ptr::from_ref(race) as usize,
            info: &race.ship_info,
            fleet: &race.fleet,
            characteristics: &race.characteristics,
            intel: &race.cyborg_control,
            assets,
            custom,
            callbacks: [
                race.uninit_func.map_or(0, |f| f as usize),
                race.preprocess_func.map_or(0, |f| f as usize),
                race.postprocess_func.map_or(0, |f| f as usize),
                race.init_weapon_func.map_or(0, |f| f as usize),
            ],
            code_loaded: !race.CodeRef.is_null(),
        }
    }

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
                i64::from(self.intel.ManeuverabilityIndex),
            ),
            ("cyborg.range", i64::from(self.intel.WeaponRange)),
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
        .zip(self.callbacks)
        .chain(std::iter::once((
            "race.intelligence",
            self.intel.intelligence_func.map_or(0, |f| f as usize),
        ))) {
            value(
                fields,
                prefix,
                name,
                crate::ships::hook_registry::name_for_species(hooks, address, self.species),
            );
        }
        value(fields, prefix, "race.code.loaded", self.code_loaded);
        self.assets.write(fields, prefix, journal);
        self.custom.write(fields, prefix, journal, hooks);
    }
}

pub(super) fn characteristics(fields: &mut Fields, prefix: &str, c: &CHARACTERISTIC_STUFF) {
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

    #[test]
    fn checked_ship_pool_serializes_the_borrowed_ship() {
        // SAFETY: STARSHIP consists of integers, pointers and optional callbacks.
        let mut state: STARSHIP = unsafe { std::mem::zeroed() };
        state.SpeciesID = SPECIES_ID_EARTHLING_ID;
        state.playerNr = 1;
        state.ShipFacing = 31;
        state.crew_level = 17;
        let address = std::ptr::from_mut(&mut state).cast();
        let queue = crate::comm::locdata::CQueue {
            head: address,
            tail: address,
            pq_tab: address.cast(),
            object_size: std::mem::size_of_val(&state) as u16,
            num_objects: 1,
            ..Default::default()
        };
        // SAFETY: the local pool remains live and immutable until ships is dropped.
        let ships = unsafe { Ship::queue(&queue) };
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
        // SAFETY: descriptor fields are integers, pointers or optional callbacks.
        let mut descriptor: RACE_DESC = unsafe { std::mem::zeroed() };
        descriptor.ship_info.max_energy = 23;
        descriptor.ship_info.icons_rsc = c"test.icons".as_ptr();
        descriptor.characteristics.max_thrust = 513;
        descriptor.characteristics.thrust_increment = 257;
        // SAFETY: the descriptor's only non-null resource is a static C string.
        let assets =
            unsafe { assets::Assets::borrow(&descriptor.ship_info, &descriptor.ship_data) };
        let resources = ShipResources {
            icons: Frame(None),
            strings: StringEntry(None),
            race: Some(Race {
                species: SPECIES_ID_EARTHLING_ID,
                address: std::ptr::from_ref(&descriptor) as usize,
                info: &descriptor.ship_info,
                fleet: &descriptor.fleet,
                characteristics: &descriptor.characteristics,
                intel: &descriptor.cyborg_control,
                assets,
                custom: RaceData::None,
                callbacks: [0; 4],
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
