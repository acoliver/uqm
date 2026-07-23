/*
 * P00 Linked Harness — Production Member Extraction Proof (Implementation)
 *
 * This file references production symbols from the C archive to prove
 * they are extractable. The symbols are declared extern and their addresses
 * are stored in volatile globals to prevent the linker from optimizing
 * away the references.
 *
 * @plan PLAN-20260723-RUNTIME-AUTOMATION.P00 §8
 */

#include "p00_harness.h"

/*
 * Forward declarations of production symbols.
 * These match the actual C definitions and force the linker to extract
 * the corresponding .o members from libuqm_c.a.
 *
 * We use compatible-but-minimal signatures so the harness compiles
 * independently. The linker only needs the symbol name to resolve.
 */

/* gameinp_rust_main.o — DoInput takes (void*, BOOLEAN) */
extern void DoInput(void *pInputState, int resetInput);
extern int AnyButtonPress(void);

/* confirm.c.o */
extern int DoConfirmExit(void);

/* sdl_common.c.o */
extern void TFB_ProcessEvents(void);
extern void TFB_SwapBuffers(int force_full_redraw);

/* input.c.o — ProcessInputEvent takes (const SDL_Event*) */
extern void ProcessInputEvent(const void *Event);

/* dcqueue.c.o — TFB_FlushGraphicsEx takes BOOLEAN skip_swap */
extern void TFB_FlushGraphicsEx(int skip_swap);

static int mutation_mode = 0;

int
p00_harness_set_mutation (int mode)
{
	int prev = mutation_mode;
	mutation_mode = mode;
	return prev;
}

int
p00_harness_get_mutation (void)
{
	return mutation_mode;
}

/*
 * Store function addresses in volatile globals to force extraction.
 * The linker cannot discard these references.
 * We use void* to avoid volatile function-pointer type issues.
 */
static volatile void *g_symbol_refs[8];
static volatile int g_symbol_count = 0;

int
p00_harness_verify_symbols (void)
{
	int idx = 0;

	if (mutation_mode != 1)
	{
		g_symbol_refs[idx++] = (void *) (uintptr_t) &DoInput;
		g_symbol_refs[idx++] = (void *) (uintptr_t) &AnyButtonPress;
	}

	if (mutation_mode != 2)
	{
		g_symbol_refs[idx++] = (void *) (uintptr_t) &DoConfirmExit;
	}

	if (mutation_mode != 3)
	{
		g_symbol_refs[idx++] = (void *) (uintptr_t) &TFB_ProcessEvents;
		g_symbol_refs[idx++] = (void *) (uintptr_t) &TFB_SwapBuffers;
	}

	if (mutation_mode != 4)
	{
		g_symbol_refs[idx++] = (void *) (uintptr_t) &ProcessInputEvent;
	}

	if (mutation_mode != 5)
	{
		g_symbol_refs[idx++] = (void *) (uintptr_t) &TFB_FlushGraphicsEx;
	}

	g_symbol_count = idx;

	/* Touch all stored function pointers */
	volatile int count = g_symbol_count;
	for (int i = 0; i < count; i++)
	{
		if (g_symbol_refs[i] == (void *) 0)
			return -1;
	}

	return idx;
}
