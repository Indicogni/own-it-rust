//! Milestone m16 (Chapters 12 and 16): aggregation and the text report.
//!
//! A [`Summary`] keeps counters, not records: per level, and per *group*
//! (the record's target, its level, or the value of one field). It owns its
//! group names, so it can outlive the lines it was built from.

use crate::level::Level;
use crate::record::Record;
use crate::sink::Sink;
use std::collections::{BTreeMap, BinaryHeap, HashMap};
use std::{cmp::Reverse, fmt, io, ops::AddAssign};

/// The numeric field whose values the report aggregates.
pub const LATENCY_KEY: &str = "latency_ms";

/// What the report groups records by.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Group {
    /// The record's target.
    Target,
    /// The record's level (shown as its canonical name).
    Level,
    /// The raw value of the first field with this key; `-` when missing.
    Field(String),
}

impl Group {
    /// m16 A. The group name of `record`: its target, its level's
    /// [`Level::as_str`], or the raw field value (`"-"` if the field is
    /// missing). The name borrows from the line (or is `'static`).
    pub fn key_of<'a>(&self, record: &Record<'a>) -> &'a str {
        match self {
            Group::Target => record.target,
            Group::Level => record.level.as_str(),
            Group::Field(key) => record.get(key).unwrap_or("-"),
        }
    }
}

/// m16 B. `"target"` is [`Group::Target`], `"level"` is [`Group::Level`],
/// anything else is a field name.
impl From<&str> for Group {
    fn from(name: &str) -> Self {
        match name {
            "target" => Group::Target,
            "level" => Group::Level,
            other => Group::Field(other.to_string()),
        }
    }
}

impl fmt::Display for Group {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Group::Target => f.write_str("target"),
            Group::Level => f.write_str("level"),
            Group::Field(key) => f.write_str(key),
        }
    }
}

/// Counters for one group.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GroupStats {
    /// Records in the group.
    pub count: u64,
    /// Records at [`Level::Error`].
    pub errors: u64,
    /// Saturating sum of the valid [`LATENCY_KEY`] values.
    pub latency_sum: u64,
    /// Number of valid [`LATENCY_KEY`] values.
    pub latency_count: u64,
    /// The largest valid [`LATENCY_KEY`] value.
    pub latency_max: Option<u64>,
}

impl GroupStats {
    /// m16 C. The mean latency ([`crate::num::mean`]), `None` without values.
    pub fn mean_latency(&self) -> Option<f64> {
        crate::num::mean(self.latency_sum, self.latency_count)
    }
}

/// m16 D. Merge another group's counters into this one (sums saturate).
impl AddAssign<&GroupStats> for GroupStats {
    fn add_assign(&mut self, other: &GroupStats) {
        self.count += other.count;
        self.errors += other.errors;
        self.latency_sum = self.latency_sum.saturating_add(other.latency_sum);
        self.latency_count += other.latency_count;
        self.latency_max = self.latency_max.max(other.latency_max);
    }
}

/// Aggregated counters for many records.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Summary {
    /// What the groups are.
    pub group: Group,
    /// Records added.
    pub records: u64,
    /// Records per level. Only levels that occurred have an entry.
    pub levels: BTreeMap<Level, u64>,
    /// Counters per group name.
    pub groups: HashMap<String, GroupStats>,
}

impl Summary {
    /// An empty summary.
    pub fn new(group: Group) -> Self {
        Summary {
            group,
            records: 0,
            levels: BTreeMap::new(),
            groups: HashMap::new(),
        }
    }

    /// m16 E. Count one record: `records`, its level, and its group's
    /// `count`, `errors`, and latency (a [`LATENCY_KEY`] value that is
    /// missing or not a number is ignored).
    ///
    /// Allocate the group name only the first time a group appears: look it
    /// up by `&str` before inserting an owned `String`.
    pub fn add(&mut self, record: &Record<'_>) {
        self.records += 1;
        *self.levels.entry(record.level).or_insert(0) += 1;
        let key = self.group.key_of(record);
        let stats = match self.groups.get_mut(key) {
            Some(stats) => stats,
            None => self.groups.entry(key.to_string()).or_default(),
        };
        stats.count += 1;
        if record.level == Level::Error {
            stats.errors += 1;
        }
        if let Ok(Some(latency)) = record.number(LATENCY_KEY) {
            stats.latency_sum = stats.latency_sum.saturating_add(latency);
            stats.latency_count += 1;
            stats.latency_max = stats.latency_max.max(Some(latency));
        }
    }

