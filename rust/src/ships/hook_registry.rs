//! Stable names for the race callbacks. The addresses are taken in this crate,
//! the one that assigns them: a function pointer's address is only stable
//! inside the crate that owns the function.

mod assigned;
mod races;

pub fn hooks() -> Vec<(usize, &'static str)> {
    races::hooks()
        .into_iter()
        .chain(assigned::hooks())
        .collect()
}
