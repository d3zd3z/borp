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

fn main() {
    let pid = borp::lock::get_process_id();
    println!("pid: {:}", serde_json::to_string(&pid).unwrap());
    println!("pid: {:}", pid.to_filename());
}
