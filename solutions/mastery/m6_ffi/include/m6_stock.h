/*
 * m6_stock.h - the C interface of the capstone library.
 *
 * You write this header by hand, from the contract in src/l08_capstone.rs.
 * The C test program ctests/l08_stock_test.c includes it.
 */
#ifndef M6_STOCK_H
#define M6_STOCK_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* An opaque stock of items. C code only ever holds a pointer to one. */
typedef struct m6_stock m6_stock;

/* The table that a stock is built on (see vendor/kvtab/kvtab.h). */
struct kvtab;

/* Status codes, returned as `int`. */
#define M6_STOCK_OK 0
#define M6_STOCK_ERR_NULL 1      /* a required pointer was NULL */
#define M6_STOCK_ERR_UTF8 2      /* an item name is not valid UTF-8 */
#define M6_STOCK_ERR_UNKNOWN 3   /* the item is not in the stock */
#define M6_STOCK_ERR_RANGE 4     /* the quantity would leave 0..INT64_MAX */
#define M6_STOCK_ERR_FULL 5      /* no room for a new item */
#define M6_STOCK_ERR_TOO_SMALL 6 /* the caller's buffer is too small */
#define M6_STOCK_ERR_STOPPED 7   /* the visitor asked to stop */
#define M6_STOCK_ERR_PANIC 8     /* a panic was caught inside the library */
#define M6_STOCK_ERR_INTERNAL 9  /* the underlying table misbehaved */

/*
 * Called by m6_stock_for_each for one item. `item` is valid only until the
 * visitor returns. Return 0 to continue and any other value to stop. The
 * visitor must not change the stock.
 */
typedef int (*m6_stock_visit_fn)(const char *item, int64_t quantity,
                                 void *userdata);

/*
 * Threads: a stock has no lock. Two threads must never be inside calls on
 * the same stock at the same time.
 */

/* Creates a stock for at most `capacity` items. NULL when `capacity` is 0
 * or memory is not available. Release with m6_stock_close. */
m6_stock *m6_stock_open(size_t capacity);

/* Destroys a stock. `stock` may be NULL. Do not use it afterwards. */
void m6_stock_close(m6_stock *stock);

/*
 * Adds `delta` (which may be negative) to the quantity of `item`. An unknown
 * item starts at 0. On success the new quantity is stored in `*quantity`
 * unless `quantity` is NULL.
 * Errors (nothing changes, `*quantity` is not written): M6_STOCK_ERR_NULL,
 * M6_STOCK_ERR_UTF8, M6_STOCK_ERR_RANGE, M6_STOCK_ERR_FULL.
 */
int m6_stock_adjust(m6_stock *stock, const char *item, int64_t delta,
                    int64_t *quantity);

/*
 * Stores the quantity of `item` in `*quantity`.
 * Errors: M6_STOCK_ERR_NULL, M6_STOCK_ERR_UTF8, M6_STOCK_ERR_UNKNOWN.
 */
int m6_stock_quantity(const m6_stock *stock, const char *item,
                      int64_t *quantity);

/* The number of items. 0 when `stock` is NULL. */
size_t m6_stock_len(const m6_stock *stock);

/*
 * Writes a report into `buf`: one line `item=quantity\n` per item, sorted by
 * item name in byte order, followed by a NUL. `buf_len` is the size of `buf`.
 * Unless `needed` is NULL, `*needed` is set to the number of bytes that the
 * report needs, including the NUL, whether or not it fitted. When `buf_len`
 * is too small, `buf` is not written. `buf` may be NULL when `buf_len` is 0.
 * Errors: M6_STOCK_ERR_NULL, M6_STOCK_ERR_TOO_SMALL.
 */
int m6_stock_report(const m6_stock *stock, char *buf, size_t buf_len,
                    size_t *needed);

/*
 * Calls `visit` once per item, in the order in which the items were first
 * added, and passes `userdata` through unchanged.
 * Errors: M6_STOCK_ERR_NULL, M6_STOCK_ERR_STOPPED.
 */
int m6_stock_for_each(const m6_stock *stock, m6_stock_visit_fn visit,
                      void *userdata);

/*
 * The underlying table, for reading with the kvtab functions. The stock
 * still owns it: never free it and never change it. NULL when `stock` is NULL.
 */
struct kvtab *m6_stock_table(const m6_stock *stock);

/*
 * A description of the most recent failed m6_stock call on the calling
 * thread, or "" if none has failed. Never NULL. The pointer is valid until
 * the next failed m6_stock call on the same thread, or until that thread
 * ends. Never free it; copy the text if you need it for longer.
 */
const char *m6_stock_last_error(void);

/* Panics inside the library on purpose. Returns M6_STOCK_ERR_PANIC. */
int m6_stock_debug_panic(void);

#ifdef __cplusplus
}
#endif

#endif /* M6_STOCK_H */
