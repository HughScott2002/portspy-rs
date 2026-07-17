//! The one place we talk to C directly.
//!
//! We resolve a numeric user id (like 1000) into a name (like "jr") by calling
//! `getpwuid` from the system C library, and we ask "are we root?" with
//! `geteuid`. These are the classic "read system computing" calls: there is no
//! file that hands you the answer, you ask libc and it consults the password
//! database for you.
//!
//! Everything unsafe about C is contained in THIS file. The rest of the project
//! only sees the two safe functions at the bottom.

use std::ffi::CStr;
use std::os::raw::c_char;

/// Mirror of C's `struct passwd` (from <pwd.h>), field for field and in order.
///
/// `#[repr(C)]` tells Rust to lay this out exactly the way C does, so the
/// pointer libc returns lines up with our fields. We only actually read
/// `pw_name`, but every field must be present so the offsets are correct.
#[repr(C)]
struct Passwd {
    pw_name: *const c_char,   // username
    pw_passwd: *const c_char, // (legacy) password
    pw_uid: u32,              // user id
    pw_gid: u32,              // group id
    pw_gecos: *const c_char,  // full name / comment
    pw_dir: *const c_char,    // home directory
    pw_shell: *const c_char,  // login shell
}

// The actual C functions. `unsafe extern` says: these are foreign, and calling
// them is unsafe because Rust cannot check what C does.
unsafe extern "C" {
    fn getpwuid(uid: u32) -> *const Passwd;
    fn geteuid() -> u32;
}

/// The effective user id of THIS process. 0 means we are running as root.
pub fn current_euid() -> u32 {
    // Reading a uid has no arguments to get wrong; the call is trivially safe.
    unsafe { geteuid() }
}

/// True if we are root. When we are not, some other users' processes will be
/// unreadable, which is why the CLI prints a "try sudo" hint.
pub fn is_root() -> bool {
    current_euid() == 0
}

/// Resolve a numeric uid to a user name, or "" if it cannot be resolved.
///
/// Data flow: `u32`  ->  ask libc  ->  read the C string it points at  ->
/// owned Rust `String`.
pub fn username_for_uid(uid: u32) -> String {
    // SAFETY: getpwuid either returns null or a pointer to a valid Passwd that
    // libc owns. We never write through it and we copy the name out
    // immediately, so the borrow does not outlive the call.
    unsafe {
        let entry = getpwuid(uid);
        if entry.is_null() {
            return String::new();
        }

        let name_ptr = (*entry).pw_name;
        if name_ptr.is_null() {
            return String::new();
        }

        // Wrap the raw C string (nul-terminated) and copy it into a Rust String.
        let name = CStr::from_ptr(name_ptr);
        name.to_string_lossy().into_owned()
    }
}
