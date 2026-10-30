# Talking-pet fleet preparation

The connected Arilou checkpoint has two Human cruisers and a 32-crew flagship.
That fleet lost against the ten Umgah guards. Changing which cruiser fought
first still left two guards undefeated.

The scenario retains its initial equipment and story assertions, then prepares
for the fight through ordinary player inputs. It visits the Alpha Octantis
Melnorme, buys fuel with the remaining credits and returns to Sol. Selling the
cruisers and one turning jet pays for a second crew pod and the first crew
refill. The ship then surveys the rainbow world at catalog entry 62 and sells
its location to the Melnorme at Zeeman.

Melnorme credits cannot buy Starbase equipment directly. Four fuel purchase
and resale trips between Alpha Centauri and Sol turn those earned credits into
RU. The final Starbase visit restores 100 flagship crew and purchases and crews
five replacement Human cruisers. The scenario observes those purchases before
departure. The existing eleven thrusters and forward gun remain installed.
The escort purchases take priority over another crew pod and a dynamo: the
150-crew, two-cruiser preparation passed both headless clocks but still lost
the flagship during the final native guard engagement.

On the return to Beta Orionis, hostile hyperspace patrols use ordinary escape.
A separate Umgah patrol offers `bye_zombie`, so the script explicitly leaves
that peaceful conversation before approaching the homeworld again. The
homeworld Talking Pet dialogue, ten-guard fight, acquisition, protection and
save assertions remain unchanged. The route uses the original connected save,
seed and scenario-wide budgets; no save bytes or game implementation change.

## Acceptance

Replay completed at 50,973 admitted inputs and 103,643 presentations. The
real-clock headless run completed at 51,432 inputs and 105,365 presentations.
Native run `S719-pet-five-escorts-1030` finalized successfully at 50,345 inputs
and 98,449 presentations. Its semantic result records ten enemy ships defeated,
two player ships remaining, normal Cyborg speed and 100 flagship crew. The
independent image comparison inspected the original OS captures at checkpoints
0272, 0613 and 0972 and found visible UQM dialogue and menus.

The full 188-scenario recording passed with existing watchdogs unchanged. The
recorder was built locally with one worker because the repository recorder has
no serial option; tracked tooling was restored afterward (issue #798). Only this
scenario's generated count payload and its inventory entry changed. The official
recorded-tree parity check passed. All 34 original actions, including six
assertions, remain an ordered subsequence; the seed, save and budgets are
unchanged. Review skipped per Andrew 2026-10-09.
