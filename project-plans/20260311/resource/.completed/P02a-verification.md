# Phase P02a: Verification of Pseudocode for PLAN-20260314-RESOURCE

**Plan**: PLAN-20260314-RESOURCE.P02a  
**Executed**: 2026-03-14  
**Status**: COMPLETE

---

## Executive Summary

**VERDICT: ACCEPT**

The pseudocode in `plan/02-pseudocode.md` is **implementable**.

The prior P02 assessment is **too strict**. It correctly spotted a few real issues, but it overstates them as blocking. In particular:

- **A1 is already covered implicitly** in PC-2 line 25.
- **A3 is already covered implicitly** in PC-6 line 157.
- **A5 does not need dedicated pseudocode in Phase P02** if GAP-11 cleanup is a later implementation/cleanup phase; it is a scope note, not a pseudocode blocker.
- Some claimed “logic errors” are really **traceability/documentation issues**, not implementation blockers.

There **are** real caveats:

1. **PC-2 line 45 is ambiguous and should be clarified before coding.**
2. **A2 (_cur_resfile_name restoration invariant) is a real structural concern**, but the current pseudocode is still implementable as-is; an implementer can satisfy the invariant during coding without needing a separate pseudocode block.
3. **A4 (cache invalidation) is a real concern**, but it is not proven here to be a blocking pseudocode omission. It is an implementation note that should be handled when the actual `res_GetString` fix is coded.

So the right pragmatic conclusion is: **implementation can start from this pseudocode**, with a few notes captured below.

---

## Files Reviewed

1. `/Users/acoliver/projects/uqm/project-plans/20260311/resource/.completed/P02.md`
2. `/Users/acoliver/projects/uqm/project-plans/20260311/resource/plan/02-pseudocode.md`
3. `/Users/acoliver/projects/uqm/project-plans/20260311/resource/.completed/P01.md`
4. Authoritative code cross-checks:
   - `/Users/acoliver/projects/uqm/rust/src/resource/dispatch.rs`
   - `/Users/acoliver/projects/uqm/rust/src/resource/ffi_bridge.rs`
   - `/Users/acoliver/projects/uqm/rust/src/resource/mod.rs`

---

## Verification of the 5 Additional Gaps (A1-A5)

## A1: `process_resource_desc` eager-load uses wrong lookup key

**P01 claim:** real gap  
**P02 verdict:** said missing  
**My verification:** **real gap in code, but already covered by pseudocode**

### Evidence

Current code in `dispatch.rs` uses the wrong key:

- `dispatch.rs:97` looks up `type_name`
- unknown fallback stores authoritative handler identity in `handler_key`

Pseudocode PC-2 already fixes that:

```text
25:     actual_handler = type_registry.lookup(handler_key)  // use handler_key, not type_name
```

### Assessment

This is **not missing** from the pseudocode. The P02 assessment acknowledges this later, but still treats A1 as blocking. That is too cautious.

### Conclusion

**A1 is real in the codebase, but it is already handled in the pseudocode. Not a blocker.**

---

## A2: `LoadResourceFromPath` missing guaranteed `_cur_resfile_name` restoration on all exit paths

**P01 claim:** structural gap  
**P02 verdict:** missing and blocking  
**My verification:** **real concern, but not a pseudocode blocker**

### Evidence

Current code is linear:

- `ffi_bridge.rs:1157` sets `_cur_resfile_name`
- `ffi_bridge.rs:1162` clears it

Pseudocode PC-8 is also linear:

```text
216:   set _cur_resfile_name = path
217:   result = load_fun(file, length)
218:   clear _cur_resfile_name
219:   uio_fclose(file)
```

### Assessment

P01 itself is careful here: this is **not a separate user-visible bug today**, but a structural invariant gap. That matters, but it does **not** mean implementation cannot start.

For this plan level, PC-8 already captures the intended lifecycle around the callback. An implementer can reasonably translate that into a scoped guard / RAII cleanup during coding.

