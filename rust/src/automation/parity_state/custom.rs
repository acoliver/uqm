//! Per-instance race data, borrowed using the actual race-owned types.
#[cfg(not(test))]
use super::resources::{Frame, FrameSet};
use super::{ships::characteristics, value, Fields};
use crate::automation::parity::Journal;
use crate::ships::native::abi::*;
#[cfg(not(test))]
use crate::ships::races::{
    androsynth_native::engine::AndrosynthData,
    pkunk_native::engine::{PkunkData, LAST_SOUND},
    sis_native::engine::SisData,
    umgah_native::engine::UmgahData,
};

pub(super) enum RaceData<'a> {
    None,
    #[cfg(not(test))]
    Androsynth(&'a AndrosynthData),
    #[cfg(not(test))]
    Pkunk(&'a PkunkData),
    #[cfg(not(test))]
    Umgah(&'a UmgahData),
    Mmrnmhrm(&'a CHARACTERISTIC_STUFF),
    #[cfg(not(test))]
    Sis(&'a SisData),
}

impl<'a> RaceData<'a> {
    /// # Safety
    /// `pointer` is the given species' actual live descriptor data, immutable for `'a`.
    #[cfg(not(test))]
    pub(super) unsafe fn borrow(species: u32, descriptor: &'a RACE_DESC) -> Self {
        let pointer = descriptor.data;
        if pointer.is_null() {
            return Self::None;
        }
        // SAFETY: each native initializer owns the corresponding concrete allocation.
        unsafe {
            match species {
                SPECIES_ID_ANDROSYNTH_ID => Self::Androsynth(&*pointer.cast()),
                SPECIES_ID_PKUNK_ID => Self::Pkunk(&*pointer.cast()),
                SPECIES_ID_UMGAH_ID => Self::Umgah(&*pointer.cast()),
                SPECIES_ID_MMRNMHRM_ID => Self::Mmrnmhrm(&*pointer.cast()),
                SPECIES_ID_SIS_SHIP_ID => Self::Sis(&*pointer.cast()),
                _ => panic!("unexpected custom data for species {species}"),
            }
        }
    }

    pub(super) fn write(
        &self,
        fields: &mut Fields,
        prefix: &str,
        _journal: &Journal,
        _hooks: &[(usize, &'static str)],
    ) {
        let prefix = format!("{prefix}.custom");
        value(fields, &prefix, "present", !matches!(self, Self::None));
        match self {
            Self::None => (),
            #[cfg(not(test))]
            Self::Androsynth(data) => hook(
                fields,
                &prefix,
                "collision",
                data.collision_func.map_or(0, |f| f as usize),
                SPECIES_ID_ANDROSYNTH_ID,
                _hooks,
            ),
            #[cfg(not(test))]
            Self::Pkunk(data) => {
                value(
                    fields,
                    &prefix,
                    "phoenix",
                    _journal.identity(data.hPhoenix as usize),
                );
                for (name, callback) in [
                    ("saved_preprocess", data.saved_preprocess_func),
                    ("saved_postprocess", data.saved_postprocess_func),
                    ("saved_death", data.saved_death_func),
                ] {
                    hook(
                        fields,
                        &prefix,
                        name,
                        callback.map_or(0, |f| f as usize),
                        SPECIES_ID_PKUNK_ID,
                        _hooks,
                    );
                }
            }
            #[cfg(not(test))]
            Self::Umgah(data) => value(fields, &prefix, "prev_facing", data.prev_facing),
            Self::Mmrnmhrm(data) => {
                characteristics(fields, &format!("{prefix}.characteristics"), data)
            }
            #[cfg(not(test))]
            Self::Sis(data) => {
                value(fields, &prefix, "num_trackers", data.num_trackers);
                value(fields, &prefix, "num_blasters", data.num_blasters);
                for (index, missile) in data.missiles.iter().enumerate() {
                    missile_fields(
                        fields,
                        &format!("{prefix}.missiles.{index}"),
                        missile,
                        _journal,
                        _hooks,
                    );
                }
            }
        }
    }
}

#[cfg(not(test))]
pub(super) fn pkunk_sound(fields: &mut Fields) {
    value(
        fields,
        "race.pkunk",
        "last_sound",
        LAST_SOUND.load(std::sync::atomic::Ordering::Relaxed),
    );
}

#[cfg(not(test))]
fn hook(
    fields: &mut Fields,
    prefix: &str,
    name: &str,
    address: usize,
    species: u32,
    hooks: &[(usize, &'static str)],
) {
    value(
        fields,
        prefix,
        name,
        crate::ships::hook_registry::name_for_species(hooks, address, species),
    );
}

#[cfg(not(test))]
fn missile_fields(
    fields: &mut Fields,
    prefix: &str,
    missile: &MISSILE_BLOCK,
    journal: &Journal,
    hooks: &[(usize, &'static str)],
) {
    for (name, number) in [
        ("cx", i64::from(missile.cx)),
        ("cy", i64::from(missile.cy)),
        ("flags", i64::from(missile.flags)),
        ("sender", i64::from(missile.sender)),
        ("pixoffs", i64::from(missile.pixoffs)),
        ("speed", i64::from(missile.speed)),
        ("hit_points", i64::from(missile.hit_points)),
        ("damage", i64::from(missile.damage)),
        ("face", i64::from(missile.face)),
        ("index", i64::from(missile.index)),
        ("life", i64::from(missile.life)),
        ("blast_offs", i64::from(missile.blast_offs)),
    ] {
        value(fields, prefix, name, number);
    }
    hook(
        fields,
        prefix,
        "preprocess",
        missile.preprocess_func.map_or(0, |f| f as usize),
        SPECIES_ID_SIS_SHIP_ID,
        hooks,
    );
    // SAFETY: RaceData holds the live SIS allocation and its missile frame arrays.
    let frame = unsafe {
        Frame(missile.farray.as_ref().and_then(|pointer| {
            pointer
                .cast::<crate::graphics::native_abi::frame_desc>()
                .as_ref()
        }))
    };
    frame.write(fields, &format!("{prefix}.farray"), journal);
    if !missile.farray.is_null() {
        let role = journal.parent(missile.farray as usize);
        let count = if role.starts_with("frame-set:") { 3 } else { 1 };
        // SAFETY: descriptor frame roles identify live three-view arrays; any other
        // engine array contributes only its guaranteed first entry.
        let set =
            unsafe { FrameSet::borrow(std::slice::from_raw_parts(missile.farray.cast(), count)) };
        set.write(fields, &format!("{prefix}.farray"), journal);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn saved_transform_characteristics_are_individual_fields() {
        let directory = super::super::test_directory();
        let journal = Journal::create(directory.path())
            .unwrap_or_else(|error| panic!("create custom state test journal: {error}"));
        let saved = CHARACTERISTIC_STUFF {
            max_thrust: 71,
            thrust_increment: 3,
            energy_regeneration: 1,
            weapon_energy_cost: 2,
            special_energy_cost: 3,
            energy_wait: 4,
            turn_wait: 5,
            thrust_wait: 6,
            weapon_wait: 7,
            special_wait: 8,
            ship_mass: 9,
        };
        let mut fields = Fields::new();
        RaceData::Mmrnmhrm(&saved).write(&mut fields, "ship", &journal, &[]);
        assert_eq!(fields.len(), 12);
        assert_eq!(fields["ship.custom.present"], "true");
        assert_eq!(fields["ship.custom.characteristics.max_thrust"], "71");
        assert_eq!(fields["ship.custom.characteristics.mass"], "9");
        let mut absent = Fields::new();
        RaceData::None.write(&mut absent, "ship", &journal, &[]);
        assert_eq!(
            absent,
            Fields::from([("ship.custom.present".into(), "false".into())])
        );
    }
}
