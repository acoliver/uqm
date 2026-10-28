# Thraddash homeworld crash regression (#379)

This scenario exercises the homeworld fight that crashed after 314 seconds in #379. The Thraddash homeworld fleet is infinite, so exhausting it is not a valid completion condition. The script selects the saved escorts through ordinary picker inputs, then selects the flagship and observes the battle before accepting its normal loss.

Production `battle_types::BATTLE_FRAME_RATE` is 24 frames per second. The original crash point is therefore 314 × 24 = 7,536 completed battle frames. The script requires 7,537 frames to pass that point without crashing. `ONE_SECOND / BATTLE_FRAME_RATE` is 840 / 24 = 35 timer ticks per frame, not 35 frames per second.

The new duration wait admits 7,538 callbacks, covering the entire 7,537-frame target plus its observation boundary rather than depending on how much combat the escorts have already completed. Its frame counter advances only for completed battle frames. After this observation the script requires genuine flagship destruction and a return to the main menu through the normal lost-battle flow. The original global input, presentation and wall-clock budgets remain unchanged.
