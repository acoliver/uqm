//! Input-free planet rendering. C remains the runtime provider at checkpoint A.

#[cfg(any(not(test), feature = "linked_c_archive"))]
pub mod c_api;
pub mod oval;
