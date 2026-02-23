/*
 *  Rust Resource Cache wrapper
 *
 *  When USE_RUST_RESOURCE is defined, this file initializes the Rust-backed
 *  resource cache.  The existing C resource system (resinit.c) remains the
 *  authority for InitResourceSystem / UninitResourceSystem; this file only
 *  adds Rust cache helpers that other C code can call.
 */

#ifdef USE_RUST_RESOURCE

#include <stddef.h>
#include "libs/reslib.h"
#include "libs/memlib.h"
#include "libs/log.h"
#include "rust_resource.h"

static int rustCacheInitialized = 0;

void
RustResourceCacheInit (void)
{
	if (rustCacheInitialized)
		return;

	if (!rust_cache_init(64 * 1024 * 1024))
	{
		log_add(log_Warning, "Failed to initialize Rust resource cache");
		return;
	}

	rustCacheInitialized = 1;
	log_add(log_Debug, "Rust resource cache initialized (64 MB)");
}

void
RustResourceCacheUninit (void)
{
	if (!rustCacheInitialized)
		return;

	rust_cache_clear();
	rustCacheInitialized = 0;
}

void
RustResourceCacheClear (void)
{
	if (rustCacheInitialized)
		rust_cache_clear();
}

size_t
RustResourceCacheSize (void)
{
	return rustCacheInitialized ? rust_cache_size() : 0;
}

size_t
RustResourceCacheCount (void)
{
	return rustCacheInitialized ? rust_cache_len() : 0;
}

#endif /* USE_RUST_RESOURCE */
