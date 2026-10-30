# Vault and ambush route preparation

The vault trip starts after the Starbase resupply route, rather than leaving
straight from the rainbow-location fuel purchase. Run the existing producers
in this order:

1. `campaign-s09h-rainbow-refuel` buys fuel with earned Melnorme credits.
2. `campaign-s10a-crew-refill` returns to Sol and hires crew to 150.
3. `campaign-s10a-full-drives` sells one Fury, installs the two missing thrusters
   and buys two turning jets.
4. `campaign-s09i-refueled-ambush` recovers the vault ships and returns to Talana.

The starting checkpoint is `dev-ambush-drives.sav`, extracted from the third
producer's passing native run. Its adjacent provenance records the paid
transactions' result: 150 crew, 18,800 raw fuel, eleven thrusters, four turning
jets and two Furies. It retains the shuttle and the locked vault. This is an
existing development-route checkpoint, not a claim of connected campaign
ancestry or a newly generated save.

The earlier direct route reached the vault system with insufficient crew for
its hostile patrols. More fuel alone did not solve that loss. The prepared
route supplies crew and maneuvering before departure. Local patrols use the
ordinary escape action, including a planet-orbit approach before each moon;
this avoids asking an Ur-Quan contact for the nonexistent `bye` response.
The moon and lander actions still have to reach their original destinations,
unlock the vault, consume the shuttle and arrange the Mycon ambush. All original
assertions, the seed and the budgets remain unchanged.
