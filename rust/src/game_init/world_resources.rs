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

#[cfg(any(not(test), feature = "linked_c_archive"))]
mod live {
    use super::gravity_keys;
    use crate::graphics::native_abi::frame_desc;
    use crate::graphics::{
        drawable_abi::DestroyDrawable,
        pixmap_abi::{CaptureDrawable, ReleaseDrawable},
        resgfx::LoadGraphicInstance,
    };
    use std::{
        ffi::{c_void, CString},
        ptr,
    };

    #[no_mangle]
    pub static mut planet: [*mut frame_desc; 3] = [ptr::null_mut(); 3];

    /// # Safety
    /// Called on the game thread with a valid planet selector and initialized resources.
    #[no_mangle]
    pub unsafe extern "C" fn load_gravity_well(selector: u8) {
        for (i, key) in gravity_keys(selector).into_iter().enumerate() {
            planet[i] = match key {
                Some(key) => {
                    CaptureDrawable(LoadGraphicInstance(CString::new(key).unwrap().as_ptr()).cast())
                }
                None => ptr::null_mut(),
            };
        }
    }

    /// # Safety
    /// Called on the game thread while the graphics resource system is live.
    #[no_mangle]
    pub unsafe extern "C" fn free_gravity_well() {
        let frames = std::slice::from_raw_parts_mut(ptr::addr_of_mut!(planet).cast(), 3);
        for frame in frames {
            DestroyDrawable(ReleaseDrawable(*frame));
            *frame = ptr::null_mut();
        }
    }

    /// # Safety
    /// Resource and graphics systems must be initialized on the game thread.
    #[no_mangle]
    pub unsafe extern "C" fn load_life_form(selector: u8) -> *mut frame_desc {
        let key = CString::new(format!("graphics.life.{selector}")).unwrap();
        CaptureDrawable(LoadGraphicInstance(key.as_ptr()).cast())
    }

    /// # Safety
    /// The music resource loader must be initialized on the game thread.
    #[no_mangle]
    pub unsafe extern "C" fn load_orbit_theme(selector: u8) -> *mut c_void {
        let key = CString::new(format!("music.orbit{}", u16::from(selector) + 1)).unwrap();
        crate::sound::resource_types::LoadMusicInstance(key.as_ptr())
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
