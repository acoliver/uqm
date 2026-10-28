# Ending presentation wait

The `ending:presentation_completed == 1` wait allows 18,340 input callbacks.
The previous 12,000-callback limit expired inside `final.txt`, before the original
ending finished. The condition, content, global budgets and later credit checks
are unchanged. This correction follows the scenario-intent decision on #719.

The bound comes from `base/cutscene/ending/{ending,victory1,victory2,final}.txt`
in `sc2/content/packages/uqm-0.8.0-content.uqm` and the interpreter in
`rust/src/front_end/intro.rs`:

- Time uses 840 ticks per second. A timed presentation callback sleeps for
  `840 / 84 = 10` ticks.
- Convert milliseconds with integer `milliseconds * 840 / 1000`. `WAIT` is
  relative to the current time; `SYNC` sets the last synchronization deadline
  relative to `StartTime`; `DSYNC` is relative to that last deadline. `RESYNC`
  resets both synchronization times. It does not accumulate `DSYNC` values.
- Screen fades add the interpreter's 84-tick completion allowance, including
  zero-duration fades. For each timer, count the ceiling of the remaining ticks
  divided by 10, plus its setup and completion callbacks.
- Assume no rendering cost or text-transition time. Those costs consume time
  without admitting presentation input and can only reduce timer callbacks.
  Charge another callback for every recognized opcode as an upper bound on
  command dispatch, plus one completion boundary for each of the four scripts.

| Script | Timer callbacks | Recognized opcodes |
|---|---:|---:|
| ending | 0 | 6 |
| victory1 | 1,188 | 640 |
| victory2 | 1,442 | 530 |
| final | 14,286 | 244 |
| Total | 16,916 | 1,420 |

Thus the content-derived upper bound is `16,916 + 1,420 + 4 = 18,340`.
It covers the complete presentation even though the wait begins after the
presentation has started. No content is skipped and no completion is inferred
from elapsed time; the wait still requires the game's completion observation.
