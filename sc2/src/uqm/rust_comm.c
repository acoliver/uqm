/*
 *  Rust Communication System wrapper
 *  
 *  Wraps Rust-implemented communication functions when USE_RUST_COMM is defined.
 *  This file provides C-callable wrapper functions that delegate to the Rust
 *  implementation via the FFI bindings declared in rust_comm.h.
 */

#define COMM_INTERNAL
#include "comm.h"

#ifdef USE_RUST_COMM
#include "rust_comm.h"

/* Initialize communication system using Rust implementation */
void
init_communication (void)
{
	rust_InitCommunication();
}

/* Uninitialize communication system using Rust implementation */
void
uninit_communication (void)
{
	rust_UninitCommunication();
}

#endif /* USE_RUST_COMM */
