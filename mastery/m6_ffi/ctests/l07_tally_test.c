/*
 * C test program for lesson 07. Do not edit it: it is the test.
 *
 * It is compiled against YOUR header include/m6_tally.h and linked with the
 * static library that Cargo builds from this package. Run it with
 *
 *     cargo test --manifest-path mastery/m6_ffi/Cargo.toml l07_
 */
#include <stdint.h>
#include <stdio.h>
#include <string.h>

#include "m6_tally.h"

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
 * C compiler compare your header with it: a declaration with another return
 * type or parameter list does not compile.
 */
static uint32_t (*const pin_abi_version)(void) = m6_tally_abi_version;
static m6_tally *(*const pin_new)(void) = m6_tally_new;
static void (*const pin_free)(m6_tally *) = m6_tally_free;
static int (*const pin_add)(m6_tally *, const char *) = m6_tally_add;
static uint64_t (*const pin_count)(const m6_tally *, const char *) = m6_tally_count;
static char *(*const pin_top)(const m6_tally *) = m6_tally_top;
static void (*const pin_string_free)(char *) = m6_tally_string_free;

int main(void) {
    m6_tally *tally;
    char *top;
    int i;

    CHECK(pin_abi_version() == 1);
    CHECK(M6_TALLY_OK == 0);
    CHECK(M6_TALLY_ERR_NULL == 1);
    CHECK(M6_TALLY_ERR_UTF8 == 2);

    tally = pin_new();
    CHECK(tally != NULL);
    if (tally == NULL) {
        printf("%d checks, %d failures\n", checks, failures);
        return 1;
    }

    /* An empty tally. */
    CHECK(pin_top(tally) == NULL);
    CHECK(pin_count(tally, "pear") == 0);

    /* Words are separated by any run of ASCII white space. */
    CHECK(pin_add(tally, "pear apple  pear\tfig\npear apple ") == M6_TALLY_OK);
    CHECK(pin_count(tally, "pear") == 3);
    CHECK(pin_count(tally, "apple") == 2);
    CHECK(pin_count(tally, "fig") == 1);
    CHECK(pin_count(tally, "plum") == 0);
    CHECK(pin_count(tally, "") == 0);
    CHECK(pin_add(tally, "") == M6_TALLY_OK);
    CHECK(pin_add(tally, "   ") == M6_TALLY_OK);
    CHECK(pin_count(tally, "pear") == 3);

    /* The most frequent word is returned as a string that the caller owns. */
    top = pin_top(tally);
    CHECK(top != NULL && strcmp(top, "pear") == 0);
    if (top != NULL) {
        top[0] = 'b'; /* the caller owns the string and may change it */
        CHECK(pin_count(tally, "pear") == 3);
    }
    pin_string_free(top);

    /* A tie goes to the smallest word in byte order. */
    CHECK(pin_add(tally, "apple") == M6_TALLY_OK);
    top = pin_top(tally);
    CHECK(top != NULL && strcmp(top, "apple") == 0);
    pin_string_free(top);

    /* Words are UTF-8; bytes that are not UTF-8 are refused as a whole. */
    CHECK(pin_add(tally, "caf\xc3\xa9 caf\xc3\xa9") == M6_TALLY_OK);
    CHECK(pin_count(tally, "caf\xc3\xa9") == 2);
    CHECK(pin_add(tally, "plum \xff\xfe plum") == M6_TALLY_ERR_UTF8);
    CHECK(pin_count(tally, "plum") == 0);

    /* NULL pointers are reported, not dereferenced. */
    CHECK(pin_add(NULL, "x") == M6_TALLY_ERR_NULL);
    CHECK(pin_add(tally, NULL) == M6_TALLY_ERR_NULL);
    CHECK(pin_count(NULL, "pear") == 0);
    CHECK(pin_count(tally, NULL) == 0);
    CHECK(pin_top(NULL) == NULL);
    pin_string_free(NULL);
    pin_free(NULL);

    /* Many strings: each one is allocated by Rust and released by Rust. */
    for (i = 0; i < 1000; i++) {
        top = pin_top(tally);
        pin_string_free(top);
    }

    pin_free(tally);

    printf("%d checks, %d failures\n", checks, failures);
    return failures == 0 ? 0 : 1;
}
