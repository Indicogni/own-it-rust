//! The 5x7 font, turned from text into a table while the crate compiles.
//!
//! `parse` is a `const fn`, so a typo in `assets/font5x7.txt` is a compile
//! error and the finished table lives in read-only memory: looking a glyph
//! up at run time is one array index, with no start-up work and no
//! allocation.

pub(crate) const GLYPH_W: i32 = 5;
pub(crate) const GLYPH_H: usize = 7;
pub(crate) const ADVANCE: i32 = 6;

/// Seven rows; bit 4 is the left-most pixel of a row.
pub(crate) type Glyph = [u8; GLYPH_H];

const FIRST: usize = 0x20;
const COUNT: usize = 0x60;
const MISSING: usize = COUNT;
/// A row value no real glyph can have: marks "no entry yet".
const UNSET: u8 = 0xFF;

static TABLE: [Glyph; COUNT + 1] = parse(include_bytes!("../assets/font5x7.txt"));

/// The glyph for `ch`. Lower-case ASCII letters share the upper-case shapes;
/// anything without an entry gets the "missing" box.
pub(crate) fn glyph(ch: char) -> &'static Glyph {
    let code = ch.to_ascii_uppercase() as usize;
    if (FIRST..FIRST + COUNT).contains(&code) {
        &TABLE[code - FIRST]
    } else {
        &TABLE[MISSING]
    }
}

const fn starts_with(src: &[u8], at: usize, end: usize, word: &[u8]) -> bool {
    if end - at < word.len() {
        return false;
    }
    let mut i = 0;
    while i < word.len() {
        if src[at + i] != word[i] {
            return false;
        }
        i += 1;
    }
    true
}

/// Returns `(start, end, next)` for the line that begins at `at`: the line
/// is `src[start..end]` without its line ending, and `next` is where the
/// following line begins.
const fn line(src: &[u8], at: usize) -> (usize, usize, usize) {
    let mut end = at;
    while end < src.len() && src[end] != b'\n' {
        end += 1;
    }
    let next = if end < src.len() { end + 1 } else { end };
    if end > at && src[end - 1] == b'\r' {
        end -= 1;
    }
    (at, end, next)
}

const fn parse(src: &[u8]) -> [Glyph; COUNT + 1] {
    let mut table = [[UNSET; GLYPH_H]; COUNT + 1];
    let mut at = 0;
    while at < src.len() {
        let (start, end, next) = line(src, at);
        at = next;
        if start == end || src[start] == b';' {
            continue;
        }
        assert!(
            starts_with(src, start, end, b"glyph "),
            "font: expected a `glyph` line"
        );
        let name = start + 6;
        let slot = if starts_with(src, name, end, b"missing") && end - name == 7 {
            MISSING
        } else {
            assert!(end - name == 1, "font: a glyph name is one character");
            let code = src[name] as usize;
            assert!(
                code > FIRST && code < FIRST + COUNT,
                "font: glyph outside printable ASCII"
            );
            code - FIRST
        };
        assert!(table[slot][0] == UNSET, "font: glyph defined twice");
        let mut row = 0;
        while row < GLYPH_H {
            let (start, end, next) = line(src, at);
            at = next;
            assert!(
                end - start == GLYPH_W as usize,
                "font: a row has five cells"
            );
            let mut bits = 0u8;
            let mut col = 0;
            while col < GLYPH_W as usize {
                bits <<= 1;
                match src[start + col] {
                    b'#' => bits |= 1,
                    b'.' => {}
                    _ => panic!("font: a cell is `#` or `.`"),
                }
                col += 1;
            }
            table[slot][row] = bits;
            row += 1;
        }
    }
    assert!(table[MISSING][0] != UNSET, "font: no `glyph missing` entry");
    // Fill the gaps so a lookup never has to branch on "is it defined".
    let mut i = 0;
    while i < COUNT {
        if table[i][0] == UNSET {
            table[i] = table[MISSING];
        }
        i += 1;
    }
    table
}
