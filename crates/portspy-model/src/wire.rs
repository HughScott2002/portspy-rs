//! The wire format: how records travel from the native agent to the browser.
//!
//! We do NOT use JSON. A hand-written JSON parser is fiddly and easy to get
//! wrong. Instead we use a format you can read at a glance:
//!
//!   * one record per line   (records separated by '\n')
//!   * one field per column   (fields separated by '\t')
//!   * always exactly FIELD_COUNT columns, in a fixed order
//!
//! Field order (index in the line):
//!   0 proto   1 addr   2 port   3 state   4 pid   5 process   6 user   7 cmd
//!
//! Both sides of the wire use THIS file: the native server encodes, and the
//! wasm module decodes / re-encodes. Same code, two targets.

use crate::record::{MAX_FIELD_LEN, Record};

/// Number of columns in every encoded line. Kept in one place so encode and
/// decode can never disagree.
pub const FIELD_COUNT: usize = 8;

const FIELD_SEPARATOR: char = '\t';
const RECORD_SEPARATOR: char = '\n';

/// Turn a list of records into one text blob.
///
/// Data flow: `&[Record]`  ->  one String, ready to send over HTTP.
pub fn encode(records: &[Record]) -> String {
    let mut out = String::new();

    for (index, record) in records.iter().enumerate() {
        // Separate records with a newline, but not before the very first one.
        if index > 0 {
            out.push(RECORD_SEPARATOR);
        }

        // Build this line field by field, in the fixed order documented above.
        push_field(&mut out, &record.proto);
        out.push(FIELD_SEPARATOR);
        push_field(&mut out, &record.addr);
        out.push(FIELD_SEPARATOR);
        push_field(&mut out, &record.port.to_string());
        out.push(FIELD_SEPARATOR);
        push_field(&mut out, &record.state);
        out.push(FIELD_SEPARATOR);
        push_field(&mut out, &record.pid.to_string());
        out.push(FIELD_SEPARATOR);
        push_field(&mut out, &record.process);
        out.push(FIELD_SEPARATOR);
        push_field(&mut out, &record.user);
        out.push(FIELD_SEPARATOR);
        push_field(&mut out, &record.cmd);
    }

    out
}

/// Turn a text blob back into records.
///
/// Data flow: one String  ->  `Vec<Record>`. Lines that do not have exactly
/// FIELD_COUNT columns are skipped rather than trusted.
pub fn decode(text: &str) -> Vec<Record> {
    let mut records: Vec<Record> = Vec::new();

    for line in text.split(RECORD_SEPARATOR) {
        // An empty line carries no record; ignore it.
        if line.is_empty() {
            continue;
        }

        // Split into columns. We collect first so we can check the count
        // explicitly before trusting any single field.
        let fields: Vec<&str> = line.split(FIELD_SEPARATOR).collect();
        if fields.len() != FIELD_COUNT {
            // Malformed line. Skip it instead of guessing.
            continue;
        }

        let record = Record {
            proto: fields[0].to_string(),
            addr: fields[1].to_string(),
            port: parse_u16(fields[2]),
            state: fields[3].to_string(),
            pid: parse_i32(fields[4]),
            process: fields[5].to_string(),
            user: fields[6].to_string(),
            cmd: fields[7].to_string(),
        };
        records.push(record);
    }

    records
}

/// Append one field, making it safe for the format.
///
/// The separators are structural, so a field is not allowed to contain them.
/// We also cap the length. Both are silent repairs, not errors: a weird command
/// line should never be able to corrupt the whole stream.
fn push_field(out: &mut String, value: &str) {
    let mut written = 0usize;

    for character in value.chars() {
        if written >= MAX_FIELD_LEN {
            break;
        }

        // Replace anything that would break the framing with a plain space.
        let safe = match character {
            '\t' | '\n' | '\r' => ' ',
            other => other,
        };

        out.push(safe);
        written += 1;
    }
}

/// Parse a port. On any garbage, fall back to 0 rather than panicking.
fn parse_u16(text: &str) -> u16 {
    match text.parse::<u16>() {
        Ok(value) => value,
        Err(_) => 0,
    }
}

/// Parse a pid. On any garbage, fall back to 0 (PID_UNKNOWN).
fn parse_i32(text: &str) -> i32 {
    match text.parse::<i32>() {
        Ok(value) => value,
        Err(_) => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*; // pull in everything from this file, including private fns
    // fn test_encode() {
    //     let record = Record {
    //         proto: "tcp".to_string(),
    //         addr: "127.0.0.54".to_string(),
    //         port: parse_u16("43"),
    //         state: "active".to_string(),
    //         pid: parse_i32("2039"),
    //         process: "".to_string(),
    //         user: "root".to_string(),
    //         cmd: "asd".to_string(),
    //     };
    //     assert_eq!(record, "")
    // }

    #[test]
    fn test_decode() {
        assert_eq!(decode(""), vec![])
    }

    #[test]
    fn test_parse_i32() {
        assert_eq!(parse_i32("2"), 2);
        assert_eq!(parse_i32("-2"), -2);
    }
}
