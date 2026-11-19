//! Planet, life-form and orbital music resources, ported from cons_res.c.

const PLANET_TYPES: [&str; 59] = [
    "oolite",
    "yttric",
    "quasidegenerate",
    "lanthanide",
    "treasure",
    "urea",
    "metal",
    "radioactive",
    "opalescent",
    "cyanic",
    "acid",
    "alkali",
    "halide",
    "green",
    "copper",
    "carbide",
    "ultramarine",
    "noble",
    "azure",
    "chondrite",
    "purple",
    "superdense",
    "pellucid",
    "dust",
    "crimson",
    "cimmerian",
    "infrared",
    "selenic",
    "auric",
    "fluorescent",
    "ultraviolet",
    "plutonic",
    "rainbow",
    "shattered",
    "sapphire",
    "organic",
    "xenolithic",
    "redux",
    "primordial",
    "emerald",
    "chlorine",
    "magnetic",
    "water",
    "telluric",
    "hydrocarbon",
    "iodine",
    "vinylogous",
    "ruby",
    "magma",
    "maroon",
    "bluegas",
    "cyangas",
    "greengas",
    "greygas",
    "orangegas",
    "purplegas",
    "redgas",
    "violetgas",
    "yellowgas",
];
const PLANET_SIZES: [&str; 3] = ["large", "medium", "small"];

fn gravity_keys(selector: u8) -> [Option<String>; 3] {
    if selector == PLANET_TYPES.len() as u8 {
        [Some("planet.samatra.large".into()), None, None]
    } else {
        let kind = if selector & 0x80 != 0 {
            "slaveshield"
        } else {
            PLANET_TYPES[selector as usize]
        };
        PLANET_SIZES.map(|size| Some(format!("planet.{kind}.{size}")))
    }
}

#[cfg(not(test))]
pub(crate) mod live {
    use super::gravity_keys;
    use std::{ffi::CString, ptr};

    pub fn load_gravity_well(kernel: &mut crate::engine::Kernel, selector: u8) {
        for (frame, key) in kernel.planet.iter_mut().zip(gravity_keys(selector)) {
            *frame = key.map_or(ptr::null_mut(), |key| {
                crate::engine::battle::frames::load_graphic(&CString::new(key).unwrap())
            });
        }
        crate::collections::observation::observe_views(
            &kernel.planet,
            "engine-global:planet",
            |frame| *frame as usize,
        );
    }

    pub fn free_gravity_well(kernel: &mut crate::engine::Kernel) {
        crate::engine::battle::frames::free_image(&mut kernel.planet, "engine-global:planet");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn planet_resource_order_and_special_selectors() {
        assert_eq!(
            gravity_keys(0),
            [
                Some("planet.oolite.large".into()),
                Some("planet.oolite.medium".into()),
                Some("planet.oolite.small".into())
            ]
        );
        assert_eq!(
            gravity_keys(58)[2].as_deref(),
            Some("planet.yellowgas.small")
        );
        assert_eq!(
            gravity_keys(59),
            [Some("planet.samatra.large".into()), None, None]
        );
        for selector in [0x80, 0x80 | 58] {
            assert_eq!(
                gravity_keys(selector)[0].as_deref(),
                Some("planet.slaveshield.large")
            );
        }
    }
}

#[cfg(not(test))]
pub use live::free_gravity_well;
