//! Race effects implemented by modes over the supplied owners.
use crate::battle::battle_native::engine::rgb15;
use crate::battle::tactrans_native::engine::*;
use crate::engine::battle::race_services::{RaceServices, StarContact};
use crate::engine::battle::views::BattleSoundBank;
use crate::engine::battle::{Cx, ElementId, ShipId};
use crate::navigation::hyper_transition::{element_universe, init_transition};
use crate::planets::starmap::find_star_id;
use crate::ships::native::{abi::*, cosine, facing_to_angle, sine};

impl RaceServices for super::services::ModeServices {
    fn play_battle_sound(
        &self,
        cx: &mut Cx<'_>,
        bank: BattleSoundBank,
        index: u16,
        source: ElementId,
        priority: u8,
    ) {
        use crate::battle::sounds::engine::{play_sound, position};
        let sound = match bank {
            crate::engine::battle::views::BattleSoundBank::Ship(ship) => {
                cx.desc(ship).sound_bank().map(|bank| bank.clip(index))
            }
            crate::engine::battle::views::BattleSoundBank::Game => {
                cx.kernel.game_sounds.as_ref().map(|bank| bank.clip(index))
            }
        };
        if let Some(sound) = sound {
            let pos = position(cx, Some(source));
            play_sound(
                cx.world,
                &mut cx.kernel.channels.borrow_mut(),
                &sound,
                pos,
                Some(source),
                priority,
            );
        }
    }

    fn select_self_destruct_winner(&self, cx: &mut Cx<'_>, dead: ElementId) {
        let owner = cx.owner(dead);
        match cx.winner() {
            None => {
                let alive = crate::battle::tactrans_native::find_alive_ship(cx, dead);
                crate::battle::tactrans_native::set_winner_ship(cx, Some(alive.unwrap_or(owner)));
            }
            Some(winner) if winner == owner => {
                cx.ship_state_mut(owner).status &= !(PLAY_VICTORY_DITTY as u16);
            }
            Some(_) => {}
        }
    }

    fn hyperspace_menu(&self, cx: &mut Cx<'_>) {
        let mut observer = cx.kernel.input_runtime().observer();
        let mut io = crate::engine::observe::Io {
            observer: &mut *observer,
        };
        fn submenu(
            cx: &mut Cx<'_>,
            io: &mut crate::engine::observe::Io<'_>,
            selection: u8,
        ) -> bool {
            // SAFETY: the suspended Cx retains the campaign throughout each synchronous mode.
            match selection {
                1 => {
                    crate::planets::pstarmap::star_map(cx, io);
                    true
                }
                2 => crate::planets::devices::devices_menu(cx, io, None) != 0,
                3 => {
                    crate::planets::cargo::live::cargo_menu(cx, io);
                    true
                }
                4 => crate::planets::roster::live::roster_menu(cx, io) != 0,
                5 => cx.options_menu(io),
                _ => unreachable!("invalid hyperspace submenu"),
            }
        }
        crate::ui::menu::hyperspace::hyperspace_menu(cx, &mut io, submenu);
    }

