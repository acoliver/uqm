/*
 * Test-local C shim for main loop FFI boundary tests.
 *
 * This file defines TEST-LOCAL globals and accessor functions that
 * mimic the shape of the real UQM accessors, WITHOUT linking against
 * actual UQM globals. This allows Tier 2 boundary tests to run in
 * `cargo test` without the full UQM C codebase.
 *
 * The shim provides:
 *   - test_set_activity(u16) / test_get_activity() -> u16
 *
 * These are the ONLY symbols the Rust test externs link against.
 *
 * @plan PLAN-20260707-MAINLOOP.P03
 */

#include <stdint.h>

/* Test-local activity global. Initialized to zero. */
static uint16_t test_current_activity = 0;

void
test_set_activity (uint16_t val)
{
	test_current_activity = val;
}

uint16_t
test_get_activity (void)
{
	return test_current_activity;
}
