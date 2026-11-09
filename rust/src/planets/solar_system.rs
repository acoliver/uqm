//! The solar-system visit owned by exploration, not published through a pointer.

use crate::engine::{
    battle::Cx,
    input::Frame,
    observe::{CheckpointView, Injected, InputView, Io},
    world::{
        generation::{Body, SolarSnapshot, SolarSys},
        StarId,
    },
    World,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemLocation {
    Outer,
    Inner(u8),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BodyLocation {
    pub star: u16,
    pub planet: u8,
    pub moon: u8,
}

impl BodyLocation {
    pub fn new(star: StarId, body: Body) -> Self {
        Self {
            star: u16::try_from(star.0).expect("star exceeds planet persistence encoding"),
            planet: body.planet() as u8,
            moon: body.persistence_moon(),
        }
    }

    pub fn node_key(self, node: u8) -> (u16, u8, u8, u8) {
        (self.star, self.planet, self.moon, node)
    }
}

pub struct SolarSystem {
    pub sys: SolarSys<'static>,
    star: StarId,
    location: SystemLocation,
    selected: Option<Body>,
    in_orbit: bool,
    pub in_ip_flight: bool,
    pub wait_intersect: u16,
    pub draw_order: [Option<u8>; 17],
    pub turn_counter: u8,
    pub turn_wait: u8,
    pub thrust_counter: u8,
    pub max_ship_speed: u8,
}

impl SolarSystem {
    pub fn new(world: &World, star: StarId) -> Self {
        let descriptor = &world.catalog()[star.0];
        Self {
            sys: SolarSys::new(
                &super::tables::PLANET_ARRAY.0,
                descriptor.star_type,
                descriptor.index,
                crate::math::RandomContext::with_seed(12345),
            ),
            star,
            location: SystemLocation::Outer,
            selected: None,
            in_orbit: false,
            in_ip_flight: false,
            wait_intersect: 0,
            draw_order: [None; 17],
            turn_counter: 0,
            turn_wait: 0,
            thrust_counter: 0,
            max_ship_speed: 0,
        }
    }

    pub fn star(&self) -> StarId {
        self.star
    }

    pub fn location(&self) -> SystemLocation {
        self.location
    }

    pub fn selected_body(&self) -> Option<Body> {
        self.selected
    }

    pub fn in_orbit(&self) -> bool {
        self.in_orbit
    }

    pub fn enter_inner(&mut self, planet: u8) {
        assert!(
            planet < self.sys.sun.num_planets,
            "inner planet is not in the system"
        );
        self.location = SystemLocation::Inner(planet);
        self.selected = Some(Body::Planet(planet));
        self.in_orbit = false;
    }

    pub fn enter_orbit(&mut self, body: Body) {
        assert_eq!(
            self.location,
            SystemLocation::Inner(body.planet() as u8),
            "orbit body belongs to another inner system",
        );
        if let Body::Moon { planet, moon } = body {
            assert!(
                moon < self.sys.planets[usize::from(planet)].num_planets,
                "orbit moon is not in the inner system",
            );
        }
        self.selected = Some(body);
        self.in_orbit = true;
    }

    /// Interrupted orbit input retains its selected body until visit teardown.
    pub fn finish_orbit(&mut self) {
        self.in_orbit = false;
    }

    pub fn leave_inner(&mut self) {
        self.location = SystemLocation::Outer;
        self.selected = None;
        self.in_orbit = false;
    }

    pub fn snapshot(&self, screen: (i32, i32)) -> SolarSnapshot {
        SolarSnapshot {
            star: self.star,
            screen,
            in_ip_flight: self.in_ip_flight,
            in_orbit: self.in_orbit,
            wait_intersect: self.wait_intersect,
            inner_planet: match self.location {
                SystemLocation::Outer => None,
                SystemLocation::Inner(planet) => Some(planet),
            },
            selected: self.selected,
            zoom_radius: self.sys.sun.radius,
            sun_location: self.sys.sun.location,
            planet_count: self.sys.sun.num_planets,
            planets: self.sys.planets.each_ref().map(Into::into),
            moons: self.sys.moons.each_ref().map(Into::into),
        }
    }

    pub fn refresh_frame(&self, cx: &mut Cx<'_>) -> SolarSnapshot {
        let snapshot = self.snapshot(cx.screen_dimensions());
        cx.solar_frame = Some(snapshot);
        snapshot
    }

    pub fn observe(&self, cx: &mut Cx<'_>, io: &mut Io<'_>) {
        let snapshot = self.refresh_frame(cx);
        io.observer
            .checkpoint(&CheckpointView::Solar(Some(&snapshot)));
    }

    pub fn run_input(
        &mut self,
        cx: &mut Cx<'_>,
        io: &mut Io<'_>,
        reset: bool,
        step: impl FnMut(&mut Cx<'_>, &mut Io<'_>, &mut SolarSystem) -> bool,
    ) {
        cx.run_input_frame(io, reset, &mut SolarInput { solar: self, step });
    }

    pub fn body_location(&self, body: Body) -> BodyLocation {
        BodyLocation::new(self.star, body)
    }
}

struct SolarInput<'a, Step> {
    solar: &'a mut SolarSystem,
    step: Step,
}

impl<Step> Frame for SolarInput<'_, Step>
where
    Step: FnMut(&mut Cx<'_>, &mut Io<'_>, &mut SolarSystem) -> bool,
{
    fn admit(&mut self, cx: &mut Cx<'_>, io: &mut Io<'_>, view: InputView) -> Injected {
        self.solar.observe(cx, io);
        crate::engine::input::admit_observer(cx, io, view)
    }

    fn frame(&mut self, cx: &mut Cx<'_>, io: &mut Io<'_>) -> bool {
        (self.step)(cx, io, self.solar)
    }
}

#[cfg(test)]
mod tests;
