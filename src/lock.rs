//! Borg locks.
//!
//! Borg uses a filesystem-based locking mechanism that ultimately relies on the atomic nature of
//! 'mkdir'.  There is a bit more complexity beyond this that allows a directory to hold shared
//! read-only locks, but that can be upgraded to an exclusive lock if possible.

use hostname;
use libc;
use serde_json;
use std::fs::{self, File, OpenOptions};
use std::io;
use std::mem;
use std::path::{Path, PathBuf};

use {Error, ErrorKind, Result};

/// Locks start out `Unlocked`.  They can then be transitioned through the states of
/// `Unlocked`, `Shared`, and `Exclusive`.
pub enum State {
    Unlocked, Shared, Exclusive,
}

/// An exclusive lock is a directory-based lock.  There is a file within the directory that is used
/// to help identify the creator of the lock, but it isn't itself necessary for the semantics of
/// the lock.
#[derive(Debug)]
pub struct ExclusiveLock {
    /// The pathname of the directory holding the lock.
    dir: PathBuf,
    /// The filename within this directory identifying the lock.
    file: PathBuf,
}

impl ExclusiveLock {
    /// Create a new exclusive lock, returning the Ok(lock) if it could be aquired.  Otherwise, it
    /// return Err to indicate that the lock could not be created.  `path` should be the directory
    /// name of the lock.
    /// TODO: This should have a small timeout with retries.
    pub fn new(dir: PathBuf) -> Result<ExclusiveLock> {
        let file = dir.join(get_process_id().to_filename());

        match fs::create_dir(&dir) {
            Ok(()) => (),
            Err(ref e) if e.kind() == io::ErrorKind::AlreadyExists => {
                return Err(Error::from_kind(ErrorKind::LockError(dir)));
            }
            Err(e) => return Err(e.into()),
        }

        // Create the lock at this point, so that it will be removed if there is a problem creating
        // the file within it.
        let el = ExclusiveLock {
            dir: dir,
            file: file,
        };

        // Make the informative file so to help identify the lock.
        let _ = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&el.file)?;

        Ok(el)
    }
}

impl Drop for ExclusiveLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.file);
        let _ = fs::remove_dir(&self.dir);
    }
}

/// The lock roster type itself.
#[derive(Serialize, Deserialize, Debug)]
enum Roster {
    /// There are no locks in the roster.  Never serialized as such, and is indicated by the file
    /// being missing entirely.
    Empty,
    /// Indicates a shared lock, along with the holders of the lock.
    #[serde(rename = "shared")]
    Shared(Vec<ProcessId>),
    /// Indicates an exclusive lock, along with the holders of the lock (should only be one).
    #[serde(rename = "exclusive")]
    Exclusive(Vec<ProcessId>),
}

impl Roster {
    /// Try loading a roster from a file.  The ordinary error of no such file will result in an
    /// empty roster, but other errors will be kept.
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Roster> {
        let file = match File::open(path) {
            Ok(f) => f,
            Err(ref e) if e.kind() == io::ErrorKind::NotFound => {
                return Ok(Roster::Empty);
            }
            Err(e) => return Err(e.into()),
        };

        Ok(serde_json::from_reader(file)?)
    }

    /// Attempt to convert this roster to an exclusive lock.  If possible, writes the new roster
    /// out.  This should only be done when the surrounding exclusive lock is already taken.
    pub fn make_exclusive<P: AsRef<Path>>(&mut self, id: ProcessId, path: P) -> Result<()> {
        match *self {
            Roster::Empty => (),
            _ => return Err(ErrorKind::LockError(path.as_ref().to_path_buf()).into()),
        }

        *self = Roster::Exclusive(vec![id]);
        self.update(path)
    }

    /// Update the roster file with the current state.
    pub fn update<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        match *self {
            Roster::Empty => {
                match fs::remove_file(path) {
                    Ok(()) => (),
                    Err(ref e) if e.kind() == io::ErrorKind::NotFound => (),
                    Err(e) => return Err(e.into()),
                }
            }
            _ => {
                let file = File::create(path)?;
                serde_json::to_writer(file, self)?;
            }
        }
        Ok(())
    }
}

/// A general lock.
#[derive(Debug)]
pub struct Lock {
    /// The directory containing the lock.
    path: PathBuf,
    /// The prefix of a file/directory name.  For example "lock" will create an exclusive lock of
    /// "lock.exclusive" and a roster of "lock.roster".
    base: String,

    /// The exclusive lock, if it is held.
    exclusive: Option<ExclusiveLock>,

    /// Cached version of our ID.  Kept so it doesn't change.
    id: ProcessId,

    /// The roster.
    roster: Roster,
}

impl Lock {
    /// Construct a new lock in the given directory, with the given base (prefix) for the name.
    /// The lock starts unlocked, so no operations are performed that could fail.
    pub fn new(path: PathBuf, base: String) -> Lock {
        Lock {
            path: path,
            base: base,
            exclusive: None,
            id: get_process_id(),
            roster: Roster::Empty,
        }
    }

    /// Try to aquire an exclusive lock, returning Ok if this is possible.  Otherwise will return
    /// Err of a LockError if not possible.
    pub fn lock_exclusive(&mut self) -> Result<()> {
        if self.exclusive.is_some() {
            panic!("Use error, attempt to aquire multiple locks");
        }

        let el = ExclusiveLock::new(self.exclusive_name())?;

        let rname = self.roster_name();
        self.roster = Roster::load(&rname)?;
        self.roster.make_exclusive(self.id.clone(), &rname)?;

        self.exclusive = Some(el);
        Ok(())
    }

    /// Release the current lock.
    pub fn release(&mut self) -> Result<()> {
        let rost = mem::replace(&mut self.roster, Roster::Empty);
        match rost {
            Roster::Empty => return Ok(()),
            Roster::Exclusive(_) => Roster::Empty.update(self.roster_name()),
            Roster::Shared(_) => unimplemented!(),
        }
    }

    /// Get the path name for the exclusive lock.
    fn exclusive_name(&self) -> PathBuf {
        self.path.join(format!("{}.exclusive", self.base))
    }

    /// Get the filename of the roster file.
    fn roster_name(&self) -> PathBuf {
        self.path.join(format!("{}.roster", self.base))
    }
}

impl Drop for Lock {
    fn drop(&mut self) {
        let _ = self.release();
    }
}

// Grumble, getpid (and Pid) from nix is useless, because it provides no way of getting the pid out
// of the field.  So, let's do our own wrapper.
fn getpid() -> i32 {
    unsafe { libc::getpid() }
}

/// An identifier for the current process.  The tuple consists of the hostname, pid, and a
/// thread-id (which is zero currently).
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
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