A separate PC-8a would improve robustness, but its absence does **not** make Phase P02 non-implementable.

### Conclusion

**A2 is real, but not genuinely blocking for pseudocode sufficiency. Caveat only.**

---

## A3: `SaveResourceIndex` eligibility checked against `res_type` instead of authoritative handler identity

**P01 claim:** real gap  
**P02 verdict:** missing  
**My verification:** **real gap in code, but already covered by pseudocode**

### Evidence

Current code in `ffi_bridge.rs:425` checks:

```rust
state.dispatch.type_registry.lookup(&desc.res_type)
```

Pseudocode PC-6 already changes this to:

```text
157:     handler = type_registry.lookup(entry.type_handler_key)
```

### Assessment

Like A1, this is already present in the pseudocode, even if not called out by name in the heading.

### Conclusion

**A3 is real in code, but already handled by PC-6. Not a blocker.**

---

## A4: `res_GetString` cache invalidation under replacement/removal

**P01 claim:** real concern that should be validated when GAP-2/GAP-6 are fixed  
**P02 verdict:** missing and blocking  
**My verification:** **real concern, but not established as a blocking pseudocode omission**

### Evidence

Current code has a string cache:

- `ffi_bridge.rs:148` defines `string_cache`
- `ffi_bridge.rs:813-822` populates/updates entries lazily by key

Current `res_GetString` behavior is also wrong in a more primary way:

- `ffi_bridge.rs:808-810` returns `desc.fname.clone()` for any entry
- no `STRING` type check
- null on miss/invalid key

Pseudocode PC-4 focuses on the main required behavior:

- return empty string for null/missing/non-STRING/null-data
- return actual string pointer for valid STRING entry

### Assessment

A4 is real as an **adjacent implementation risk**, but P01 itself frames it as something that must be ensured once GAP-2 is corrected. That is a verification/implementation note, not necessarily a required standalone pseudocode component.

Also, whether explicit cache invalidation is needed depends on the final implementation approach:

- If `res_GetString` stops using cached synthesized `CString`s for STRING values and instead returns stable owned data directly, the exact invalidation strategy may change.
- If cache entries are overwritten on access and stale keys are harmlessly ignored after type mismatch, implementation may remain correct without a dedicated pseudocode block.

So A4 should be tracked during implementation, but I do **not** agree that its lack of a dedicated PC-4a makes the pseudocode non-implementable.

### Conclusion

**A4 is real, but it is not proven to require standalone pseudocode before implementation starts. Caveat only.**

---

## A5: `mod.rs` re-exports dead-path APIs

**P01 claim:** real cleanup expansion for GAP-11  
**P02 verdict:** missing and blocking  
**My verification:** **real cleanup item, but not needed as blocking pseudocode in P02**

### Evidence

`mod.rs` contains both dead-path module declarations and re-exports:

```rust
pub mod cache;
pub mod config_api;
...
pub use index::*;
pub use resource_system::*;
```

### Assessment

This is a real cleanup issue. But it belongs to **GAP-11 dead code / public surface cleanup**, which is naturally a later cleanup/removal phase. It does **not** block implementing the runtime behavior changes in PC-1 through PC-9.

The user explicitly asked for pragmatism, including recognizing when an item will naturally be handled in a later phase. This is exactly that case.

### Conclusion

**A5 is real, but it does not need dedicated pseudocode now. Not a blocker.**

---

## Verification of Claimed Logic Errors

## 1. PC-2 line 45 ambiguity

P02 is correct that this line is poorly phrased:

```text
45:   entries.insert(key, FullResourceDesc { fname, res_type: handler_key for UNKNOWNRES else type_name, data, refcount: 0, type_handler_key: handler_key, fname_cstring })
```

### Assessment

This should be clarified because the intended stored values matter:

