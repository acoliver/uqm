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

    use crate::ships::races::arilou_native::engine::init_arilou;

    use crate::ships::races::blackurq_native::engine::init_black_urquan;

    use crate::ships::races::chenjesu_native::engine::init_chenjesu;

    use crate::ships::races::chmmr_native::engine::init_chmmr;

    use crate::ships::races::druuge_native::engine::init_druuge;

    use crate::ships::races::human_native::engine::init_human;

    use crate::ships::races::ilwrath_native::engine::init_ilwrath;

    use crate::ships::races::melnorme_native::engine::init_melnorme;

    use crate::ships::races::mmrnmhrm_native::engine::init_mmrnmhrm;

    use crate::ships::races::mycon_native::engine::init_mycon;

    use crate::ships::races::orz_native::engine::init_orz;

    use crate::ships::races::pkunk_native::engine::init_pkunk;

    use crate::ships::races::shofixti_native::engine::init_shofixti;

    use crate::ships::races::slylandro_native::engine::init_slylandro;

    use crate::ships::races::spathi_native::engine::init_spathi;

    use crate::ships::races::supox_native::engine::init_supox;

    use crate::ships::races::syreen_native::engine::init_syreen;

    use crate::ships::races::thraddash_native::engine::init_thraddash;

    use crate::ships::races::umgah_native::engine::init_umgah;

    use crate::ships::races::urquan_native::engine::init_urquan;

    use crate::ships::races::utwig_native::engine::init_utwig;

    use crate::ships::races::yehat_native::engine::init_yehat;

    use crate::ships::races::zoqfot_native::engine::init_zoqfotpik;

    use crate::ships::races::samatra::engine::init_samatra;

    use crate::ships::races::sis_native::engine::init_sis;

    use crate::ships::races::probe_native::engine::init_probe;
    // Resource IDs follow uqm.rmp's SHIP values, not SPECIES_ID ordering.
    const INITIALIZERS: [Option<Initializer>; 28] = [
        None,
        Some(init_arilou),
        Some(init_black_urquan),
        Some(init_chenjesu),
        Some(init_chmmr),
        Some(init_druuge),
        Some(init_human),
        Some(init_ilwrath),
        Some(init_melnorme),
        Some(init_mmrnmhrm),
        Some(init_mycon),
        Some(init_orz),
        Some(init_pkunk),
        Some(init_shofixti),
        Some(init_slylandro),
        Some(init_spathi),
        Some(init_supox),
        Some(init_syreen),
        Some(init_thraddash),
        Some(init_umgah),
        Some(init_urquan),
        Some(init_utwig),
        None,
        Some(init_yehat),
        Some(init_zoqfotpik),
        Some(init_samatra),
        Some(init_sis),
        Some(init_probe),
    ];

    unsafe extern "C" fn get_code_res_data(ship_id: *const c_char, out: *mut ResourceData) {
        let which = libc::atoi(ship_id) as u8;
        let mut allocation = rust_hmalloc(mem::size_of::<RACE_DESC>());
        let native = match which {
            0 => crate::ships::races::race(crate::ships::native::abi::SPECIES_ID_ANDROSYNTH_ID),
            22 => crate::ships::races::race(crate::ships::native::abi::SPECIES_ID_VUX_ID),
            _ => None,
        };
        if let Some(race) = native {
            allocation.cast::<RACE_DESC>().write((race.descriptor)());
            (*out).ptr = allocation;
            return;
        }
        let desc = if let Some(Some(init)) = INITIALIZERS.get(which as usize) {
            init()
        } else {
            crate::log_abi::log_message(
                3,
                &format!("Unknown SHIP identifier '{}'", c_int::from(which)),
            );
            ptr::null_mut()
        };
        if desc.is_null() {
            rust_hfree(allocation);
            allocation = ptr::null_mut();
        } else {
            allocation.cast::<RACE_DESC>().write(*desc);
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
