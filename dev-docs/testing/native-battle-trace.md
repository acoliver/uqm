# Native battle divergence traces

Native automation writes `native_battle` records into `automation/trace.jsonl` using the existing ordered observer sink and its 64 MiB member reservation. These records use schema 3. Other records retain schema 2, and headless parity journals and traces remain unchanged.

Each sampled combat input has both sides' current and pulsed seven-key arrays, mapped through the selected input templates, plus the current and pulsed menu arrays used by the picker. An `advanced` sample captures the frame after simulation, including the actual ship input bitfields produced by human, computer or network control. Both samples contain RNG stream state, ditty state, ship position/velocity/facing/crew/energy/status and fleet counters. Samples are fixed-width and exclude non-combat flight and all non-battle menus.

Each replacement gate evaluation produces a record containing its original ditty value, the exact `plr_playing_current` result it consumed, the resulting ditty value and the blocking decision. `track_playing: null` means the original short circuit did not read the audio worker. Diagnostics never make a second audio query or advance/reseed the RNG.

All records include the admitted input count, battle frame counter, current presentation count, elapsed milliseconds and picker active/generation state. `delta_ms` measures milliseconds since the preceding record of the same phase (input, advanced or gate); it is null for the first sample. Gate records occur at the actual evaluation boundary, including multiple evaluations within a frame.

Compare two retained suite directories, scenario directories or automation directories:

```sh
cd rust
cargo xtask native-battle-diff LEFT_RUN_DIR RIGHT_RUN_DIR
```

The comparison streams bounded JSONL records, compares corresponding battle observations in order, and prints the first differing input tick and nested field, including an array index. It compares input identity, battle state/gate/picker, presentation count and timing, in that order. Timing remains evidence and is not normalized or ignored. A difference exits nonzero. Self-comparison reports the record count and exits zero. Missing diagnostics, malformed records and truncated records fail instead of reporting equality. A suite comparison pairs scenario directories by their retained directory names.

The trace does not retroactively diagnose runs made before this instrumentation, and ship samples do not include every projectile or resource field. They distinguish input, picker, audio gate, RNG and sampled fighter-state differences without claiming complete simulation parity. Existing headless journals retain the broader element-level parity proof.
