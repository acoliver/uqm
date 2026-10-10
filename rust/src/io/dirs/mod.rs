//! Rust port of `sc2/src/libs/file/dirs.c`: `expandPath`, `mkdirhier`,
//! `createDirectory` and `getHomeDir`.
//!
//! The behaviour lives in safe Rust over byte strings ([`expand`],
//! [`mkdirhier`]); [`c_abi`] holds the four `extern "C"` entry points with the
//! exact C signatures from `sc2/src/libs/file.h`. Paths stay `&[u8]` /
//! `OsStr` throughout: C treats them as raw bytes, so nothing here validates
//! or lossily converts UTF-8.
//!
//! # Activation
//!
//! The entry points in [`c_abi`] carry no export attribute while the C
//! `dirs.c.o` provider is still selected: exporting the same symbols from both
//! would be a duplicate-provider link. Activation gives each one its C symbol
//! with `#[export_name = "expandPath"]` (and `mkdirhier`, `getHomeDir`,
//! `createDirectory`) in the same change that removes `dirs.c` from the
//! build (`sc2/src/libs/file/Makeinfo`, the `native-inputs.json` and
//! provider-manifest entries for `dirs.c.o`, and the ownership-ledger rows).
//! `dir_prep::expand_path` is a separate String-based approximation of
//! `expandPath` used by the Rust-owned startup; it does not follow this
//! contract (it accepts a bare `~`, has no length limit and keeps `..`).
//!
//! # Allocation
//!
//! Neither `mkdirhier` nor `expandPath` hands memory to its caller, and the C
//! `HMalloc`/`HFree` pairs in `dirs.c` only wrap scratch buffers that never
//! leave the function. They are plain Rust `Vec`s here, so no allocator
//! compatibility question arises. `getHomeDir` returns a pointer the caller
//! never frees (see [`process_env::home_dir_pointer`]).

pub mod c_abi;
pub mod expand;
pub mod mkdirhier;
pub mod process_env;

#[cfg(test)]
mod c_abi_tests;
#[cfg(test)]
mod expand_model_tests;
#[cfg(test)]
mod expand_tests;
#[cfg(test)]
mod mkdirhier_tests;
#[cfg(test)]
mod test_support;
