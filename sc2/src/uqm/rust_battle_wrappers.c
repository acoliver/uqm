/*
 * rust_battle_wrappers.c — C wrapper functions for Rust battle loop
 *
 * @plan PLAN-20260320-BATTLEPT2.P13
 * @requirement REQ-SYMBOL-ABI, REQ-BUILD-COEXISTENCE
 *
 * When USE_RUST_BATTLE_LOOP is defined, these wrappers preserve the
 * original C symbol names while delegating to Rust FFI exports.
 * This file is only compiled when USE_RUST_BATTLE_LOOP is enabled.
 */

#ifdef USE_RUST_BATTLE_LOOP

#include "battle.h"
#include "init.h"
#include "intel.h"
#include "races.h"

/* Rust FFI exports */
extern int32_t rust_battle_entry(void);
extern int32_t rust_battle_frame(void);
extern int32_t rust_battle_init_ships(void);
extern void rust_battle_uninit_ships(void);
extern void rust_battle_init_space(void);
extern void rust_battle_uninit_space(void);
extern uint32_t rust_computer_intelligence(ELEMENT *ShipPtr, void *evaluate);
extern void rust_battle_song(int32_t do_play);
extern void rust_free_battle_song(void);
extern uint8_t rust_get_player_order(void);

/*
 * Battle() — top-level battle entry point
 * Original: battle.c:396-516
 * External callers: encount.c, melee.c
 */
BOOLEAN
Battle (BATTLE_STATE *bs)
{
	(void)bs; /* Rust manages its own state */
	return (BOOLEAN)(rust_battle_entry() != 0);
}

/*
 * InitShips() — initialize battle arena and ships
 * Original: init.c:182-250
 * External callers: battle.c (Battle)
 */
COUNT
InitShips (void)
{
	return (COUNT)rust_battle_init_ships();
}

/*
 * UninitShips() — tear down battle arena
 * Original: init.c:277-361
 * External callers: battle.c (Battle)
 */
void
UninitShips (void)
{
	rust_battle_uninit_ships();
}

/*
 * InitSpace() — load shared battle assets (ref-counted)
 * Original: init.c:117-148
 */
BOOLEAN
InitSpace (void)
{
	rust_battle_init_space();
	return TRUE;
}

/*
 * UninitSpace() — release shared battle assets (ref-counted)
 * Original: init.c:150-162
 */
void
UninitSpace (void)
{
	rust_battle_uninit_space();
}

/*
 * computer_intelligence() — AI dispatch
 * Original: intel.c (full file)
 * Note: This replaces the C AI with Rust dispatch.
 * Race-specific intelligence still callbacks to C race modules.
 */
void
computer_intelligence (ELEMENT *ShipPtr, EVALUATE_DESC *evaluate)
{
	rust_computer_intelligence(ShipPtr, (void *)evaluate);
}

/*
 * BattleSong() — load/play battle music
 * Original: battle.c:234-249
 */
void
BattleSong (BOOLEAN DoPlay)
{
	rust_battle_song((int32_t)DoPlay);
}

/*
 * FreeBattleSong() — release battle music resources
 * Original: battle.c:251-256
 */
void
FreeBattleSong (void)
{
	rust_free_battle_song();
}

/*
 * GetPlayerOrder() — determine input processing order
 * Original: battle.c:357-372
 */
BYTE
GetPlayerOrder (COUNT num_ships)
{
	(void)num_ships;
	return (BYTE)rust_get_player_order();
}

#endif /* USE_RUST_BATTLE_LOOP */
