use super::abi::*;

#[no_mangle]
pub static mut netStateData: [NetStateData; 10] = [
    NetStateData {
        name: c"unconnected".as_ptr(),
    },
    NetStateData {
        name: c"connecting".as_ptr(),
    },
    NetStateData {
        name: c"init".as_ptr(),
    },
    NetStateData {
        name: c"inSetup".as_ptr(),
    },
    NetStateData {
        name: c"preBattle".as_ptr(),
    },
    NetStateData {
        name: c"interBattle".as_ptr(),
    },
    NetStateData {
        name: c"selectShip".as_ptr(),
    },
    NetStateData {
        name: c"inBattle".as_ptr(),
    },
    NetStateData {
        name: c"endingBattle".as_ptr(),
    },
    NetStateData {
        name: c"endingBattle2".as_ptr(),
    },
];

/// # Safety
/// Consumes live state data with its installed release function, on the game thread.
#[no_mangle]
pub unsafe extern "C" fn NetConnectionStateData_release(data: *mut NetConnectionStateData) {
    (*data).releaseFunction.expect("state release function")(data);
}
