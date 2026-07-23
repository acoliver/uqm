# Pseudocode 003: Watchdog, Ordered I/O, Fallback, and Finalization

Normative contract: `../authoritative-execution-contract.md` §§2-4.

```text
201: WATCHDOG(kind, now)
202:   candidate = checked_add(applicable_seen, 1); overflow => typed counter failure
203:   STORE candidate in pure proposed state
204:   IF input_seen >= max_input RETURN InputTimeout
205:   IF present_seen >= max_present RETURN PresentationTimeout
206:   IF elapsed >= timeout RETURN WallTimeout
207:   IF clock regressed RETURN ClockRegression
208:   RETURN Admit
209: // max=3 timeline: callback 1 stores 1/admit; 2 stores 2/admit; 3 stores 3/timeout
210:
211: RESERVE(record/effect)
212:   under runtime mutex allocate checked sequence + state_version + payload
213:   create RAII reservation capable of publishing cancellation
214:   unlock runtime before effects/wait/I/O
215: ORDERED_PUBLISH
216:   acquire dedicated sink synchronization (never runtime mutex)
217:   wait synchronously until sequence == next_to_publish
218:   write/flush exact record OR publish in-memory cancelled slot after fatal sink error
219:   advance cursor; notify; remember result
220: COMMIT
221:   under runtime mutex validate sequence/version/generation/terminal; advance or fail stale
222:
223: FALLBACK(class)
224:   first-wins CAS terminal mirror; release-store abort; clear capture generation
225:   cancel open reservation so cursor cannot gap
226:   release owned keys from fixed lock-free mask/value mirror outside mutex
227:   read activity; write activity OR exact 0x4000 outside mutex
228:   return callback-specific conservative result
229:
230: FINALIZE
231:   phase CAS Running/Terminal -> Finalizing; clear ACTIVE and capture generation
232:   wait for active shell count zero without runtime lock
233:   take runtime once; close reservation stream; publish all cancellations in order
234:   attempt exactly one ordered run_end
235:   flush/recover/sync/close trace and drop every automation handle
236:   store final status mirror; mark Finalized
237:   call teardown_subsystems only afterward
238:   active run creates active receipt; inactive smoke creates separate inactive receipt
239:   each receipt follows actual teardown and closed mode-specific handles
```
