//! Restart menu ownership, frame rules and campaign-start orchestration.
pub mod do_restart;
pub mod menu_logic;
pub mod orchestration;
pub mod restart_ops;
pub mod types;

#[cfg(test)]
mod owned_tests;
