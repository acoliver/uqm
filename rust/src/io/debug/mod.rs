//! Rust port of `sc2/src/libs/uio/debug.c`: the interactive UIO debug
//! console and its fifteen commands.
//!
//! The console is safe Rust over injected streams: [`DebugSession`] holds the
//! working directory and runs one command ([`DebugSession::execute`]) or a
//! whole input ([`DebugSession::interactive`]) against any `BufRead` and two
//! writers. Every command performs the same public UIO operations as the C
//! (`uio_open`, `uio_getDirList`, `uio_mkdir`, ...) on the repository it is
//! given; nothing is mirrored. [`tokens`] is `makeArgs`, [`commands`] the
//! `debugCommands` table and its functions, [`listing`] the text of
//! `uio_printMounts` and `uio_DirHandle_print`, and [`c_abi`] the `FILE *`
//! boundary of `uio_debugInteractive`.
//!
//! # Traced contract
//!
//! `debug.c.o` defines two symbols. `debugCommands` is the table of
//! `{name, fun}` rows; its element type is private to `debug.c` and no header
//! or other file declares it, so nothing outside the provider can use it and
//! the Rust table [`commands::DEBUG_COMMANDS`] needs no C layout.
//! `uio_debugInteractive(FILE *, FILE *, FILE *)` is declared in `debug.h`
//! (which `io.h` includes) and called nowhere: its only mention is a
//! commented-out line in `uqmdebug.c`. The C reads the repository from the
//! global `repository` of `setup.c`; the Rust takes it as an argument, so
//! that global is not referenced from here. `STAND_ALONE` (`main`,
//! `initRepository`, `unInitRepository`, `debugMountOne` with developer-machine
//! paths) is defined by no build and is not ported. `uio_MEM_DEBUG` and
//! `HAVE_GLOB` are not defined by the build either, so `mem` prints the
//! "not compiled in" line and `ls` uses the prefix and literal matchers.
//!
//! # Where the Rust differs from the C
//!
//! * A token with a NUL byte is a C string cut short in the C; a line that
//!   has one is rejected (`Command line contains a NUL byte.`).
//! * An over-long line (1023 bytes or more before the newline) is reported
//!   and discarded to its end; the C reported it and then ran the tail as the
//!   next command (a `TODO` in the source).
//! * A last line with no newline runs if it fits; the C called it "too long".
//! * A read error ends the session with the error; the C cleared it and read
//!   again, forever if it persisted. A failed write to the console streams
//!   ends the session the same way (the C ignored `fprintf` failures, and
//!   `cat` did arithmetic on `-1` after a failed `write`).
//! * `writetest` closes its handle when the write fails, and `fwritetest`
//!   does not close a stream that failed to open (the C leaked the first and
//!   passed `NULL` to `uio_fclose` for the second).
//! * `cat` prints the full byte count, not the count modulo 2^32.
//! * `stat` of a time `ctime` cannot show is the `EOVERFLOW` error, not a
//!   `NULL` dereference.
//! * `exec` reports a program that cannot be started as the C showed it: an
//!   `execvp() failed` line, then `Exit status: 1`. The child inherits the
//!   process's own standard streams, as in the C, not the injected ones.
//! * `pwd` prints `[]` and the virtual path without its leading slash. The C
//!   printed `[` + the root part + `]` + the rest; every directory handle
//!   `debug.c` opens has an empty root part (`uio_openDir` and
//!   `uio_openDirRelative` with flags 0), and the Rust handle keeps only the
//!   virtual path.
//! * `mount` shows what the Rust repository records for each mount: the file
//!   system, the mount point, the read-only flag and, as "directory name",
//!   the host directory (without its leading slash) of a directory mount or
//!   the archive directory of an archive mount.
//!
//! # Scope
//!
//! Unix only: `exec` needs fork and exec. Windows and Symbian behaviour
//! (`O_BINARY`, the `exec` stub message) is not ported.
//!
//! # Activation
//!
//! Nothing here is exported for `debugCommands` or `uio_debugInteractive`
//! while `debug.c.o` is the selected provider. The two functions the C
//! commands call, `uio_printMounts` and `uio_DirHandle_print`, are
//! exported by `uio_bridge` and now write real output through [`listing`].
//! Retirement removes `debug.c` and `debug.h` (and the `io.h` include of it)
//! in the same change as the `Makeinfo`, `native-inputs.json`, provider
//! manifest and ledger rows; no Rust symbol has to replace the two C ones.

pub mod c_abi;
pub mod commands;
mod errno;
pub mod listing;
mod repo;
mod session;
pub mod tokens;

pub use errno::Errno;
pub use session::{CommandStatus, DebugSession, OpenError, Streams};

#[cfg(test)]
mod command_tests;
#[cfg(test)]
mod interactive_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod world;
