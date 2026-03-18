/*
 * Internal helper for uio_vfprintf formatting support.
 * This is NOT an exported UIO ABI symbol - it is compiled into the Rust
 * static library and called only from the Rust uio_vfprintf implementation.
 *
 * @plan PLAN-20260314-FILE-IO.P04
 * @requirement REQ-FIO-STREAM-WRITE
 */

#include <stdarg.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

// Forward declarations for Rust-implemented functions
typedef struct uio_Stream uio_Stream;
extern int uio_vfprintf(uio_Stream *stream, const char *format, va_list args);

/*
 * Format a va_list into a newly allocated buffer.
 * Returns NULL on error (and errno may be set).
 * Caller must free the returned buffer using free().
 *
 * This is an internal-only helper, not an exported uio_* symbol.
 */
char *uio_vfprintf_format_helper(const char *format, va_list args) {
    char *buf;
    size_t bufSize = 128;
    
    if (format == NULL) {
        return NULL;
    }

    buf = malloc(bufSize);
    if (buf == NULL) {
        return NULL;
    }

    for (;;) {
        va_list args_copy;
        va_copy(args_copy, args);
        int printResult = vsnprintf(buf, bufSize, format, args_copy);
        va_end(args_copy);
        
        if (printResult < 0) {
            // Buffer not large enough, no size hint available
            bufSize *= 2;
        } else if ((unsigned int)printResult >= bufSize) {
            // Buffer too small, printResult has the required size
            bufSize = printResult + 1;
        } else {
            // Success
            return buf;
        }

        char *newBuf = realloc(buf, bufSize);
        if (newBuf == NULL) {
            free(buf);
            return NULL;
        }
        buf = newBuf;
    }
}

/*
 * Variadic wrapper for uio_vfprintf.
 * This IS an exported UIO ABI symbol, but implemented in C to handle variadic args.
 *
 * @plan PLAN-20260314-FILE-IO.P04
 * @requirement REQ-FIO-STREAM-WRITE
 */
int uio_fprintf(uio_Stream *stream, const char *format, ...) {
    va_list args;
    int result;
    
    va_start(args, format);
    result = uio_vfprintf(stream, format, args);
    va_end(args);
    
    return result;
}
