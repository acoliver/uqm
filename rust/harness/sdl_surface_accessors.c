/*
 * SDL Surface ABI-Authoritative Accessors — Implementation
 *
 * Compiled against the same linked SDL2 headers as production code.
 * All field access uses the real SDL_Surface/SDL_PixelFormat types.
 *
 * @plan PLAN-20260723-RUNTIME-AUTOMATION.P00
 */

#include "sdl_surface_accessors.h"

#include <string.h>

/* Test fault injection is thread-local so parallel tests cannot contaminate
 * production-helper calls made by another test thread. */
static _Thread_local int inject_lock_failure = 0;

/* ---- Surface field accessors ---- */

int32_t
uqm_sdl_surface_w (const SDL_Surface *surf)
{
	return surf->w;
}

int32_t
uqm_sdl_surface_h (const SDL_Surface *surf)
{
	return surf->h;
}

int32_t
uqm_sdl_surface_pitch (const SDL_Surface *surf)
{
	return surf->pitch;
}

void *
uqm_sdl_surface_pixels (const SDL_Surface *surf)
{
	return surf->pixels;
}

uint32_t
uqm_sdl_surface_flags (const SDL_Surface *surf)
{
	return surf->flags;
}
const SDL_PixelFormat *
uqm_sdl_surface_format (const SDL_Surface *surf)
{
	return surf->format;
}


/* ---- PixelFormat accessors ---- */

uint8_t
uqm_sdl_format_bpp (const SDL_PixelFormat *fmt)
{
	return fmt->BitsPerPixel;
}

uint8_t
uqm_sdl_format_bytesPerPixel (const SDL_PixelFormat *fmt)
{
	return fmt->BytesPerPixel;
}

uint32_t
uqm_sdl_format_Rmask (const SDL_PixelFormat *fmt)
{
	return fmt->Rmask;
}

uint32_t
uqm_sdl_format_Gmask (const SDL_PixelFormat *fmt)
{
	return fmt->Gmask;
}

uint32_t
uqm_sdl_format_Bmask (const SDL_PixelFormat *fmt)
{
	return fmt->Bmask;
}

uint32_t
uqm_sdl_format_Amask (const SDL_PixelFormat *fmt)
{
	return fmt->Amask;
}

/* ---- SDL_MUSTLOCK wrapper ----
 * SDL_MUSTLOCK is defined as ((surface)->flags & SDL_RLEACCEL) in SDL2.
 * It is NOT a function symbol — we provide one here.
 */
SDL_bool
uqm_sdl_must_lock (const SDL_Surface *surf)
{
	return SDL_MUSTLOCK (surf) ? SDL_TRUE : SDL_FALSE;
}

/* ---- Shared production lock-copy-unlock helper ---- */

int
uqm_sdl_lock_copy_unlock (SDL_Surface *surf, void *dst, size_t len)
{
	if (surf == NULL || dst == NULL || len == 0)
		return -2;

	if (uqm_sdl_is_lock_failure_injected ())
	{
		/* Simulated lock failure — do NOT read pixels */
		return -1;
	}

	if (SDL_LockSurface (surf) != 0)
	{
		/* Real lock failure — do NOT read pixels */
		return -1;
	}

	/* Lock succeeded — copy and always unlock */
	memcpy (dst, surf->pixels, len);
	SDL_UnlockSurface (surf);
	return 0;
}

/* ---- Lock/unlock pair for tests ---- */

int
uqm_sdl_lock (SDL_Surface *surf)
{
	if (surf == NULL)
		return -1;

	if (uqm_sdl_is_lock_failure_injected ())
		return -1;

	if (SDL_LockSurface (surf) != 0)
		return -1;

	return 0;
}

void
uqm_sdl_unlock (SDL_Surface *surf)
{
	if (surf != NULL)
		SDL_UnlockSurface (surf);
}

/* ---- Test support: create MUSTLOCK surface ---- */

SDL_Surface *
uqm_sdl_create_mustlock_surface (int width, int height)
{
	/* SDL2 2.32.x on macOS: SDL_SetSurfaceRLE does not set the SDL_RLEACCEL
	 * flag until the surface is actually RLE-encoded (which happens lazily
	 * during blit). Since SDL_MUSTLOCK is just `flags & SDL_RLEACCEL`,
	 * we enable RLE and then set the flag directly to create a deterministic
	 * surface that satisfies the SDL_MUSTLOCK predicate. The surface is
	 * still valid for lock/unlock — SDL_LockSurface handles the RLE
	 * decompression path when the flag is set. */
	SDL_Surface *surf = SDL_CreateRGBSurface (
			0,
			width, height, 32,
			0xFF000000, 0x00FF0000, 0x0000FF00, 0x000000FF);
	if (surf != NULL)
	{
		SDL_SetSurfaceRLE (surf, 1);
		/* Set the RLEACCEL flag so SDL_MUSTLOCK returns true.
		 * This is the same flag SDL would set after actual RLE encoding. */
		surf->flags |= SDL_RLEACCEL;
	}
	return surf;
}

/* ---- Fault injection ---- */

void
uqm_sdl_inject_lock_failure (int enable)
{
	inject_lock_failure = enable;
}

int
uqm_sdl_is_lock_failure_injected (void)
{
	return inject_lock_failure;
}