    fn spawn_ion_trail(&self, cx: &mut Cx<'_>, ship: ElementId) {
        assert_ne!(cx.element(ship).flags & PLAYER_SHIP as u16, 0);
        let Some(e) = cx.alloc_element() else {
            return;
        };
        let owner = cx.owner(ship);
        let angle = facing_to_angle(cx.ship_state(owner).facing as i32) + HALF_CIRCLE as i32;
        let mut rect = cx.frame_rect(cx.ship_frame(owner, 0));
        rect.extent.height = ((rect.extent.height as i32 + rect.corner.y as i32) << ONE_SHIFT) as _;
        cx.insert_before(e, cx.elements().head());
        cx.element_mut(e).player = NEUTRAL_PLAYER_NUM as i16;
        cx.element_mut(e).flags = (APPEARING | FINITE_LIFE | NONSOLID) as u16;
        cx.element_mut(e).set_thrust_wait(1);
        cx.element_mut(e).set_life_span(1);
        cx.primitive_style_mut(e).kind = gfx_object_POINT_PRIM as u8;
        cx.primitive_style_mut(e).color = rgb15(31, 21, 0);
        cx.element_mut(e).color_cycle = 0;
        cx.set_star_image(e);
        cx.element_mut(e).current.location = cx.element(ship).current.location;
        cx.element_mut(e).current.location.x = cx
            .element(e)
            .current
            .location
            .x
            .wrapping_add(cosine(angle, rect.extent.height as i32));
        cx.element_mut(e).current.location.y = cx
            .element(e)
            .current
            .location
            .y
            .wrapping_add(sine(angle, rect.extent.height as i32));
        cx.element_mut(e).death = Some(ElementHook::Native(cycle_ion_trail));
        cx.set_parent(e, Some(owner));
        cx.copy_visual_to_next(e, cx.element(e).current);
        let life = cx.element(e).life_span().wrapping_sub(1);
        cx.element_mut(e).set_life_span(life);
        cx.element_mut(e).flags |= PRE_PROCESS as u16;
    }

    fn play_ditty(&self, cx: &mut Cx<'_>, ship: ShipId) {
        play_ditty(cx, ship);
    }

    fn replace_dead_ship(&self, cx: &mut Cx<'_>, dead: ElementId) {
        new_ship_with_picker(cx, dead, |cx, ship, side| {
            crate::battle::ship_native::engine::next_star_ship(cx, Some(ship), side)
        });
    }

    fn stop_battle_music(&self, cx: &mut Cx<'_>) {
        stop_all_battle_music(cx.kernel);
    }

    fn start_ship_explosion(&self, cx: &mut Cx<'_>, ship: ElementId, play_sound: bool) {
        crate::battle::tactrans_native::start_ship_explosion(cx, ship, play_sound);
    }

    fn select_winner(&self, cx: &mut Cx<'_>, dead: ElementId) {
        crate::battle::tactrans_native::set_winner_ship(
            cx,
            crate::battle::tactrans_native::find_alive_ship(cx, dead),
        );
    }

    fn record_ship_death(&self, cx: &mut Cx<'_>, dead: ElementId) {
        let owner = cx.owner(dead);
        if u32::from(cx.element(dead).mass) <= MAX_SHIP_MASS {
            let player = cx.ship_state(owner).player;
            assert!(player >= 0);
            cx.b.ship_lost(player as usize);
        }
        if cx.world.activity() as u8 == SUPER_MELEE as u8 {
            crate::supermelee::native::battle_picker::live::ship_death(cx, owner);
        }
    }

    fn star_contact(&self, cx: &Cx<'_>, id: ElementId) -> StarContact {
        let point = element_universe(cx, id);
        let star = find_star_id(cx.world, cx.world.catalog(), None, point, 5, 5)
            .unwrap_or_else(|| panic!("star collision has no catalog entry"));
        let position = cx.world.catalog()[star.0].star_pt;
        StarContact {
            star,
            position,
            current: cx.world.current_star == Some(star),
        }
    }

    fn init_hyper_transition(&self, cx: &mut Cx<'_>, source: ElementId, ship: ElementId, kind: u8) {
        init_transition(cx, source, ship, kind);
    }

    fn hyper_transition(&self, cx: &mut Cx<'_>, id: ElementId) -> bool {
        crate::navigation::hyper_transition::hyper_transition(cx, id)
    }

    fn move_hyperspace(&self, cx: &mut Cx<'_>, dx: &mut i16, dy: &mut i16) {
        let geometry = crate::navigation::hyper_units::Geometry::new(cx.screen_dimensions());
        crate::navigation::hyperspace::move_sis(
            cx,
            dx,
            dy,
            geometry,
            crate::navigation::hyperspace::live::debit_sis_fuel,
        );
    }

    fn unlink_element(&self, cx: &mut Cx<'_>, id: ElementId) {
        crate::battle::sounds::engine::unlink(cx, id);
    }
}
