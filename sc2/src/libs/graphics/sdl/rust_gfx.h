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

/* ---- Canvas FFI Bridge (P15) ---- */

/* Opaque handle for Rust canvas operations on an SDL_Surface */
typedef struct SurfaceCanvas SurfaceCanvas;

/* Lifecycle */
SurfaceCanvas* rust_canvas_from_surface(SDL_Surface *surface);
void rust_canvas_destroy(SurfaceCanvas *canvas);

/* Drawing operations (stubs — returns 0 success, -1 error) */
int rust_canvas_draw_line(SurfaceCanvas *canvas, int x1, int y1, int x2, int y2, Uint32 color);
int rust_canvas_draw_rect(SurfaceCanvas *canvas, int x, int y, int w, int h, Uint32 color);
int rust_canvas_fill_rect(SurfaceCanvas *canvas, int x, int y, int w, int h, Uint32 color);
int rust_canvas_copy(SurfaceCanvas *dst, const SurfaceCanvas *src, const SDL_Rect *src_rect,
                     int dst_x, int dst_y);
int rust_canvas_draw_image(SurfaceCanvas *canvas, const Uint8 *image_data,
                           int image_w, int image_h, int x, int y);
int rust_canvas_draw_fontchar(SurfaceCanvas *canvas, const Uint8 *glyph_data,
                              int glyph_w, int glyph_h, int x, int y, Uint32 color);

/* Scissor (clipping) */
int rust_canvas_set_scissor(SurfaceCanvas *canvas, int x, int y, int w, int h);
int rust_canvas_clear_scissor(SurfaceCanvas *canvas);

/* Query */
int rust_canvas_get_extent(SurfaceCanvas *canvas, int *w, int *h);

/* ---- DCQ FFI Bridge (P18-P20) ---- */

/* Lifecycle */
int rust_dcq_init(void);
void rust_dcq_uninit(void);

/* Push draw commands */
int rust_dcq_push_drawline(int x1, int y1, int x2, int y2, Uint32 color);
int rust_dcq_push_drawrect(int x, int y, int w, int h, Uint32 color);
int rust_dcq_push_fillrect(int x, int y, int w, int h, Uint32 color);
int rust_dcq_push_drawimage(Uint32 image_id, int x, int y);
int rust_dcq_push_copy(const SDL_Rect *src_rect, int src_screen, int dst_x, int dst_y);
int rust_dcq_push_copytoimage(Uint32 image_id, const SDL_Rect *src_rect);
int rust_dcq_push_deleteimage(Uint32 image_id);
int rust_dcq_push_waitsignal(void);
int rust_dcq_push_reinitvideo(int driver, int flags, int width, int height);
int rust_dcq_push_setpalette(Uint32 colormap_id);
int rust_dcq_push_scissor_enable(int x, int y, int w, int h);
int rust_dcq_push_scissor_disable(void);

/* Flush / batch / screen */
int rust_dcq_flush(void);
int rust_dcq_batch(void);
int rust_dcq_unbatch(void);
int rust_dcq_set_screen(int index);
int rust_dcq_get_screen(void);
int rust_dcq_len(void);

#ifdef __cplusplus
}
#endif

#endif /* RUST_GFX_H */
