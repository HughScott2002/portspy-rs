//! Sorting and filtering. Pure functions over a list of records.
//!
//! This is the logic the browser wants: the user types in a search box or
//! clicks a column header, and the same Rust code that could run natively runs
//! instead inside wasm to reshape the table. No I/O, so it compiles anywhere.

use crate::record::Record;

/// Which column to sort by.
///
/// Represented as a plain enum here, but it crosses the wasm boundary as a
/// small integer (see `from_u32`), because that boundary can only pass numbers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SortKey {
    Port,
    Pid,
    Process,
    User,
}

impl SortKey {
    /// Decode the integer the browser sends. Anything unexpected falls back to
    /// sorting by port, which is the most useful default.
    pub fn from_u32(value: u32) -> SortKey {
        match value {
            0 => SortKey::Port,
            1 => SortKey::Pid,
            2 => SortKey::Process,
            3 => SortKey::User,
            _ => SortKey::Port,
        }
    }
}

/// Sort records in place by the chosen column, ascending.
///
/// Data flow: `&mut Vec<Record>` is reordered; nothing is added or removed.
pub fn sort(records: &mut [Record], key: SortKey) {
    records.sort_by(|left, right| match key {
        SortKey::Port => left.port.cmp(&right.port),
        SortKey::Pid => left.pid.cmp(&right.pid),
        SortKey::Process => left.process.cmp(&right.process),
        SortKey::User => left.user.cmp(&right.user),
    });
}

/// Keep only the records that mention `needle` somewhere.
///
/// An empty needle keeps everything. Matching is case-insensitive and checks
/// every visible field, so typing "node" or "3000" both work.
///
/// Data flow: `&[Record]` in  ->  a NEW (smaller) `Vec<Record>` out. The input
/// is left untouched, which keeps the "raw" list intact for the next search.
pub fn filter(records: &[Record], needle: &str) -> Vec<Record> {
    if needle.is_empty() {
        return records.to_vec();
    }

    let needle_lower = needle.to_lowercase();
    let mut kept = Vec::new();

    for record in records {
        if record_matches(record, &needle_lower) {
            kept.push(record.clone());
        }
    }

    kept
}

/// True if any field of `record` contains `needle_lower` (already lowercased).
fn record_matches(record: &Record, needle_lower: &str) -> bool {
    // Build a lowercase haystack of every field, then do one search. Explicit
    // and easy to extend if we add columns later.
    let mut haystack = String::new();
    haystack.push_str(&record.proto.to_lowercase());
    haystack.push(' ');
    haystack.push_str(&record.addr.to_lowercase());
    haystack.push(' ');
    haystack.push_str(&record.port.to_string());
    haystack.push(' ');
    haystack.push_str(&record.pid.to_string());
    haystack.push(' ');
    haystack.push_str(&record.process.to_lowercase());
    haystack.push(' ');
    haystack.push_str(&record.user.to_lowercase());
    haystack.push(' ');
    haystack.push_str(&record.cmd.to_lowercase());

    haystack.contains(needle_lower)
}
