# Tool corrections

Each patch here is applied to a hash-verified tool distribution during gate
setup, and the file it produces is hash-verified in turn, so what the gate runs
stays pinned end to end. The digests live beside the tool in `rust/ci/gates.json`.

## lizard-1.24.0-rust-char-literal.patch

lizard's Rust reader matches a character literal as a lifetime. The leftover
closing quote opens a string that runs to the next apostrophe in the file, and
everything between is never parsed. Its lifetime expression also stops before
the `#` in a valid raw lifetime such as `'r#type`; lizard then consumes the rest
of that source line as a preprocessor token.

`lizard-rust-char-literal-repro.rs` covers ASCII, Unicode, and escaped character
literals; ordinary and raw lifetimes; a label; and a function after those forms.
Stock lizard 1.24.0 and the character-only correction do not report all three
functions. With this patch, lizard reports all three.

The failures are silent. lizard reports fewer functions rather than an error, so
a threshold gate reading its output cannot distinguish "nothing is too complex"
from "most of the file was never measured". On `rust/xtask/src/ci/exec.rs` at
commit 9c1dbdf24 the two points were 2600 lines apart, so stock lizard saw 74 of
151 functions and reported a 31-line function as 2631 lines.

The complexity mutation prepends this reproduction to its over-limit function.
That contract fails unless lizard reads through all of the covered Rust tokens
and rejects the function that follows them.

Remove this patch once an upstream release carries the fix, and drop
`source_patch` from the lizard entry in the authority at the same time.
