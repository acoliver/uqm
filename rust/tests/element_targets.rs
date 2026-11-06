#[path = "support/battle.rs"]
mod battle;
#[path = "support/battle_context.rs"]
mod battle_context;

#[test]
fn target_identity_does_not_rebind_when_its_slot_is_reused() {
    battle_context::run(|_| {
        let _owner = battle::Battle::new(2);
        // SAFETY: the fixture exclusively retains the published battle and its pools.
        unsafe {
            uqm_rust::app::bridge::enter(|cx| {
                let source = cx
                    .alloc_element()
                    .unwrap_or_else(|| panic!("fixture pool is full"));
                let target = cx
                    .alloc_element()
                    .unwrap_or_else(|| panic!("fixture pool is full"));
                cx.set_target(source, Some(target));
                cx.dispose(target);
                let replacement = cx
                    .alloc_element()
                    .unwrap_or_else(|| panic!("fixture pool is full"));
                assert_ne!(target, replacement);
                assert!(!cx.elements().contains(target));
                assert_eq!(cx.target(source), Some(target));
                cx.set_target(source, None);
                cx.dispose(replacement);
                cx.dispose(source);
            });
        }
    });
}
