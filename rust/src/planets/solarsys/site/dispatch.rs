//! Conversion from the retained mode records to the content-generation API.

use super::Site;
use crate::engine::battle::Cx;
use crate::engine::observe::Io;
use crate::engine::world::generation::{Body, SolarSys};
use crate::engine::world::planet::PlanetDesc;
use crate::planets::generate::{generator, SiteHost, SystemGenerator};
use crate::ships::native::abi::SOLARSYS_STATE;
use std::cell::RefCell;
use std::ptr::NonNull;

impl Site<'_, '_, '_> {
    /// # Safety
    /// The system and random stream must be initialized and exclusively available
    /// between content callbacks. No reference into them may survive this call.
    pub unsafe fn with_system<R>(
        &mut self,
        run: impl FnOnce(&dyn SystemGenerator, &mut SolarSys<'_>, &mut dyn SiteHost) -> R,
    ) -> R {
        let record = self.record.as_ptr();
        let star_type = {
            let game = self.game.borrow();
            // SAFETY: the retained world selects a live star in the current catalog.
            unsafe { (*game.world.star_pointer()).star_type }
        };
        let index = self.index;
        // SAFETY: the host retains the initialized, disjoint records until the callback returns.
        let records = unsafe {
            crate::engine::world::generation::GenerationRecords {
                sun: NonNull::from(&mut (*record).SunDesc[0]),
                planets: NonNull::from(&mut (*record).PlanetDesc),
                moons: NonNull::from(&mut (*record).MoonDesc),
                info: NonNull::from(&mut (*record).SysInfo),
                scene: NonNull::from(&mut (*record).PlanetSideFrame[1]),
            }
        };
        // SAFETY: the caller retains exclusive records and RNG, and the definitions stay live.
        let mut system = unsafe {
            let rng = NonNull::new(super::super::SysGenRNG)
                .unwrap_or_else(|| panic!("system random stream"));
            SolarSys::from_records(
                records,
                rng,
                crate::planets::ffi::plan_data(),
                star_type,
                index,
            )
        };
        run(generator(index), &mut system, self)
    }
}

/// # Safety
/// As [`Site::with_system`].
pub unsafe fn with_system<R>(
    game: &mut Cx<'_>,
    io: &mut Io<'_>,
    record: *mut SOLARSYS_STATE,
    run: impl FnOnce(&dyn SystemGenerator, &mut SolarSys<'_>, &mut dyn SiteHost) -> R,
) -> (
    R,
    Option<crate::planets::orbit::presentation::OrbitPresentation>,
) {
    let game = RefCell::new(Cx {
        b: game.b,
        world: game.world,
        kernel: game.kernel,
        rng: game.rng,
    });
    let io = RefCell::new(Io {
        observer: &mut *io.observer,
    });
    // SAFETY: the caller provides exclusive initialized records and RNG through this host.
    unsafe {
        let mut site = Site::new(&game, &io, record, None);
        (site.with_system(run), site.presentation.take())
    }
}

/// # Safety
/// The descriptor belongs to this system's planet or moon array, with its
/// parent link already initialized.
pub unsafe fn body(record: *const SOLARSYS_STATE, world: *const PlanetDesc) -> Body {
    // SAFETY: the caller retains the initialized arrays and the descriptor's parent link.
    let (sun, first, parent) = unsafe {
        (
            std::ptr::addr_of!((*record).SunDesc[0]),
            std::ptr::addr_of!((*record).PlanetDesc).cast::<PlanetDesc>(),
            (*world).prev_desc.cast_const(),
        )
    };
    if parent == sun {
        // SAFETY: world belongs to the caller's planet array.
        let planet = unsafe { world.offset_from(first) };
        Body::Planet(u8::try_from(planet).unwrap_or_else(|error| panic!("planet index: {error:?}")))
    } else {
        // SAFETY: the caller retains the initialized moon array.
        let moons = unsafe { std::ptr::addr_of!((*record).MoonDesc) }.cast::<PlanetDesc>();
        // SAFETY: the moon's parent belongs to the caller's planet array.
        let planet = unsafe { (*world).prev_desc.cast_const().offset_from(first) };
        let planet = u8::try_from(planet).unwrap_or_else(|error| panic!("parent index: {error:?}"));
        // SAFETY: world belongs to the caller's moon array.
        let moon = unsafe { world.offset_from(moons) };
        Body::Moon {
            planet,
            moon: u8::try_from(moon).unwrap_or_else(|error| panic!("moon index: {error:?}")),
        }
    }
}
