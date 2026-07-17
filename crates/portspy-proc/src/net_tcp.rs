//! Parse the kernel's list of TCP sockets from /proc/net/tcp and tcp6.
//!
//! The kernel exposes every TCP socket as a line of text. A trimmed example
//! (IPv4, /proc/net/tcp):
//!
//!   sl  local_address rem_address   st ... uid ...  inode
//!    0: 0100007F:1538 00000000:0000 0A ... 1000 ... 45678
//!
//! We care about four columns:
//!   local_address -> "IP:PORT", both hex, IP in little-endian byte order
//!   st            -> socket state, hex. "0A" (== 10) means LISTEN.
//!   uid           -> the owning user id
//!   inode         -> a number that also appears under /proc/<pid>/fd, which is
//!                    how we later connect a socket to its process.

/// The hex state code for a listening socket. We only keep these.
const STATE_LISTEN: &str = "0A";

/// A raw socket parsed straight from /proc, before we know its process.
pub struct RawSocket {
    pub addr: String, // human-readable local IP
    pub port: u16,    // local port
    pub uid: u32,     // owning user id
    pub inode: u64,   // socket inode, links to /proc/<pid>/fd
}

/// Parse the full text of one /proc/net/tcp[6] file into listening sockets.
///
/// `is_v6` selects how to decode the address column (4 bytes vs 16 bytes).
/// Data flow: file text  ->  `Vec<RawSocket>` (LISTEN sockets only).
pub fn parse_listening(contents: &str, is_v6: bool) -> Vec<RawSocket> {
    let mut sockets = Vec::new();

    for (line_number, line) in contents.lines().enumerate() {
        // The very first line is the column header; skip it.
        if line_number == 0 {
            continue;
        }

        // Columns are separated by runs of spaces.
        let columns: Vec<&str> = line.split_whitespace().collect();

        // We need at least up to the inode column (index 9) to proceed.
        if columns.len() < 10 {
            continue;
        }

        let local = columns[1]; // "IP:PORT" in hex
        let state = columns[3]; // socket state in hex
        let uid_text = columns[7];
        let inode_text = columns[9];

        // Keep only listening sockets.
        if state != STATE_LISTEN {
            continue;
        }

        // "IP:PORT" -> split on the single ':'.
        let (ip_hex, port_hex) = match local.split_once(':') {
            Some(parts) => parts,
            None => continue,
        };

        let port = match u16::from_str_radix(port_hex, 16) {
            Ok(value) => value,
            Err(_) => continue,
        };

        let addr = if is_v6 {
            decode_ipv6(ip_hex)
        } else {
            decode_ipv4(ip_hex)
        };

        let uid = uid_text.parse::<u32>().unwrap_or(0);
        let inode = inode_text.parse::<u64>().unwrap_or(0);

        sockets.push(RawSocket {
            addr,
            port,
            uid,
            inode,
        });
    }

    sockets
}

/// Decode an IPv4 address written as 8 hex chars, little-endian.
///
/// "0100007F" is the four bytes 01 00 00 7F, least-significant first, so the
/// real address is 127.0.0.1. We read the bytes back-to-front and print them.
fn decode_ipv4(ip_hex: &str) -> String {
    if ip_hex.len() != 8 {
        return String::from("?");
    }

    let mut bytes = [0u8; 4];
    for i in 0..4 {
        let start = i * 2;
        let byte_hex = &ip_hex[start..start + 2];
        bytes[i] = u8::from_str_radix(byte_hex, 16).unwrap_or(0);
    }

    // Little-endian: byte 0 is the LAST octet, so reverse when printing.
    format!("{}.{}.{}.{}", bytes[3], bytes[2], bytes[1], bytes[0])
}

/// Decode an IPv6 address written as 32 hex chars.
///
/// Full IPv6 formatting is fiddly; for a port inspector the common cases are
/// what matter. We special-case the all-zeros "listen on everything" address
/// as "::", and otherwise show the eight 16-bit groups.
fn decode_ipv6(ip_hex: &str) -> String {
    if ip_hex.len() != 32 {
        return String::from("?");
    }

    // All zeros means "any address".
    let all_zero = ip_hex.chars().all(|c| c == '0');
    if all_zero {
        return String::from("::");
    }

    // Otherwise group into 8 blocks of 4 hex chars for a readable-enough form.
    let mut groups: Vec<&str> = Vec::new();
    for i in 0..8 {
        let start = i * 4;
        groups.push(&ip_hex[start..start + 4]);
    }
    groups.join(":")
}
