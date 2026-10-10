//! Fixture and integration rows for the resource index and the UIO mount
//! stack, run against the real Rust providers through their public C-ABI
//! entry points. The inputs and expected results are declared in
//! [`declared`]; the other modules only drive them.

mod addon;
mod config;
mod declared;
mod descriptors;
mod game;
mod lifecycle;
mod paths;
mod res;
mod stock;
mod uio_api;
