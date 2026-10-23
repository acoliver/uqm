# Configuration

The game uses Cargo features, command-line options and the player's configuration
files. There are no C preprocessor switches or generated build.vars.

`audio_heart` is enabled by default. `debug-process` enables native-window test
support. Neither selects a C implementation.

Run `rust/target/release/uqm --help` for options. Common options include
`--contentdir`, `--configdir`, `--res`, `--windowed`, `--fullscreen`, `--scale`,
`--musicvol`, `--sfxvol` and `--speechvol`. Configuration loading lives in
`rust/src/mainloop/options.rs`.

Automation uses `--automation-script` and `--automation-output`; normal play
leaves both unset. The native runner supplies isolated configuration directories
for scenario runs, including any declared starting save.
