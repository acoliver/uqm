use super::abi::*;

pub(super) fn meaningful(state: NetState) -> bool {
    [
        NetState_NetState_init,
        NetState_NetState_preBattle,
        NetState_NetState_selectShip,
        NetState_NetState_interBattle,
        NetState_NetState_inBattle,
        NetState_NetState_endingBattle,
        NetState_NetState_endingBattle2,
    ]
    .contains(&state)
}
