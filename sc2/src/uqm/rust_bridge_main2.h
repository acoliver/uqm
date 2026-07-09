/*
 * rust_bridge_main2.h -- Prototypes for Rust-owned main() C bridge
 */

#ifndef RUST_BRIDGE_MAIN2_H
#define RUST_BRIDGE_MAIN2_H

#include "libs/compiler.h"

/* ---- Global option setters ---- */
void uqm_set_snddriver (int val);
void uqm_set_soundflags (int val);
void uqm_set_player_control_template (int idx, int val);
void uqm_set_opt3doMusic (int val);
void uqm_set_optRemixMusic (int val);
void uqm_set_optSpeech (int val);
void uqm_set_optSubtitles (int val);
void uqm_set_optStereoSFX (int val);
void uqm_set_optKeepAspectRatio (int val);
void uqm_set_optWhichCoarseScan (int val);
void uqm_set_optWhichMenu (int val);
void uqm_set_optWhichFonts (int val);
void uqm_set_optWhichIntro (int val);
void uqm_set_optWhichShield (int val);
void uqm_set_optSmoothScroll (int val);
void uqm_set_optMeleeScale (int val);
void uqm_set_optGamma (float val);
void uqm_set_optAddons (const char **addons);
void uqm_set_musicVolumeScale (float val);
void uqm_set_sfxVolumeScale (float val);
void uqm_set_speechVolumeScale (float val);

/* ---- Config loading ---- */
void uqm_init_config_dir (const char *configDirName);
void uqm_load_resource_index (void);

/* ---- Directory preparation ---- */
void uqm_prepare_content_dir (const char *contentDirName,
		const char *addonDirName, const char *execFile);
void uqm_prepare_melee_dir (void);
void uqm_prepare_save_dir (void);
void uqm_prepare_shadow_addons (const char **addons);
void uqm_unprepare_all_dirs (void);

/* ---- Init/teardown wrappers ---- */
void uqm_log_init_threads (void);
void uqm_init_task_system (void);
void uqm_alarm_init (void);
void uqm_callback_init (void);
void uqm_init_color_maps (void);
void uqm_cleanup_task_system (void);
void uqm_callback_uninit (void);
void uqm_alarm_uninit (void);

/* ---- Control template cleanup ---- */
void uqm_remove_old_control_templates (void);

/* ---- Input setup ---- */
void uqm_set_player_controls (int p1, int p2);
void uqm_setup_input_vectors (void);

/* ---- Config options parsing ---- */
void uqm_get_user_config_options (void);

/* ---- Addon cleanup ---- */
void uqm_free_options_addons (void);

#endif /* RUST_BRIDGE_MAIN2_H */
