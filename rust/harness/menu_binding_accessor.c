/*
 * Menu Binding Accessor — Implementation
 *
 * Uses production res_GetString and VControl_ParseGesture to query
 * the actual resolved binding from loaded resources.
 *
 * @plan PLAN-20260723-RUNTIME-AUTOMATION.P00
 */

#include "menu_binding_accessor.h"

#include "libs/input/sdl/vcontrol.h"
#include "libs/reslib.h"
#include <stdio.h>
#include <string.h>

MenuBindingResult
uqm_query_menu_binding (const char *menu_name)
{
	MenuBindingResult result = { 0, 0, 0, 0 };
	char buf[40];

	if (menu_name == NULL)
		return result;

	buf[39] = '\0';

	int i = 1;
	while (1)
	{
		VCONTROL_GESTURE g;
		snprintf (buf, 39, "menu.%s.%d", menu_name, i);

		/* Check if this resource string exists */
		if (!res_IsString (buf))
			break;

		/* Parse through production VControl_ParseGesture */
		VControl_ParseGesture (&g, res_GetString (buf));

		result.num_alternates++;

		/* Select the first VCONTROL_KEY binding */
		if (!result.found && g.type == VCONTROL_KEY)
		{
			result.found = 1;
			result.key_code = g.gesture.key;
			result.binding_id = i;
		}

		i++;
	}

	return result;
}
