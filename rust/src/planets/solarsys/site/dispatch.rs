//! Disjoint campaign/effect and generation loans for one content invocation.

use super::Site;
use crate::engine::battle::Cx;
use crate::engine::observe::Io;
use crate::engine::world::generation::SolarSys;
use crate::planets::generate::{generator, SiteHost, SystemGenerator};
use crate::planets::solar_system::SolarSystem;
use std::cell::RefCell;

impl Site<'_, '_, '_> {
    pub fn invoke<R>(
        &mut self,
        system: &mut SolarSys<'_>,
        run: impl FnOnce(&dyn SystemGenerator, &mut SolarSys<'_>, &mut dyn SiteHost) -> R,
    ) -> R {
        run(generator(self.index), system, self)
    }
}

pub fn invoke<R>(
    game: &mut Cx<'_>,
    io: &mut Io<'_>,
    solar: &mut SolarSystem,
    run: impl FnOnce(&dyn SystemGenerator, &mut SolarSys<'_>, &mut dyn SiteHost) -> R,
) -> (
    R,
    Option<crate::planets::orbit::presentation::OrbitPresentation>,
) {
    solar.observe(game, io);
    let result = {
        let game = RefCell::new(game.reborrow());
        let io = RefCell::new(Io {
            observer: &mut *io.observer,
        });
        let mut pickup = crate::planet_side::special_effects::PickupContext::NoLander;
        let mut site = Site::new(
            &game,
            &io,
            solar.star(),
            solar.sys.star_index(),
            solar.in_orbit(),
            &mut pickup,
            None,
        );
        site.background = solar
            .stars
            .as_ref()
            .map(|owner| owner.lease().context_frame(0));
        (site.invoke(&mut solar.sys, run), site.presentation.take())
    };
    solar.apply_input_effects(game);
    solar.observe(game, io);
    result
}
