//! SHIP resource records and initializer dispatch translated from uqm/dummy.c.
#[cfg(not(test))]
pub(crate) mod engine {
    use crate::{
        memory::{rust_hfree, rust_hmalloc},
        resource::{
            ffi_bridge::{res_DetachResource, res_GetResource, InstallResTypeVectors},
            ffi_types::ResourceData,
        },
        ships::native::abi::RACE_DESC,
    };
    use std::{
        ffi::{c_char, c_int, c_void},
        mem, ptr,
    };

    type Initializer = unsafe extern "C" fn() -> *mut RACE_DESC;

    use crate::ships::races::chenjesu_native::engine::init_chenjesu;

    use crate::ships::races::orz_native::engine::init_orz;

    use crate::ships::races::samatra::engine::init_samatra;

    // Resource IDs follow uqm.rmp's SHIP values, not SPECIES_ID ordering.
    pub(crate) const INITIALIZERS: [Option<Initializer>; 28] = [
        None,
        None,
        None,
        Some(init_chenjesu),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        Some(init_orz),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        Some(init_samatra),
        None,
        None,
    ];

    /// # Safety
    /// Call on the game thread; the returned template borrows loaded global resources.
    pub unsafe fn descriptor(species: u32) -> RACE_DESC {
        // SAFETY: The caller owns the game-thread resource context and retains all descriptor resources.

        use crate::engine::battle::record::DescriptorSource;
        let source = if let Some(race) = crate::ships::races::race(species) {
            DescriptorSource::Native(race)
        } else {
            const CODE_BY_SPECIES: [usize; 29] = [
                usize::MAX,
                1,
                4,
                6,
                11,
                12,
                13,
                15,
                16,
                18,
                21,
                22,
                23,
                8,
                5,
                7,
                10,
                14,
                19,
                20,
                24,
                17,
                2,
                0,
                3,
                9,
                26,
                25,
                27,
            ];
            let code = *CODE_BY_SPECIES
                .get(species as usize)
                .unwrap_or_else(|| panic!("{}", "known species"));
            let init = crate::ships::code_resources::engine::INITIALIZERS
                .get(code)
                .and_then(|f| *f)
                .unwrap_or_else(|| panic!("{}", "legacy species initializer"));
            DescriptorSource::Legacy(init)
        };
        crate::engine::battle::access::descriptor_template(source)
            .unwrap_or_else(|| panic!("legacy initializer returned no descriptor"))
            .into_legacy()
    }

