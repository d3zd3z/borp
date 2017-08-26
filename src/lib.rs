//! Borp: A rust implementation of Borg

#[macro_use]
extern crate serde_derive;

#[macro_use]
extern crate error_chain;

extern crate hostname;
extern crate libc;
extern crate serde;
extern crate serde_json;

use std::io;
use std::path::PathBuf;

pub mod lock;

error_chain! {
    types {
        Error, ErrorKind, ChainErr, Result;
    }

    foreign_links {
        IoError(io::Error);
        JsonError(serde_json::Error);
    }

    errors {
        LockError(path: PathBuf) {
            description("Unable to acquire lock")
            display("Unable to get lock at {:?}", path)
        }
    }
}
