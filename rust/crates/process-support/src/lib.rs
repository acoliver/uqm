//! Process supervision shared by game automation and standalone developer tools.

pub mod child_session;
pub mod identity;
#[cfg(unix)]
pub mod os;
