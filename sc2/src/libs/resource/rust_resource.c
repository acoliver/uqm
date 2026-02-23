/*
 *  Rust Resource System wrapper
 *  
 *  When USE_RUST_RESOURCE is defined, this file provides a Rust-backed
 *  resource cache layer that integrates with the existing C resource system.
 *  The C resource system (resinit.c) remains the authority for
 *  InitResourceSystem / UninitResourceSystem; we hook into its lifecycle
 *  to also bring up the Rust cache and loader.
 */

#ifdef USE_RUST_RESOURCE

#include <stddef.h>
#include "libs/reslib.h"
#include "libs/memlib.h"
#include "libs/log.h"
#include "rust_resource.h"

static int rustResourceInitialized = 0;

/*
 * Initialize the Rust resource cache.
 * Called after the C resource system is already up.
 */
void
RustResourceInit (void)
{
	if (rustResourceInitialized)
		return;

	/* Initialize cache with 64MB limit */
	if (!rust_cache_init(64 * 1024 * 1024))
	{
		log_add(log_Warning, "Failed to initialize Rust resource cache");
		/* Continue without cache — not fatal */
	}

	rustResourceInitialized = 1;
	log_add(log_Debug, "Rust resource cache initialized (64 MB)");
}

void
RustResourceUninit (void)
{
	if (!rustResourceInitialized)
		return;

	rust_cache_clear();
	rustResourceInitialized = 0;
}

/*
 * Cache-aware resource loading.
 * Falls through to the Rust loader if available, otherwise returns NULL
 * and lets the C resource system handle it.
 */
void*
RustResourceLoad (const char *name, size_t *size)
{
	size_t sz = 0;
	uint8 *data;

	if (!name || !rustResourceInitialized)
		return NULL;

	/* Check cache first */
	data = (uint8*)rust_cache_get(name, &sz);
	if (data)
	{
		if (size) *size = sz;
		return data;
	}

	/* Load from Rust loader */
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
RustResourceFree (void *data, size_t size)
{
	if (data)
	{
		rust_resource_free((uint8*)data, size);
	}
}

BOOLEAN
RustResourceExists (const char *name)
{
	if (!name || !rustResourceInitialized)
		return FALSE;

	return rust_resource_exists(name) != 0;
}

/* Cache management */
void
RustResourceCacheClear (void)
{
	if (rustResourceInitialized)
		rust_cache_clear();
}

size_t
RustResourceCacheSize (void)
{
	return rustResourceInitialized ? rust_cache_size() : 0;
}

size_t
RustResourceCacheCount (void)
{
	return rustResourceInitialized ? rust_cache_len() : 0;
}

#endif /* USE_RUST_RESOURCE */
