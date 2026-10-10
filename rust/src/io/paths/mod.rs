//! Rust port of `sc2/src/libs/uio/paths.c`: the 21 functions of `paths.h`.
//!
//! The rules live in safe Rust over byte strings ([`components`], [`unc`]);
//! [`comp_list`] owns the `uio_PathComp` list layout and its allocation;
//! [`c_abi`] holds the `extern "C"` entry points with the header's exact
//! signatures. Paths stay `&[u8]`: C treats them as raw bytes, so nothing
//! here validates or lossily converts UTF-8.
//!
//! # Where the Rust differs from the C
//!
//! The C has defects that a faithful transcription would reproduce, none
//! reachable from a native caller (the only callers outside `paths.c` are
//! `zip.c`, which uses the two component walkers, and `uioutils.c`, which
//! uses `uio_skipUNCServerShare`). Each is ported as the code's evident
//! intent:
//!
//! * `composePath` omits the separators from the length it allocates and
//!   returns, then writes them (a heap overflow for two or more components);
//! * `decomposePath` copies each name from the end of the component, not its
//!   start;
//! * `uio_getUNCServerShare` loops forever on a non-empty share and
//!   overwrites the first byte of the server name;
//! * `uio_PathComp_new` leaves `next` uninitialised; here it is null.
//!
//! # Scope
//!
//! POSIX rules only: `/` separates. Drive letters, UNC prefixes in
//! `decomposePath`/`composePath` and `\` as a separator
//! (`HAVE_DRIVE_LETTERS`, `HAVE_UNC_PATHS`, `BACKSLASH_IS_PATH_SEPARATOR`)
//! are Windows/Symbian configurations that this module does not port.
//!
//! # Activation
//!
//! The functions in [`c_abi`] stay unexported until `paths.c.o` is retired,
//! like those of [`super::dirs`]. Activation gives each its C symbol with
//! `#[export_name = "..."]` in the same change that removes `paths.c` from
//! `sc2/src/libs/uio/Makeinfo`, the `native-inputs.json` and provider
//! manifest entries for `paths.c.o`, and the ownership-ledger rows. Selected
//! Rust already calls [`components::valid_path_name`] (mount-point checks in
//! `uio_bridge`).

pub mod c_abi;
pub mod comp_list;
pub mod components;
pub mod unc;

#[cfg(test)]
mod c_abi_tests;
#[cfg(test)]
mod components_tests;
#[cfg(test)]
mod model_tests;
