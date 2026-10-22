/*
 * Variadic log entry points. Stable Rust cannot define C variadics, so
 * these forward their va_list to the Rust implementation in
 * rust/src/log_abi.rs.
 */

#include "uqmlog.h"

void
log_add (log_Level level, const char *fmt, ...)
{
	va_list list;

	va_start (list, fmt);
	log_addV (level, fmt, list);
	va_end (list);
}

void
log_add_nothread (log_Level level, const char *fmt, ...)
{
	va_list list;

	va_start (list, fmt);
	log_add_nothreadV (level, fmt, list);
	va_end (list);
}
