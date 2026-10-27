//! The save menu persistence entry points, called on the game thread.
pub(crate) mod load;
pub(crate) mod save;
pub(super) use load::finish_load;

fn complete(result: anyhow::Result<()>, operation: &str, witness: Option<(u16, bool)>) -> u32 {
    match result {
        Ok(()) => {
            if let Some((slot, loading)) = witness {
                crate::collections::observation::record_save(slot, loading);
            }
            1
        }
        Err(error) => {
            log::error!("{operation}: {error}");
            0
        }
    }
}
