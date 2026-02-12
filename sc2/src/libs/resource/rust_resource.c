/*
 *  Rust Resource System wrapper
 *  
 *  When USE_RUST_RESOURCE is defined, this file provides the resource
 *  loading implementation via the Rust FFI bindings.
 */

#ifdef USE_RUST_RESOURCE

#include <stddef.h>
#include "libs/reslib.h"
#include "libs/memlib.h"
#include "libs/log.h"
#include "rust_resource.h"

/* Forward declarations of Rust FFI functions */
extern int rust_init_resource_system(const char* base_path);
extern int rust_uninit_resource_system(void);
extern int rust_load_index(const char* path);
extern char* rust_get_string_resource(const char* name);
extern void rust_free_string(char* str);
extern int rust_resource_loader_init(const char* base_path, const char* index_path);
extern void rust_resource_loader_uninit(void);
extern uint8* rust_resource_load(const char* name, size_t* out_size);
extern void rust_resource_free(uint8* data, size_t size);
extern int rust_resource_exists(const char* name);
extern int rust_cache_init(size_t max_size);
extern void rust_cache_uninit(void);
extern const uint8* rust_cache_get(const char* key, size_t* out_size);
extern int rust_cache_insert(const char* key, const uint8* data, size_t size);
extern void rust_cache_remove(const char* key);
extern void rust_cache_clear(void);
extern size_t rust_cache_size(void);
extern size_t rust_cache_len(void);

static int resourceSystemInitialized = 0;

BOOLEAN
InitResourceSystem (const char *basePath, const char *indexPath)
{
	if (resourceSystemInitialized)
		return TRUE;

	if (!rust_resource_loader_init(basePath, indexPath))
	{
		log_add(log_Warning, "Failed to initialize Rust resource loader");
		return FALSE;
	}

	/* Initialize cache with 64MB limit */
	if (!rust_cache_init(64 * 1024 * 1024))
	{
		log_add(log_Warning, "Failed to initialize Rust resource cache");
		/* Continue without cache */
	}

	resourceSystemInitialized = 1;
	log_add(log_Debug, "Rust resource system initialized");
	return TRUE;
}

void
UninitResourceSystem (void)
{
	if (!resourceSystemInitialized)
		return;

	rust_cache_uninit();
	rust_resource_loader_uninit();
	resourceSystemInitialized = 0;
}

/* Load a resource by name */
void*
LoadResource (const char *name, size_t *size)
{
	size_t sz = 0;
	uint8 *data;

	if (!name || !resourceSystemInitialized)
		return NULL;

	/* Check cache first */
	data = (uint8*)rust_cache_get(name, &sz);
	if (data)
	{
		if (size) *size = sz;
		return data;
	}

	/* Load from disk */
	data = rust_resource_load(name, &sz);
	if (!data)
	{
		if (size) *size = 0;
		return NULL;
	}

	/* Add to cache */
	rust_cache_insert(name, data, sz);

	if (size) *size = sz;
	return data;
}

void
FreeResource (void *data, size_t size)
{
	if (data)
	{
		rust_resource_free((uint8*)data, size);
	}
}

BOOLEAN
ResourceExists (const char *name)
{
	if (!name || !resourceSystemInitialized)
		return FALSE;

	return rust_resource_exists(name) != 0;
}

/* Get a string resource (caller must free with FreeStringResource) */
char*
GetStringResource (const char *name)
{
	if (!name || !resourceSystemInitialized)
		return NULL;

	return rust_get_string_resource(name);
}

void
FreeStringResource (char *str)
{
	if (str)
	{
		rust_free_string(str);
	}
}

/* Cache management */
void
ClearResourceCache (void)
{
	rust_cache_clear();
}

size_t
GetResourceCacheSize (void)
{
	return rust_cache_size();
}

size_t
GetResourceCacheCount (void)
{
	return rust_cache_len();
}

#endif /* USE_RUST_RESOURCE */
