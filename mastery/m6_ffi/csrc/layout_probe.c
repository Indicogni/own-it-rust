/*
 * Layout probe: questions about kvtab.h that only the C compiler can answer.
 *
 * Rust cannot read a C header. This file is compiled by build.rs with the
 * same C compiler and the same flags as kvtab.c, and the tests of lesson 03
 * compare its answers with what your Rust declarations say.
 *
 * SPDX-License-Identifier: MIT
 */
#include <stddef.h>
#include <string.h>

#include "kvtab.h"

/* The offset of `s` is the alignment of kvtab_stats (works in C89 too). */
struct m6_align_probe {
    char c;
    kvtab_stats s;
};

size_t m6_probe_stats_size(void) {
    return sizeof(kvtab_stats);
}

size_t m6_probe_stats_align(void) {
    return offsetof(struct m6_align_probe, s);
}

/* The offset of a field of kvtab_stats, or (size_t)-1 for an unknown name. */
size_t m6_probe_stats_offset(const char *field) {
    if (strcmp(field, "len") == 0) return offsetof(kvtab_stats, len);
    if (strcmp(field, "capacity") == 0) return offsetof(kvtab_stats, capacity);
    if (strcmp(field, "value_bytes") == 0) return offsetof(kvtab_stats, value_bytes);
    if (strcmp(field, "puts") == 0) return offsetof(kvtab_stats, puts);
    if (strcmp(field, "full") == 0) return offsetof(kvtab_stats, full);
    return (size_t)-1;
}

/* The value of an enumerator of kvtab.h, or -1 for an unknown name. */
int m6_probe_constant(const char *name) {
    if (strcmp(name, "KVTAB_OK") == 0) return KVTAB_OK;
    if (strcmp(name, "KVTAB_ERR_NULL") == 0) return KVTAB_ERR_NULL;
    if (strcmp(name, "KVTAB_ERR_NOT_FOUND") == 0) return KVTAB_ERR_NOT_FOUND;
    if (strcmp(name, "KVTAB_ERR_FULL") == 0) return KVTAB_ERR_FULL;
    if (strcmp(name, "KVTAB_ERR_TOO_SMALL") == 0) return KVTAB_ERR_TOO_SMALL;
    if (strcmp(name, "KVTAB_ERR_NOMEM") == 0) return KVTAB_ERR_NOMEM;
    if (strcmp(name, "KVTAB_ERR_STOPPED") == 0) return KVTAB_ERR_STOPPED;
    if (strcmp(name, "KVTAB_EVENT_PUT") == 0) return KVTAB_EVENT_PUT;
    if (strcmp(name, "KVTAB_EVENT_REMOVE") == 0) return KVTAB_EVENT_REMOVE;
    return -1;
}
