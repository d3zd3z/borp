//! Borp: A re-implementation of Borg in Rust
//!
//! There are several purposes here.  One is to have a project of reasonable complexity to work on
//! in Rust.  Another is to understand how Borg works better.  Ultimately, this may become a useful
//! alternative implementation itself.
//!
//! But, to start with, we're going to just try decoding the borg backups themselves.

extern crate serde_json;
extern crate borp;

// Things that will need to be implemented:
// - [ ] Lock
// - [ ] Config file
// - [ ] Repository
// - [ ] LoggedIO
// - [ ] Index files
// - [ ] Manifest

use borp::lock::Lock;
use std::path::Path;

fn main() {
    let mut lock = Lock::new(Path::new(".").to_path_buf(), "lock".to_string());
    lock.lock_shared().unwrap();
    println!("lock: {:?}", lock);
    {
        use std::process::Command;
        Command::new("cat").arg("lock.roster").status().unwrap();
        Command::new("echo").status().unwrap();
    }
}
