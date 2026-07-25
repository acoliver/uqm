/*
 *  Rust Communication System wrapper
 *
 *  All communication logic has been ported to Rust. This file now contains
 *  ONLY the init/uninit forwarders that C code (setup.c) calls during
 *  game startup and shutdown. All rendering and conversation logic lives
 *  in rust/src/comm/sis_graphics.rs.
 *
 *  @plan PLAN-20260314-COMM.P05b
 */

#define COMM_INTERNAL
#include "comm.h"

#ifdef USE_RUST_COMM
#include "rust_comm.h"

/* Initialize communication system using Rust implementation.
 * Called from C code (setup.c) — must remain here as a C entry point. */
void
init_communication (void)
{
	rust_InitCommunication ();
}

/* Uninitialize communication system using Rust implementation. */
void
uninit_communication (void)
{
	rust_UninitCommunication ();
}

#endif /* USE_RUST_COMM */