    fn native_race(which: u8) -> Option<&'static crate::engine::battle::Race> {
        match which {
            0 => crate::ships::races::race(crate::ships::native::abi::SPECIES_ID_ANDROSYNTH_ID),
            1 => crate::ships::races::race(crate::ships::native::abi::SPECIES_ID_ARILOU_ID),
            6 => crate::ships::races::race(crate::ships::native::abi::SPECIES_ID_EARTHLING_ID),
            12 => crate::ships::races::race(crate::ships::native::abi::SPECIES_ID_PKUNK_ID),
            13 => crate::ships::races::race(crate::ships::native::abi::SPECIES_ID_SHOFIXTI_ID),
            7 => crate::ships::races::race(crate::ships::native::abi::SPECIES_ID_ILWRATH_ID),
            8 => crate::ships::races::race(crate::ships::native::abi::SPECIES_ID_MELNORME_ID),
            18 => crate::ships::races::race(crate::ships::native::abi::SPECIES_ID_THRADDASH_ID),
            26 => crate::ships::races::race(crate::ships::native::abi::SPECIES_ID_SIS_SHIP_ID),
            22 => crate::ships::races::race(crate::ships::native::abi::SPECIES_ID_VUX_ID),
            2 => crate::ships::races::race(crate::ships::native::abi::SPECIES_ID_KOHR_AH_ID),
            5 => crate::ships::races::race(crate::ships::native::abi::SPECIES_ID_DRUUGE_ID),
            21 => crate::ships::races::race(crate::ships::native::abi::SPECIES_ID_UTWIG_ID),
            24 => crate::ships::races::race(crate::ships::native::abi::SPECIES_ID_ZOQFOTPIK_ID),
            10 => crate::ships::races::race(crate::ships::native::abi::SPECIES_ID_MYCON_ID),
            23 => crate::ships::races::race(crate::ships::native::abi::SPECIES_ID_YEHAT_ID),
            16 => crate::ships::races::race(crate::ships::native::abi::SPECIES_ID_SUPOX_ID),
            17 => crate::ships::races::race(crate::ships::native::abi::SPECIES_ID_SYREEN_ID),
            9 => crate::ships::races::race(crate::ships::native::abi::SPECIES_ID_MMRNMHRM_ID),
            20 => crate::ships::races::race(crate::ships::native::abi::SPECIES_ID_UR_QUAN_ID),
            27 => crate::ships::races::race(crate::ships::native::abi::SPECIES_ID_UR_QUAN_PROBE_ID),
            14 => crate::ships::races::race(crate::ships::native::abi::SPECIES_ID_SLYLANDRO_ID),
            19 => crate::ships::races::race(crate::ships::native::abi::SPECIES_ID_UMGAH_ID),
            4 => crate::ships::races::race(crate::ships::native::abi::SPECIES_ID_CHMMR_ID),
            15 => crate::ships::races::race(crate::ships::native::abi::SPECIES_ID_SPATHI_ID),
            _ => None,
        }
    }

    unsafe extern "C" fn get_code_res_data(ship_id: *const c_char, out: *mut ResourceData) {
        let which = libc::atoi(ship_id) as u8;
        let mut allocation = rust_hmalloc(mem::size_of::<RACE_DESC>());
        let native = native_race(which);
        use crate::engine::battle::record::DescriptorSource;
        let source = if let Some(race) = native {
            Some(DescriptorSource::Native(race))
        } else if let Some(Some(init)) = INITIALIZERS.get(which as usize) {
            Some(DescriptorSource::Legacy(*init))
        } else {
            crate::log_abi::log_message(
                3,
                &format!("Unknown SHIP identifier '{}'", c_int::from(which)),
            );
            None
        };
        let descriptor =
            source.and_then(|source| crate::engine::battle::access::descriptor_template(source));
        if let Some(descriptor) = descriptor {
            let record = descriptor.into_legacy();
            allocation.cast::<RACE_DESC>().write(record);
        } else {
            rust_hfree(allocation);
            allocation = ptr::null_mut();
        }
        (*out).ptr = allocation;
    }
    unsafe extern "C" fn release_code_res_data(data: *mut c_void) -> c_int {
        rust_hfree(data);
        1
    }
    /// # Safety
    /// Register on the resource owner thread after InitResourceSystem.
    #[no_mangle]
    pub unsafe extern "C" fn InstallCodeResType() -> c_int {
        InstallResTypeVectors(
            c"SHIP".as_ptr(),
            Some(get_code_res_data),
            Some(release_code_res_data),
            None,
        )
    }
    #[no_mangle]
    pub unsafe extern "C" fn LoadCodeResInstance(res: *const c_char) -> *mut c_void {
        let data = res_GetResource(res);
        if !data.is_null() {
            res_DetachResource(res);
        }
        data
    }
    #[no_mangle]
    pub unsafe extern "C" fn DestroyCodeRes(code: *mut c_void) -> u32 {
        rust_hfree(code);
        1
    }
    #[no_mangle]
    pub unsafe extern "C" fn CaptureCodeRes(
        code: *mut c_void,
        _data: *mut c_void,
        out: *mut *mut c_void,
    ) -> *mut c_void {
        if code.is_null() {
            crate::log_abi::log_message(1, "dummy.c::CaptureCodeRes() hCode==NULL! FATAL!");
            return ptr::null_mut();
        }
        // CODERES_STRUCT contains only its aligned RACE_DESC, at offset zero.
        *out = code;
        code
    }
    #[no_mangle]
    pub unsafe extern "C" fn ReleaseCodeRes(code: *mut c_void) -> *mut c_void {
        code
    }
}
