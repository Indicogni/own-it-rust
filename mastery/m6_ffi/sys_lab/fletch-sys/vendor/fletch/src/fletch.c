/*
 * fletch - Fletcher checksums. See include/fletch.h.
 *
 * SPDX-License-Identifier: MIT
 */
#include "fletch.h"

uint16_t fletch16(const uint8_t *data, size_t len) {
    uint32_t a = 0;
    uint32_t b = 0;
    size_t i;
    for (i = 0; i < len; i++) {
        a = (a + data[i]) % 255;
        b = (b + a) % 255;
    }
    return (uint16_t)((b << 8) | a);
}

#ifdef FLETCH_WIDE
uint32_t fletch32(const uint8_t *data, size_t len) {
    uint32_t a = 0;
    uint32_t b = 0;
    size_t i;
    for (i = 0; i < len; i += 2) {
        uint32_t word = data[i];
        if (i + 1 < len) {
            word |= (uint32_t)data[i + 1] << 8;
        }
        a = (a + word) % 65535;
        b = (b + a) % 65535;
    }
    return (b << 16) | a;
}
#endif

int fletch_has_wide(void) {
#ifdef FLETCH_WIDE
    return 1;
#else
    return 0;
#endif
}
