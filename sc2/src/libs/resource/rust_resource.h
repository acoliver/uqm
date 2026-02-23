/*
 *  Rust Resource System header
 *  
 *  Provides extern declarations for the Rust-implemented resource system.
 *  When USE_RUST_RESOURCE is defined, this system is used instead of
 *  the C resource implementation.
 */

#ifndef LIBS_RESOURCE_RUST_RESOURCE_H_
#define LIBS_RESOURCE_RUST_RESOURCE_H_

#include "types.h"
#include <stddef.h>

#ifdef USE_RUST_RESOURCE

/*
 * Rust Resource System FFI functions
 * Defined in rust/src/resource/ffi.rs and exported via staticlib
 */

/* Resource System Lifecycle */
extern int rust_init_resource_system(const char* base_path);
extern void rust_uninit_resource_system(void);
extern int rust_load_index(const char* path);

/* String Resources */
extern char* rust_get_string_resource(const char* name);
extern void rust_free_string(char* str);

/* Resource Loader */
extern int rust_resource_loader_init(const char* base_path, const char* index_path);
extern void rust_resource_loader_uninit(void);
extern uint8* rust_resource_load(const char* name, size_t* out_size);
extern void rust_resource_free(uint8* data, size_t size);
extern int rust_resource_exists(const char* name);

/* Resource Cache */
extern int rust_cache_init(size_t max_size);
extern void rust_cache_clear(void);
extern const uint8* rust_cache_get(const char* key, size_t* out_size);
extern void rust_cache_insert(const char* key, const uint8* data, size_t size);
extern size_t rust_cache_size(void);
extern size_t rust_cache_len(void);

#endif /* USE_RUST_RESOURCE */

#endif /* LIBS_RESOURCE_RUST_RESOURCE_H_ */
