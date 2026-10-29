# Probe flight and menu scenarios

The v2, v5, v6 and v7 scripts handle incidental probe contacts before resuming their flight or menu checks. Every original step, assertion condition and budget remains unchanged. Added actions hail each probe, select its offered `bye_1`, `bye_2` or `bye_3` response, choose the starter cruiser, and use the existing `fight_battle` aiming/fire controls while holding point defense. They wait for hyperspace reentry and its entry animation before releasing player controls or opening the flagship menu. v2 also explicitly navigates back to Sol before its original return assertion.

The cruiser can survive these contacts without changing combat rules. Under exact replay PCM completion, the first three fights each take 412 callbacks and leave 14, 10 and 6 crew respectively. These observations establish that the earlier Cyborg fleet losses do not make these contacts unwinnable. The scripts use player inputs, not replacements for ship state or battle outcomes.

The missing-facing failure came from trying to turn after an encounter had removed the flight ship. Comparison with the original C does not justify a game-state correction:

- `1ea06965e^:sc2/src/uqm/hyper.c`, `unhyper_transition`, saves the ship's facing plus one, clears `IN_BATTLE` and sets `START_ENCOUNTER`. Rust `navigation/hyper_transition.rs::unhyper` follows that sequence.
- `f17869754^:sc2/src/uqm/init.c`, `UninitShips`, clears the race queues outside `IN_ENCOUNTER`. Rust `ships/native_adapters.rs` does the same.
- C `SaveSisHyperState` explicitly permits an absent flagship element on the hyperspace encounter screen. The Rust facing reader's `None` agrees with that lifecycle. Existing `savegame::runtime::hyper_save_tests` cover both live-owner facing and an absent flagship.
- `92c630d70^:sc2/src/uqm/hyper.c`, `check_hyperspace_encounter`, uses 100 percent probe probability before the starbase becomes available. Rust `navigation/hyperspace.rs` retains that rule.

No saved facing is substituted for a live facing, and an interrupted turn is not accepted as completed. v1 remains unresolved: its original callback budget has only 1,044 statically available callbacks, insufficient for the demonstrated two-contact handling and return navigation. Its script is unchanged.
