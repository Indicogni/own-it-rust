/*
 * m6_tally.h - the C interface of the word tally of lesson 07.
 *
 * You write this header by hand, from the contract in src/l07_export.rs.
 * The C test program ctests/l07_tally_test.c includes it.
 */
#ifndef M6_TALLY_H
#define M6_TALLY_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* An opaque word tally. C code only ever holds a pointer to one. */
typedef struct m6_tally m6_tally;

/* Status codes, returned as `int`. */
#define M6_TALLY_OK 0
#define M6_TALLY_ERR_NULL 1 /* a required pointer was NULL */
#define M6_TALLY_ERR_UTF8 2 /* the text is not valid UTF-8 */

/* The version of this interface. It changes when the ABI changes. */
uint32_t m6_tally_abi_version(void);

/* Creates an empty tally. Never returns NULL. Release with m6_tally_free. */
m6_tally *m6_tally_new(void);

/* Destroys a tally. `tally` may be NULL. Do not use it afterwards. */
void m6_tally_free(m6_tally *tally);

/*
 * Counts every word of `text`. Words are separated by ASCII white space.
 * Errors: M6_TALLY_ERR_NULL; M6_TALLY_ERR_UTF8 (then nothing is counted).
 * `text` is only read during the call.
 */
int m6_tally_add(m6_tally *tally, const char *text);

/* How often `word` was counted. 0 when a pointer is NULL or the word is unknown. */
uint64_t m6_tally_count(const m6_tally *tally, const char *word);

/*
 * The most frequent word, as a new NUL-terminated string. Among words with
 * the same count, the smallest in byte order wins. Returns NULL when `tally`
 * is NULL or empty.
 * The caller owns the string and must release it with m6_tally_string_free,
 * and with nothing else: it was not allocated by malloc.
 * The caller may change the bytes of the string but not its length: do not
 * write a NUL into it. m6_tally_string_free finds the size of the allocation
 * by looking for the NUL.
 */
char *m6_tally_top(const m6_tally *tally);

/* Releases a string returned by m6_tally_top. `text` may be NULL. */
void m6_tally_string_free(char *text);

#ifdef __cplusplus
}
#endif

#endif /* M6_TALLY_H */
