# Tracker reference fixtures

These small MODs and stereo float32 little-endian PCM files were decoded by the workstation's external libopenmpt 0.8.4, not by UQM's player. Production does not link this library. The library file SHA-256 was `dc914b304c7fde2ebcdb36d49ab561bd911ac829e4a41c05816cbb0f695ae53a`.

`generate_reference.py` constructs the modules and uses Python ctypes with `/opt/homebrew/lib/libopenmpt.dylib`. It enables nearest-neighbor interpolation, disables volume ramping and Amiga hardware emulation, selects stop-at-end playback and 200 percent stereo separation. The separation setting reproduces the established hard-left/right defaults for these fixtures, which have no explicit panning command. Shipped Pkunk's explicit 800 command is compared separately using libopenmpt's interactive2 channel panning and 100 percent separation, avoiding stereo-width extrapolation beyond a hard pan.

Run from the workspace root:

```sh
python3 rust/tests/fixtures/tracker/generate_reference.py
cd rust
cargo +1.97.1 test --test tracker_reference -- --nocapture
```

`reference.json` records each MOD's SHA-256, requested PCM prefix frame count, full externally rendered frame count, rendered duration, and libopenmpt's estimated duration. The tempo estimate differs from actual rendering: a row's BPM change applies after its first tick, and fractional tick frames are truncated independently. UQM's length contract follows rendered frames. Test tolerances remain one i16 quantization step for peak gain and 0.003 mean absolute float error for nearest-neighbor PCM. The non-looping fixture also verifies the sample-end mixer-offset decay at explicit external reference points within one i16 quantization step.

The fixtures exercise speed and BPM changes, pattern break, backward order jump, finite E6x loops, EEx row delay, arpeggio, slides, volume changes, note quantization including shipped short periods, sustained period 907, empty one-word instruments, gain and non-looping sample termination. They do not establish full ProTracker waveform parity.

`shipped-durations.json` records external duration measurements for all 63 installed UQM 0.8.0 modules. The optional integration tests read the existing content files rather than duplicating them here. Execute them with `cargo +1.97.1 test --test tracker_reference -- --ignored --nocapture`. Comparison PCM and direct-exit logs are written under `tmp/issue34-final-20261008/continuation-commfix/decoder/`.
