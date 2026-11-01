# Probe encounter v1

This scenario preserves its four displacement proofs, three right-angle turns, menu transitions, dispatch checks and returned-to-Sol assertion. Incidental probe contacts after the first and third displacement proofs use the Human cruiser and held point defense from the #774 repair. After the second contact, a 32-callback thrust pulse clears the encounter's reentry position before the remaining original turn and leg. This keeps the return-map opening away from Sol's natural entry boundary in native play. Explicit Sol autopilot after the fourth displacement then returns the fleet before the original 100-callback wait and arrival assertion. Without that clearance, native flight can naturally enter Sol while the return map is still opening, while replay finishes the square short of Sol. All clearance and return movement uses ordinary player controls. No controller or game behavior changes.

## Opening communication deadline

The #720 orchestrator decision permits tightening only the existing 9,000-callback opening-communication wait. Measurements on `33de94b74`, with the unchanged scenario, were:

| Mode | Wait entry input | Completion input | Elapsed callbacks |
|---|---:|---:|---:|
| Replay | 846 | 3,953 | 3,107 |
| Real-clock headless | 841 | 3,714 | 2,873 |
| Native real clock | 842 | 3,539 | 2,697 |

Wait entry is the input immediately after `dispatch_verified:encounter=24:dialogue=18`; completion is the `communication_completed:count=1:minimum=1` observation. Replay action-phase records independently identify the wait entry at input 846. Native measurements came from `tmp/pt-runs/P720-opening`, launched through `tmp/pt-runs/iterm-run.sh` with OS screenshot checkpoints. Both unchanged headless runs and the native run subsequently reproduced the known semantic failure; these measurements concern the completed opening wait, not a successful baseline scenario.

The new deadline is `ceil((2 × max(3107, 2873, 2697)) / 100) × 100 = 6300` callbacks. Lowering the deadline frees 2,700 statically admitted callbacks for the explicit probe-contact and return actions. No other existing wait changes. The original global limits remain 15,030 inputs, 28,060 presentations and 480 wall-clock seconds. Static validation admits the complete repaired script under those limits.
