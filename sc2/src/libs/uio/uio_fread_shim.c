#include <stdio.h>
#include "port.h"
#include "libs/uio.h"
#include "rust_bridge.h"

size_t rust_uio_fread(void *buf, size_t size, size_t nmemb, uio_Stream *stream);

size_t uio_fread(void *buf, size_t size, size_t nmemb, uio_Stream *stream)
{
    char logbuf[256];
    snprintf(logbuf, sizeof(logbuf), "C_SHIM: uio_fread buf=%p size=%zu nmemb=%zu stream=%p", buf, size, nmemb, (void *)stream);
    rust_bridge_log(logbuf);
    return rust_uio_fread(buf, size, nmemb, stream);
}
