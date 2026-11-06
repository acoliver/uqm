#[path = "support/battle.rs"]
mod battle;
#[path = "support/battle_context.rs"]
mod battle_context;

use uqm_rust::collections::observation::{self, Event};
use uqm_rust::ships::native::abi::SPECIES_ID_EARTHLING_ID;

#[test]
fn projected_images_record_provenance_without_borrowing_stack_storage() {
    battle_context::run(|_| {
        let pool = battle::Battle::new(8);
        // SAFETY: the serialized fixture retains all slots, descriptors and frame arrays.
        unsafe {
            let source = battle::Ship::new(&pool, SPECIES_ID_EARTHLING_ID, 0);
            let destination = battle::Object::new(&pool);
            observation::enable();
            observation::drain();
            let expected = uqm_rust::app::bridge::enter(|cx| {
                let visual = cx.element(source.id).current;
                cx.copy_image_to_current(destination.id, visual.image.clone());
                cx.copy_visual_to_next(destination.id, visual);
                let destination = &cx.elements()[destination.id];
                [
                    (&raw const destination.current.image.frame) as usize,
                    (&raw const destination.next.image.frame) as usize,
                ]
            });
            let events = observation::drain();
            assert!(
                events
                    .iter()
                    .all(|event| !matches!(event, Event::ImageCopy { .. })),
                "a projected value has no retained source address to copy provenance from"
            );
            for target in expected {
                assert!(
                    events.iter().any(|event| matches!(event,
                        Event::ImageFrame { slot, .. } if *slot == target
                    )),
                    "frame provenance missing for projected image at {target:#x}"
                );
            }
        }
    });
}
