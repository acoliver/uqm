/*
 * rust_supermelee.h — Rust SuperMelee FFI declarations
 *
 * When USE_RUST_SUPERMELEE is defined, meleesetup.c redirects
 * serialization and cost functions to Rust implementations.
 */

#ifndef RUST_SUPERMELEE_H
#define RUST_SUPERMELEE_H

#ifdef USE_RUST_SUPERMELEE

#include "types.h"
#include <stdint.h>
#include <stddef.h>

/* Rust FFI exports (from rust/src/supermelee/setup/ffi.rs) */
extern int rust_supermelee_team_serialize(
		const uint8_t *ships, const uint8_t *name,
		uint8_t *out_buf, size_t buf_len);
extern int rust_supermelee_team_deserialize(
		const uint8_t *in_buf, size_t buf_len,
		uint8_t *out_ships, uint8_t *out_name);
extern uint16_t rust_supermelee_ship_cost(uint8_t ship_id);
extern uint16_t rust_supermelee_fleet_value(const uint8_t *ships);
extern size_t rust_supermelee_team_serial_size(void);

#endif /* USE_RUST_SUPERMELEE */

#endif /* RUST_SUPERMELEE_H */
