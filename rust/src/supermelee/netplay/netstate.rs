use super::abi::*;

pub(super) fn state_name(state: NetState) -> &'static str {
    [
        "unconnected",
        "connecting",
        "init",
        "inSetup",
        "preBattle",
        "interBattle",
        "selectShip",
        "inBattle",
        "endingBattle",
        "endingBattle2",
    ][state as usize]
}

/// # Safety
/// Consumes live state data with its installed release function, on the game thread.
#[no_mangle]
pub unsafe extern "C" fn NetConnectionStateData_release(data: *mut NetConnectionStateData) {
    (*data).releaseFunction.expect("state release function")(data);
}
