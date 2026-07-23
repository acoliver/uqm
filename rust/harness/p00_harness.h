/*
 * P00 Linked Harness — Production Member Extraction Proof
 *
 * This harness proves that the deterministic libuqm_c.a archive contains
 * the required source-grounded production symbols. It links against the
 * real C archive (via Cargo build.rs force-load) and calls/references
 * production symbols to prove they are extractable.
 *
 * The harness mechanism is:
 *   1. build.rs compiles harness sources + shim into a separate archive
 *   2. build.rs emits force-load link arguments for the harness test target
 *   3. The test references production symbols, forcing their extraction
 *
 * @plan PLAN-20260723-RUNTIME-AUTOMATION.P00 §8
 */

#ifndef UQM_P00_HARNESS_H
#define UQM_P00_HARNESS_H

#include <stdint.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

/*
 * Harness entry: reference all required production symbols and verify
 * they are extractable from the linked archive.
 *
 * Returns 0 on success, negative on failure.
 */
int p00_harness_verify_symbols(void);

/*
 * Mutation mode: deliberately bypass one production helper to prove
 * the harness fails. Used for mutation testing.
 *
 *   0 = normal (all symbols present)
 *   1 = bypass DoInput (should fail to link)
 *   2 = bypass DoConfirmExit
 *   3 = bypass TFB_ProcessEvents
 *   4 = bypass ProcessInputEvent
 *   5 = bypass TFB_FlushGraphicsEx
 */
int p00_harness_set_mutation(int mode);
int p00_harness_get_mutation(void);

#ifdef __cplusplus
}
#endif

#endif /* UQM_P00_HARNESS_H */
