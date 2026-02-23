/*
 *  Rust Resource System header
 *  
 *  Provides extern declarations for the Rust-backed resource cache and
 *  loader.  The C resource system (reslib.h) remains the authority for
 *  InitResourceSystem / UninitResourceSystem; these Rust helpers are
 *  called from within that lifecycle.
 */

#ifndef LIBS_RESOURCE_RUST_RESOURCE_H_
#define LIBS_RESOURCE_RUST_RESOURCE_H_

#include "types.h"
#include <stddef.h>

#ifdef USE_RUST_RESOURCE

/* ---- Rust FFI functions (defined in rust/src/resource/ffi.rs) ---- */

/* Resource System */
extern int rust_init_resource_system(const char* base_path);
extern void rust_uninit_resource_system(void);
extern int rust_load_index(const char* path);
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

/* ---- C wrapper functions (defined in rust_resource.c) ---- */

void RustResourceInit(void);
void RustResourceUninit(void);
void* RustResourceLoad(const char *name, size_t *size);
void RustResourceFree(void *data, size_t size);
BOOLEAN RustResourceExists(const char *name);
void RustResourceCacheClear(void);
size_t RustResourceCacheSize(void);
size_t RustResourceCacheCount(void);

#endif /* USE_RUST_RESOURCE */

#endif /* LIBS_RESOURCE_RUST_RESOURCE_H_ */
