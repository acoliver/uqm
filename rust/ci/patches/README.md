# Tool corrections

Each patch here is applied to a hash-verified tool distribution during gate
setup, and the file it produces is hash-verified in turn, so what the gate runs
stays pinned end to end. The digests live beside the tool in `rust/ci/gates.json`.

## lizard-1.24.0-rust-char-literal.patch

lizard's Rust reader matches a character literal as a lifetime. The leftover
closing quote opens a string that runs to the next apostrophe in the file, and
everything between is never parsed.

`lizard-rust-char-literal-repro.rs.txt` reproduces it in eight lines: stock lizard
1.24.0 reports one function there, and only the second one. With the patch it
reports both.

The failure is silent. lizard reports fewer functions rather than an error, so a
threshold gate reading its output cannot distinguish "nothing is too complex"
from "most of the file was never measured". On `rust/xtask/src/ci/exec.rs` at
commit 9c1dbdf24 the two points were 2600 lines apart, so stock lizard saw 74 of
151 functions and reported a 31-line function as 2631 lines.

The reproduction carries a `.txt` suffix so it stays documentation rather than
joining the tracked Rust the gates measure. Copy it to a `.rs` file to run it.

Remove this patch once an upstream release carries the fix, and drop
`source_patch` from the lizard entry in the authority at the same time.
