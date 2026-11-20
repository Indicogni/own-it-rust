/*
 * kvtab - a tiny key-value table with a fixed capacity.
 * See kvtab.h for the contract of every function.
 *
 * SPDX-License-Identifier: MIT
 */
#include "kvtab.h"

#include <stdlib.h>
#include <string.h>

struct entry {
    char *key;            /* owned, NUL-terminated */
    unsigned char *value; /* owned, never NULL */
    size_t value_len;
};

struct kvtab {
    struct entry *entries; /* `capacity` slots; the first `len` are in use */
    size_t len;
    size_t capacity;
    unsigned int puts;
    kvtab_observe_fn observe;
    kvtab_drop_fn drop;
    void *userdata;
};

/* Copies n bytes. Allocates at least one byte, so the result is not NULL. */
static unsigned char *copy_bytes(const unsigned char *src, size_t n) {
    unsigned char *copy = (unsigned char *)malloc(n != 0 ? n : 1);
    if (copy != NULL && n != 0) {
        memcpy(copy, src, n);
    }
    return copy;
}

static struct entry *find(const kvtab *t, const char *key) {
    size_t i;
    for (i = 0; i < t->len; i++) {
        if (strcmp(t->entries[i].key, key) == 0) {
            return &t->entries[i];
        }
    }
    return NULL;
}

const char *kvtab_version(void) {
    return "kvtab 1.0.0";
}

const char *kvtab_strerror(int status) {
    switch (status) {
    case KVTAB_OK: return "success";
    case KVTAB_ERR_NULL: return "a required pointer was NULL";
    case KVTAB_ERR_NOT_FOUND: return "key not found";
    case KVTAB_ERR_FULL: return "table is full";
    case KVTAB_ERR_TOO_SMALL: return "buffer too small";
    case KVTAB_ERR_NOMEM: return "out of memory";
    case KVTAB_ERR_STOPPED: return "stopped by the visitor";
    default: return "unknown status";
    }
}

kvtab *kvtab_new(size_t capacity) {
    kvtab *t;
    if (capacity == 0) {
        return NULL;
    }
    t = (kvtab *)calloc(1, sizeof(*t));
    if (t == NULL) {
        return NULL;
    }
    /* calloc checks capacity * sizeof(struct entry) for overflow. */
    t->entries = (struct entry *)calloc(capacity, sizeof(struct entry));
    if (t->entries == NULL) {
        free(t);
        return NULL;
    }
    t->capacity = capacity;
    return t;
}

void kvtab_free(kvtab *t) {
    size_t i;
    if (t == NULL) {
        return;
    }
    for (i = 0; i < t->len; i++) {
        free(t->entries[i].key);
        free(t->entries[i].value);
    }
    if (t->drop != NULL) {
        t->drop(t->userdata);
    }
    free(t->entries);
    free(t);
}

int kvtab_put(kvtab *t, const char *key, const unsigned char *value,
              size_t value_len) {
    struct entry *e;
    unsigned char *copy;
    if (t == NULL || key == NULL || (value == NULL && value_len != 0)) {
        return KVTAB_ERR_NULL;
    }
    e = find(t, key);
    if (e == NULL && t->len == t->capacity) {
        return KVTAB_ERR_FULL;
    }
    copy = copy_bytes(value, value_len);
    if (copy == NULL) {
        return KVTAB_ERR_NOMEM;
    }
    if (e != NULL) {
        free(e->value);
    } else {
        size_t key_size = strlen(key) + 1;
        char *key_copy = (char *)malloc(key_size);
        if (key_copy == NULL) {
            free(copy);
            return KVTAB_ERR_NOMEM;
        }
        memcpy(key_copy, key, key_size);
        e = &t->entries[t->len++];
        e->key = key_copy;
    }
    e->value = copy;
    e->value_len = value_len;
    t->puts++;
    if (t->observe != NULL) {
        t->observe(KVTAB_EVENT_PUT, e->key, t->userdata);
    }
    return KVTAB_OK;
}

int kvtab_get(const kvtab *t, const char *key, unsigned char *buf, size_t *len) {
    const struct entry *e;
    size_t room;
    if (t == NULL || key == NULL || len == NULL) {
        return KVTAB_ERR_NULL;
    }
    e = find(t, key);
    if (e == NULL) {
        return KVTAB_ERR_NOT_FOUND;
    }
    room = *len;
    *len = e->value_len;
    if (room < e->value_len) {
        return KVTAB_ERR_TOO_SMALL;
    }
    if (e->value_len != 0) {
        if (buf == NULL) {
            return KVTAB_ERR_NULL;
        }
        memcpy(buf, e->value, e->value_len);
    }
    return KVTAB_OK;
}

const unsigned char *kvtab_peek(const kvtab *t, const char *key, size_t *len) {
    const struct entry *e = NULL;
    if (t != NULL && key != NULL && len != NULL) {
        e = find(t, key);
    }
    if (len != NULL) {
        *len = e != NULL ? e->value_len : 0;
    }
    return e != NULL ? e->value : NULL;
}

int kvtab_remove(kvtab *t, const char *key) {
    struct entry *e;
    struct entry removed;
    size_t index;
    if (t == NULL || key == NULL) {
        return KVTAB_ERR_NULL;
    }
    e = find(t, key);
    if (e == NULL) {
        return KVTAB_ERR_NOT_FOUND;
    }
    removed = *e;
    index = (size_t)(e - t->entries);
    memmove(e, e + 1, (t->len - index - 1) * sizeof(struct entry));
    t->len--;
    if (t->observe != NULL) {
        t->observe(KVTAB_EVENT_REMOVE, removed.key, t->userdata);
    }
    free(removed.key);
    free(removed.value);
    return KVTAB_OK;
}

size_t kvtab_len(const kvtab *t) {
    return t != NULL ? t->len : 0;
}

int kvtab_get_stats(const kvtab *t, kvtab_stats *out) {
    size_t i;
    if (t == NULL || out == NULL) {
        return KVTAB_ERR_NULL;
    }
    memset(out, 0, sizeof(*out));
    out->len = t->len;
    out->capacity = t->capacity;
    for (i = 0; i < t->len; i++) {
        out->value_bytes += t->entries[i].value_len;
    }
    out->puts = t->puts;
    out->full = (unsigned char)(t->len == t->capacity);
    return KVTAB_OK;
}

int kvtab_for_each(const kvtab *t, kvtab_visit_fn visit, void *userdata) {
    size_t i;
    if (t == NULL || visit == NULL) {
        return KVTAB_ERR_NULL;
    }
    for (i = 0; i < t->len; i++) {
        const struct entry *e = &t->entries[i];
        if (visit(e->key, e->value, e->value_len, userdata) != 0) {
            return KVTAB_ERR_STOPPED;
        }
    }
    return KVTAB_OK;
}

int kvtab_set_observer(kvtab *t, kvtab_observe_fn observe, void *userdata,
                       kvtab_drop_fn drop) {
    if (t == NULL) {
        return KVTAB_ERR_NULL;
    }
    if (t->drop != NULL) {
        t->drop(t->userdata);
    }
    t->observe = observe;
    t->userdata = userdata;
    t->drop = drop;
    return KVTAB_OK;
}
