/*
 * rust_bridge_main2.c -- C bridge for Rust-owned main() init/teardown
 *
 * @plan PLAN-20260707-BINARY-INVERSION.P02
 *
 * Provides wrapper functions for:
 * - Global variable setters (options parsed in Rust, set in C globals)
 * - Config loading sequence (uses C-specific uio_DirHandle)
 * - Directory preparation that sets C-global state
 */

#include "options.h"
#include "setup.h"
#include "libs/compiler.h"
#include "libs/uio.h"
#include "libs/reslib.h"
#include "libs/graphics/gfx_common.h"
#include "libs/graphics/cmap.h"
#include "libs/sound/sound.h"
#include "libs/input/input_common.h"
#include "libs/inplib.h"
#include "libs/callback.h"
#include "libs/tasklib.h"
#include "libs/time/timecommon.h"
#include "libs/memlib.h"
#include "libs/threadlib.h"
#include "libs/log.h"
#include "libs/callback/async.h"
#include "libs/callback/alarm.h"
#include "controls.h"
#include "init.h"
#include "starcon.h"
#include "port.h"

#include <stdio.h>
#include <string.h>

/* Forward declare globals from uqm.c */
extern int snddriver, soundflags;

/* ---- Global option setters ---- */

void
uqm_set_snddriver (int val) { snddriver = val; }

void
uqm_set_soundflags (int val) { soundflags = val; }

void
uqm_set_player_control_template (int idx, int val) {
	if (idx >= 0 && idx < NUM_PLAYERS)
		PlayerControls[idx] = val;
}

void
uqm_set_opt3doMusic (int val) { opt3doMusic = val; }

void
uqm_set_optRemixMusic (int val) { optRemixMusic = val; }

void
uqm_set_optSpeech (int val) { optSpeech = val; }

void
uqm_set_optSubtitles (int val) { optSubtitles = val; }

void
uqm_set_optStereoSFX (int val) { optStereoSFX = val; }

void
uqm_set_optKeepAspectRatio (int val) { optKeepAspectRatio = val; }

void
uqm_set_optWhichCoarseScan (int val) { optWhichCoarseScan = val; }

void
uqm_set_optWhichMenu (int val) { optWhichMenu = val; }

void
uqm_set_optWhichFonts (int val) { optWhichFonts = val; }

void
uqm_set_optWhichIntro (int val) { optWhichIntro = val; }

void
uqm_set_optWhichShield (int val) { optWhichShield = val; }

void
uqm_set_optSmoothScroll (int val) { optSmoothScroll = val; }

void
uqm_set_optMeleeScale (int val) { optMeleeScale = val; }

void
uqm_set_optGamma (float val) { optGamma = val; }

void
uqm_set_optAddons (const char **addons) { optAddons = addons; }

void
uqm_set_musicVolumeScale (float val) { musicVolumeScale = val; }

void
uqm_set_sfxVolumeScale (float val) { sfxVolumeScale = val; }

void
uqm_set_speechVolumeScale (float val) { speechVolumeScale = val; }

/* ---- Config loading sequence ---- */
/* Does prepareConfigDir + LoadResourceIndex("uqm.cfg") */
void
uqm_init_config_dir (const char *configDirName) {
	prepareConfigDir (configDirName);
}

void
uqm_load_resource_index (void) {
	/* configDir is a global set by prepareConfigDir */
	LoadResourceIndex (configDir, "uqm.cfg", "config.");
}

/* ---- Directory preparation wrappers ---- */

void
uqm_prepare_content_dir (const char *contentDirName,
		const char *addonDirName, const char *execFile) {
	prepareContentDir (contentDirName, addonDirName, execFile);
}

void
uqm_prepare_melee_dir (void) {
	prepareMeleeDir ();
}

void
uqm_prepare_save_dir (void) {
	prepareSaveDir ();
}

void
uqm_prepare_shadow_addons (const char **addons) {
	prepareShadowAddons (addons);
}

void
uqm_unprepare_all_dirs (void) {
	unprepareAllDirs ();
}

/* ---- Init/teardown wrappers that C main() calls directly ---- */

void
uqm_log_init_threads (void) { log_initThreads (); }

void
uqm_init_task_system (void) { InitTaskSystem (); }

void
uqm_alarm_init (void) { Alarm_init (); }

void
uqm_callback_init (void) { Callback_init (); }

void
uqm_init_color_maps (void) { InitColorMaps (); }

void
uqm_cleanup_task_system (void) { CleanupTaskSystem (); }

void
uqm_callback_uninit (void) { Callback_uninit (); }

void
uqm_alarm_uninit (void) { Alarm_uninit (); }

/* ---- res_Remove wrappers (control template cleanup from uqm.c:365-377) ---- */

void
uqm_remove_old_control_templates (void) {
	int i;
	for (i = 0; i < 6; ++i) {
		char cfgkey[64];
		snprintf (cfgkey, sizeof (cfgkey), "config.keys.%d.name", i + 1);
		cfgkey[sizeof (cfgkey) - 1] = '\0';
		res_Remove (cfgkey);
	}
}

/* ---- Input vector setup (wraps ImmediateInputState globals) ---- */

void
uqm_set_player_controls (int p1, int p2) {
	PlayerControls[0] = p1;
	PlayerControls[1] = p2;
}

void
uqm_setup_input_vectors (void) {
	TFB_SetInputVectors (ImmediateInputState.menu, NUM_MENU_KEYS,
			(volatile int *)ImmediateInputState.key, NUM_TEMPLATES, NUM_KEYS);
}

/* ---- Config options parsing (wraps function in uqm.c) ---- */
/* getUserConfigOptions is called inside uqm_c_do_init(). This wrapper is
 * a no-op stub kept for ABI compatibility but the actual parsing happens
 * in uqm_c_do_init() which owns the options struct. */

/* ---- Addon cleanup (uqm.c:842 in original main) ---- */
/* optAddons is the global set from options.addons in uqm_c_do_init().
 * Freeing optAddons frees the same allocation. */
void
uqm_free_options_addons (void) {
	if (optAddons)
		HFree (optAddons);
}
