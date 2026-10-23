//! Input-free planet rendering, including the live oval ABI provider.

#[cfg(not(test))]
pub mod c_api;
pub mod oval;
pub mod sphere_geometry;
pub mod topo_gen;
pub mod topo_scale;
pub mod topography;
