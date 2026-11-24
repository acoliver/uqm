//! Hyperspace resource operations over the retained kernel.

use crate::{graphics::context::ContextLease, Kernel};

fn context(context: Option<crate::graphics::context::ContextId>, name: &str) -> ContextLease {
    ContextLease::retain(
        context.unwrap_or_else(|| panic!("hyperspace {name} context is not initialized")),
    )
}

pub fn radar_context(kernel: &Kernel) -> ContextLease {
    context(kernel.radar_context, "radar")
}

pub fn space_context(kernel: &Kernel) -> ContextLease {
    context(kernel.space_context, "space")
}

pub fn status_context(kernel: &Kernel) -> ContextLease {
    context(kernel.status_context, "status")
}

pub fn load(kernel: &mut Kernel) {
    use crate::battle::frames::load_graphic;
    use crate::graphics::palette_cycle::PaletteCycle;
    if kernel.hyperstars[0].is_null() {
        kernel.hyperstars[0] = load_graphic(&kernel.content(), c"graphics.ambient");
        kernel.hyperstars[1] = load_graphic(&kernel.content(), c"graphics.hyperstars");
        kernel.cmaps[0] = PaletteCycle::load(&kernel.content(), c"colortable.hyperspace");
        kernel.hyperstars[2] = load_graphic(&kernel.content(), c"graphics.quasispace");
        kernel.cmaps[1] = PaletteCycle::load(&kernel.content(), c"colortable.quasispace");
    }
}

/// Release the hyperspace frames and palettes after their last user.
pub fn free_hyper_data(kernel: &mut Kernel) {
    for frame in &mut kernel.hyperstars {
        drop(crate::graphics::drawable::LoadedDrawable::from_frame(
            std::mem::take(frame),
        ));
    }
    for map in &mut kernel.cmaps {
        *map = None;
    }
}

pub fn select_palette(kernel: &Kernel, quasispace: bool) {
    if let Some(palette) = &kernel.cmaps[usize::from(quasispace)] {
        palette.apply();
    }
}

pub fn free_hyperspace(kernel: &mut Kernel) {
    crate::collections::observation::swap_image_view(
        &mut kernel.hyperstars[0],
        &mut kernel.stars_in_space,
    );
}
