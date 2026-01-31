/*
 * Rust Graphics Driver FFI Header
 *
 * Rust owns ALL SDL initialization - window, renderer, surfaces.
 * C code gets SDL_Surface pointers for drawing operations.
 * Used when USE_RUST_GFX is defined.
 */

#ifndef RUST_GFX_H
#define RUST_GFX_H

#include "port.h"
#include SDL_INCLUDE(SDL.h)

#ifdef __cplusplus
extern "C" {
#endif

/* Initialization - Rust takes over all SDL graphics */
int rust_gfx_init(int driver, int flags, const char *renderer, int width, int height);
void rust_gfx_uninit(void);

/* Screen access - returns SDL_Surface* that C code can draw to */
SDL_Surface* rust_gfx_get_sdl_screen(void);
SDL_Surface* rust_gfx_get_transition_screen(void);
SDL_Surface* rust_gfx_get_screen_surface(int screen);
SDL_Surface* rust_gfx_get_format_conv_surf(void);

/* TFB_GRAPHICS_BACKEND vtable functions */
void rust_gfx_preprocess(int force_redraw, int transition_amount, int fade_amount);
void rust_gfx_postprocess(void);
void rust_gfx_upload_transition_screen(void);
void rust_gfx_screen(int screen, Uint8 alpha, SDL_Rect *rect);
void rust_gfx_color(Uint8 r, Uint8 g, Uint8 b, Uint8 a, SDL_Rect *rect);

/* Event processing */
int rust_gfx_process_events(void);

/* Gamma correction */
int rust_gfx_set_gamma(float gamma);

/* Fullscreen toggle */
int rust_gfx_toggle_fullscreen(void);
int rust_gfx_is_fullscreen(void);

/* Screen dimensions */
int rust_gfx_get_width(void);
int rust_gfx_get_height(void);

#ifdef __cplusplus
}
#endif

#endif /* RUST_GFX_H */
