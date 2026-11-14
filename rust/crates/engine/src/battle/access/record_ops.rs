//! Engine-owned operations on battle records and retained projections.

#[cfg(not(test))]
use crate::battle::flags::*;
#[cfg(not(test))]
use crate::battle::{Cx, ElementId, ShipId, Subject, WeaponOrigin};

#[cfg(not(test))]
impl Cx<'_> {
    /// Predict contact from value projections without moving either live element.
    pub fn plot_intercept(&mut self, a: &Subject, b: &Subject, turns: u16, margin: u16) -> u16 {
        crate::battle::intercept::plot_intercept(self, a, b, turns, margin)
    }
    pub fn track_ship(&mut self, id: ElementId, facing: &mut u16) -> i16 {
        crate::battle::tracking::track_ship(self, id, facing)
    }
    pub fn gravity(&mut self, id: ElementId) -> bool {
        crate::battle::gravity::calculate_gravity(self, id, self.screen_dimensions())
    }
    pub fn matter_conflict(&mut self, id: ElementId) -> bool {
        let control_for = |element| {
            crate::battle::gravity::control(
                self,
                element,
                self.equivalent_frame(
                    self.element_frame_set_first(element, false),
                    self.element(element).current.image.frame.view(),
                ),
            )
        };
        let mut control = control_for(id);
        crate::battle::gravity::time_space_matter_conflict(self, id, |test| {
            let mut other = control_for(test);
            self.projected_intersect(&mut control, &mut other, MAX_TIME_VALUE as u16) != 0
        })
    }
    pub fn object_cloaked(&mut self, id: ElementId) -> bool {
        let style = self.primitive_style(id);
        style.kind >= crate::graphics::native_abi::EMPTY_PRIMITIVE
            || (u32::from(style.kind) == 2
                && style.color.r == 0
                && style.color.g == 0
                && style.color.b == 0
                && style.color.a == 255)
    }
    pub fn subject_cloaked(&self, subject: &Subject) -> bool {
        let style = self.subject_style(subject);
        style.kind >= crate::graphics::native_abi::EMPTY_PRIMITIVE
            || (u32::from(style.kind) == 2
                && style.color.r == 0
                && style.color.g == 0
                && style.color.b == 0
                && style.color.a == 255)
    }
    pub fn track_subject(&mut self, subject: &mut Subject, facing: &mut u16) -> i16 {
        crate::battle::tracking::track_subject(self, subject, facing)
    }

    pub fn projected_intersect(
        &self,
        a: &mut crate::graphics::native_abi::INTERSECT_CONTROL,
        b: &mut crate::graphics::native_abi::INTERSECT_CONTROL,
        time: u16,
    ) -> u16 {
        // SAFETY: callers retain both intersect frames for the duration of the projection.
        unsafe { crate::graphics::geometry_ffi::DrawablesIntersect(a, b, time) }
    }

    pub fn spawn_weapons(
        &mut self,
        ship: ShipId,
        origin: WeaponOrigin,
    ) -> crate::battle::WeaponSpawn {
        let cx = self;
        let parent = std::ptr::from_ref(cx.ship(ship)) as usize;
        if let Some(race) = cx.desc(ship).race {
            let Some(callback) = cx
                .desc(ship)
                .race_callbacks
                .weapon
                .resolve(race.init_weapon)
            else {
                return Vec::new();
            };
            super::invoke(callback as usize, parent, || callback(cx, ship, origin))
        } else {
            panic!("ship has no race");
        }
    }
    pub fn invoke_ship_process(&mut self, ship: ShipId, post: bool) -> bool {
        let cx = self;
        let desc = cx.desc(ship);
        if let Some(race) = desc.race {
            let callback = if post {
                desc.race_callbacks.post.resolve(race.postprocess)
            } else {
                desc.race_callbacks.pre.resolve(race.preprocess)
            };
            if let Some(callback) = callback {
                let parent = std::ptr::from_ref(cx.ship(ship)) as usize;
                super::invoke(callback as usize, parent, || callback(cx, ship));
                true
            } else {
                false
            }
        } else {
            panic!("ship has no race");
        }
    }
}
