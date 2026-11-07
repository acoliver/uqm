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
        // SAFETY: the host retains the initialized, disjoint records and RNG until this callback returns.
        unsafe {
            let record = self.record.as_ptr();
            let star_type = {
                let game = self.game.borrow();
                (*game
                    .world
                    .star_pointer(crate::planets::starmap::current_catalog()))
                .star_type
            };
            let index = self.index;
            let mut system = SolarSys::from_records(
                crate::engine::world::generation::GenerationRecords {
                    sun: NonNull::from(&mut (*record).SunDesc[0]),
                    planets: NonNull::from(&mut (*record).PlanetDesc),
                    moons: NonNull::from(&mut (*record).MoonDesc),
                    info: NonNull::from(&mut (*record).SysInfo),
                    scene: NonNull::from(&mut (*record).PlanetSideFrame[1]),
                },
                NonNull::new(super::super::SysGenRNG)
                    .unwrap_or_else(|| panic!("system random stream")),
                crate::planets::ffi::plan_data(),
                star_type,
                index,
            );
            run(generator(index), &mut system, self)
        }
    }
}

/// # Safety
/// As [`Site::with_system`].
pub unsafe fn with_system<R>(
    game: &mut Cx<'_>,
    io: &mut Io<'_>,
    record: *mut SOLARSYS_STATE,
    run: impl FnOnce(&dyn SystemGenerator, &mut SolarSys<'_>, &mut dyn SiteHost) -> R,
) -> R {
    let game = RefCell::new(Cx {
        b: game.b,
        world: game.world,
        kernel: game.kernel,
        rng: game.rng,
    });
    let io = RefCell::new(Io {
        observer: &mut *io.observer,
    });
    // SAFETY: the caller provides the exclusive live record for this temporary host.
    unsafe { Site::new(&game, &io, record).with_system(run) }
}

/// # Safety
/// The descriptor belongs to this system's planet or moon array, with its
/// parent link already initialized.
pub unsafe fn body(record: *const SOLARSYS_STATE, world: *const PlanetDesc) -> Body {
    // SAFETY: the descriptor and parent belong to the initialized arrays required by the caller.
    unsafe {
        let sun = std::ptr::addr_of!((*record).SunDesc[0]);
        let first = std::ptr::addr_of!((*record).PlanetDesc).cast::<PlanetDesc>();
        if (*world).prev_desc.cast_const() == sun {
            Body::Planet(
                u8::try_from(world.offset_from(first))
                    .unwrap_or_else(|error| panic!("planet index: {error:?}")),
            )
        } else {
            let moons = std::ptr::addr_of!((*record).MoonDesc).cast::<PlanetDesc>();
            Body::Moon {
                planet: u8::try_from((*world).prev_desc.cast_const().offset_from(first))
                    .unwrap_or_else(|error| panic!("parent index: {error:?}")),
                moon: u8::try_from(world.offset_from(moons))
                    .unwrap_or_else(|error| panic!("moon index: {error:?}")),
            }
        }
    }
}
