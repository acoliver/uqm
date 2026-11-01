#[path = "support/battle.rs"]
mod battle;
#[path = "support/battle_context.rs"]
mod battle_context;

use battle::{Battle, Ship};
use uqm_rust::engine::battle::views::Laser;
use uqm_rust::ships::native::abi::*;

#[test]
fn laser_construction_preserves_fields_and_pool_exhaustion() {
    battle_context::run(|| {
        let _owner = Battle::new(1);
        // SAFETY: the serialized fixture owns the published context and its pool.
        unsafe {
            uqm_rust::app::bridge::enter(|cx| {
                let laser = Laser {
                    origin: POINT { x: 100, y: 200 },
                    end: POINT { x: 300, y: -400 },
                    flags: IGNORE_SIMILAR as u16,
                    sender: 1,
                    pixoffs: 0,
                    face: 0,
                    color: Color { r: 11, g: 22, b: 33, a: 255 },
                };
                let id = cx.spawn_laser(laser).unwrap();
                let state = cx.element(id);
                assert_eq!(state.player, 1);
                assert_eq!(state.flags, (APPEARING | FINITE_LIFE | IGNORE_SIMILAR) as u16);
                assert_eq!((state.hits, state.mass, state.life, state.thrust), (1, 1, 1, 1));
                assert_eq!((state.current.location.x, state.current.location.y), (100, 200));
                assert_eq!(state.velocity.get_current_components(), (300 << 5, -400 << 5));
                let style = cx.primitive_style(id);
                assert_eq!(style.kind, gfx_object_LINE_PRIM as u8);
                assert_eq!((style.color.r, style.color.g, style.color.b), (11, 22, 33));
                assert!(cx.spawn_laser(laser).is_none());
                cx.dispose(id);
            });
        }
    });
}

#[test]
fn gravity_reads_next_positions_and_preserves_queue_order() {
    battle_context::run(|| {
        let owner = Battle::new(4);
        let ship = Ship::bare(&owner, SPECIES_ID_VUX_ID, 0);
        // SAFETY: the serialized fixture owns all records for this call.
        unsafe {
            uqm_rust::app::bridge::enter(|cx| {
                let planet = cx.alloc_element().unwrap();
                cx.element_mut(planet).mass = 200;
                cx.element_mut(planet).flags = 0;
                cx.element_mut(planet).current.location = POINT { x: 4000, y: 3000 };
                cx.element_mut(planet).next.location = POINT { x: 400, y: 400 };
                cx.put(planet);
                cx.element_mut(ship.id).current.location = POINT { x: 400, y: 400 };
                cx.element_mut(ship.id).next.location = POINT { x: 400, y: 400 };
                cx.element_mut(ship.id).flags = PLAYER_SHIP as u16;
                let head = cx.elements().head();
                assert!(!cx.gravity(ship.id));
                cx.element_mut(ship.id).flags |= PRE_PROCESS as u16;
                assert!(cx.gravity(ship.id));
                assert_eq!(cx.elements().head(), head);
                cx.unlink(planet);
                cx.dispose(planet);
            });
        }
    });
}
