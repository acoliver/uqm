//! Rust port of `sc2/src/libs/uio/uioutils.c`: the seven functions of
//! `uioutils.h` that are not `static inline`.
//!
//! The rules live in safe Rust ([`dos_time`], [`dos_path`], and the size
//! arithmetic of [`arrays`]); [`c_abi`] holds the `extern "C"` entry points
//! with the header's exact signatures.
//!
//! # Where the Rust differs from the C
//!
//! Each difference is a C defect that a faithful transcription would
//! reproduce; none is reachable from a native caller (the only caller outside
//! `uioutils.c` is `zip.c`, and it uses only `dosToUnixTime`).
//!
//! * `excludeArray` steps over the tail with `sizeof (elementSize)` (the size
//!   of a `size_t`) instead of `elementSize`. The element size is used.
//! * `insertArray`/`excludeArray` and their pointer variants copy out of the
//!   caller's array for any `insertPos`/`startPos`/`numExclude`, valid or not.
//!   An out-of-range or overflowing argument is a caller bug here: logged,
//!   then the process aborts (see [`c_abi`]).
//! * `dosToUnixTime` is wrong by one day for every day of March in a leap
//!   year after 1980 (its leap-day test is `month <= 2` on a month index
//!   where March is 2) and counts 2100 as a leap year. The decoder uses the
//!   Gregorian calendar, the same one the `zip` crate validates entries with.
//!   The C result for an invalid date is unspecified; the decoder reports it
//!   as an error and the C entry point returns 0, the "no time" value
//!   `ZipEntryStat::mtime` already uses.
//! * `dosToUnixPath` fixes up the separator between the UNC server and share
//!   in the *source* string (writing through a `const char *`), after the
//!   prefix was copied, so the result never saw the fix-up. The result gets
//!   it and the source is left alone.
//!
//! # Time zones
//!
//! `dosToUnixTime` is pure arithmetic: a DOS date and time carry no zone and
//! the C never calls `mktime` or `localtime`, so the result reads them as
//! UTC. The same decoder serves `zip_stat`, so a path stat and the C entry
//! point agree.
//!
//! # Scope
//!
//! `\` is converted unconditionally, as the C does: this is a byte
//! transformation with no platform rule, drive letters or Unicode handling.
//! No Windows or Symbian configuration is claimed.
//!
//! # Activation
//!
//! The functions in [`c_abi`] stay unexported until `uioutils.c.o` is
//! retired, like those of [`super::paths`]. Activation gives each its C
//! symbol with `#[export_name = "..."]` in the same change that removes
//! `uioutils.c` from `sc2/src/libs/uio/Makeinfo`, the `native-inputs.json`
//! and provider manifest entries, and the ledger rows. `uioutils.h` also
//! holds `static inline` helpers (`unconst`, `makeUInt32`, ...) that C
//! includers use; those stay with the header until its includers are ported.

pub mod arrays;
pub mod c_abi;
pub mod dos_path;
pub mod dos_time;

#[cfg(test)]
mod arrays_model_tests;
#[cfg(test)]
mod c_abi_tests;
#[cfg(test)]
mod dos_time_tests;