    /// m16 F. The `k` largest groups by `count`, largest first; equal counts
    /// in ascending name order. Fewer than `k` if there are fewer groups.
    ///
    /// Use a [`BinaryHeap`] that never holds more than `k` entries (with
    /// [`Reverse`] it is a min-heap: pop the worst when it grows past `k`),
    /// so the cost is O(n log k), not a full sort.
    pub fn top(&self, k: usize) -> Vec<(&str, &GroupStats)> {
        let mut heap = BinaryHeap::with_capacity(k.saturating_add(1).min(self.groups.len() + 1));
        for (name, stats) in &self.groups {
            heap.push(Reverse((stats.count, Reverse(name.as_str()))));
            if heap.len() > k {
                heap.pop();
            }
        }
        heap.into_sorted_vec()
            .into_iter()
            .map(|Reverse((_, Reverse(name)))| (name, &self.groups[name]))
            .collect()
    }
}

/// m16 G. Merge another summary into this one, as if all of its records had
/// been added here. Both must have the same `group` (`debug_assert_eq!`).
impl AddAssign for Summary {
    fn add_assign(&mut self, other: Summary) {
        debug_assert_eq!(self.group, other.group);
        self.records += other.records;
        for (level, n) in other.levels {
            *self.levels.entry(level).or_insert(0) += n;
        }
        for (name, stats) in other.groups {
            *self.groups.entry(name).or_default() += &stats;
        }
    }
}

/// m16 H. A summary is a sink: `accept` calls [`Summary::add`].
impl Sink for Summary {
    fn accept(&mut self, record: &Record<'_>) -> io::Result<()> {
        self.add(record);
        Ok(())
    }
}

/// m16 I. The report printed by `loglens report`. Every line ends in `\n`:
///
/// ```text
/// records: 8
/// levels: TRACE=0 DEBUG=1 INFO=4 WARN=1 ERROR=2
/// top 10 by target:
///   auth    count=4 errors=1 mean_latency_ms=122.0 max_latency_ms=310
///   db      count=2 errors=1 mean_latency_ms=2508.0 max_latency_ms=5012
///   billing count=1 errors=0 mean_latency_ms=- max_latency_ms=-
/// ```
///
/// - `levels:` lists all five levels in order, including zeros.
/// - The heading shows the requested `top` and the group ([`Group`]'s
///   `Display`), even when fewer groups exist.
/// - Rows come from [`Summary::top`]. Names are left-aligned and padded to
///   the longest name among the rows shown. The mean has one decimal; a
///   missing mean or maximum is `-`.
/// - With no rows, the heading is followed by the line `  (none)`.
pub fn render(summary: &Summary, top: usize) -> String {
    use fmt::Write;
    let mut out = String::new();
    let _ = writeln!(out, "records: {}", summary.records);
    out.push_str("levels:");
    for level in Level::ALL {
        let n = summary.levels.get(&level).copied().unwrap_or(0);
        let _ = write!(out, " {level}={n}");
    }
    let _ = writeln!(out, "\ntop {top} by {}:", summary.group);
    let rows = summary.top(top);
    if rows.is_empty() {
        out.push_str("  (none)\n");
    }
    let width = rows
        .iter()
        .map(|(name, _)| name.chars().count())
        .max()
        .unwrap_or(0);
    for (name, stats) in rows {
        let _ = write!(
            out,
            "  {name:<width$} count={} errors={} mean_latency_ms=",
            stats.count, stats.errors
        );
        match stats.mean_latency() {
            Some(mean) => {
                let _ = write!(out, "{mean:.1}");
            }
            None => out.push('-'),
        }
        out.push_str(" max_latency_ms=");
        match stats.latency_max {
            Some(max) => {
                let _ = writeln!(out, "{max}");
            }
            None => out.push_str("-\n"),
        }
    }
    out
}
