/* GPL-2.0-or-later. Stable Rust cannot define C variadic functions. */
#include "commglue.h"
#include <stdarg.h>

extern UNICODE *rust_append_response_fragment (UNICODE *, int, const UNICODE *);
extern void rust_finish_response (UNICODE *, UNICODE *);

void
construct_response (UNICODE *buf, int phrase, ...)
{
	UNICODE *start = buf;
	const UNICODE *name;
	va_list args;
	va_start (args, phrase);
	do
	{
		name = va_arg (args, const UNICODE *);
		buf = rust_append_response_fragment (buf, phrase, name);
		if (name)
		{
			phrase = va_arg (args, int);
			if (phrase == (RESPONSE_REF)-1)
				name = NULL;
		}
	} while (name);
	va_end (args);
	rust_finish_response (start, buf);
}
