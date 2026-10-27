//! Descriptor asset keys and borrowed loaded handles, never pointer values.
use super::super::{
    resources::{Frame, Resource, StringEntry},
    value, Fields,
};
use crate::graphics::native_abi::frame_desc;
use crate::{
    automation::parity::Journal,
    ships::native::abi::{DATA_STUFF, FRAME, RESOURCE, SHIP_INFO},
};
use std::ffi::CStr;

pub(super) struct Assets<'a> {
    keys: Vec<(&'static str, Resource<'a>)>,
    frames: Vec<(&'static str, Frame<'a>)>,
    strings: StringEntry<'a>,
    sounds: StringEntry<'a>,
    music: usize,
    sets: [(&'static str, &'a [FRAME; 3]); 3],
}

impl<'a> Assets<'a> {
    /// # Safety
    /// The descriptor's keys and loaded resources are immutable and live for `'a`.
    pub(super) unsafe fn borrow(info: &'a SHIP_INFO, data: &'a DATA_STUFF) -> Self {
        let keys = resource_keys(info, data)
            .into_iter()
            .map(|(name, key)| {
                let key = if key.is_null() {
                    None
                } else {
                    // SAFETY: non-null keys are NUL-terminated strings owned by
                    // the descriptor graph, held immutable for this borrow.
                    Some(unsafe { CStr::from_ptr(key) })
                };
                (name, Resource(key))
            })
            .collect();
        let frames = frame_handles(info, data)
            .into_iter()
            .map(|(name, pointer)| {
                (
                    name,
                    // SAFETY: each non-null handle belongs to the descriptor graph
                    // held live and immutable by the caller for this borrow.
                    Frame(unsafe { pointer.cast::<frame_desc>().as_ref() }),
                )
            })
            .collect();
        // SAFETY: both string entries belong to the immutable descriptor graph.
        let (strings, sounds) = unsafe { (info.race_strings.as_ref(), data.ship_sounds.as_ref()) };
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
        for (role, views) in self.sets {
            journal.register_parent(views.as_ptr() as usize, format!("frame-set:{owner}:{role}"));
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

fn resource_keys(info: &SHIP_INFO, data: &DATA_STUFF) -> [(&'static str, RESOURCE); 15] {
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

fn frame_handles(info: &SHIP_INFO, data: &DATA_STUFF) -> [(&'static str, FRAME); 16] {
    let captain = &data.captain_control;
    [
        ("race.icons", info.icons),
        ("race.melee_icon", info.melee_icon),
        ("data.captain.background", captain.background),
        ("data.captain.turn", captain.turn),
        ("data.captain.thrust", captain.thrust),
        ("data.captain.weapon", captain.weapon),
        ("data.captain.special", captain.special),
        ("data.ship.0", data.ship[0]),
        ("data.ship.1", data.ship[1]),
        ("data.ship.2", data.ship[2]),
        ("data.weapon.0", data.weapon[0]),
        ("data.weapon.1", data.weapon[1]),
        ("data.weapon.2", data.weapon[2]),
        ("data.special.0", data.special[0]),
        ("data.special.1", data.special[1]),
        ("data.special.2", data.special[2]),
    ]
}
