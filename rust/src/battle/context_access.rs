//! Game and resource operations wired to the explicit battle context.
use crate::engine::battle::{Cx, ElementId, ShipId};
use crate::ships::native::abi::*;

mod status_native_access {
    use super::*;
    use crate::battle::status_native::engine::*;
    use crate::battle::status_native::explosion_flash;

    pub trait StatusOps {
        fn draw_explosion_flash(&mut self, life: u16, y: i16);
        fn draw_energy_label(&mut self, y: i16, change: i16);
        fn draw_captain_controls(&mut self, ship: ShipId, y: i16, delta: u16, current: u16);
    }

    impl StatusOps for Cx<'_> {
        fn draw_explosion_flash(&mut self, life: u16, y: i16) {
            let Some(context) = std::ptr::NonNull::new(self.kernel.status_context)
                .map(crate::graphics::context_abi::ContextLease::retain)
                .and_then(|context| context.selected_context())
            else {
                return;
            };
            explosion_flash(life, y, |color, r, fill| {
                if fill {
                    context.fill_rectangle(color, r);
                } else {
                    context.draw_rectangle(color, r);
                }
            });
        }
        fn draw_energy_label(&mut self, y: i16, change: i16) {
            let Some(context) = std::ptr::NonNull::new(self.kernel.status_context)
                .map(crate::graphics::context_abi::ContextLease::retain)
                .and_then(|context| context.selected_context())
            else {
                return;
            };
            let frames = crate::graphics::drawable_abi::retain_frame_allocations(&[self
                .kernel
                .status_frame]);
            crate::ui::shipstat::draw_crew_energy_labels(
                &context,
                &frames[0],
                self.kernel.configuration.preferences().menu == OPT_PC as i32,
                y,
                change,
            );
        }
        fn draw_captain_controls(&mut self, ship: ShipId, y: i16, delta: u16, current: u16) {
            captains_window(self, ship, y, delta, current, 2);
        }
    }
}

pub(crate) mod ai_legacy;
pub(crate) mod reader_legacy;

pub(crate) mod links;

pub(crate) mod navigation;

pub(crate) mod lifecycle;

pub(crate) mod observe;

pub(crate) mod fixture;

pub use status_native_access::StatusOps;

pub trait ContextOps {
    fn finish_battle(&mut self);

    fn negotiate_battle(
        &mut self,
        net: &dyn crate::engine::battle::net::BattleNet,
        state: u32,
    ) -> bool;

    fn draw_final_battle_captain(&mut self, ship: ShipId);
    fn start_battle_song_if_ready(&mut self, ship: ShipId);

    fn preprocess_status(&mut self, id: ElementId);

    fn draw_captain_controls_pass(
        &mut self,
        ship: ShipId,
        y: i16,
        delta: u16,
        current: u16,
        pass: u16,
    );
}

impl ContextOps for Cx<'_> {
    fn finish_battle(&mut self) {
        let mut current = self.elements().head();
        while let Some(id) = current {
            current = self.elements().succ(id);
            if self.element(id).flags & PLAYER_SHIP as u16 != 0 {
                self.retain_terminal_outcome(self.owner(id));
            }
        }
        self.finish_battle_storage();
    }

    fn negotiate_battle(
        &mut self,
        net: &dyn crate::engine::battle::net::BattleNet,
        state: u32,
    ) -> bool {
        let runtime = self.kernel.input_runtime();
        let mut observer = runtime.observer();
        let mut io = crate::engine::observe::Io {
            observer: observer.as_mut(),
        };
        crate::supermelee::native::protocol::with_battle_driver(self, &mut io, |driver| {
            net.negotiate(true, state, driver)
        })
    }

    fn draw_final_battle_captain(&mut self, ship: ShipId) {
        let frame = self.take_captain_background(ship);
        // SAFETY: this consumes the live descriptor's captain release authority.
        unsafe {
            let old = SetContext(self.kernel.status_context);
            let mut stamp = STAMP {
                origin: POINT { x: 0, y: 0 },
                frame,
            };
            DrawStamp(&mut stamp);
            DestroyDrawable(ReleaseDrawable(frame));
            SetContext(old);
        }
    }

    fn start_battle_song_if_ready(&mut self, ship: ShipId) {
        use crate::battle::tactrans_native::opponent_alive;
        if !crate::sound::music::plr_playing_current() && opponent_alive(self, ship) {
            crate::battle::battle_native::engine::battle_song(self.kernel, self.world, true);
        }
    }

    fn preprocess_status(&mut self, id: ElementId) {
        crate::battle::status_native::preprocess_status(self, id)
    }

    fn draw_captain_controls_pass(
        &mut self,
        ship: ShipId,
        y: i16,
        delta: u16,
        current: u16,
        pass: u16,
    ) {
        crate::battle::status_native::engine::captains_window(self, ship, y, delta, current, pass);
    }
}
