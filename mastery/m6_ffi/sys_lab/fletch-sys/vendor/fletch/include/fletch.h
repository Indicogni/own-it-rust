/*
 * fletch - Fletcher checksums.
 *
 * Build: compile src/fletch.c with this directory on the include path.
 * Define FLETCH_WIDE to also build the 32-bit checksum.
 *
 * SPDX-License-Identifier: MIT
 */
#ifndef FLETCH_H
#define FLETCH_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Fletcher-16 of `len` bytes. `data` is not read when `len` is 0. */
uint16_t fletch16(const uint8_t *data, size_t len);

#ifdef FLETCH_WIDE
/*
 * Fletcher-32 of `len` bytes, taken as little-endian 16-bit words. An odd
 * last byte is padded with zero. `data` is not read when `len` is 0.
 * This function exists only in a build with FLETCH_WIDE defined.
 */
uint32_t fletch32(const uint8_t *data, size_t len);
#endif

/* 1 when this build of the library contains fletch32, else 0. */
int fletch_has_wide(void);

#ifdef __cplusplus
}
#endif

#endif /* FLETCH_H */
