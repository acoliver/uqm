pub mod bridge;

/// Own the battle context across the existing synchronous game lifecycle.
///
/// # Safety
/// Run only on the game thread. The legacy queue and GlobData authorities must
/// outlive the call; initialization and cleanup occur inside this boundary.
pub unsafe fn with_battle<R>(call: impl FnOnce() -> R) -> R {
    use crate::engine::battle::legacy::disp_q;
    use crate::engine::battle::{Battle, Cx, World};
    use crate::ships::native::abi::GlobData;
    // SAFETY: all pointers derive directly from the authority's raw address.
    unsafe {
        let mut battle = Battle::new(&raw mut disp_q);
        let mut world = World::new(
            &raw mut GlobData.Game_state.CurrentActivity,
            &raw mut GlobData.Game_state.ip_group_q,
        );
        let mut cx = Cx {
            b: &mut battle,
            world: &mut world,
            rng: crate::math::random,
        };
        bridge::publish(&mut cx, call)
    }
}
