use uqm_engine::battle::net::NetState;

pub(super) fn meaningful(state: NetState) -> bool {
    [
        NetState::Init,
        NetState::PreBattle,
        NetState::SelectShip,
        NetState::InterBattle,
        NetState::InBattle,
        NetState::EndingBattle,
        NetState::EndingBattle2,
    ]
    .contains(&state)
}
