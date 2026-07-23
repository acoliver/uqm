/*
 * Menu Binding Accessor — Initialized-Child Production Query
 *
 * This is a narrow transitional accessor that reads the same loaded resource
 * as register_menu_controls in input.c, parses each alternate through
 * production VControl_ParseGesture, and returns the first VCONTROL_KEY binding.
 *
 * It must be called from an initialized child (after TFB_InitInput and
 * resource loading). The parent never assumes an SDL key.
 *
 * @plan PLAN-20260723-RUNTIME-AUTOMATION.P00
 */

#ifndef UQM_MENU_BINDING_ACCESSOR_H
#define UQM_MENU_BINDING_ACCESSOR_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/*
 * Result of a menu binding query.
 *
 *   found:        1 if a VCONTROL_KEY binding was found, 0 otherwise
 *   key_code:     The SDL keycode of the binding (valid only if found=1)
 *   binding_id:   Stable identifier for the binding (alternate index)
 *   num_alternates: Total alternates found for this menu control
 */
typedef struct {
	int found;
	int32_t key_code;
	int binding_id;
	int num_alternates;
} MenuBindingResult;

/*
 * Query the menu.<name>.N binding for a specific menu control.
 *
 * Iterates menu.<name>.1, menu.<name>.2, ... exactly as register_menu_controls,
 * parses each with production VControl_ParseGesture, and selects the first
 * VCONTROL_KEY. Returns the result.
 *
 * MUST be called from an initialized child after TFB_InitInput + resource load.
 *
 * @param menu_name  The menu control name (e.g., "down", "up", "select")
 * @return           MenuBindingResult with found/key_code/binding_id
 */
MenuBindingResult uqm_query_menu_binding(const char *menu_name);

#ifdef __cplusplus
}
#endif

#endif /* UQM_MENU_BINDING_ACCESSOR_H */
