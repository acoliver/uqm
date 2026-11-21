//! The save menu persistence entry points, called on the game thread.
pub mod load;
pub mod save;

/// The save-slot service the game options menu reaches through the kernel.
pub const SLOTS: uqm_engine::save::slots::SaveSlots = uqm_engine::save::slots::SaveSlots {
    describe: load::load_game_descriptions,
    save: |cx, solar, slot, summary, name| save::save_game(cx, solar, slot, summary, name) != 0,
    load: |cx, slot| load::load_game(slot, load::LoadRequest::Restore(cx)) != 0,
};

fn complete(result: anyhow::Result<()>, operation: &str, witness: Option<(u16, bool)>) -> u32 {
    match result {
        Ok(()) => {
            if let Some((slot, loading)) = witness {
                uqm_engine::collections::observation::record_save(slot, loading);
            }
            1
        }
        Err(error) => {
            log::error!("{operation}: {error}");
            0
        }
    }
}
