//! Connect a socket inode to the process that owns it.
//!
//! /proc/net/tcp gave us a socket's inode number, but not its process. The link
//! lives under each process: /proc/<pid>/fd/ holds one symlink per open file
//! descriptor, and a socket's symlink points at the text "socket:[<inode>]".
//!
//! So the plan is:
//!   1. walk every /proc/<pid>/fd/ directory,
//!   2. read each symlink, and if it is a socket, remember inode -> pid,
//!   3. hand back a lookup table.
//!
//! Directories we are not allowed to read (other users' processes when we are
//! not root) are skipped silently; that just means some ports show no owner.

use std::collections::HashMap;
use std::fs;

/// TigerStyle: bound the walk. A machine should never have anywhere near this
/// many processes, and if something is wrong we stop instead of spinning.
const MAX_PROCESSES: usize = 65_536;

/// Bound the file descriptors we inspect per process for the same reason.
const MAX_FDS_PER_PROCESS: usize = 65_536;

/// A ready-to-query map from socket inode to owning pid.
pub type InodeToPid = HashMap<u64, i32>;

/// Build the inode -> pid table by scanning /proc.
///
/// Data flow: the /proc filesystem  ->  a HashMap you can look inodes up in.
pub fn build_inode_to_pid() -> InodeToPid {
    let mut table: InodeToPid = HashMap::new();

    let entries = match fs::read_dir("/proc") {
        Ok(entries) => entries,
        // If /proc is missing we simply return an empty table.
        Err(_) => return table,
    };

    let mut processes_seen = 0usize;

    for entry in entries {
        if processes_seen >= MAX_PROCESSES {
            break;
        }

        let entry = match entry {
            Ok(entry) => entry,
            Err(_) => continue,
        };

        // Directory names under /proc are many things; the numeric ones are
        // process ids. Anything non-numeric (like "net" or "self") we skip.
        let file_name = entry.file_name();
        let name = match file_name.to_str() {
            Some(name) => name,
            None => continue,
        };
        let pid = match name.parse::<i32>() {
            Ok(pid) => pid,
            Err(_) => continue,
        };

        processes_seen += 1;
        scan_process_fds(pid, &mut table);
    }

    table
}

/// Look at one process's open descriptors and record any sockets it owns.
fn scan_process_fds(pid: i32, table: &mut InodeToPid) {
    let fd_dir = format!("/proc/{}/fd", pid);

    let fds = match fs::read_dir(&fd_dir) {
        Ok(fds) => fds,
        // Permission denied (not our process) or the process just exited.
        Err(_) => return,
    };

    let mut fds_seen = 0usize;

    for fd in fds {
        if fds_seen >= MAX_FDS_PER_PROCESS {
            break;
        }
        fds_seen += 1;

        let fd = match fd {
            Ok(fd) => fd,
            Err(_) => continue,
        };

        // Each fd is a symlink. Reading it tells us what the fd points at.
        let target = match fs::read_link(fd.path()) {
            Ok(target) => target,
            Err(_) => continue,
        };
        let target = target.to_string_lossy();

        // Sockets look like "socket:[12345]". Pull the number out of the middle.
        if let Some(inode) = parse_socket_inode(&target) {
            // First writer wins; that is fine, any owning pid is useful.
            table.entry(inode).or_insert(pid);
        }
    }
}

/// Extract the inode from a symlink target like "socket:[12345]".
///
/// Returns None for any target that is not a socket.
fn parse_socket_inode(target: &str) -> Option<u64> {
    let inside = target.strip_prefix("socket:[")?;
    let number = inside.strip_suffix(']')?;
    number.parse::<u64>().ok()
}

/// The short process name from /proc/<pid>/comm, or "" if unreadable.
pub fn process_name(pid: i32) -> String {
    let path = format!("/proc/{}/comm", pid);
    match fs::read_to_string(&path) {
        Ok(text) => text.trim_end().to_string(),
        Err(_) => String::new(),
    }
}

/// The full command line from /proc/<pid>/cmdline, or "" if unreadable.
///
/// The kernel separates arguments with NUL bytes; we turn those into spaces so
/// the result reads like the command you typed.
pub fn command_line(pid: i32) -> String {
    let path = format!("/proc/{}/cmdline", pid);

    let bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(_) => return String::new(),
    };

    let mut text = String::new();
    for byte in bytes {
        if byte == 0 {
            text.push(' ');
        } else {
            text.push(byte as char);
        }
    }

    text.trim().to_string()
}
