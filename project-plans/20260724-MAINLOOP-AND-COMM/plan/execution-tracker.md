# Execution Tracker — PLAN-20260724-MAINLOOP-AND-COMM (Revised)

## Core problem: Dual data ownership
C and Rust each own copies of the same data (GlobData, CommData, activity flags).
The bridge (4,537 lines C, 542+712 FFI functions) manually synchronizes them.
The solution: consolidate ownership to Rust first, then port logic on top.

| Order | ID | Role | Status | Notes |
|---:|---|---|---|---|
| 1 | P09 | worker: consolidate game state ownership to Rust | NOT STARTED | Move GlobData to Rust, make C read through FFI |
| 2 | P09a | verifier | NOT STARTED | verify state sync + proof |
| 3 | P10 | worker: consolidate CommData ownership to Rust | NOT STARTED | Eliminate LOCDATA dual copy |
| 4 | P10a | verifier | NOT STARTED | verify CommData sync |
| 5 | P11 | worker: port RaceCommunication + InitCommunication | NOT STARTED | Native dispatch, reads Rust state |
| 6 | P11a | verifier | NOT STARTED | verify dispatch + proof |
| 7 | P12 | worker: port Arilou dialogue state machine | NOT STARTED | Reference race port |
| 8 | P12a | verifier | NOT STARTED | verify Arilou + proof |
| 9 | P13 | worker: port race batch 1 (6 races) | NOT STARTED | Starbase, Spathi, Orz, Ilwrath, Chmmr |
| 10 | P13a | verifier | NOT STARTED | |
| 11 | P14 | worker: port race batch 2 (6 races) | NOT STARTED | Melnorme, Mycon, Pkunk, Druuge, Syreen, Utwig |
| 12 | P14a | verifier | NOT STARTED | |
| 13 | P15 | worker: port race batch 3 (remaining races) | NOT STARTED | Ur-Quan, Vux, Yehat, etc. |
| 14 | P15a | verifier | NOT STARTED | |
| 15 | P16 | worker: port ExploreSolarSys | NOT STARTED | Planet exploration dispatch |
| 16 | P16a | verifier | NOT STARTED | |
| 17 | P17 | worker: port VisitStarBase | NOT STARTED | Starbase dispatch |
| 18 | P17a | verifier | NOT STARTED | |
| 19 | P18 | worker: port InstallBombAtEarth + hyperspace | NOT STARTED | Hyperspace dispatch |
| 20 | P18a | verifier | NOT STARTED | |
| 21 | P19 | worker: wire Battle dispatch to Rust | NOT STARTED | Combat dispatch |
| 22 | P19a | verifier | NOT STARTED | |
| 23 | P20 | worker: automation proof scripts | NOT STARTED | All dispatch target proofs |
| 24 | P20a | verifier | NOT STARTED | |
| 25 | P21 | worker: final verification | NOT STARTED | All gates + all proofs |
| 26 | P21a | verifier | NOT STARTED | Acceptance |

## Key architectural change
Before: C owns GlobData/CommData → Rust reads through FFI → bridge synchronizes
After:  Rust owns GlobData/CommData → C reads through FFI → no dual ownership