//! Deterministic boundary journal. Host timing and pointer identities never enter its files.

use std::collections::BTreeMap;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

use sha2::{Digest, Sha256};
mod bindings;
mod frames;

pub type Fields = BTreeMap<String, String>;

pub fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)[..8]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

pub struct Journal {
    trace: BufWriter<File>,
    fields: BufWriter<File>,
    step: u64,
    jitter: Option<u64>,
    slots: BTreeMap<usize, (u64, u64)>,
    next_slot: u64,
    events: Vec<String>,
    generations: Vec<u64>,
    owners: BTreeMap<usize, (i16, u32)>,
    parents: BTreeMap<usize, String>,
    resources: BTreeMap<usize, String>,
    resource_groups: BTreeMap<usize, [usize; 3]>,
    next_resource: u64,
    spawn_count: [u64; 2],
    pub(super) space_active: bool,
    frames: BTreeMap<usize, (String, u16)>,
    frame_sets: BTreeMap<usize, Vec<usize>>,
    retired_drawables: std::collections::BTreeSet<String>,
    bindings: bindings::Bindings,
}

impl Journal {
    pub fn create(directory: &Path) -> std::io::Result<Self> {
        let jitter = std::env::var("UQM_REPLAY_JITTER")
            .ok()
            .map(|value| {
                value.parse::<u64>().map_err(|error| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        format!("UQM_REPLAY_JITTER must be an integer: {error}"),
                    )
                })
            })
            .transpose()?;
        Ok(Self {
            trace: BufWriter::new(File::create(directory.join("parity.trace"))?),
            fields: BufWriter::new(File::create(directory.join("parity.fields"))?),
            step: 0,
            jitter,
            slots: BTreeMap::new(),
            next_slot: 0,
            events: Vec::new(),
            generations: Vec::new(),
            owners: BTreeMap::new(),
            parents: BTreeMap::new(),
            resources: BTreeMap::new(),
            resource_groups: BTreeMap::new(),
            next_resource: 0,
            spawn_count: [0; 2],
            space_active: false,
            frames: BTreeMap::new(),
            frame_sets: BTreeMap::new(),
            retired_drawables: std::collections::BTreeSet::new(),
            bindings: bindings::Bindings::default(),
        })
    }

    fn boundary(&mut self) {
        if let Some(state) = &mut self.jitter {
            *state = state.wrapping_add(0x9e3779b97f4a7c15);
            let mut value = *state;
            value = (value ^ (value >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
            value = (value ^ (value >> 27)).wrapping_mul(0x94d049bb133111eb);
            value ^= value >> 31;
            std::thread::sleep(std::time::Duration::from_millis(value % 21));
        }
    }

    pub fn allocate(&mut self, address: usize) {
        self.forget_element_images(address);
        if address == 0 {
            self.events.push("pool-full".into());
            return;
        }
        if let Some((slot, generation)) = self.slots.get_mut(&address) {
            self.generations[*slot as usize] += 1;
            *generation = self.generations[*slot as usize];
        } else {
            let slot = self.next_slot;
            self.next_slot += 1;
            if self.generations.len() == slot as usize {
                self.generations.push(0);
            }
            self.generations[slot as usize] += 1;
            self.slots
                .insert(address, (slot, self.generations[slot as usize]));
        }
    }

    pub fn reset_pool(&mut self) {
        self.bindings = bindings::Bindings::default();
        self.slots.clear();
        self.next_slot = 0;
    }

    pub fn identity(&self, address: usize) -> String {
        if address == 0 {
            return "none".into();
        }
        let Some((slot, generation)) = self.slots.get(&address) else {
            panic!("unobserved element allocation at {address:#x}");
        };
        format!("{slot}:{generation}")
    }

    pub fn event(&mut self, event: String) {
        self.events.push(event);
    }

    pub fn parent(&self, address: usize) -> String {
        if address == 0 {
            return "none".into();
        }
        match self.parents.get(&address) {
            Some(name) => name.clone(),
            None => panic!("unregistered logical parent at {address:#x}"),
        }
    }
    pub fn register_parent(&mut self, address: usize, name: String) {
        if name.starts_with("group:") || name.starts_with("frame-set:") {
            self.parents.insert(address, name);
        } else {
            self.parents.entry(address).or_insert(name);
        }
    }
    pub fn resource(&self, address: usize) -> String {
        if address == 0 {
            return "none".into();
        }
        match self.resources.get(&address) {
            Some(name) => name.clone(),
            None => panic!("unobserved drawable allocation at {address:#x}"),
        }
    }

    pub fn register_resource_group(&mut self, descriptor: usize, assets: [usize; 3]) {
        self.resource_groups.insert(descriptor, assets);
    }

    fn release_resource_group(&mut self, descriptor: usize, icons: bool, battle: bool) {
        self.resources.remove(&descriptor);
        if let Some([strings, sounds, music]) = self.resource_groups.remove(&descriptor) {
            for (address, released) in [(strings, icons), (sounds, battle), (music, battle)] {
                if released {
                    self.resources.remove(&address);
                }
            }
        }
    }

    pub fn register_resource(&mut self, address: usize, kind: &str) -> String {
        if address == 0 {
            return "none".into();
        }
        self.resources
            .entry(address)
            .or_insert_with(|| {
                let identity = format!("{kind}:{}", self.next_resource);
                self.next_resource += 1;
                identity
            })
            .clone()
    }

    pub fn owner(&mut self, address: usize, side: i16, species: u32) {
        self.owners.entry(address).or_insert((side, species));
    }

    pub fn drain_events(&mut self, registry: &[(usize, &'static str)]) {
        use crate::collections::observation::{drain, Event};
        for event in drain() {
            match event {
                Event::Allocate(address) => self.allocate(address),
                Event::Free(address) => {
                    self.events.push(format!("free {}", self.identity(address)));
                    self.forget_element_images(address);
                }
                Event::ResetPool => self.reset_pool(),
                Event::SpaceActive(active) => {
                    self.space_active = active;
                    self.events.push(format!("space-active {active}"));
                }
                Event::ResourcesFree {
                    descriptor,
                    icons,
                    battle,
                } => {
                    self.release_resource_group(descriptor, icons, battle);
                }
                Event::Frame {
                    address,
                    owner,
                    index,
                } => self.register_frame(address, owner, index),
                Event::DrawableFree(address) => self.retire_drawable(address),
                Event::Drawable(address) => {
                    if address != 0 {
                        self.resources
                            .insert(address, format!("drawable:{}", self.next_resource));
                        self.next_resource += 1;
                    }
                }
                Event::Spawn {
                    address,
                    side,
                    species,
                } => {
                    self.owners.insert(address, (side, species));
                    let side_index = match side {
                        0 => 0,
                        1 => 1,
                        other => panic!("spawned ship has invalid side {other}"),
                    };
                    let count = &mut self.spawn_count[side_index];
                    *count += 1;
                    self.parents
                        .insert(address, format!("ship:{side}:{count}:{species}"));
                    self.event(format!("spawn {side} {species}"));
                }
                Event::Hook { parent, callback } => {
                    let name = crate::ships::hook_registry::name_for_species(
                        registry,
                        callback,
                        self.owner_species(parent).1,
                    );
                    self.events
                        .push(format!("hook {} {name}", self.owner_species(parent).1));
                    if name.ends_with("::ship_death") {
                        let (side, species) = self.owner_species(parent);
                        self.events.push(format!("death {side} {species}"));
                    }
                }
            }
        }
    }

    pub fn species(&self, parent: usize) -> u32 {
        self.owners.get(&parent).map_or(0, |owner| owner.1)
    }

    fn owner_species(&self, parent: usize) -> (i16, u32) {
        if parent == 0 {
            (-1, 0)
        } else {
            match self.owners.get(&parent) {
                Some(owner) => *owner,
                None => panic!("hook owner was not observed at {parent:#x}"),
            }
        }
    }

    pub fn input(&mut self, count: u64, activity: u16, keys: &str) -> std::io::Result<()> {
        self.boundary();
        writeln!(self.trace, "I {count} {activity} {keys}")
    }

    pub fn checkpoint(&mut self, activity: u16, crc: u32, fields: &Fields) -> std::io::Result<()> {
        self.boundary();
        self.step += 1;
        let bytes = serde_json::to_vec(fields)?;
        write!(
            self.trace,
            "S {} {activity} {crc:08x} {}",
            self.step,
            digest(&bytes)
        )?;
        for event in self.events.drain(..) {
            write!(self.trace, " [{event}]")?;
        }
        writeln!(self.trace)?;
        writeln!(
            self.fields,
            "{} {}",
            self.step,
            String::from_utf8_lossy(&bytes)
        )
    }

    pub fn present(&mut self, frame: &super::capture::PresentedFrame) -> std::io::Result<()> {
        self.boundary();
        writeln!(
            self.trace,
            "P {} {} {}",
            frame.count,
            frame.generation,
            digest(&frame.rgba)
        )
    }

    pub fn flush(&mut self) -> std::io::Result<()> {
        self.trace.flush()?;
        self.fields.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retired_descriptor_resources_do_not_depend_on_allocator_reuse(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let one = tempfile::tempdir()?;
        let two = tempfile::tempdir()?;
        let mut reused = Journal::create(one.path())?;
        let mut fresh = Journal::create(two.path())?;
        for journal in [&mut reused, &mut fresh] {
            for (address, kind) in [
                (10, "descriptor"),
                (20, "strings"),
                (30, "sounds"),
                (40, "music"),
            ] {
                journal.register_resource(address, kind);
            }
            journal.register_resource_group(10, [20, 30, 40]);
            journal.release_resource_group(10, true, true);
        }
        for ((old, new), kind) in [(10, 100), (20, 200), (30, 300), (40, 400)]
            .into_iter()
            .zip(["descriptor", "strings", "sounds", "music"])
        {
            assert_eq!(
                reused.register_resource(old, kind),
                fresh.register_resource(new, kind)
            );
        }
        reused.register_resource_group(10, [20, 30, 40]);
        let strings = reused.resource(20);
        reused.release_resource_group(10, false, true);
        assert_eq!(
            reused.resource(20),
            strings,
            "icons not released must retain identity"
        );
        Ok(())
    }

    #[test]
    fn boundaries_and_fields_are_stable_and_exclude_addresses(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let directory = tempfile::tempdir()?;
        let mut journal = Journal::create(directory.path())?;
        journal.allocate(0x1234);
        journal.allocate(0x5678);
        assert_eq!(journal.identity(0), "none");
        assert_eq!(journal.identity(0x1234), "0:1");
        journal.allocate(0x1234);
        assert_eq!(journal.identity(0x1234), "0:2");
        assert_eq!(journal.identity(0x5678), "1:1");
        let fields = Fields::from([("crew".into(), "12".into())]);
        journal.input(1, 512, "weapon")?;
        journal.event("spawn 0 7".into());
        journal.checkpoint(512, 0x1234, &fields)?;
        journal.present(&super::super::capture::PresentedFrame {
            count: 1,
            generation: 0,
            width: 1,
            height: 1,
            rgba: vec![0, 0, 0, 255],
        })?;
        journal.flush()?;
        assert_eq!(
            std::fs::read_to_string(directory.path().join("parity.trace"))?,
            format!(
                "I 1 512 weapon\nS 1 512 00001234 {} [spawn 0 7]\nP 1 0 {}\n",
                digest(&serde_json::to_vec(&fields)?),
                digest(&[0, 0, 0, 255])
            )
        );
        assert_eq!(
            std::fs::read_to_string(directory.path().join("parity.fields"))?,
            "1 {\"crew\":\"12\"}\n"
        );
        assert_ne!(digest(b"crew=12"), digest(b"crew=11"));
        Ok(())
    }

    #[test]
    fn pool_reinitialization_does_not_depend_on_heap_address_reuse(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let left = tempfile::tempdir()?;
        let right = tempfile::tempdir()?;
        let mut a = Journal::create(left.path())?;
        let mut b = Journal::create(right.path())?;
        a.allocate(100);
        b.allocate(200);
        a.reset_pool();
        b.reset_pool();
        a.allocate(100);
        b.allocate(300);
        assert_eq!(a.identity(100), "0:2");
        assert_eq!(a.identity(100), b.identity(300));
        Ok(())
    }

    #[test]
    #[should_panic(expected = "unobserved element allocation")]
    fn unknown_identity_fails() {
        let directory = tempfile::tempdir().unwrap();
        Journal::create(directory.path()).unwrap().identity(1);
    }

    #[test]
    fn media_identity_distinguishes_loaded_handles_without_recording_addresses(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let left = tempfile::tempdir()?;
        let right = tempfile::tempdir()?;
        let mut a = Journal::create(left.path())?;
        let mut b = Journal::create(right.path())?;
        assert_eq!(a.register_resource(0, "music"), "none");
        assert_eq!(
            a.register_resource(10, "string-table"),
            b.register_resource(1000, "string-table")
        );
        assert_eq!(a.resource(10), a.register_resource(10, "string-table"));
        assert_ne!(a.resource(10), a.register_resource(20, "string-table"));
        assert_eq!(a.register_resource(30, "music"), "music:2");
        Ok(())
    }

    #[test]
    fn snapshot_owner_seeding_preserves_pending_old_spawn_events(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let directory = tempfile::tempdir()?;
        let mut journal = Journal::create(directory.path())?;
        journal.owner(17, 0, 3);
        journal.owner(17, 1, 8);
        assert_eq!(journal.owner_species(17), (0, 3));
        Ok(())
    }

    #[test]
    fn reused_group_address_refreshes_logical_identity_without_replacing_spawn_identity(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let directory = tempfile::tempdir()?;
        let mut journal = Journal::create(directory.path())?;
        journal.register_parent(17, "group:31".into());
        journal.register_parent(17, "group:32".into());
        assert_eq!(journal.parent(17), "group:32");
        journal.parents.insert(18, "ship:0:2:5".into());
        journal.register_parent(18, "ships.0.0".into());
        assert_eq!(journal.parent(18), "ship:0:2:5");
        journal.register_parent(19, "frame-set:ship:0:1:5:ship".into());
        journal.register_parent(19, "frame-set:ship:0:2:5:ship".into());
        journal.register_parent(19, "frame-single:fallback".into());
        assert_eq!(journal.parent(19), "frame-set:ship:0:2:5:ship");
        Ok(())
    }
}
