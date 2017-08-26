//! Borg locks.
//!
//! Borg uses a filesystem-based locking mechanism that ultimately relies on the atomic nature of
//! 'mkdir'.  There is a bit more complexity beyond this that allows a directory to hold shared
//! read-only locks, but that can be upgraded to an exclusive lock if possible.

use hostname;
use libc;

// Grumble, getpid (and Pid) from nix is useless, because it provides no way of getting the pid out
// of the field.  So, let's do our own wrapper.
fn getpid() -> i32 {
    unsafe { libc::getpid() }
}

/// An identifier for the current process.  The tuple consists of the hostname, pid, and a
/// thread-id (which is zero currently).
#[derive(Serialize, Deserialize, Debug)]
pub struct ProcessId(pub String, pub i32, pub i32);

impl ProcessId {
    /// Generate a string representation of this `ProcessId` suitable for use as a filename.
    pub fn to_filename(&self) -> String {
        format!("{}.{}-{}", self.0, self.1, self.2)
    }
}

/// Get the ProcessId identifier for the current process, used to identify locks.  The tuple
/// consists of the current hostname, the current process id, and the current thread ID (which is
/// always zero).  Currently, this will panic if the hostname cannot be retrieved.
pub fn get_process_id() -> ProcessId {
    let host = hostname::get_hostname().expect("Getting current hostname");
    let pid = getpid();

    ProcessId(host, pid, 0)
}
