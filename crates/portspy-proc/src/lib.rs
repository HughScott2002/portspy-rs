//! portspy-proc
//!
//! The native "agent" that reads the operating system. Its single public
//! function, `collect`, produces the `Vec<Record>` everything else displays.
//!
//! The steps, in order:
//!   1. read the kernel's listening sockets       (net_tcp)
//!   2. build a socket-inode -> pid lookup table   (pid_map)
//!   3. for each socket, attach its process + user (pid_map + ffi)
//!
//! Modules below each own exactly one of those concerns.

mod ffi;
mod net_tcp;
mod pid_map;

use portspy_model::Record;
use portspy_model::record::PID_UNKNOWN;

// Re-export the one helper the CLI needs for its "try sudo" hint.
pub use ffi::is_root;

/// Read every listening TCP port and who owns it.
///
/// Data flow (top to bottom):
///   /proc/net/tcp[6]  ->  raw sockets
///   /proc/*/fd        ->  inode -> pid table
///   join them + libc  ->  Vec<Record>
pub fn collect() -> Vec<Record> {
    // Step 1: raw sockets from both IPv4 and IPv6 tables.
    let mut raw_sockets = Vec::new();
    raw_sockets.extend(read_socket_file("/proc/net/tcp", false));
    raw_sockets.extend(read_socket_file("/proc/net/tcp6", true));

    // Step 2: one scan of /proc gives us inode -> pid for every process we can
    // see. We do this once, not per socket, so it stays cheap.
    let inode_to_pid = pid_map::build_inode_to_pid();

    // Step 3: turn each raw socket into a full Record.
    let mut records = Vec::new();
    for socket in &raw_sockets {
        // Who owns the socket? Look its inode up in the table.
        let pid = match inode_to_pid.get(&socket.inode) {
            Some(pid) => *pid,
            None => PID_UNKNOWN,
        };

        // With a pid we can read the process name and command line.
        // Without one, those stay empty.
        let process;
        let cmd;
        if pid == PID_UNKNOWN {
            process = String::new();
            cmd = String::new();
        } else {
            process = pid_map::process_name(pid);
            cmd = pid_map::command_line(pid);
        }

        // The user comes straight from the socket's uid via libc.
        let user = ffi::username_for_uid(socket.uid);

        let proto = if socket.addr.contains(':') && socket.addr != "?" {
            // IPv6 addresses contain colons (e.g. "::"); IPv4 never does.
            String::from("tcp6")
        } else {
            String::from("tcp")
        };

        records.push(Record {
            proto,
            addr: socket.addr.clone(),
            port: socket.port,
            state: String::from("LISTEN"),
            pid,
            process,
            user,
            cmd,
        });
    }

    records
}

/// Read one /proc socket file and parse it, returning [] if it is missing.
fn read_socket_file(path: &str, is_v6: bool) -> Vec<net_tcp::RawSocket> {
    match std::fs::read_to_string(path) {
        Ok(contents) => net_tcp::parse_listening(&contents, is_v6),
        Err(_) => Vec::new(),
    }
}
