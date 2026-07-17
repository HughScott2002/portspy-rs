//! One listening socket and the process that owns it.

/// Upper bound on the length of any single text field.
///
/// TigerStyle: put an explicit limit on everything. A process command line can
/// be long; we refuse to carry more than this many bytes per field so a hostile
/// or buggy process cannot make us allocate without bound.
pub const MAX_FIELD_LEN: usize = 512;

/// A `pid` of 0 means "we could not find the owning process". Real process ids
/// start at 1, so 0 is a safe sentinel.
pub const PID_UNKNOWN: i32 = 0;

/// One row of the output: a single listening port and everything we learned
/// about it.
///
/// Every field is owned (`String`, not `&str`) so a `Record` is completely
/// self-contained and easy to pass around, store, and reason about.
#[derive(Clone, Debug, PartialEq)]
pub struct Record {
    /// "tcp" for IPv4, "tcp6" for IPv6.
    pub proto: String,
    /// Human-readable local address, e.g. "0.0.0.0" or "::".
    pub addr: String,
    /// The TCP port that is being listened on.
    pub port: u16,
    /// Socket state. We only ever collect listening sockets, so this is
    /// always "LISTEN", but we keep it explicit rather than implied.
    pub state: String,
    /// Owning process id, or `PID_UNKNOWN` if we could not resolve it.
    pub pid: i32,
    /// Short process name (e.g. "node"), or "" if unknown.
    pub process: String,
    /// Owning user name (e.g. "jr"), or "" if unknown.
    pub user: String,
    /// Full command line, or "" if unknown / not readable.
    pub cmd: String,
}

impl Record {
    /// The exact shell command that would free this port.
    ///
    /// Returned as text so both the terminal table and the browser can show the
    /// same string. If we never found the pid, we say so instead of guessing.
    pub fn kill_command(&self) -> String {
        if self.pid == PID_UNKNOWN {
            return String::from("unknown pid (try sudo)");
        }
        format!("kill {}", self.pid)
    }
}
