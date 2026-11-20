//! Sets of characters.
//!
//! While a class is being parsed it is a [`RangeSet`]: a list of inclusive
//! ranges of scalar values that can be merged, folded and complemented.
//! The compiler turns the finished set into a [`CharSet`], which answers
//! "is this character in the set?" quickly and is never changed again.

/// The largest Unicode scalar value.
const MAX_SCALAR: u32 = char::MAX as u32;

/// `\d`
pub(crate) const DIGIT: &[(char, char)] = &[('0', '9')];
/// `\w`
pub(crate) const WORD: &[(char, char)] = &[('0', '9'), ('A', 'Z'), ('_', '_'), ('a', 'z')];
/// `\s`: tab, line feed, vertical tab, form feed, carriage return, space.
pub(crate) const SPACE: &[(char, char)] = &[('\t', '\r'), (' ', ' ')];

/// True for the characters of `\w`. `\b` and `\B` use the same definition.
pub(crate) fn is_word(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// Inclusive ranges of `u32` code points, sorted, with a gap between any
/// two of them.
///
/// The ranges are `u32` and not `char` so that "the next code point" needs
/// no special case around the surrogate gap. A surrogate inside a range does
/// no harm: no `char` ever has that value, so it never matches anything.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct RangeSet {
    ranges: Vec<(u32, u32)>,
}

impl RangeSet {
    /// The set of all characters in the given ranges, which may come in any
    /// order and may overlap. One sort for the whole set: a class with
    /// twenty thousand members must not be sorted twenty thousand times.
    pub(crate) fn new(mut ranges: Vec<(u32, u32)>) -> RangeSet {
        ranges.sort_unstable();
        let mut merged: Vec<(u32, u32)> = Vec::with_capacity(ranges.len());
        for (lo, hi) in ranges {
            match merged.last_mut() {
                // Overlapping or touching. `last.1 + 1` cannot overflow:
                // code points stay below 0x11_0000.
                Some(last) if lo <= last.1 + 1 => last.1 = last.1.max(hi),
                _ => merged.push((lo, hi)),
            }
        }
        RangeSet { ranges: merged }
    }

    pub(crate) fn from_chars(ranges: &[(char, char)]) -> RangeSet {
        RangeSet::new(
            ranges
                .iter()
                .map(|&(lo, hi)| (lo as u32, hi as u32))
                .collect(),
        )
    }

    /// The ranges, for building a larger set from this one.
    pub(crate) fn ranges(&self) -> &[(u32, u32)] {
        &self.ranges
    }

    /// Every scalar value that is not in the set.
    pub(crate) fn complement(&self) -> RangeSet {
        let mut ranges = Vec::with_capacity(self.ranges.len() + 1);
        let mut next = 0;
        for &(lo, hi) in &self.ranges {
            if lo > next {
                ranges.push((next, lo - 1));
            }
            next = hi + 1;
        }
        if next <= MAX_SCALAR {
            ranges.push((next, MAX_SCALAR));
        }
        // Built in order with gaps between the ranges: already normal.
        RangeSet { ranges }
    }

    /// The set with, for every ASCII letter in it, the same letter in the
    /// other case added (SPEC 3.9).
    pub(crate) fn fold_ascii_case(&self) -> RangeSet {
        const CASE_BIT: u32 = 0x20;
        let mut ranges = self.ranges.clone();
        for &(lo, hi) in &self.ranges {
            // The part of the range inside `A..=Z` also selects `a..=z`,
            // and the other way round.
            let (upper_lo, upper_hi) = (lo.max('A' as u32), hi.min('Z' as u32));
            if upper_lo <= upper_hi {
                ranges.push((upper_lo + CASE_BIT, upper_hi + CASE_BIT));
            }
            let (lower_lo, lower_hi) = (lo.max('a' as u32), hi.min('z' as u32));
            if lower_lo <= lower_hi {
                ranges.push((lower_lo - CASE_BIT, lower_hi - CASE_BIT));
            }
        }
        RangeSet::new(ranges)
    }

    #[cfg(test)]
    pub(crate) fn contains(&self, c: char) -> bool {
        let c = c as u32;
        self.ranges.iter().any(|&(lo, hi)| lo <= c && c <= hi)
    }
}

/// A finished set, built for fast lookup.
///
/// Most text is ASCII, so the first 128 code points live in a bitmap: one
/// shift and one mask decide membership. Everything else is found by binary
/// search in the sorted ranges.
#[derive(Clone, Debug)]
pub(crate) struct CharSet {
    ascii: u128,
    /// Ranges that reach above U+007F, sorted, not overlapping.
    others: Box<[(u32, u32)]>,
}

impl CharSet {
    pub(crate) fn new(set: &RangeSet) -> CharSet {
        let mut ascii = 0_u128;
        let mut others = Vec::new();
        for &(lo, hi) in &set.ranges {
            for c in lo..=hi.min(0x7F) {
                ascii |= 1 << c;
            }
            if hi > 0x7F {
                others.push((lo.max(0x80), hi));
            }
        }
        CharSet {
            ascii,
            others: others.into_boxed_slice(),
        }
    }

    #[inline]
    pub(crate) fn contains(&self, c: char) -> bool {
        let c = c as u32;
        if c < 0x80 {
            return self.ascii >> c & 1 == 1;
        }
        self.others
            .binary_search_by(|&(lo, hi)| {
                if hi < c {
                    std::cmp::Ordering::Less
                } else if lo > c {
                    std::cmp::Ordering::Greater
                } else {
                    std::cmp::Ordering::Equal
                }
            })
            .is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ranges_are_sorted_and_merged() {
        // d-f touches a-c; o-z overlaps m-p.
        let set = RangeSet::from_chars(&[('m', 'p'), ('a', 'c'), ('d', 'f'), ('o', 'z')]);
        assert_eq!(
            set.ranges,
            [('a' as u32, 'f' as u32), ('m' as u32, 'z' as u32)]
        );
    }

    #[test]
    fn complement_covers_everything_else() {
        let set = RangeSet::from_chars(&[('\0', 'a'), ('c', char::MAX)]);
        assert_eq!(set.complement().ranges, [('b' as u32, 'b' as u32)]);
        assert_eq!(RangeSet::default().complement().ranges, [(0, MAX_SCALAR)]);
        assert_eq!(
            RangeSet::default().complement().complement(),
            RangeSet::default()
        );
    }

    #[test]
    fn folding_adds_the_other_case_of_ascii_letters_only() {
        // X Y Z [ \ ] ^ _ ` a b c
        let set = RangeSet::from_chars(&[('X', 'c'), ('é', 'é')]).fold_ascii_case();
        for c in "XYZxyzABCabc[_`é".chars() {
            assert!(set.contains(c), "{c:?} must be in the folded set");
        }
        for c in "DWdwÉ".chars() {
            assert!(!set.contains(c), "{c:?} must not be in the folded set");
        }
    }

    #[test]
    fn char_set_agrees_with_range_set() {
        let set = RangeSet::from_chars(&[('a', 'f'), ('~', 'é'), ('α', 'ω'), ('😀', '😏')]);
        let fast = CharSet::new(&set);
        let probes = [
            'a',
            'f',
            'g',
            '}',
            '~',
            '\u{7f}',
            '\u{80}',
            'é',
            'ê',
            'α',
            'π',
            'ω',
            'ϊ',
            '😀',
            '😎',
            '😐',
            char::MAX,
        ];
        for c in probes {
            assert_eq!(fast.contains(c), set.contains(c), "lookup of {c:?}");
        }
    }
}
