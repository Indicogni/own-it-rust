/*
 * C test program for the capstone. Do not edit it: it is the test.
 *
 * It is compiled against YOUR header include/m6_stock.h and linked with the
 * static library that Cargo builds from this package. Run it with
 *
 *     cargo test --manifest-path mastery/m6_ffi/Cargo.toml l08_
 */
#include <stdint.h>
#include <stdio.h>
#include <string.h>

#include "kvtab.h"
#include "m6_stock.h"

static int checks = 0;
static int failures = 0;

#define CHECK(condition)                                                    \
    do {                                                                    \
        checks++;                                                           \
        if (!(condition)) {                                                 \
            failures++;                                                     \
            printf("FAIL line %d: %s\n", __LINE__, #condition);             \
        }                                                                   \
    } while (0)

/*
 * The contract fixes the type of every function. These initialisers make the
 * C compiler compare your header with it.
 */
static m6_stock *(*const pin_open)(size_t) = m6_stock_open;
static void (*const pin_close)(m6_stock *) = m6_stock_close;
static int (*const pin_adjust)(m6_stock *, const char *, int64_t, int64_t *) = m6_stock_adjust;
static int (*const pin_quantity)(const m6_stock *, const char *, int64_t *) = m6_stock_quantity;
static size_t (*const pin_len)(const m6_stock *) = m6_stock_len;
static int (*const pin_report)(const m6_stock *, char *, size_t, size_t *) = m6_stock_report;
static int (*const pin_for_each)(const m6_stock *, m6_stock_visit_fn, void *) = m6_stock_for_each;
static struct kvtab *(*const pin_table)(const m6_stock *) = m6_stock_table;
static const char *(*const pin_last_error)(void) = m6_stock_last_error;
static int (*const pin_debug_panic)(void) = m6_stock_debug_panic;

/* What the visitor collects through `userdata`. */
struct walk {
    int calls;
    int stop_at; /* stop when `calls` reaches this number; 0 = never */
    int64_t total;
    char order[64]; /* the first letter of each item, in call order */
};

static int visit(const char *item, int64_t quantity, void *userdata) {
    struct walk *walk = (struct walk *)userdata;
    if (walk->calls < 63) {
        walk->order[walk->calls] = item[0];
    }
    walk->calls++;
    walk->total += quantity;
    return walk->stop_at != 0 && walk->calls >= walk->stop_at;
}

static void test_quantities(void) {
    m6_stock *stock;
    int64_t quantity = -77;

    CHECK(pin_open(0) == NULL);
    stock = pin_open(3);
    CHECK(stock != NULL);
    if (stock == NULL) {
        return;
    }
    CHECK(pin_len(stock) == 0);

    CHECK(pin_adjust(stock, "widget", 5, &quantity) == M6_STOCK_OK);
    CHECK(quantity == 5);
    CHECK(pin_adjust(stock, "widget", -2, &quantity) == M6_STOCK_OK);
    CHECK(quantity == 3);
    CHECK(pin_len(stock) == 1);

    /* Going below zero is refused, and nothing changes. */
    quantity = -77;
    CHECK(pin_adjust(stock, "widget", -4, &quantity) == M6_STOCK_ERR_RANGE);
    CHECK(quantity == -77);
    CHECK(pin_quantity(stock, "widget", &quantity) == M6_STOCK_OK);
    CHECK(quantity == 3);

    /* A refused adjustment of an unknown item does not create the item. */
    CHECK(pin_adjust(stock, "ghost", -1, NULL) == M6_STOCK_ERR_RANGE);
    CHECK(pin_len(stock) == 1);
    CHECK(pin_quantity(stock, "ghost", &quantity) == M6_STOCK_ERR_UNKNOWN);
    CHECK(strstr(pin_last_error(), "ghost") != NULL);

    /* `quantity` may be NULL; a delta of 0 creates an item with quantity 0. */
    CHECK(pin_adjust(stock, "gear", 10, NULL) == M6_STOCK_OK);
    CHECK(pin_adjust(stock, "axle", 0, &quantity) == M6_STOCK_OK);
    CHECK(quantity == 0);
    CHECK(pin_len(stock) == 3);

    /* The stock is full now: no new item, but known items still work. */
    CHECK(pin_adjust(stock, "nut", 1, NULL) == M6_STOCK_ERR_FULL);
    CHECK(pin_len(stock) == 3);
    CHECK(pin_adjust(stock, "gear", 1, &quantity) == M6_STOCK_OK);
    CHECK(quantity == 11);

    /* The largest quantity is INT64_MAX. */
    CHECK(pin_adjust(stock, "axle", INT64_MAX, &quantity) == M6_STOCK_OK);
    CHECK(quantity == INT64_MAX);
    CHECK(pin_adjust(stock, "axle", 1, &quantity) == M6_STOCK_ERR_RANGE);
    CHECK(pin_quantity(stock, "axle", &quantity) == M6_STOCK_OK);
    CHECK(quantity == INT64_MAX);
    CHECK(pin_adjust(stock, "axle", INT64_MIN, NULL) == M6_STOCK_ERR_RANGE);

    /* Bad arguments are reported, not dereferenced. */
    CHECK(pin_adjust(NULL, "widget", 1, NULL) == M6_STOCK_ERR_NULL);
    CHECK(pin_adjust(stock, NULL, 1, NULL) == M6_STOCK_ERR_NULL);
    CHECK(pin_adjust(stock, "\xff\xfe", 1, NULL) == M6_STOCK_ERR_UTF8);
    CHECK(pin_quantity(NULL, "widget", &quantity) == M6_STOCK_ERR_NULL);
    CHECK(pin_quantity(stock, NULL, &quantity) == M6_STOCK_ERR_NULL);
    CHECK(pin_quantity(stock, "widget", NULL) == M6_STOCK_ERR_NULL);
    CHECK(pin_quantity(stock, "\xff", &quantity) == M6_STOCK_ERR_UTF8);
    CHECK(pin_len(NULL) == 0);
    CHECK(pin_last_error() != NULL && pin_last_error()[0] != '\0');

    pin_close(stock);
    pin_close(NULL);
}

static void test_report(void) {
    static const char expected[] = "axle=7\ngear=10\nwidget=3\n";
    m6_stock *stock = pin_open(4);
    char buf[64];
    size_t needed = 0;

    CHECK(stock != NULL);
    if (stock == NULL) {
        return;
    }
    /* An empty stock has an empty report: just the NUL. */
    memset(buf, 'x', sizeof(buf));
    CHECK(pin_report(stock, buf, sizeof(buf), &needed) == M6_STOCK_OK);
    CHECK(needed == 1);
    CHECK(buf[0] == '\0');

    CHECK(pin_adjust(stock, "widget", 3, NULL) == M6_STOCK_OK);
    CHECK(pin_adjust(stock, "gear", 10, NULL) == M6_STOCK_OK);
    CHECK(pin_adjust(stock, "axle", 7, NULL) == M6_STOCK_OK);

    /* First call: no buffer, ask for the size. */
    needed = 0;
    CHECK(pin_report(stock, NULL, 0, &needed) == M6_STOCK_ERR_TOO_SMALL);
    CHECK(needed == sizeof(expected));

    /* A buffer that is one byte too small is not written at all. */
    memset(buf, 'x', sizeof(buf));
    needed = 0;
    CHECK(pin_report(stock, buf, sizeof(expected) - 1, &needed) == M6_STOCK_ERR_TOO_SMALL);
    CHECK(needed == sizeof(expected));
    CHECK(buf[0] == 'x' && buf[sizeof(expected) - 2] == 'x');

    /* A buffer of exactly the needed size. Lines are sorted by item name. */
    memset(buf, 'x', sizeof(buf));
    CHECK(pin_report(stock, buf, sizeof(expected), &needed) == M6_STOCK_OK);
    CHECK(memcmp(buf, expected, sizeof(expected)) == 0);
    CHECK(buf[sizeof(expected)] == 'x');

    /* `needed` may be NULL. */
    memset(buf, 'x', sizeof(buf));
    CHECK(pin_report(stock, buf, sizeof(buf), NULL) == M6_STOCK_OK);
    CHECK(strcmp(buf, expected) == 0);
    CHECK(pin_report(NULL, buf, sizeof(buf), &needed) == M6_STOCK_ERR_NULL);

    pin_close(stock);
}

static void test_for_each(void) {
    m6_stock *stock = pin_open(4);
    struct walk walk;

    CHECK(stock != NULL);
    if (stock == NULL) {
        return;
    }
    CHECK(pin_adjust(stock, "widget", 3, NULL) == M6_STOCK_OK);
    CHECK(pin_adjust(stock, "gear", 10, NULL) == M6_STOCK_OK);
    CHECK(pin_adjust(stock, "axle", 7, NULL) == M6_STOCK_OK);
    CHECK(pin_adjust(stock, "widget", 1, NULL) == M6_STOCK_OK);

    /* All items, in the order in which they were first added. */
    memset(&walk, 0, sizeof(walk));
    CHECK(pin_for_each(stock, visit, &walk) == M6_STOCK_OK);
    CHECK(walk.calls == 3);
    CHECK(walk.total == 21);
    CHECK(strcmp(walk.order, "wga") == 0);

    /* The visitor can stop the walk. */
    memset(&walk, 0, sizeof(walk));
    walk.stop_at = 2;
    CHECK(pin_for_each(stock, visit, &walk) == M6_STOCK_ERR_STOPPED);
    CHECK(walk.calls == 2);
    CHECK(walk.total == 14);

    CHECK(pin_for_each(stock, NULL, &walk) == M6_STOCK_ERR_NULL);
    CHECK(pin_for_each(NULL, visit, &walk) == M6_STOCK_ERR_NULL);

    pin_close(stock);
}

/* The stock must be built on a kvtab table, with 8 little-endian bytes per
 * quantity. This test reads the table with the C library directly. */
static void test_storage(void) {
    m6_stock *stock = pin_open(2);
    struct kvtab *table;
    const unsigned char *bytes;
    size_t len = 0;

    CHECK(stock != NULL);
    if (stock == NULL) {
        return;
    }
    CHECK(pin_table(NULL) == NULL);
    CHECK(pin_adjust(stock, "widget", 258, NULL) == M6_STOCK_OK);
    table = pin_table(stock);
    CHECK(table != NULL);
    if (table != NULL) {
        CHECK(kvtab_len(table) == 1);
        bytes = kvtab_peek(table, "widget", &len);
        CHECK(bytes != NULL && len == 8);
        if (bytes != NULL && len == 8) {
            /* 258 = 0x0102: low byte first. */
            CHECK(bytes[0] == 2 && bytes[1] == 1 && bytes[2] == 0 && bytes[7] == 0);
        }
    }
    pin_close(stock);
}

static void test_panic(void) {
    m6_stock *stock = pin_open(1);
    int64_t quantity = 0;

    CHECK(stock != NULL);
    if (stock == NULL) {
        return;
    }
    CHECK(pin_adjust(stock, "widget", 3, NULL) == M6_STOCK_OK);

    /* A panic inside the library must come back as a status code. If this
     * program dies here, the panic reached the `extern "C"` boundary. */
    CHECK(pin_debug_panic() == M6_STOCK_ERR_PANIC);
    CHECK(strstr(pin_last_error(), "deliberate panic") != NULL);

    /* The library is still usable afterwards. */
    CHECK(pin_quantity(stock, "widget", &quantity) == M6_STOCK_OK);
    CHECK(quantity == 3);
    pin_close(stock);
}

int main(void) {
    /* Before anything has failed on this thread, the message is empty. */
    CHECK(pin_last_error() != NULL && pin_last_error()[0] == '\0');

    CHECK(M6_STOCK_OK == 0);
    CHECK(M6_STOCK_ERR_NULL == 1);
    CHECK(M6_STOCK_ERR_UTF8 == 2);
    CHECK(M6_STOCK_ERR_UNKNOWN == 3);
    CHECK(M6_STOCK_ERR_RANGE == 4);
    CHECK(M6_STOCK_ERR_FULL == 5);
    CHECK(M6_STOCK_ERR_TOO_SMALL == 6);
    CHECK(M6_STOCK_ERR_STOPPED == 7);
    CHECK(M6_STOCK_ERR_PANIC == 8);
    CHECK(M6_STOCK_ERR_INTERNAL == 9);

    test_quantities();
    test_report();
    test_for_each();
    test_storage();
    test_panic();

    printf("%d checks, %d failures\n", checks, failures);
    return failures == 0 ? 0 : 1;
}
