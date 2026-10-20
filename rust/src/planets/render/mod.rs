//! Input-free planet rendering, including the live oval ABI provider.

#[cfg(any(not(test), feature = "linked_c_archive"))]
pub mod c_api;
pub mod oval;
pub mod topo_gen;
pub mod topography;
