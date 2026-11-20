/*
 * kvtab - a tiny key-value table with a fixed capacity.
 *
 * Keys are NUL-terminated strings. Values are arbitrary bytes. The table
 * keeps its entries in insertion order and copies every key and value that
 * you give it.
 *
 * Threads: no function uses global state, and a table has no lock of its own.
 * Two threads must never be inside calls on the same table at the same time,
 * whatever the functions are. Calls on one table may come from different
 * threads as long as you make sure that they happen one after another.
 *
 * SPDX-License-Identifier: MIT
 */
#ifndef KVTAB_H
#define KVTAB_H

#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

/* An opaque table. Create one with kvtab_new and release it with kvtab_free. */
typedef struct kvtab kvtab;

/* Status codes. Functions that return `int` return one of these. */
enum kvtab_status {
    KVTAB_OK = 0,
    KVTAB_ERR_NULL = 1,      /* a required pointer argument was NULL */
    KVTAB_ERR_NOT_FOUND = 2, /* the key is not in the table */
    KVTAB_ERR_FULL = 3,      /* a new key does not fit: len == capacity */
    KVTAB_ERR_TOO_SMALL = 4, /* the caller's buffer is too small */
    KVTAB_ERR_NOMEM = 5,     /* malloc failed */
    KVTAB_ERR_STOPPED = 6    /* a visitor asked kvtab_for_each to stop */
};

/* Events reported to an observer. */
enum kvtab_event {
    KVTAB_EVENT_PUT = 1,
    KVTAB_EVENT_REMOVE = 2
};

/* A snapshot of a table's counters, filled in by kvtab_get_stats. */
typedef struct kvtab_stats {
    size_t len;         /* number of entries */
    size_t capacity;    /* maximum number of entries */
    size_t value_bytes; /* total size of all stored values */
    unsigned int puts;  /* successful kvtab_put calls since creation */
    unsigned char full; /* 1 when len == capacity, else 0 */
} kvtab_stats;

/*
 * Called by kvtab_for_each for one entry. `key` and `value` point into the
 * table and are valid only until the visitor returns. Return 0 to continue
 * and any other value to stop. The visitor may call the functions that take
 * a `const kvtab *` on the same table. It must not change the table: no
 * kvtab_put, kvtab_remove, kvtab_set_observer or kvtab_free.
 */
typedef int (*kvtab_visit_fn)(const char *key, const unsigned char *value,
                              size_t value_len, void *userdata);

/*
 * Called after each successful kvtab_put (KVTAB_EVENT_PUT) and kvtab_remove
 * (KVTAB_EVENT_REMOVE). `key` is valid only until the observer returns. The
 * observer must not call any function on the same table.
 */
typedef void (*kvtab_observe_fn)(int event, const char *key, void *userdata);

/* Releases the userdata of an observer. Called exactly once per userdata. */
typedef void (*kvtab_drop_fn)(void *userdata);

/* The library version, for example "kvtab 1.0.0". Static: never free it. */
const char *kvtab_version(void);

/* An English description of a status code. Static: never free it. */
const char *kvtab_strerror(int status);

/*
 * Creates an empty table for at most `capacity` entries. Returns NULL when
 * `capacity` is 0 or when memory is not available.
 */
kvtab *kvtab_new(size_t capacity);

/*
 * Destroys a table and everything that it owns. If an observer is installed
 * and has a drop function, calls drop(userdata) once. `t` may be NULL.
 * After this call `t` and every pointer obtained from it are invalid.
 */
void kvtab_free(kvtab *t);

/*
 * Stores a copy of `key` and of the `value_len` bytes at `value`. An existing
 * key keeps its position and gets the new value. `value` is not read when
 * `value_len` is 0.
 * Errors: KVTAB_ERR_NULL (`t` or `key` is NULL, or `value` is NULL while
 * `value_len` is not 0), KVTAB_ERR_FULL, KVTAB_ERR_NOMEM.
 */
int kvtab_put(kvtab *t, const char *key, const unsigned char *value,
              size_t value_len);

/*
 * Copies the value of `key` into the caller's buffer.
 * On entry `*len` is the size of `buf` in bytes. When the key exists, `*len`
 * is set to the size of the value, whether or not it fitted. When `*len` was
 * too small, nothing is copied and the result is KVTAB_ERR_TOO_SMALL.
 * `buf` may be NULL when `*len` is 0 on entry.
 * Errors: KVTAB_ERR_NULL, KVTAB_ERR_NOT_FOUND, KVTAB_ERR_TOO_SMALL.
 */
int kvtab_get(const kvtab *t, const char *key, unsigned char *buf, size_t *len);

/*
 * Returns a pointer to the stored value of `key` without copying it, and
 * writes its size to `*len`. Returns NULL (and writes 0) when the key is not
 * in the table or an argument is NULL. The pointer is never NULL for a key
 * that exists, even when the value is empty.
 * The pointer is valid until the next call to kvtab_put, kvtab_remove or
 * kvtab_free on this table. Never free it and never write through it.
 */
const unsigned char *kvtab_peek(const kvtab *t, const char *key, size_t *len);

/*
 * Removes `key`. Later entries keep their order.
 * Errors: KVTAB_ERR_NULL, KVTAB_ERR_NOT_FOUND.
 */
int kvtab_remove(kvtab *t, const char *key);

/* The number of entries. Returns 0 when `t` is NULL. */
size_t kvtab_len(const kvtab *t);

/* Fills `*out`. Errors: KVTAB_ERR_NULL. */
int kvtab_get_stats(const kvtab *t, kvtab_stats *out);

/*
 * Calls `visit` once per entry, in insertion order, and passes `userdata`
 * through unchanged. Stops early when `visit` returns a value other than 0;
 * the result is then KVTAB_ERR_STOPPED.
 * Errors: KVTAB_ERR_NULL (`t` or `visit` is NULL), KVTAB_ERR_STOPPED.
 */
int kvtab_for_each(const kvtab *t, kvtab_visit_fn visit, void *userdata);

/*
 * Installs an observer and hands `userdata` to the table. The table calls
 * drop(userdata) exactly once: when another call to this function replaces
 * the observer, or in kvtab_free. `observe` and `drop` may each be NULL;
 * passing NULL for both removes the observer.
 * Errors: KVTAB_ERR_NULL (`t` is NULL). In that case nothing was installed:
 * `drop` is NOT called and the caller still owns `userdata`.
 */
int kvtab_set_observer(kvtab *t, kvtab_observe_fn observe, void *userdata,
                       kvtab_drop_fn drop);

#ifdef __cplusplus
}
#endif

#endif /* KVTAB_H */
