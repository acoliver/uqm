//! SHIP resource records and initializer dispatch translated from uqm/dummy.c.
#[cfg(not(test))]
mod engine {
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
    extern "C" {
        fn init_androsynth() -> *mut RACE_DESC;
        fn init_arilou() -> *mut RACE_DESC;
        fn init_black_urquan() -> *mut RACE_DESC;
        fn init_chenjesu() -> *mut RACE_DESC;
        fn init_chmmr() -> *mut RACE_DESC;
        fn init_druuge() -> *mut RACE_DESC;
        fn init_human() -> *mut RACE_DESC;
        fn init_ilwrath() -> *mut RACE_DESC;
        fn init_melnorme() -> *mut RACE_DESC;
        fn init_mmrnmhrm() -> *mut RACE_DESC;
        fn init_mycon() -> *mut RACE_DESC;
        fn init_orz() -> *mut RACE_DESC;
        fn init_pkunk() -> *mut RACE_DESC;
        fn init_shofixti() -> *mut RACE_DESC;
        fn init_slylandro() -> *mut RACE_DESC;
        fn init_spathi() -> *mut RACE_DESC;
        fn init_supox() -> *mut RACE_DESC;
        fn init_syreen() -> *mut RACE_DESC;
        fn init_thraddash() -> *mut RACE_DESC;
        fn init_umgah() -> *mut RACE_DESC;
        fn init_urquan() -> *mut RACE_DESC;
        fn init_utwig() -> *mut RACE_DESC;
        fn init_vux() -> *mut RACE_DESC;
        fn init_yehat() -> *mut RACE_DESC;
        fn init_zoqfotpik() -> *mut RACE_DESC;
        fn init_samatra() -> *mut RACE_DESC;
        fn init_sis() -> *mut RACE_DESC;
        fn init_probe() -> *mut RACE_DESC;

    }
    // Resource IDs follow uqm.rmp's SHIP values, not SPECIES_ID ordering.
    const INITIALIZERS: [Initializer; 28] = [
        init_androsynth,
        init_arilou,
        init_black_urquan,
        init_chenjesu,
        init_chmmr,
        init_druuge,
        init_human,
        init_ilwrath,
        init_melnorme,
        init_mmrnmhrm,
        init_mycon,
        init_orz,
        init_pkunk,
        init_shofixti,
        init_slylandro,
        init_spathi,
        init_supox,
        init_syreen,
        init_thraddash,
        init_umgah,
        init_urquan,
        init_utwig,
        init_vux,
        init_yehat,
        init_zoqfotpik,
        init_samatra,
        init_sis,
        init_probe,
    ];

    unsafe extern "C" fn get_code_res_data(ship_id: *const c_char, out: *mut ResourceData) {
        let which = libc::atoi(ship_id) as u8;
        let mut allocation = rust_hmalloc(mem::size_of::<RACE_DESC>());
        let desc = if let Some(init) = INITIALIZERS.get(which as usize) {
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
