# Pseudocode 004: Presentation, Inactive Transport, and ChildSession

Normative contract: `../authoritative-execution-contract.md` §§4,6-9.

```text
301: RUST_GFX_POSTPROCESS complete extern shell under catch_unwind
302:   run existing present; normal return defines committed presentation
303:   generation = acquire requested_capture_generation
304:   IF generation != 0 copy logical surface with exact generation
305:     use sdl2::sys ABI types or linked C accessors for format/MUSTLOCK
306:     validate pointer/w/h/pitch/BPP/masks/checked lengths
307:     IF mustlock: real SDL_LockSurface; guard real unlock; failure means no read
308:   release graphics state; then run standard reservation/effect/publish/commit shell
309: CAPTURE_TRANSACTION
310:   create-new temporary path in destination directory; encode/flush/recover/sync/close
311:   publish exclusive final name without overwrite; classify directory sync
312:   ordered capture record; matching generation/version commit; stale cannot advance
313:
314: INACTIVE_TRANSPORT (proof option only)
315:   setup exclusive 0600 Unix datagram socket + 256-bit nonce + typed command IDs
316:   before C TFB_ProcessEvents at DoInput and TaskSwitch/Sleep pumps:
317:     bounded nonblocking authenticate; ack every accept/reject; only SDL_PushEvent
318:   query actual initialized child menu.down.N via production VControl parser accessor
319:   construct ABI-authoritative keydown/up; no guessed parent default
320:   count C SDL_PollEvent before ProcessInputEvent
321:   count matching Rust VControl dispatch
322:   count post-ordinary-UpdateInputState observation and ack key_observed
323:   push real SDL_QUIT only after evidence ack; ack quit_polled only from C poll
324:   stop only after lifecycle observes QuitPosted; normal teardown
325:   close socket/counters/acks; create separate inactive teardown receipt
326:   require active-gate/service/setter=0; ABI-entry may be nonzero
327:
328: CHILD_SESSION
329:   own Child, identity, stdin/stdout/stderr, bounded reader threads, socket, manifest
330:   normal: try_wait Some is the one stored reap -> close parent handles -> drain EOF -> join
331:   failure before reap: record -> cooperative stop -> child-only kill if live -> wait retry EINTR
332:            -> close handles -> join readers -> remove socket -> orphan check
333:   kill/reader/join error never skips wait; explicit finish must reach Complete
334:   Drop is only nonpanicking emergency kill/wait/close backstop
335: PROOF validates typed transition, cross-callback trace order, digests, receipts,
336:   real linked symbols/locks, child Complete, no orphan; then writes report
```