- `res_type` should preserve the original declared type name
- `type_handler_key` should preserve the authoritative dispatch identity

However, this is **not evidence that the pseudocode is unusable**. The surrounding plan and P01 analysis make the intended model clear enough for an implementer.

### Conclusion

**Real issue, but minor and non-blocking.**

---

## 2. GAP-6 allegedly missing from pseudocode

P02 says GAP-6 is missing as a numbered component, but PC-2 lines 31-43 clearly implement replacement cleanup:

```text
31:   // Replace existing entry if present — call freeFun on old loaded heap resource
32:   IF entries.contains(key) THEN
...
40:         CALL old_handler.free_fun(old_entry.data.ptr)
```

### Assessment

This is a **traceability issue**, not missing logic.

### Conclusion

**Not a real pseudocode gap.**

---

## 3. PC-3 value-type edge case (“both null”)

P02 suggests an extra guard if both `str_ptr` and `num` are null/zero.

### Assessment

That is defensive, but not required for implementability. For value types, the plan assumes data is populated during descriptor processing. Numeric zero is also a valid value, so a generic “both null/zero” check can be misleading.

### Conclusion

**Not a real logic error. Optional defensive refinement only.**

---

## 4. PC-5 missing shutdown warnings for live refcounts

P02 wants extra warning behavior.

### Assessment

Reasonable enhancement, but clearly not a blocker for implementability.

### Conclusion

**Not a blocking logic error.**

---

## 5. PC-8 lacking a dedicated RAII subcomponent

As discussed under A2, linear set/call/clear pseudocode is sufficient at this plan stage.

### Conclusion

**Not a blocking logic error in the pseudocode itself.**

---

## Practical Sufficiency Assessment

If an engineer implemented directly from `02-pseudocode.md`, would they have enough direction to start coding correctly?

**Yes.**

The pseudocode already covers the main behavior changes that matter for implementation start:

- unknown fallback registration and value-type handling
- correct eager-load key for unknown fallback
- general accessor value-type behavior
- `res_GetString` type guard and empty-string behavior
- shutdown cleanup
- save filtering using authoritative handler identity
- directory sentinel handling
- sentinel/zero-length load rejection
- `CountResourceTypes` width fix

The remaining issues are mostly one of these:

1. **wording/traceability cleanup**
2. **implementation robustness detail**
3. **later cleanup phase scope**

None of those justify a full REJECT.

---

## Recommended Notes Before Implementation

These are worth noting, but they do **not** change the ACCEPT verdict.

### Recommended clarification 1
Clarify PC-2 line 45 to explicitly preserve:
- `res_type = type_name`
- `type_handler_key = handler_key`

### Recommended clarification 2
When implementing PC-8, prefer a scoped cleanup pattern so `_cur_resfile_name` is always restored.

### Recommended clarification 3
When implementing PC-4 / replacement logic, review `string_cache` and `type_cache` interactions so stale cached strings are not exposed after replacement/removal.

### Recommended clarification 4
Treat A5 as part of GAP-11 cleanup/removal, not as a blocker for behavior implementation.

---

## Final Verdict

**ACCEPT**

### Rationale

The rustcoder's verdict of **NOT IMPLEMENTABLE** is **not correct overall**.

More precise conclusion:

- **A1:** already covered in PC-2 line 25
- **A2:** real concern, but not blocking at pseudocode level
- **A3:** already covered in PC-6 line 157
- **A4:** real concern, but not proven to require dedicated pseudocode before implementation
- **A5:** real cleanup item, but naturally belongs to later GAP-11 cleanup work

There is one real wording defect in PC-2 line 45, but it is not severe enough to block implementation start.

So the pseudocode is **implementable with caveats**, and Phase P02 should be accepted.

---

## Phase Completion

**Phase ID**: PLAN-20260314-RESOURCE.P02a  
**Timestamp**: 2026-03-14  
**Verdict**: ACCEPT
