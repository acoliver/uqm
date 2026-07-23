/*
 * SDL Surface ABI-Authoritative Accessors
 *
 * These accessors are compiled against the SAME linked SDL2 headers as the
 * production C code. They provide safe, ABI-authoritative access to
 * SDL_Surface fields (width, height, pitch, pixels, format, BPP, masks)
 * and the SDL_MUSTLOCK macro, which is NOT a function symbol.
 *
 * The lock-copy-unlock helper (rust_sdl_lock_copy_unlock) is the single
 * shared production helper used by both real capture code and tests.
 * It performs: SDL_LockSurface -> memcpy -> SDL_UnlockSurface, with the
 * unlock in all paths (including failure).
 *
 * @plan PLAN-20260723-RUNTIME-AUTOMATION.P00
 */

#ifndef UQM_SDL_SURFACE_ACCESSORS_H
#define UQM_SDL_SURFACE_ACCESSORS_H

#include <stdint.h>
#include <stddef.h>

/* Include the real SDL2 headers — same ABI as production code */
#include <SDL.h>

#ifdef __cplusplus
extern "C" {
#endif

/* ---- Surface field accessors (ABI-authoritative) ---- */

int32_t uqm_sdl_surface_w(const SDL_Surface *surf);
int32_t uqm_sdl_surface_h(const SDL_Surface *surf);
int32_t uqm_sdl_surface_pitch(const SDL_Surface *surf);
void *uqm_sdl_surface_pixels(const SDL_Surface *surf);
uint32_t uqm_sdl_surface_flags(const SDL_Surface *surf);

const SDL_PixelFormat *uqm_sdl_surface_format(const SDL_Surface *surf);
/* SDL_PixelFormat accessors (do NOT use hand-written partial struct) */
uint8_t uqm_sdl_format_bpp(const SDL_PixelFormat *fmt);
uint8_t uqm_sdl_format_bytesPerPixel(const SDL_PixelFormat *fmt);
uint32_t uqm_sdl_format_Rmask(const SDL_PixelFormat *fmt);
uint32_t uqm_sdl_format_Gmask(const SDL_PixelFormat *fmt);
uint32_t uqm_sdl_format_Bmask(const SDL_PixelFormat *fmt);
uint32_t uqm_sdl_format_Amask(const SDL_PixelFormat *fmt);

/* SDL_MUSTLOCK is a macro, not a function — this wraps it */
SDL_bool uqm_sdl_must_lock(const SDL_Surface *surf);

/* ---- Shared production lock-copy-unlock helper ----
 *
 * Locks the surface, copies 'len' bytes from its pixel buffer into 'dst',
 * then unlocks. Returns 0 on success, -1 on lock failure, -2 on null/invalid.
 *
 * This is the ONE shared helper. Both production capture code and linked
 * tests MUST call this — no duplicated lock/copy logic.
 */
int uqm_sdl_lock_copy_unlock(SDL_Surface *surf, void *dst, size_t len);

/* ---- Shared production lock helper (for tests that need lock/no-read) ----
 *
 * Locks the surface and returns 0 on success, -1 on failure.
 * Always pairs with uqm_sdl_unlock. Used for injected lock-failure tests
 * where we need to verify that the helper returns error and does not read.
 */
int uqm_sdl_lock(SDL_Surface *surf);
void uqm_sdl_unlock(SDL_Surface *surf);

/* ---- Test support: create a surface that MUSTLOCK ----
 *
 * Creates a real SDL_Surface via SDL_CreateRGBSurfaceWithFormat with flags
 * that set SDL_MUSTLOCK. Used to prove the real MUSTLOCK predicate.
 * Returns NULL on failure.
 */
SDL_Surface *uqm_sdl_create_mustlock_surface(int width, int height);

/* ---- Test support: inject lock failure ----
 *
 * Sets a process-local flag that causes uqm_sdl_lock_copy_unlock to
 * simulate SDL_LockSurface failure (returns -1 without calling the real
 * lock). This is for fault-injection testing only.
 */
void uqm_sdl_inject_lock_failure(int enable);
int uqm_sdl_is_lock_failure_injected(void);

#ifdef __cplusplus
}
#endif

#endif /* UQM_SDL_SURFACE_ACCESSORS_H */
