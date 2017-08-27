//! Borp: A rust implementation of Borg

#[macro_use]
extern crate serde_derive;

#[macro_use]
extern crate error_chain;

// TODO: Perhaps make this its own crate so we don't pollute all of the namespace with these
// macros.
#[macro_use]
extern crate nom;

extern crate data_encoding;
extern crate hostname;
extern crate libc;
extern crate serde;
extern crate serde_json;

use std::io;
use std::path::PathBuf;

pub mod config;
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
