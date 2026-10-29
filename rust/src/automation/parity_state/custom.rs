//! Per-instance race data, borrowed using the actual race-owned types.
#[cfg(not(test))]
use super::resources::Frame;
use super::{ships::characteristics, value, Fields};
use crate::automation::parity::Journal;
pub(super) use crate::engine::battle::access::observe::CustomData as RaceData;
#[cfg(not(test))]
use crate::engine::battle::kernel::LAST_PKUNK_SOUND as LAST_SOUND;
use crate::ships::native::abi::*;
impl RaceData<'_> {
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
                data.map_or(0, |f| f.address()),
                SPECIES_ID_ANDROSYNTH_ID,
                _hooks,
            ),
            #[cfg(not(test))]
            Self::Pkunk { phoenix, hooks } => {
                value(fields, &prefix, "phoenix", _journal.identity(*phoenix));
                for (name, callback) in [
                    ("saved_preprocess", hooks[0]),
                    ("saved_postprocess", hooks[1]),
                    ("saved_death", hooks[2]),
                ] {
                    hook(
                        fields,
                        &prefix,
                        name,
                        callback.map_or(0, |f| f.address()),
                        SPECIES_ID_PKUNK_ID,
                        _hooks,
                    );
                }
            }
            #[cfg(not(test))]
            Self::NativePkunk { phoenix, saved } => {
                value(fields, &prefix, "phoenix", _journal.identity(*phoenix));
                for (name, callback) in ["saved_preprocess", "saved_postprocess", "saved_death"]
                    .into_iter()
                    .zip(saved)
                {
                    hook(
                        fields,
                        &prefix,
                        name,
                        callback.map_or(0, |f| f.address()),
                        SPECIES_ID_PKUNK_ID,
                        _hooks,
                    );
                }
            }
            #[cfg(not(test))]
            Self::NativeUmgah(previous) => value(fields, &prefix, "prev_facing", *previous),
            #[cfg(not(test))]
            Self::Umgah(data) => value(fields, &prefix, "prev_facing", data.prev_facing),
            Self::Mmrnmhrm(data) => {
                characteristics(fields, &format!("{prefix}.characteristics"), data)
            }
            #[cfg(not(test))]
            Self::Sis(data) => {
                value(fields, &prefix, "num_trackers", data.trackers);
                value(fields, &prefix, "num_blasters", data.blasters);
                for (index, missile) in data.missiles.iter().enumerate() {
                    missile_fields(
                        fields,
                        &format!("{prefix}.missiles.{index}"),
                        &missile.unwrap_or_default(),
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
    missile: &crate::engine::battle::race::FlagshipWeapon,
    journal: &Journal,
    hooks: &[(usize, &'static str)],
) {
    for (name, number) in [
        ("cx", 0),
        ("cy", 0),
        ("flags", i64::from(missile.flags)),
        ("sender", 0),
        ("pixoffs", i64::from(missile.pixel_offset)),
        ("speed", i64::from(missile.speed)),
        ("hit_points", i64::from(missile.hit_points)),
        ("damage", i64::from(missile.damage)),
        ("face", i64::from(missile.facing)),
        ("index", i64::from(missile.index)),
        ("life", i64::from(missile.life)),
        ("blast_offs", i64::from(missile.blast_offset)),
    ] {
        value(fields, prefix, name, number);
    }
    hook(
        fields,
        prefix,
        "preprocess",
        missile.preprocess.map_or(0, |f| f as usize),
        SPECIES_ID_SIS_SHIP_ID,
        hooks,
    );
    // Module configuration has no loaded frame array, just as the original launch templates.
    Frame(None).write(fields, &format!("{prefix}.farray"), journal);
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

/// Decode the legacy address token without accessing its element record.
///
/// # Safety
/// The initialized display pool must remain owned by this synchronous snapshot.
#[cfg(not(test))]
pub(in crate::automation) unsafe fn phoenix_address(
    phoenix: Option<crate::engine::battle::ElementId>,
) -> usize {
    let Some(id) = phoenix else {
        return 0;
    };
    // Pkunk retains this token after free. Legacy observation follows the pool
    // address even if its slot is reused; live element operations still check gen.
    // SAFETY: the snapshot owns the initialized queue; only its address is read.
    let queue = unsafe { &*crate::c_bindings::live_queues::display_list() };
    assert!(
        !queue.pq_tab.is_null(),
        "Pkunk snapshot pool is not initialized"
    );
    assert!(
        id.slot < queue.num_objects,
        "Pkunk snapshot slot outside pool"
    );
    assert!(queue.object_size > 0, "Pkunk snapshot has no record stride");
    queue.pq_tab as usize + usize::from(id.slot) * usize::from(queue.object_size)
}
