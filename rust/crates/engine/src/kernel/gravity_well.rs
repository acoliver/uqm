//! Planet graphics of the battle gravity well, ported from cons_res.c.

use super::Kernel;
use std::ffi::CStr;

/// The large, medium and small resource keys of each planet type, in the
/// order of `PlanetFrameArray` (cons_res.c).
macro_rules! planet_keys {
    ($($kind:literal),* $(,)?) => {
        [$([
            key(concat!("planet.", $kind, ".large\0")),
            key(concat!("planet.", $kind, ".medium\0")),
            key(concat!("planet.", $kind, ".small\0")),
        ]),*]
    };
}

const fn key(text: &'static str) -> &'static CStr {
    match CStr::from_bytes_with_nul(text.as_bytes()) {
        Ok(key) => key,
        Err(_) => panic!("planet resource key"),
    }
}

const PLANET_KEYS: [[&CStr; 3]; 59] = planet_keys!(
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
);
const SLAVE_SHIELD_KEYS: [&CStr; 3] = [
    c"planet.slaveshield.large",
    c"planet.slaveshield.medium",
    c"planet.slaveshield.small",
];

fn gravity_keys(selector: u8) -> [Option<&'static CStr>; 3] {
    if selector == PLANET_KEYS.len() as u8 {
        [Some(c"planet.samatra.large"), None, None]
    } else if selector & 0x80 != 0 {
        SLAVE_SHIELD_KEYS.map(Some)
    } else {
        PLANET_KEYS[selector as usize].map(Some)
    }
}

impl Kernel {
    /// Loads the planet graphic sizes of `selector`: a planet type, the
    /// Samatra (one past the last type) or a slave-shielded world (bit 7).
    pub fn load_gravity_well(&mut self, selector: u8) {
        let content = self.content();
        for (frame, key) in self.planet.iter_mut().zip(gravity_keys(selector)) {
            *frame = key.map_or(crate::graphics::drawable::FrameId::NULL, |key| {
                crate::battle::frames::load_graphic(&content, key)
                    .unwrap_or(crate::graphics::drawable::FrameId::NULL)
            });
        }
        crate::collections::observation::observe_views(
            &self.planet,
            "engine-global:planet",
            |frame| frame.bits(),
        );
    }

    pub fn free_gravity_well(&mut self) {
        crate::battle::frames::free_image(&mut self.planet, "engine-global:planet");
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
                Some(c"planet.oolite.large"),
                Some(c"planet.oolite.medium"),
                Some(c"planet.oolite.small")
            ]
        );
        assert_eq!(gravity_keys(58)[2], Some(c"planet.yellowgas.small"));
        assert_eq!(
            gravity_keys(59),
            [Some(c"planet.samatra.large"), None, None]
        );
        for selector in [0x80, 0x80 | 58] {
            assert_eq!(gravity_keys(selector)[0], Some(c"planet.slaveshield.large"));
        }
    }
}
